use core::fmt::Write;

use cortex_m::delay::Delay;
use embedded_hal::digital::OutputPin;
use fugit::RateExtU32;
use rp_pico::hal::clocks::ClocksManager;
use rp_pico::hal::gpio::{FunctionUart, PullNone};
use rp_pico::hal::uart::UartPeripheral;
use rp_pico::hal::uart::{DataBits, StopBits, UartConfig};
use rp_pico::hal::Clock;
use rp_pico::pac::{RESETS, UART0};
use rp_pico::Pins;

use jartis::uart::{Uart, UartPins};

/// Runs the RFID flasher program
pub fn rfid_flasher_main(
    uart_device: UART0,
    resets: &mut RESETS,
    clocks: ClocksManager,
    pins: Pins,
    mut delay: Delay,
) -> ! {
    let mut led = pins.led.into_push_pull_output();

    let uart_pins: UartPins = (
        // UART TX (characters sent from RP2040) on pin 1 (GPIO0)
        pins.gpio0.reconfigure::<FunctionUart, PullNone>(),
        // UART RX (characters received by RP2040) on pin 2 (GPIO1)
        pins.gpio1.reconfigure::<FunctionUart, PullNone>(),
    );

    let mut uart: Uart = UartPeripheral::new(uart_device, uart_pins, resets)
        .enable(
            UartConfig::new(9600_u32.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    writeln!(uart, "Running RFID flasher").unwrap();

    // Blink the LED at 1 Hz
    loop {
        led.set_high().unwrap();
        delay.delay_ms(500);
        led.set_low().unwrap();
        delay.delay_ms(500);
        writeln!(uart, "Running RFID flasher again").unwrap();
    }
}
