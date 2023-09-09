//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

mod byte_reader;
mod piicodev_bme280;
mod piicodev_buzzer;
mod piicodev_rgb;
mod piicodev_ssd1306;
mod piicodev_unified;
mod piicodev_vl53l1x;
mod pins;
mod time;
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

use crate::piicodev_rgb::PiicoDevRGB;
use core::cell::RefCell;
use core::fmt::Write;
use embassy_executor::{Executor, Spawner};
use embedded_hal::digital::v2::OutputPin;
use fugit::RateExtU32;
use piicodev_buzzer::notes::Note;
use piicodev_buzzer::piicodev_buzzer::{BuzzerVolume, PiicoDevBuzzer};
use piicodev_unified::{I2CUnifiedMachine, GPIO89I2C};
use piicodev_vl53l1x::piicodev_vl53l1x::PiicoDevVL53L1X;
use static_cell::make_static;

const FLASH_TIMERS: &[u32] = &[200, 1000, 100, 500];

#[embassy_executor::task]
async fn run_jingle_async(
    spawner: Spawner,
    comms: RefCell<I2CUnifiedMachine>,
    state: &'static cyw43::State,
) -> ! {
    const DELAY: u16 = 40;
    let mut comms = comms.borrow_mut();
    let distance_sensor: PiicoDevVL53L1X = PiicoDevVL53L1X::new(None, &mut comms);
    let mut led: PiicoDevRGB = PiicoDevRGB::new(&mut comms);
    let mut buzzer: PiicoDevBuzzer = PiicoDevBuzzer::new(&mut comms, Some(BuzzerVolume::Low));
    led.set_brightness(16, &mut comms).unwrap();

    let mut has_played: bool = false;

    loop {
        let distance_reading_mm: u16 = distance_sensor.read(&mut comms).unwrap();
        comms.delay(DELAY as u32);
        if writeln!(comms.uart, "Distance: {}mm", distance_reading_mm).is_err() {
            // Wiring probably isn't set up correctly
        }

        let mut note: Note = Note::A3;
        // Green light
        if distance_reading_mm < 190 {
            led.set_pixel(0, (0, 255, 0));
            note = Note::A4;
        } else {
            led.set_pixel(0, (0, 0, 0));
        }

        // Yellow light
        if distance_reading_mm < 120 {
            led.set_pixel(1, (255, 255, 0));
            note = Note::A5;
        } else {
            led.set_pixel(1, (0, 0, 0))
        }

        // Red light
        if distance_reading_mm < 60 {
            led.set_pixel(2, (255, 0, 0));
            note = Note::A6;
            if !has_played {
                writeln!(comms.uart, "WUNESCAEB").unwrap();
                // buzzer.play_song(&HARMONY, &mut comms);
                has_played = true;
            }
        } else {
            led.set_pixel(2, (0, 0, 0));
        }

        if distance_reading_mm >= 190 {
            led.clear(&mut comms).unwrap();
        } else {
            led.show(&mut comms).unwrap();
            // Note is guaranteed to not be null in this flow
            // buzzer.tone(&note, DELAY, &mut comms).unwrap();
        }
    }
}

