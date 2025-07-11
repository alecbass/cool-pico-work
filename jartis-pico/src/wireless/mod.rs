use core::fmt::{Write, write};

use cortex_m::delay::Delay;
use cortex_m::prelude::_embedded_hal_blocking_spi_Write;
use cyw43::SpiBusCyw43;
use defmt::*;
use embassy_executor::Spawner;
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::SpiBus;
use fugit::RateExtU32;
use jartis::uart::{Uart, UartPins};
use panic_probe as _;
use rp_pico::Pins;
use rp_pico::hal::Clock;
use rp_pico::hal::clocks::ClocksManager;
use rp_pico::hal::gpio;
use rp_pico::hal::gpio::bank0::Gpio23;
use rp_pico::hal::gpio::{FunctionSioOutput, Pin, PullDown, PullUp};
use rp_pico::hal::gpio::{
    FunctionUart, PullNone,
    bank0::{Gpio0, Gpio1},
};
use rp_pico::hal::pac;
use rp_pico::hal::prelude::*;
use rp_pico::hal::spi::{self, ValidSpiPinout};
use rp_pico::hal::uart::UartPeripheral;
use rp_pico::hal::uart::{DataBits, StopBits, UartConfig};
use rp_pico::pac::{RESETS, SPI0, UART0};

use embassy_rp as _;

// bind_interrupts!(struct Irqs {
//     PIO0_IRQ_0 => InterruptHandler<PIO0>;
// });

unsafe extern "C" {
    fn connectToWifi() -> core::ffi::c_int;
}

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
    cs: Pin<gpio::bank0::Gpio19, FunctionSioOutput, PullDown>,
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
        self.cs.set_low().unwrap();
        let mut buffer = big_buffer_to_u8(read);
        let status = self.spi.read(&mut buffer);
        self.cs.set_high().unwrap();

        0
    }

    async fn cmd_write(&mut self, write: &[u32]) -> u32 {
        self.cs.set_low().unwrap();
        let buffer = big_buffer_to_u8(write);
        let status = SpiBus::write(&mut self.spi, &buffer).unwrap();
        self.cs.set_high().unwrap();

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

    unsafe {
        connectToWifi();
    }

    // Set up our SPI pins into the correct mode
    let spi_sclk: gpio::Pin<_, gpio::FunctionSpi, gpio::PullNone> = pins.gpio22.reconfigure();
    let spi_mosi: gpio::Pin<_, gpio::FunctionSpi, gpio::PullNone> = pins.gpio7.reconfigure(); // SPI0 TX
    let spi_miso: gpio::Pin<_, gpio::FunctionSpi, gpio::PullUp> = pins.gpio16.reconfigure(); // SPIO RX
    let spi_cs = pins.gpio19.into_push_pull_output();

    // Create the SPI driver instance for the SPI0 device
    let spi = spi::Spi::<_, _, _, 8>::new(spi0, (spi_mosi, spi_miso, spi_sclk));

    // Exchange the uninitialised SPI driver for an initialised one
    let spi = spi.init(
        &mut resets,
        clocks.peripheral_clock.freq(),
        400.kHz(), // card initialization happens at low baud rate
        embedded_hal::spi::MODE_0,
    );

    let spi_wrapper = CustomSpiWrapper { spi, cs: spi_cs };

    let cyw43_firmware = include_bytes!("../../../cyw43/43439A0.bin");
    let clm = include_bytes!("../../../cyw43/43439A0_clm.bin");

    // let pwr = Output::new(embassy_peripherals.PIN_23, Level::Low); // embassy
    let mut pwr = pins.b_power_save.into_push_pull_output(); // rp-pico
    pwr.set_low().unwrap();

    info!("oh my");

    // let cs = Output::new(embassy_peripherals.PIN_25, Level::High); // embassy
    // writeln!(uart, "got embassy pin 25").unwrap();
    // // let cs = pins.led.into_push_pull_output().set_high(); // rp-pico
    // let mut pio = Pio::new(embassy_peripherals.PIO0, Irqs);
    // writeln!(uart, "got pio").unwrap();
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
    writeln!(uart, "got piospi").unwrap();

    // TODO: Look at implementing SpiBusCyw43 for rp2040-hal's PIO

    let (_net_device, mut control, runner) =
        cyw43::new(state, pwr, spi_wrapper, cyw43_firmware).await;
    writeln!(uart, "made cyw43").unwrap();
    unwrap!(spawner.spawn(cyw43_task(runner)));
    writeln!(uart, "spawned runner!!!!").unwrap();

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    writeln!(uart, "set power!!!!").unwrap();
    // let delay = Duration::from_secs(1);

    loop {
        writeln!(uart, "hiiiiee").unwrap();
        // delay.delay_ms(250);
        info!("led on!");
        control.gpio_set(0, true).await;
        delay.delay_ms(1000);

        info!("led off!");
        control.gpio_set(0, false).await;
        delay.delay_ms(1000);
    }
}
