use core::fmt::Write;

use cortex_m::delay::Delay;
use embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin};
use fugit::RateExtU32;
use rp_pico::hal::clocks::ClocksManager;
use rp_pico::hal::gpio::{FunctionUart, PullNone};
use rp_pico::hal::uart::UartPeripheral;
use rp_pico::hal::uart::{DataBits, StopBits, UartConfig};
use rp_pico::hal::Clock;
use rp_pico::pac::{RESETS, UART0};
use rp_pico::Pins;

use jartis::uart::{Uart, UartPins};

use super::state::State;

/// Runs the RFID flasher program
pub fn rfid_flasher_main(
    uart_device: UART0,
    resets: &mut RESETS,
    clocks: ClocksManager,
    pins: Pins,
    mut delay: Delay,
) -> ! {
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

    let mut state = State::new();

    // Reset GPIO15 for a button to read input
    let mut button_pin = pins.gpio15.into_pull_down_input();
    let mut is_holding_toggle_button = false;

    // Blink the LED at 1 Hz
    loop {
        // Swap the state if the button has been pressed
        let is_button_pressed = button_pin.is_high().unwrap_or(false);
        let should_toggle = is_button_pressed && !is_holding_toggle_button;

        if should_toggle {
            // Toggle to the other mode and mark the button as being held so it doesn't keep toggling
            is_holding_toggle_button = true;
            state.toggle_mode();
        } else if !is_button_pressed {
            // Release the button so it can be toggled upon the next press
            is_holding_toggle_button = false;
        }

        if state.is_reading() {
            writeln!(uart, "Reading...").unwrap();
        } else {
            writeln!(uart, "Writing...").unwrap();
        }

        delay.delay_ms(50);
    }
}
