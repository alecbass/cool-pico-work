use cortex_m::delay::Delay;
use cyw43::SpiBusCyw43;
use defmt::*;
use embassy_executor::Spawner;
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::SpiBus;
use fugit::RateExtU32;
use jartis::uart::{Uart, UartPins};
use panic_probe as _;
use pio::pio_asm;
use rp_pico::hal::dma::Word;
use rp_pico::hal::pio::SM0;
use rp_pico::Pins;
use rp_pico::hal;
use rp_pico::hal::Clock;
use rp_pico::hal::clocks::ClocksManager;
use rp_pico::hal::gpio;
use rp_pico::hal::gpio::FunctionPio0;
use rp_pico::hal::gpio::bank0::Gpio23;
use rp_pico::hal::gpio::{FunctionSioOutput, Pin, PullDown, PullUp};
use rp_pico::hal::gpio::{
    FunctionUart, PullNone,
    bank0::{Gpio0, Gpio1},
};
use rp_pico::hal::pio::PIOExt;
use rp_pico::hal::prelude::*;
use rp_pico::hal::spi::{self, ValidSpiPinout};
use rp_pico::hal::uart::UartPeripheral;
use rp_pico::hal::uart::{DataBits, StopBits, UartConfig};
use rp_pico::pac::{PIO0, RESETS, SPI0, UART0};

use embassy_rp as _;

// bind_interrupts!(struct Irqs {
//     PIO0_IRQ_0 => InterruptHandler<PIO0>;
// });

// unsafe extern "C" {
//     fn connectToWifi() -> core::ffi::c_int;
// }

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<
        'static,
        Pin<Gpio23, FunctionSioOutput, PullDown>,
        CustomSpiWrapper<
            SPI0,
            (
                // Hardcoded pins as embassy_executor::task does not support generics, sadly
                Pin<gpio::bank0::Gpio7, gpio::FunctionSpi, PullNone>,
                Pin<gpio::bank0::Gpio16, gpio::FunctionSpi, PullUp>,
                Pin<gpio::bank0::Gpio22, gpio::FunctionSpi, PullNone>,
            ),
        >,
    >,
) -> ! {
    runner.run().await
}

/// Wrapper for the SPI bus that implements the `SpiBusCyw43`
/// This is only its own struct due to orphan implementation rules
pub struct CustomSpiWrapper<D: spi::SpiDevice, P: spi::ValidSpiPinout<D>> {
    spi: spi::Spi<spi::Enabled, D, P, 8>,
    sm: hal::pio::StateMachine<(PIO0, SM0), hal::pio::Stopped>
    cs: Pin<gpio::bank0::Gpio19, FunctionSioOutput, PullDown>,
    rx: hal::pio::Rx<(PIO0, SM0), Word>,
    tx: hal::pio::Tx<(PIO0, SM0), Word>,
}

impl<D, P> CustomSpiWrapper<D, P> 
where D: spi::SpiDevice, P: spi::ValidSpiPinout<D>
{
    /// Write data to peripheral and return status.
    pub async fn write(&mut self, write: &[u32]) -> u32 {
        // NOTE: This is copied directly from cyw43-pio;s PioSpi implementation
        self.sm.set_enable(false);
        let write_bits = write.len() * 32 - 1;
        let read_bits = 31;

        trace!("write={} read={}", write_bits, read_bits);

        unsafe {
            self.sm.set_x(write_bits as u32);
            self.sm.set_y(read_bits as u32);
            self.sm.set_pindir(0b1);
            self.sm.exec_jmp(self.wrap_target);
        }

        self.sm.set_enable(true);

        self.sm.tx().dma_push(self.dma.reborrow(), write, false).await;

        let mut status = 0;
        self.sm
            .rx()
            .dma_pull(self.dma.reborrow(), slice::from_mut(&mut status), false)
            .await;
        status
    }
}

fn big_buffer_to_u8(big_buffer: &[u32]) -> [u8; 1028] {
    const BUFFER_LENGTH: usize = 1028;

    // Map the array of 32-bit numbers to 16-bits
    let mut buffer: [u8; BUFFER_LENGTH] = [0; BUFFER_LENGTH];

    for i in 0..big_buffer.len() {
        let bytes = big_buffer[i].to_be_bytes(); // NOTE: This may have to be little-endian instead

        let little_buffer_i = i * 4; // Account for four bytes being read at a time
        buffer[little_buffer_i] = bytes[0];
        buffer[little_buffer_i + 1] = bytes[1];
        buffer[little_buffer_i + 2] = bytes[2];
        buffer[little_buffer_i + 3] = bytes[3];
    }

    buffer
}

