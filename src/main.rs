//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

use core::cell::RefCell;
use core::fmt::Write;

use cortex_m::delay::Delay;
use defmt::*;
use defmt_rtt as _;
use embedded_graphics::mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::text::{Alignment, Text};
use embedded_hal::digital::OutputPin;
use fugit::RateExtU32;
use panic_probe as _;

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use rp_pico as bsp;
// use sparkfun_pro_micro_rp2040 as bsp;

use bsp::entry;
use bsp::hal::gpio::{FunctionI2C, FunctionUart, PullNone, PullUp};
use bsp::hal::i2c::I2C;
use bsp::hal::uart::{DataBits, StopBits, UartConfig, UartPeripheral};
use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    watchdog::Watchdog,
};
use bsp::Pins;
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

mod i2c;
mod piicodev_bme280;
mod piicodev_buzzer;
mod piicodev_qmc6310;
mod piicodev_rgb;
mod piicodev_ssd1306;
mod piicodev_vl53l1x;
mod uart;

use i2c::I2CHandler;
use piicodev_bme280::piicodev_bme280::PiicoDevBME280;
use piicodev_buzzer::notes::HARMONY;
use piicodev_buzzer::piicodev_buzzer::{BuzzerVolume, PiicoDevBuzzer};
use piicodev_qmc6310::{GaussRange, PiicoDevQMC6310};
use piicodev_rgb::piicodev_rgb::PiicoDevRGB;
use piicodev_ssd1306::{OLEDColour, PiicoDevSSD1306};
use piicodev_vl53l1x::piicodev_vl53l1x::PiicoDevVL53L1X;
use uart::{Uart, UartPins};

