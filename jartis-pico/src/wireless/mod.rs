use cortex_m::delay::Delay;
use defmt::*;
use embassy_executor::Spawner;
use embedded_hal::digital::OutputPin;
use panic_probe as _;
use pio::pio_asm;
use rp_pico::hal::gpio::FunctionSioOutput;
use rp_pico::hal::pio::PinState;
use rp_pico_w::Pins;
use rp_pico_w::hal;
use rp_pico_w::hal::clocks::ClocksManager;
use rp_pico_w::hal::dma::DMAExt;
use rp_pico_w::hal::gpio;
use rp_pico_w::hal::gpio::{Pin, PullUp};
use rp_pico_w::hal::pio::PIOExt;
use rp_pico_w::pac::DMA;
use rp_pico_w::pac::{PIO0, RESETS, SPI0};

use pio_spi::PioSpiCyw43;

use embassy_rp as _;

pub mod embassy_timer_driver;
mod pio_spi;

#[embassy_executor::task(pool_size = 1)]
async fn cyw43_task(
    runner: cyw43::Runner<
        'static,
        Pin<gpio::bank0::Gpio23, FunctionSioOutput, PullUp>,
        PioSpiCyw43,
    >,
) -> ! {
    debug!("running task");
    runner.run().await
}

#[embassy_executor::task(pool_size = 1)]
pub async fn wireless_main(
    spawner: Spawner,
    mut resets: RESETS,
    clocks: ClocksManager,
    pins: Pins,
    spi0: SPI0,
    pio0: PIO0,
    dma: DMA,
    mut delay: Delay,
    state: &'static mut cyw43::State,
) {
    //
    // Configure pins fro PioSpi
    //

    let mut led_pin = pins.gpio14.into_push_pull_output();
    led_pin.set_interrupt_enabled(gpio::Interrupt::EdgeLow, true); // Remove this
    led_pin.set_low().unwrap();

    // Define the CYW43 program, taken from cyw43-pio
    let default_program = pio_asm!(
        ".side_set 1"

        ".wrap_target"
        // write out x-1 bits
        "lp:"
        "out pins, 1    side 0"
        "jmp x-- lp     side 1"
        // switch directions
        "set pindirs, 0 side 0"
        "nop            side 0"
        // read in y-1 bits
        "lp2:"
        "in pins, 1     side 1"
        "jmp y-- lp2    side 0"

        // wait for event and irq host
        "wait 1 pin 0   side 0"
        "irq 0          side 0"

        ".wrap"
    );

    // .program spi_gap0_sample1
    let c_program = pio_asm!(
        ".side_set 1"

        ".wrap_target"
            "nop side 0 [1]" // DEBUG
            // always transmit multiple of 32 bytes
            // write out x-1 bits
            "lp:",
            "nop side 0 [1]" // DEBUG
            "out pins, 1             side 0 [0]" // CYW43 might be fast, use an extra delay cycle
            "jmp x-- lp              side 1"
            "public lp1_end:"
            // switch directions
            "set pindirs, 0          side 0"
            // read in y-1 bits
            "lp2:"
            "nop side 0 [1]" // DEBUG
            "in pins, 1              side 1 [1]" // CYW43 might be fast, use an extra delay cycle
            "jmp y-- lp2             side 0"
            "wait 1 pin 0            side 0" // TODO: Delete?
            "irq 0                   side 0" // TODO: Delete?
            "public end:"
        ".wrap"
    );

    // From the Pico W datasheet:
    // GPIO29 OP/IP wireless SPI CLK/ADC mode (ADC3) to measure VSYS/3
    // GPIO25 OP wireless SPI CS - when high also enables GPIO29 ADC pin to read VSYS
    // GPIO24 OP/IP wireless SPI data/IRQ
    // GPIO23 OP wireless power on signal

    // Copied logic from cyw43-pio, but using rp2040-hal pins

    // Set up our PIO pins into the correct mode. Look at cyw43_spi_gpio_setup in the C SDK for
    // reference

    // NOTE: This needs to start as high, as the CYW43 bus turns out low and then high
    let pwr: Pin<_, FunctionSioOutput, PullUp> = pins
        .wl_on
        .into_push_pull_output_in_state(embedded_hal::digital::PinState::Low)
        .reconfigure(); // GPIO23

    // This pin needs to start in a low power state
    let mut spi_sclk: Pin<_, gpio::FunctionPio0, gpio::PullDown> = pins
        .voltage_monitor_wl_clk // GPIO29
        .into_push_pull_output_in_state(embedded_hal::digital::PinState::Low)
        .reconfigure();
    spi_sclk.set_drive_strength(gpio::OutputDriveStrength::TwelveMilliAmps); // From cyw43-pio
    spi_sclk.set_slew_rate(gpio::OutputSlewRate::Fast); // From cyw43-pio
    let spi_sclk_id = spi_sclk.id().num;

    // This pin needs to start in a low power state
    // Setup IRQ (24) - also used for DO, DI
    let mut spi_mosi_miso = pins.wl_data.into_floating_input();
    spi_mosi_miso.set_sync_bypass(true);
    let mut spi_mosi_miso: gpio::Pin<_, gpio::FunctionPio0, gpio::PullNone> = spi_mosi_miso
        .into_push_pull_output_in_state(embedded_hal::digital::PinState::Low)
        .reconfigure();
    spi_mosi_miso.set_schmitt_enabled(true);
    spi_mosi_miso.set_drive_strength(gpio::OutputDriveStrength::TwelveMilliAmps); // From cyw43-pio
    spi_mosi_miso.set_slew_rate(gpio::OutputSlewRate::Fast); // From cyw43-pio
    let spi_mosi_miso_id = spi_mosi_miso.id().num;

    // SPI CS (Chip select) - This needs to be high for the CYW43 driver to work
    let spi_cs: Pin<_, gpio::FunctionSioOutput, gpio::PullNone> = pins
        .wl_cs
        .into_push_pull_output_in_state(embedded_hal::digital::PinState::Low)
        .reconfigure(); // GPIO25

    // Initialize and start PIO
    let (mut pio, sm0, _, _, _) = pio0.split(&mut resets);
    let installed = pio.install(&c_program.program).unwrap();
    let wrap_target = installed.wrap_target();

    // Taken from the Pico C SDK in the CYW43 PIO SPI bus
    const CYW43_PIO_CLOCK_DIV_INT: u16 = 2;
    const CYW43_PIO_CLOCK_DIV_FRAC: u8 = 0; // as slow as possible (0 is interpreted as 65536). Taken from the
    let (int, frac) = (CYW43_PIO_CLOCK_DIV_INT, CYW43_PIO_CLOCK_DIV_FRAC);

    // Match cwy43-pio's pins, shift and clock divider configuration
    let (mut sm, rx, tx) = hal::pio::PIOBuilder::from_installed_program(installed)
        .out_pins(spi_mosi_miso_id, 1)
        .in_pin_base(spi_mosi_miso_id)
        .set_pins(spi_mosi_miso_id, 1)
        .side_set_pin_base(spi_sclk_id) // TODO: Review if needed
        .out_shift_direction(hal::pio::ShiftDirection::Left) // Look at the sm_config_set_in_shift C call where it specifies shifting both in and out to the left
        .in_shift_direction(hal::pio::ShiftDirection::Left)
        .pull_threshold(32) // Reading 32-bit words
        .push_threshold(32) // Writing 32-bit words
        .autopush(true) // Matching embassy's shift_in.auto_fill = true
        .autopull(true) // Matching embassy's shift_out.auto_fill = true
        .clock_divisor_fixed_point(int, frac)
        .build(sm0);

    // The GPIO pins need to be configured as outputs
    sm.set_pindirs([
        (spi_mosi_miso_id, hal::pio::PinDir::Output),
        (spi_sclk_id, hal::pio::PinDir::Output),
    ]);
    sm.set_pins([
        (spi_mosi_miso_id, PinState::Low),
        (spi_sclk_id, PinState::Low),
    ]);

    // Set up DMA
    let dma = dma.split(&mut resets);

    info!("creating SPI wrapper");
    let spi_wrapper = PioSpiCyw43::new(sm, spi_cs, tx, rx, wrap_target, dma);

    let cyw43_firmware = include_bytes!("../../../cyw43/43439A0.bin");
    let clm = include_bytes!("../../../cyw43/43439A0_clm.bin");

    embassy_futures::yield_now().await;
    info!("yielded");

    let (_net_device, mut control, runner) =
        cyw43::new(state, pwr, spi_wrapper, cyw43_firmware).await;
    info!("initialised cyw43");
    info!("spawning task");
    unwrap!(spawner.spawn(cyw43_task(runner)));
    info!("initialising control");
    control.init(clm).await;
    core::panic!("hehehehe");
    info!("initialised control");
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;
    info!("set control power");

    loop {
        // delay.delay_ms(250);
        info!("led on!");
        control.gpio_set(0, true).await;
        delay.delay_ms(1000);

        info!("led off!");
        control.gpio_set(0, false).await;
        delay.delay_ms(1000);
    }
}