///
/// Terrible implementation to allow rp2040-hal's SPIO to be used with the cyw43 driver
impl<D, P> SpiBusCyw43 for CustomSpiWrapper<D, P>
where
    D: spi::SpiDevice,
    P: spi::ValidSpiPinout<D>,
{
    async fn cmd_read(&mut self, write: u32, read: &mut [u32]) -> u32 {
        // self.cs.set_low().unwrap();
        // // let mut buffer = big_buffer_to_u8(read);
        // // let status = self.spi.read(&mut buffer);
        // let status = self.cmd_read(write, read).await;
        // self.cs.set_high().unwrap();
        // status
        0
    }

    // fn read<'a>(&'a mut self, write: u32, read: &mut [u32]) -> Result<(), Self::Error> {
    //     trace!("spi read {}", words.len());
    //     self.dio.into_floating_input();
    //     for read in read.iter_mut() {
    //         let mut w = 0;
    //         for _ in 0..32 {
    //             w <<= 1;
    //
    //             cortex_m::asm::nop();
    //             // rising edge, sample data
    //             if self.dio.is_high().unwrap() {
    //                 w |= 0x01;
    //             }
    //             self.clk.set_high().unwrap();
    //
    //             cortex_m::asm::nop();
    //             // falling edge
    //             self.clk.set_low().unwrap();
    //         }
    //         *word = w
    //     }
    //
    //     trace!("spi read result: {:x}", words);
    //     Ok(())
    // }

    //     fn write<'a>(&'a mut self, words: &'a [u32]) -> Result<(), Self::Error> {
    //     trace!("spi write {:x}", words);
    //     self.dio.into_push_pull_output();
    //     for word in words {
    //         let mut word = *word;
    //         for _ in 0..32 {
    //             // falling edge, setup data
    //             cortex_m::asm::nop();
    //             self.clk.set_low().unwrap();
    //             if word & 0x8000_0000 == 0 {
    //                 self.dio.set_low().unwrap();
    //             } else {
    //                 self.dio.set_high().unwrap();
    //             }
    //
    //             cortex_m::asm::nop();
    //             // rising edge
    //             self.clk.set_high().unwrap();
    //
    //             word <<= 1;
    //         }
    //     }
    //     self.clk.set_low().unwrap();
    //
    //     self.dio.into_floating_input();
    //     Ok(())
    // }

    async fn cmd_write(&mut self, write: &[u32]) -> u32 {
        0
    }

    async fn wait_for_event(&mut self) {
        while self.spi.is_busy() {}
    }
}