#[link(name = "jartis")]
extern "C" {
    pub fn connectToWifi() -> i32;
}

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
    let mut uart: Uart = UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
        .enable(
            UartConfig::new(9600.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    let mut led = pins.led.into_push_pull_output();
    led.set_high().unwrap();

    writeln!(uart, "hieeeeGGGGeee").unwrap();

    let connection_attempt = unsafe { connectToWifi() };

    loop {
        writeln!(uart, "Continuing... {}", connection_attempt).unwrap();
        delay.delay_ms(1000);
    }

    // Write something to the UART on start-up so we can check the output pin
    // is wired correctly.

    let mut i2c: I2CHandler = I2C::i2c0(
        pac.I2C0,
        pins.gpio8.reconfigure::<FunctionI2C, PullUp>(), // sda
        pins.gpio9.reconfigure::<FunctionI2C, PullUp>(), // scl
        400.kHz(),
        &mut pac.RESETS,
        125_000_000.Hz(),
    );

    let declination = 12.64; // Brisbane
    let mut magnetometer = PiicoDevQMC6310::new(None, Some(GaussRange::Gauss1200), declination);

    if magnetometer.init(&mut i2c).is_err() {
        writeln!(uart, "Failed to initialise magnetometer").unwrap();
    }
    delay.delay_ms(5);

    delay.delay_ms(1000);

    // if magnetometer
    //     .calibrate(false, &mut i2c, &mut uart, &mut delay)
    //     .is_err()
    // {
    //     writeln!(uart, "Failed to calibrate magnetometer").unwrap();
    // }

    loop {
        let reading = magnetometer.read_polar(&mut i2c, &mut uart);

        if let Ok(reading) = reading {
            writeln!(uart, "Polar: {}°", reading.polar as u16).unwrap();
        } else {
            writeln!(uart, "Failed to read magnetometer").unwrap();
        }

        delay.delay_ms(100);
    }

    let interface = I2CDisplayInterface::new(i2c);

    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();

    display.init().unwrap();

    // NOTE: SSD1306 only supports binary colours: on and off (white and black)
    let white_text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    let black_text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::Off)
        .build();

    let white_rectangle_style = PrimitiveStyleBuilder::new()
        .fill_color(BinaryColor::On)
        .stroke_color(BinaryColor::On)
        .stroke_width(3)
        .build();

    Rectangle::new(Point::zero(), Size::new(128, 64))
        .into_styled(white_rectangle_style)
        .draw(&mut display)
        .unwrap();

    Text::with_alignment(
        "80% of boys have",
        Point::new(64, 8),
        black_text_style,
        Alignment::Center,
    )
    .draw(&mut display)
    .unwrap();

    Text::with_alignment(
        "girlfriends",
        Point::new(64, 24),
        black_text_style,
        Alignment::Center,
    )
    .draw(&mut display)
    .unwrap();

    Text::with_alignment(
        "rest 20% are having",
        Point::new(64, 40),
        black_text_style,
        Alignment::Center,
    )
    .draw(&mut display)
    .unwrap();

    Text::with_alignment(
        "a brain",
        Point::new(64, 56),
        black_text_style,
        Alignment::Center,
    )
    .draw(&mut display)
    .unwrap();

    display.flush().unwrap();

    // Turn IO devices into shared pointers
    let i2c_cell = RefCell::new(i2c);
    let uart_cell = RefCell::new(uart);
    let delay_cell = RefCell::new(delay);

    let mut distance_sensor = PiicoDevVL53L1X::new(None, &i2c_cell, &delay_cell);
    distance_sensor.init().unwrap();

    // Set up the RGB device
    let mut rgb = PiicoDevRGB::new(&i2c_cell);

    // Turn the LED on
    rgb.power_led(true).unwrap();

    // Create the buzzer
    let mut buzzer = PiicoDevBuzzer::new(&i2c_cell, &delay_cell);

    // Initialise the buzzer
    buzzer.init().unwrap();
    buzzer.volume(BuzzerVolume::Low).unwrap();

    // Initialise the temperature sensor
    let mut temperature_sensor = PiicoDevBME280::new(&i2c_cell, &delay_cell);
    temperature_sensor.init().unwrap();

    // Increases evey time the sensor reads close OR reads far consecutively
    let mut last_is_close = false;
    let mut same_reading_index = 0;

    // How long to wait until the next reading
    const NEAR_DELAY: u32 = 20;
    const FAR_DELAY: u32 = 1000;

    let mut next_delay;

    const COLOUR_MAP: [(u8, u8, u8); 6] = [
        (255, 0, 0),
        (0, 255, 0),
        (0, 0, 255),
        (255, 255, 0),
        (0, 255, 255),
        (255, 0, 255),
    ];

    // Play repeated song
    let mut note_index = 0;
    let song = HARMONY;

    // Set up the OLED display
    let mut oled = PiicoDevSSD1306::new(&i2c_cell);
    oled.init().unwrap();
    oled.fill(OLEDColour::WHITE);

    for i in 64..0 {
        oled.pixel(i, i, OLEDColour::BLACK);
    }

    oled.show().unwrap();

    loop {
        let reading = distance_sensor.read().unwrap();

        let mut uart = uart_cell.borrow_mut();

        let is_close = reading < 100;

        // Have we gone from close to near, or from near to close?
        let did_change = is_close != last_is_close;

        if did_change {
            same_reading_index = 0;
        } else {
            same_reading_index += 1;
        }

        last_is_close = is_close;

        if is_close {
            writeln!(uart, "GAMERS DETECTED!!!!! JULIA, KAFFY, SCRYBID").unwrap();
            // Set brightness
            rgb.set_brightness(20).unwrap();

            let left = COLOUR_MAP[same_reading_index % 6];
            let middle = COLOUR_MAP[(same_reading_index + 1) % 6];
            let right = COLOUR_MAP[(same_reading_index + 2) % 6];

            // Set colours
            rgb.set_pixel(0, left);
            rgb.set_pixel(1, middle);
            rgb.set_pixel(2, right);

            next_delay = NEAR_DELAY;

            // Play the next note of the song
            let (note, duration) = HARMONY[note_index];
            // buzzer.tone(&note, duration).unwrap();

            note_index += 1;

            let is_at_end_of_song = note_index >= song.len();

            if is_at_end_of_song {
                // Restart the song
                note_index = 0;
            }
        } else {
            writeln!(uart, "No gaming detected in the vicinity...").unwrap();
            // Set brightness
            rgb.set_brightness(5).unwrap();

            // Set colours
            rgb.set_pixel(0, (255, 0, 0));
            rgb.set_pixel(1, (0, 255, 0));
            rgb.set_pixel(2, (0, 0, 255));

            next_delay = FAR_DELAY;
        }

        rgb.show().unwrap();

        let readings = temperature_sensor.values().unwrap();
        let altitude = temperature_sensor.altitude(None).unwrap();
        writeln!(
            uart,
            "Temperature: {} Pressure: {} Humidity: {} Altitude: {}",
            readings.temperature, readings.pressure, readings.humidity, altitude
        )
        .unwrap();

        {
            let mut delay = delay_cell.borrow_mut();
            delay.delay_ms(next_delay);
        }
    }
}

// End of file