#[embassy_executor::task]
async fn wifi_blinky(
    spawner: Spawner,
    comms: RefCell<I2CUnifiedMachine>,
    state: &'static cyw43::State,
) -> ! {
    loop {}
}

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

    let pins = crate::pins::pins::Pins::new(
        peripherals.IO_BANK0,
        peripherals.PADS_BANK0,
        sio.gpio_bank0,
        &mut peripherals.RESETS,
    );

    let gpio0: rp_pico::Gp0Uart0Tx = pins.gpio0.into_function().into_pull_type();
    let gpio1: rp_pico::Gp1Uart0Rx = pins.gpio1.into_function().into_pull_type();

    let uart_pins = (gpio0, gpio1);

    let uart = UartPeripheral::new(peripherals.UART0, uart_pins, &mut peripherals.RESETS)
        .enable(
            // UartConfig::new(9600.Hz(), DataBits::Eight, None, StopBits::One),
            UartConfig::default(),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    let mut resets = peripherals.RESETS;
    // let hardware_args: HardwareArgs = (i2c0, delay, &pins, resets);

    let gpio8: gpio::Pin<gpio::bank0::Gpio8, gpio::FunctionI2c, gpio::PullUp> = pins
        .gpio8
        .into_function::<gpio::FunctionI2C>()
        .into_pull_type();
    let gpio9: gpio::Pin<gpio::bank0::Gpio9, gpio::FunctionI2c, gpio::PullDown> = pins
        .gpio9
        .into_function::<gpio::FunctionI2C>()
        .into_pull_type();
    // let gpio9: rp_pico::Gp9I2C0Scl = gpio9;

    let i2c: GPIO89I2C = I2C::i2c0(
        peripherals.I2C0,
        gpio8, // sda
        gpio9, // scl
        100.kHz(),
        &mut resets,
        125_000_000.Hz(),
    );

    let i2c_machine: I2CUnifiedMachine = I2CUnifiedMachine::new((i2c, delay, uart));
    let i2c_machine_shared: RefCell<I2CUnifiedMachine> = RefCell::new(i2c_machine);

    // TODO: No clue if this is right or not
    let executor: Executor = Executor::new();
    let executor: &'static mut Executor = make_static!(executor);

    let state: cyw43::State = cyw43::State::new();
    let state: &'static mut cyw43::State = make_static!(state);

    const DO_BLINKY: bool = true;
    const DO_JINGLE: bool = false;

    if DO_BLINKY {
        // These are implicitly used by the spi driver if they are in the correct mode
        let spi_cs: gpio::Pin<gpio::bank0::Gpio25, gpio::FunctionSpi, gpio::PullDown> =
            pins.wl_cs.into_function();

        // TODO should be high from the beginning :-(
        spi_cs.into_push_pull_output().set_high().unwrap();
        // spi_cs.into_push_pull_output().set_high().unwrap();

        let mut spi_clk = pins.voltage_monitor_wl_clk.into_push_pull_output();
        spi_clk.set_low().unwrap();

        let spi_mosi_miso: gpio::Pin<gpio::bank0::Gpio24, gpio::FunctionSpi, gpio::PullDown> =
            pins.wl_d.into_function();
        spi_mosi_miso.into_push_pull_output().set_low().unwrap();

        let mut comms = i2c_machine_shared.borrow_mut();
        writeln!(comms.uart, "lolll").unwrap();

        // let pwr: gpio::Pin<gpio::bank0::> = pins.b_power_save;
        // let pwr = Output::new(p.PIN_23, Level::Low);
        // let cs = Output::new(p.PIN_25, Level::High);
        // let mut pio = Pio::new(p.PIO0, Irqs);
        // let spi = PioSpi::new(&mut pio.common, pio.sm0, pio.irq0, cs, p.PIN_24, p.PIN_29, p.DMA_CH0);

        // let (_net_device, mut control, runner) = cyw43::new(&mut state, pins.vbus_detect.into(), spi, fw).await;

        // executor.run(|spawner: Spawner| {
        //     spawner
        //         .spawn(wifi_blinky(spawner, i2c_machine_shared, state))
        //         .unwrap();
        // })
    }

    if DO_JINGLE {
        executor.run(|spawner: Spawner| {
            // let spawn_token: SpawnToken<_> = task_pool.spawn(|| run(spawner, pins, state).into());
            // spawner.spawn(spawn_token).unwrap();
            spawner
                .spawn(run_jingle_async(spawner, i2c_machine_shared, state))
                .unwrap();
        });
    }

    loop {}

    // let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
    // let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

    // let spi = PioSpi::new(&mut pio.common, pio.sm0, pio.irq0, cs, p.PIN_24, p.PIN_29, p.DMA_CH0);

    // let (_net_device, mut control, runner) = cyw43::new(&mut state, pins.vbus_detect.into(), spi, fw).await;

    // control.init(clm).await;
    // control.set_power_management(cyw43::PowerManagementMode::PowerSave).await;

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

    const DO_DISTANCE: bool = true;
    const DELAY: u16 = 40;

    if DO_DISTANCE {
        let mut comms = i2c_machine_shared.borrow_mut();
        let distance_sensor: PiicoDevVL53L1X = PiicoDevVL53L1X::new(None, &mut comms);
        let mut led: PiicoDevRGB = PiicoDevRGB::new(&mut comms);
        let mut buzzer: PiicoDevBuzzer = PiicoDevBuzzer::new(&mut comms, Some(BuzzerVolume::Low));
        led.set_brightness(16, &mut comms).unwrap();

        let mut has_played: bool = false;

        loop {
            let distance_reading_mm: u16 = distance_sensor.read(&mut comms).unwrap();
            comms.delay(DELAY as u32);
            if writeln!(comms.uart, "Distance: {}mm", distance_reading_mm).is_err() {
                // Wiring probably isn't set up correctly
            }

            let mut note: Note = Note::A3;
            // Green light
            if distance_reading_mm < 190 {
                led.set_pixel(0, (0, 255, 0));
                note = Note::A4;
            } else {
                led.set_pixel(0, (0, 0, 0));
            }

            // Yellow light
            if distance_reading_mm < 120 {
                led.set_pixel(1, (255, 255, 0));
                note = Note::A5;
            } else {
                led.set_pixel(1, (0, 0, 0))
            }

            // Red light
            if distance_reading_mm < 60 {
                led.set_pixel(2, (255, 0, 0));
                note = Note::A6;
                if !has_played {
                    writeln!(comms.uart, "WUNESCAEB").unwrap();
                    // buzzer.play_song(&HARMONY, &mut comms);
                    has_played = true;
                }
            } else {
                led.set_pixel(2, (0, 0, 0));
            }

            if distance_reading_mm >= 190 {
                led.clear(&mut comms).unwrap();
            } else {
                led.show(&mut comms).unwrap();
                // Note is guaranteed to not be null in this flow
                // buzzer.tone(&note, DELAY, &mut comms).unwrap();
            }
        }
    }

    let mut count: u8 = 0;
    let mut i = i2c_machine_shared.borrow_mut();
    loop {
        count += 1;

        writeln!(i.uart, "FINALLY GOT SERIAL COMMUNICATIONS {}", count).unwrap();
        i.delay(1000);
    }
}

// End of file
