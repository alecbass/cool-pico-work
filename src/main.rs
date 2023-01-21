//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

mod piicodev_rgb;
mod piicodev_unified;

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

use crate::piicodev_rgb::PiicoDevRGB;

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

    let mut rgb = PiicoDevRGB::new(None, (i2c0, delay, pins, resets));
    // rgb.set_pixel(2, (30, 60, 90));

    rgb.i2c.flash_led();
    // let r = rgb.power_led(true);
    // if r.is_err() {
    //     rgb.i2c.flash_led();
    // } else {
    //     rgb.i2c.flash_led();
    // }
    for i in 0x08..0x88 {
        // rgb.set_i2c_addr(i).unwrap();
        // rgb.clear().unwrap();
        // rgb.power_led(i % 2 == 0).unwrap();

        // rgb.fill(130).unwrap();
    }
    // rgb.set_brightness(100).unwrap();
    // rgb.set_pixel(0, (50, 50, 50));
    // rgb.show().unwrap();
    // delay.delay_ms(200);

    // rgb.set_pixel(0, (150, 150, 150));
    // delay.delay_ms(200);

    // rgb.set_pixel(0, (200, 200, 200));
    // delay.delay_ms(200);

    loop {
        for timer in FLASH_TIMERS {
            // info!("on!");
            // led_pin.set_high().unwrap();
            // delay.delay_ms(*timer);

            // info!("off!");
            // led_pin.set_low().unwrap();
            // delay.delay_ms(*timer);
        }
    }
}

// End of file
