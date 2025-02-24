use core::fmt::Write;

use cortex_m::delay::Delay;
use embedded_hal::digital::InputPin;
use fugit::RateExtU32;
use jartis::i2c::I2CHandler;
use jartis::piicodev_rfid::rfid::PiicoDevRfid;
use jartis::piicodev_ssd1306::PiicoDevSSD1306;
use jartis::uart::{Uart, UartPins};
use rp_pico::hal::clocks::ClocksManager;
use rp_pico::hal::gpio::{FunctionI2C, FunctionUart, PullNone, PullUp};
use rp_pico::hal::uart::UartPeripheral;
use rp_pico::hal::uart::{DataBits, StopBits, UartConfig};
use rp_pico::hal::{Clock, I2C};
use rp_pico::pac::{I2C0, RESETS, UART0};
use rp_pico::Pins;

use super::state::State;

/// Updates the OLED so that the updated state can appear on it
fn update_oled(state: &State, oled: &mut PiicoDevSSD1306) -> Result<(), ()> {
    // Clear the text
    // oled.reset()?;

    if state.is_reading() {
        return oled.write_text("Reading RFID...");
    }

    oled.write_text("Writing RFID...")
}

/// Runs the RFID flasher program
pub fn rfid_flasher_main(
    uart_device: UART0,
    i2c_device: I2C0,
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

    // Create I2C for the OLED display
    let i2c: I2CHandler = I2C::i2c0(
        i2c_device,
        pins.gpio8.reconfigure::<FunctionI2C, PullUp>(), // sda
        pins.gpio9.reconfigure::<FunctionI2C, PullUp>(), // scl
        400.kHz(),
        resets,
        125_000_000.Hz(),
    );

    let mut rfid = PiicoDevRfid::new(i2c);
    let init = rfid.init(&mut delay);

    if let Err(e) = init {
        writeln!(uart, "RFID Initialisation error: {:?}", e).unwrap();
    }

    loop {
        delay.delay_ms(500);

        if let Err(e) = rfid.read_tag_id(&mut uart) {
            writeln!(uart, "Presence error: {:?}", e).unwrap();
        } else {
            writeln!(uart, "RFID worked").unwrap();
        }
    }

    let mut oled = PiicoDevSSD1306::new(i2c);

    // if let Err(()) = oled.reset() {
    //     writeln!(uart, "OLED error").unwrap();
    // }
    if let Err(()) = oled.write_text("Man this RFID guy...") {
        writeln!(uart, "OLED error").unwrap();
    }

    let mut state = State::new();
    let mut button_pin = pins.gpio15.into_pull_down_input();

    // If this is true, the next iterations of the loop won't toggle the state's mode between
    // reading or writing until it is set to false again (when the button is lifted)
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
            if let Err(()) = update_oled(&state, &mut oled) {
                writeln!(uart, "OLED error").unwrap();
            }
        } else if !is_button_pressed {
            // Release the button so it can be toggled upon the next press
            is_holding_toggle_button = false;
        }

        delay.delay_ms(50);
    }
}
