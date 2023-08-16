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
mod piicodev_vl53l1x;
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
    uart::{UartConfig, UartPeripheral},
    watchdog::Watchdog,
    I2C,
};

use core::cell::{self, RefCell};
use core::fmt::Write;
use defmt::{debug, info, println};
use embedded_hal::digital::v2::OutputPin;
use fugit::RateExtU32;
use piicodev_bme280::{piicodev_bme280::PiicoDevBME280, reading::Reading};
use piicodev_buzzer::notes::{note_to_frequency, Note, EIGHT_MELODIES, HARMONY};
use piicodev_buzzer::piicodev_buzzer::{BuzzerVolume, PiicoDevBuzzer};
use piicodev_unified::{HardwareArgs, I2CUnifiedMachine, GPIO89I2C};
use piicodev_vl53l1x::piicodev_vl53l1x::PiicoDevVL53L1X;

const FLASH_TIMERS: &[u32] = &[200, 1000, 100, 500];

#[entry]
fn main() -> ! {
    let mut peripherals: pac::Peripherals =
        pac::Peripherals::take().expect("Cannot take peripherals");

    let core: pac::CorePeripherals = pac::CorePeripherals::take().unwrap();
    let mut watchdog: Watchdog = Watchdog::new(peripherals.WATCHDOG);
    let sio: Sio = Sio::new(peripherals.SIO);

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

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let pins = rp_pico::Pins::new(
        peripherals.IO_BANK0,
        peripherals.PADS_BANK0,
        sio.gpio_bank0,
        &mut peripherals.RESETS,
    );

    let uart_pins = (
        pins.gpio0.into_mode::<gpio::FunctionUart>(),
        pins.gpio1.into_mode::<gpio::FunctionUart>(),
    );

    let mut uart = UartPeripheral::new(peripherals.UART0, uart_pins, &mut peripherals.RESETS)
        .enable(
            // UartConfig::new(9600.Hz(), DataBits::Eight, None, StopBits::One),
            UartConfig::default(),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    let i2c0 = peripherals.I2C0;
    let mut resets = peripherals.RESETS;

    // let hardware_args: HardwareArgs = (i2c0, delay, &pins, resets);

    let gpio8 = pins.gpio8.into_mode::<gpio::FunctionI2C>();
    let gpio9 = pins.gpio9.into_mode::<gpio::FunctionI2C>();

    let i2c: GPIO89I2C = I2C::i2c0(
        i2c0,
        gpio8, // sda
        gpio9, // scl
        100.kHz(),
        &mut resets,
        125_000_000.Hz(),
    );

    let i2c_machine: I2CUnifiedMachine = I2CUnifiedMachine::new((i2c, delay));
    let i2c_machine_shared: RefCell<I2CUnifiedMachine> = RefCell::new(i2c_machine);

    // let mut oled = PiicoDevSSD1306::new((i2c0, i2c1, delay, pins, resets));

    // oled.arc(60, 60, 6, 60, 90);
    // oled.show().unwrap();

    //
    // Buzzer
    //

    //
    // Atmospheric Sensor
    //

    const DO_BUZZER: bool = false;

    // if DO_BUZZER {
    //     let mut buzzer: PiicoDevBuzzer =
    //         PiicoDevBuzzer::new(&i2c_machine_shared, Some(BuzzerVolume::High));
    //     let mut sensor: PiicoDevBME280 = PiicoDevBME280::new(&i2c_machine_shared);

    //     loop {
    //         let reading = sensor.values();
    //         let (temperature, pressure, humidity) = reading;

    //         let pressure: f32 = pressure / 100.0; // convert air pressure from pascals to hPa

    //         let altitude: f32 = sensor.altitude(None);

    //         let reading: Reading = Reading {
    //             temperature,
    //             pressure,
    //             humidity,
    //             altitude,
    //         };

    //         reading.report();

    //         if reading.temperature > 25.0 {
    //             buzzer.play_song(&[(Note::A4, 1000), (Note::A5, 1000), (Note::A6, 1000)]);
    //             // buzzer.play_song(&HARMONY);
    //         }
    //     }
    // }

    const DO_DISTANCE: bool = false;

    if DO_DISTANCE {
        // let mut distance_sensor: PiicoDevVL53L1X = PiicoDevVL53L1X::new(None, &i2c_machine_shared);
        // loop {
        //     let distance_reading: u16 = distance_sensor.read().unwrap();

        //     info!("Reading {}", distance_reading);
        // }
    }

    let mut count: u8 = 0;
    let mut i = i2c_machine_shared.borrow_mut();
    loop {
        count += 1;

        writeln!(uart, "FINALLY GOT SERIAL COMMUNICATIONS {}", count).unwrap();
        i.delay(1000);
    }
}

// End of file
