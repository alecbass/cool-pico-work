//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

mod byte_reader;
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
    gpio, pac,
    sio::Sio,
    watchdog::Watchdog,
    I2C,
};

use core::cell::{self, RefCell};
use fugit::RateExtU32;
use piicodev_bme280::{piicodev_bme280::PiicoDevBME280, reading::Reading};
use piicodev_buzzer::notes::{note_to_frequency, Note, EIGHT_MELODIES, HARMONY};
use piicodev_buzzer::piicodev_buzzer::{BuzzerVolume, PiicoDevBuzzer};
use piicodev_unified::{HardwareArgs, I2CUnifiedMachine, GPIO89I2C};

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
    let mut resets = peripherals.RESETS;

    // let hardware_args: HardwareArgs = (i2c0, delay, &pins, resets);

    let gpio8 = pins.gpio8.into_mode();
    let gpio9 = pins.gpio9.into_mode();

    let i2c: GPIO89I2C = I2C::i2c0(
        i2c0,
        gpio8, // sda
        gpio9, // scl
        100.kHz(),
        &mut resets,
        125_000_000.Hz(),
    );

    let i2c_machine: I2CUnifiedMachine = I2CUnifiedMachine::new((i2c, delay));

    // let mut oled = PiicoDevSSD1306::new((i2c0, i2c1, delay, pins, resets));

    // oled.arc(60, 60, 6, 60, 90);
    // oled.show().unwrap();

    let i2c_machine_shared: RefCell<I2CUnifiedMachine> = RefCell::new(i2c_machine);

    //
    // Buzzer
    //

    let mut buzzer = PiicoDevBuzzer::new(&i2c_machine_shared, Some(BuzzerVolume::High));

    //
    // Atmospheric Sensor
    //

    const DO_BUZZER: bool = false;

    if DO_BUZZER {
        let mut sensor: PiicoDevBME280 = PiicoDevBME280::new(&i2c_machine_shared);

        loop {
            let reading = sensor.values();
            let (temperature, pressure, humidity) = reading;

            let pressure: f32 = pressure / 100.0; // convert air pressure from pascals to hPa

            let altitude: f32 = sensor.altitude(None);

            let reading: Reading = Reading {
                temperature,
                pressure,
                humidity,
                altitude,
            };

            reading.report();

            if reading.temperature > 25.0 {
                buzzer.play_song(&[(Note::A4, 1000), (Note::A5, 1000), (Note::A6, 1000)]);
                // buzzer.play_song(&HARMONY);
            }
        }
    }

    loop {}
}

// End of file
