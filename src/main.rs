//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

use bsp::entry;
use cortex_m::delay::Delay;
use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::OutputPin;
use embedded_hal::i2c::I2c;
use fugit::RateExtU32;
use panic_probe as _;

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use rp_pico as bsp;
// use sparkfun_pro_micro_rp2040 as bsp;

use bsp::hal::gpio::bank0::{Gpio0, Gpio1};
use bsp::hal::gpio::{FunctionI2C, FunctionUart, Pin, PullNone, PullUp};
use bsp::hal::i2c::I2C;
use bsp::hal::uart::{DataBits, Enabled, StopBits, UartConfig, UartPeripheral};
use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    watchdog::Watchdog,
};
use bsp::pac::UART0;
use bsp::Pins;

type UartPins = (
    Pin<Gpio0, FunctionUart, PullNone>,
    Pin<Gpio1, FunctionUart, PullNone>,
);

/// Alias the type for our UART to make things clearer.
type Uart = UartPeripheral<Enabled, UART0, UartPins>;

/// This how we transfer the UART into the Interrupt Handler
// static GLOBAL_UART: Mutex<RefCell<Option<Uart>>> = Mutex::new(RefCell::new(None));

const EXTERNAL_XTAL_FREQ_HZ: u32 = 12_000_000u32;

#[entry]
fn main() -> ! {
    info!("Program start");
    // Grab our singleton objects
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let clocks = init_clocks_and_plls(
        EXTERNAL_XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    // Lets us wait for fixed periods of time
    let mut delay = Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    // Set the pins to their default state
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // This is the correct pin on the Raspberry Pico board. On other boards, even if they have an
    // on-board LED, it might need to be changed.
    //
    // Notably, on the Pico W, the LED is not connected to any of the RP2040 GPIOs but to the cyw43 module instead.
    // One way to do that is by using [embassy](https://github.com/embassy-rs/embassy/blob/main/examples/rp/src/bin/wifi_blinky.rs)
    //
    // If you have a Pico W and want to toggle a LED with a simple GPIO output pin, you can connect an external
    // LED to one of the GPIO pins, and reference that pin here. Don't forget adding an appropriate resistor
    // in series with the LED.

    let uart_pins: UartPins = (
        // UART TX (characters sent from RP2040) on pin 1 (GPIO0)
        pins.gpio0.reconfigure::<FunctionUart, PullNone>(),
        // UART RX (characters received by RP2040) on pin 2 (GPIO1)
        pins.gpio1.reconfigure::<FunctionUart, PullNone>(),
    );

    // Make a UART on the given pins
    let uart: Uart = UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
        .enable(
            UartConfig::new(9600.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    // Write something to the UART on start-up so we can check the output pin
    // is wired correctly.

    let mut i2c = I2C::i2c0(
        pac.I2C0,
        pins.gpio8.reconfigure::<FunctionI2C, PullUp>(), // sda
        pins.gpio9.reconfigure::<FunctionI2C, PullUp>(), // scl
        400.kHz(),
        &mut pac.RESETS,
        125_000_000.Hz(),
    );

    // Peripheral address of the PiicoDev RGB device
    const RGB_ADDR: u8 = 0x1E;
    // Address of the LED
    const REG_CTRL: u8 = 0x03;
    // Address of the brightness controller
    const REG_BRIGHT: u8 = 0x06;
    // Address of where to send LED colour colours
    const REG_LED_VALS: u8 = 0x07;

    // Turn the LED on
    i2c.write(RGB_ADDR, &[REG_CTRL, 1]).unwrap();

    loop {
        for i in 0..10 {
            // Set brightness
            i2c.write(RGB_ADDR, &[REG_BRIGHT, i * 2]).unwrap();

            // Set colours
            let colours = [REG_LED_VALS, 255, 0, 0, 0, 255, 0, 0, 0, 255];
            i2c.write(RGB_ADDR, &colours).unwrap();

            uart.write_full_blocking(b"uart_interrupt example started...\n");
            delay.delay_ms(200);
        }
        // uart.read(&mut buffer).unwrap();
    }
}

// End of file
