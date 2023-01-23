//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

mod piicodev_bme280;
mod piicodev_rgb;
mod piicodev_unified;
mod utils;

use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::v2::OutputPin;
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

const FLASH_TIMERS: &[u32] = &[200, 1000, 100, 500];

#[entry]
fn main() -> ! {
    info!("Program start");
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

    let mut rgb = PiicoDevRGB::new((i2c0, i2c1, delay, pins, resets));
    rgb.set_brightness(5).unwrap();
    rgb.set_pixel(0, (255, 0, 0));
    rgb.set_pixel(1, (0, 255, 0));
    rgb.set_pixel(2, (0, 0, 255));
    rgb.show().unwrap();

    let mut i: u8 = 0;
    loop {
        let light: usize = i as usize % 3;

        let led: PQV;
        if light == 0 {
            led = (255 - i, i, 0);
        } else if light == 1 {
            led = (i, 255 - i, 0);
        } else {
            led = (i, 0, 255 - i);
        }

        rgb.set_pixel(light, led);
        rgb.i2c.delay(10);
        rgb.show().unwrap();

        i += 1;
    }
}

// End of file
