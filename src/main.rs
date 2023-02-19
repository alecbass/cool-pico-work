//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

mod piicodev_bme280;
mod piicodev_buzzer;
mod piicodev_rgb;
mod piicodev_ssd1306;
mod piicodev_unified;
mod utils;

use defmt_rtt as _;
use panic_probe as _;
use rp_pico::entry;

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
// use sparkfun_pro_micro_rp2040 as bsp;

use rp_pico::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    watchdog::Watchdog,
};

use crate::piicodev_rgb::{PiicoDevRGB, PQV};
use crate::piicodev_ssd1306::PiicoDevSSD1306;
use defmt::{debug, info};
use piicodev_bme280::piicodev_bme280::PiicoDevBME280;
use piicodev_buzzer::notes::{note_to_frequency, Note, EIGHT_MELODIES, HARMONY};
use piicodev_buzzer::piicodev_buzzer::{BuzzerVolume, PiicoDevBuzzer};
use piicodev_ssd1306::OLEDColour;

const FLASH_TIMERS: &[u32] = &[200, 1000, 100, 500];

#[entry]
fn main() -> ! {
    let mut peripherals: pac::Peripherals = pac::Peripherals::take().unwrap();

    let core: pac::CorePeripherals = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(peripherals.WATCHDOG);
    let sio = Sio::new(peripherals.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        peripherals.XOSC,
        peripherals.CLOCKS,
        peripherals.PLL_SYS,
        peripherals.PLL_USB,
        &mut peripherals.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let pins = rp_pico::Pins::new(
        peripherals.IO_BANK0,
        peripherals.PADS_BANK0,
        sio.gpio_bank0,
        &mut peripherals.RESETS,
    );

    let i2c0 = peripherals.I2C0;
    let resets = peripherals.RESETS;

    let i2c1 = peripherals.I2C1;

    // let mut oled = PiicoDevSSD1306::new((i2c0, i2c1, delay, pins, resets));

    // oled.arc(60, 60, 6, 60, 90);
    // oled.show().unwrap();

    //
    // Buzzer
    //

    // let mut buzzer =
    //     PiicoDevBuzzer::new((i2c0, i2c1, delay, pins, resets), Some(BuzzerVolume::High));

    // let mut is_led_on: bool = true;
    // for (tone, duration) in HARMONY {
    //     buzzer.power_led(is_led_on).unwrap();
    //     is_led_on = !is_led_on;

    //     buzzer.tone(tone, duration / 4).unwrap();
    //     buzzer.i2c.delay((duration / 4) as u32);
    // }

    //
    // Atmospheric Sensor
    //

    let mut sensor: PiicoDevBME280 = PiicoDevBME280::new((i2c0, i2c1, delay, pins, resets));

    loop {
        let reading = sensor.values();
        // let values = sensor.values();
        info!("READINGGG {:?}", reading);
    }
}

// End of file