#[embassy_executor::task]
pub async fn wireless_main(
    spawner: Spawner,
    uart_device: UART0,
    mut resets: RESETS,
    clocks: ClocksManager,
    pins: Pins,
    spi0: SPI0,
    pio0: PIO0,
    mut delay: Delay,
    state: &'static mut cyw43::State,
) {
    let uart_pins: UartPins<Gpio0, Gpio1> = (
        // UART TX (characters sent from RP2040) on pin 1 (GPIO0)
        pins.gpio0.reconfigure::<FunctionUart, PullNone>(),
        // UART RX (characters received by RP2040) on pin 2 (GPIO1)
        pins.gpio1.reconfigure::<FunctionUart, PullNone>(),
    );

    let mut uart: Uart<Gpio0, Gpio1> = UartPeripheral::new(uart_device, uart_pins, &mut resets)
        .enable(
            UartConfig::new(9600_u32.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    //
    // Configure pins fro PioSpi
    //

    // Create PIO
    // configure LED pin for Pio0.
    let dio_pin: Pin<_, FunctionPio0, _> = pins.gpio18.into_function();
    let dio_pin_id = dio_pin.id().num;

    // u
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

    // Copied logic from cyw43-pio, but using rp2040-hal pins
    let mut pin_io: Pin<_, _, PullNone> = dio_pin.into_pull_type();
    pin_io.set_schmitt_enabled(true);
    // dio_pin.set_input_sync_bypass(true); TODO: Find equivalent, if necessary
    pin_io.set_drive_strength(gpio::OutputDriveStrength::TwelveMilliAmps);
    pin_io.set_slew_rate(gpio::OutputSlewRate::Fast);

    // Set up our SPI pins into the correct mode
    let mut spi_sclk: gpio::Pin<_, gpio::FunctionSpi, gpio::PullNone> = pins.gpio22.reconfigure();
    spi_sclk.set_drive_strength(gpio::OutputDriveStrength::TwelveMilliAmps); // From cyw43-pio
    spi_sclk.set_slew_rate(gpio::OutputSlewRate::Fast); // From cyw43-pio
    let pin_clk_id = spi_sclk.id().num;

    let spi_mosi: gpio::Pin<_, gpio::FunctionSpi, gpio::PullNone> = pins.gpio7.reconfigure(); // SPI0 TX
    let spi_miso: gpio::Pin<_, gpio::FunctionSpi, gpio::PullUp> = pins.gpio16.reconfigure(); // SPIO RX
    let spi_cs = pins.gpio19.into_push_pull_output();

    // Initialize and start PIO
    let (mut pio, sm0, _, _, _) = pio0.split(&mut resets);
    let installed = pio.install(&default_program.program).unwrap();
    let (int, frac) = (0, 0); // as slow as possible (0 is interpreted as 65536)
    // Match cwy43-pio's pins, shift and clock divider configuration
    let (mut sm, rx, tx) = hal::pio::PIOBuilder::from_installed_program(installed)
        .out_pins(dio_pin_id, 1)
        .in_pin_base(dio_pin_id)
        .set_pins(dio_pin_id, 1)
        .out_shift_direction(hal::pio::ShiftDirection::Left)
        .in_shift_direction(hal::pio::ShiftDirection::Right)
        .autopush(true) // Matching embassy's shift_in.auto_fill = true
        .autopull(true) // Matching embassy's shift_out.auto_fill = true
        .clock_divisor_fixed_point(int, frac)
        .build(sm0);

    // The GPIO pins need to be configured as outputs
    sm.set_pindirs([
        (dio_pin_id, hal::pio::PinDir::Output),
        (pin_clk_id, hal::pio::PinDir::Output),
    ]);

    // Create the SPI driver instance for the SPI0 device
    let spi = spi::Spi::<_, _, _, 8>::new(spi0, (spi_mosi, spi_miso, spi_sclk));

    // Exchange the uninitialised SPI driver for an initialised one
    let spi = spi.init(
        &mut resets,
        clocks.peripheral_clock.freq(),
        400.kHz(), // card initialization happens at low baud rate
        embedded_hal::spi::MODE_0,
    );

    let spi_wrapper = CustomSpiWrapper { spi,sm, cs: spi_cs, rx, tx };

    let cyw43_firmware = include_bytes!("../../../cyw43/43439A0.bin");
    let clm = include_bytes!("../../../cyw43/43439A0_clm.bin");

    let mut pwr = pins.b_power_save.into_push_pull_output(); // rp-pico
    pwr.set_low().unwrap();

    info!("oh my");

    // let cs = Output::new(embassy_peripherals.PIN_25, Level::High); // embassy
    // info!("got embassy pin 25").unwrap();
    // // let cs = pins.led.into_push_pull_output().set_high(); // rp-pico
    // let mut pio = Pio::new(embassy_peripherals.PIO0, Irqs);
    // info!("got pio").unwrap();
    // let spi = PioSpi::new(
    //     &mut pio.common,
    //     pio.sm0,
    //     DEFAULT_CLOCK_DIVIDER,
    //     pio.irq0,
    //     cs,
    //     embassy_peripherals.PIN_24,
    //     embassy_peripherals.PIN_29,
    //     embassy_peripherals.DMA_CH0,
    // );
    info!("got piospi");

    // TODO: Look at implementing SpiBusCyw43 for rp2040-hal's PIO

    let (_net_device, mut control, runner) =
        cyw43::new(state, pwr, spi_wrapper, cyw43_firmware).await;
    info!("made cyw43");
    unwrap!(spawner.spawn(cyw43_task(runner)));
    info!("spawned runner!!!!");

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    info!("set power!!!!");
    // let delay = Duration::from_secs(1);

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
