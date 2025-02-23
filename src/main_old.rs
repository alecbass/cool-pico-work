//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

mod byte_reader;
// mod gspi;
mod piicodev_bme280;
mod piicodev_buzzer;
mod piicodev_rgb;
mod piicodev_ssd1306;
mod piicodev_unified;
mod piicodev_vl53l1x;
mod pins;
// mod time;
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
use cortex_m::delay::Delay;
use embassy_executor::{Executor, Spawner};
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::pio::{InterruptHandler, Pio};
use fugit::RateExtU32;
use piicodev_buzzer::notes::Note;
use piicodev_buzzer::piicodev_buzzer::{BuzzerVolume, PiicoDevBuzzer};
use piicodev_unified::{I2CUnifiedMachine, GPIO89I2C};
use piicodev_vl53l1x::piicodev_vl53l1x::PiicoDevVL53L1X;
use reqwless::client::HttpClient;
use reqwless::request::{Method, Request, RequestBuilder};
use static_cell::make_static;

const FLASH_TIMERS: &[u32] = &[200, 1000, 100, 500];

// #[embassy_executor::task]
// async fn run_jingle_async(
//     spawner: Spawner,
//     comms: RefCell<I2CUnifiedMachine>,
//     state: &'static cyw43::State,
// ) -> ! {
//     const DELAY: u16 = 40;
//     let mut comms = comms.borrow_mut();
//     let distance_sensor: PiicoDevVL53L1X = PiicoDevVL53L1X::new(None, &mut comms);
//     let mut led: PiicoDevRGB = PiicoDevRGB::new(&mut comms);
//     let mut buzzer: PiicoDevBuzzer = PiicoDevBuzzer::new(&mut comms, Some(BuzzerVolume::Low));
//     led.set_brightness(16, &mut comms).unwrap();
//
//     let mut has_played: bool = false;
//
//     loop {
//         let distance_reading_mm: u16 = distance_sensor.read(&mut comms).unwrap();
//         comms.delay(DELAY as u32);
//         if writeln!(comms.uart, "Distance: {}mm", distance_reading_mm).is_err() {
//             // Wiring probably isn't set up correctly
//         }
//
//         let mut note: Note = Note::A3;
//         // Green light
//         if distance_reading_mm < 190 {
//             led.set_pixel(0, (0, 255, 0));
//             note = Note::A4;
//         } else {
//             led.set_pixel(0, (0, 0, 0));
//         }
//
//         // Yellow light
//         if distance_reading_mm < 120 {
//             led.set_pixel(1, (255, 255, 0));
//             note = Note::A5;
//         } else {
//             led.set_pixel(1, (0, 0, 0))
//         }
//
//         // Red light
//         if distance_reading_mm < 60 {
//             led.set_pixel(2, (255, 0, 0));
//             note = Note::A6;
//             if !has_played {
//                 writeln!(comms.uart, "WUNESCAEB").unwrap();
//                 // buzzer.play_song(&HARMONY, &mut comms);
//                 has_played = true;
//             }
//         } else {
//             led.set_pixel(2, (0, 0, 0));
//         }
//
//         if distance_reading_mm >= 190 {
//             led.clear(&mut comms).unwrap();
//         } else {
//             led.show(&mut comms).unwrap();
//             // Note is guaranteed to not be null in this flow
//             // buzzer.tone(&note, DELAY, &mut comms).unwrap();
//         }
//     }
// }
//
// bind_interrupts!(struct Irqs {
//     PIO0_IRQ_0 => InterruptHandler<embassy_rp::peripherals::PIO0>;
// });
//
// #[embassy_executor::task]
// async fn wifi_task(
//     runner: cyw43::Runner<
//         'static,
//         Output<'static, embassy_rp::peripherals::PIN_23>,
//         cyw43_pio::PioSpi<
//             'static,
//             embassy_rp::peripherals::PIN_25,
//             embassy_rp::peripherals::PIO0,
//             0,
//             embassy_rp::peripherals::DMA_CH0,
//         >,
//     >,
// ) -> ! {
//     runner.run().await
// }
//
// #[embassy_executor::task]
// async fn net_task(stack: &'static embassy_net::Stack<cyw43::NetDriver<'static>>) -> ! {
//     stack.run().await
// }
//
// // TODO: Make into its own file
// struct Message<'a> {
//     current_len: usize,
//     message: &'a mut [u8],
// }
//
// impl<'a> Write for Message<'a> {
//     fn write_str(&mut self, s: &str) -> core::fmt::Result {
//         let str_bytes: &[u8] = s.as_bytes();
//         // self.current_len = str_bytes.len();
//
//         // for i in 0..self.current_len {
//         //     self.message[i] = str_bytes[i];
//         // }
//
//         // Skip over already-copied data
//         let remainder = &mut self.message[self.current_len..];
//         // Check if there is space remaining (return error instead of panicking)
//         if remainder.len() < str_bytes.len() {
//             return Err(core::fmt::Error);
//         }
//         // Make the two slices the same length
//         let remainder = &mut remainder[..str_bytes.len()];
//         // Copy
//         remainder.copy_from_slice(str_bytes);
//
//         // Update offset to avoid overwriting
//         self.current_len += str_bytes.len();
//
//         Ok(())
//     }
// }
//
// impl<'a> Message<'a> {
//     pub fn reset(&mut self) {
//         self.current_len = 0;
//         self.message.copy_from_slice(&[0; 128]);
//     }
// }
//
// #[cfg(target_arch = "x86_64")]
// #[embassy_executor::task]
// async fn wifi_blinky(
//     spawner: Spawner,
//     comms: RefCell<I2CUnifiedMachine>,
//     state: &'static mut cyw43::State,
//     embassy_peripherals: embassy_rp::Peripherals,
// ) -> ! {
//     let fw: &[u8] = include_bytes!("../cyw43-firmware/43439A0.bin");
//     let clm: &[u8] = include_bytes!("../cyw43-firmware/43439A0_clm.bin");
//     let mut comms = comms.borrow_mut();
//     writeln!(comms.uart, "hehehhe").unwrap();
//
//     let pwr = Output::new(embassy_peripherals.PIN_23, Level::Low);
//     let cs = Output::new(embassy_peripherals.PIN_25, Level::High);
//     let mut pio = Pio::new(embassy_peripherals.PIO0, Irqs);
//     let spi = cyw43_pio::PioSpi::new(
//         &mut pio.common,
//         pio.sm0,
//         pio.irq0,
//         cs,
//         embassy_peripherals.PIN_24,
//         embassy_peripherals.PIN_29,
//         embassy_peripherals.DMA_CH0,
//     );
//
//     use embassy_futures::yield_now;
//     yield_now().await;
//
//     let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
//     defmt::unwrap!(spawner.spawn(wifi_task(runner)));
//
//     control.init(clm).await;
//     control
//         .set_power_management(cyw43::PowerManagementMode::PowerSave)
//         .await;
//
//     let config = embassy_net::Config::dhcpv4(Default::default());
//     //let config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
//     //    address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 69, 2), 24),
//     //    dns_servers: Vec::new(),
//     //    gateway: Some(Ipv4Address::new(192, 168, 69, 1)),
//     //});
//
//     // Generate random seed
//     let seed = 0x0123_4567_89ab_cdef; // chosen by fair dice roll. guarenteed to be random.
//
//     // Init network stack
//     let stack = &*make_static!(embassy_net::Stack::new(
//         net_device,
//         config,
//         make_static!(embassy_net::StackResources::<2>::new()),
//         seed
//     ));
//
//     defmt::unwrap!(spawner.spawn(net_task(stack)));
//
//     // let mut scanner = control.scan().await;
//     // while let Some(bss) = scanner.next().await {
//     //     if let Ok(ssid_str) = core::str::from_utf8(&bss.ssid) {
//     //         writeln!(comms.uart, "scanned {} == {:?}", ssid_str, bss.bssid).unwrap();
//     //     }
//     // }
//
//     let wifi_network: &'static str = option_env!("WIFI_NETWORK").unwrap();
//     let wifi_password: &'static str = option_env!("WIFI_PASSWORD").unwrap();
//
//     loop {
//         //control.join_open(WIFI_NETWORK).await;
//         match control.join_wpa2(wifi_network, wifi_password).await {
//             Ok(_) => break,
//             Err(err) => {
//                 writeln!(comms.uart, "join failed with status={:?}", err).unwrap();
//             }
//         }
//     }
//
//     // And now we can use it!
//     writeln!(comms.uart, "We're in!").unwrap();
//
//     let mut rx_buffer = [0; 4096];
//     let mut tx_buffer = [0; 4096];
//     let mut buf: [u8; 4096] = [0; 4096];
//
//     let addr: embassy_net::IpEndpoint = embassy_net::IpEndpoint::new(
//         embassy_net::IpAddress::Ipv4(embassy_net::Ipv4Address([192, 168, 137, 1])),
//         40000,
//     );
//
//     const URL: &'static str = "http://192.168.0.12:40000/details";
//
//     // let request = Request::get(URL).build();
//     // embedded_io_async::Write::write(&mut request, buf);
//
//     loop {
//         let mut socket = embassy_net::tcp::TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
//         socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));
//
//         control.gpio_set(0, true).await;
//
//         if let Err(e) = socket.connect(addr).await {
//             writeln!(comms.uart, "connect error: {:?} {:?}", addr, e).unwrap();
//             comms.delay(2000);
//             continue;
//         }
//
//         writeln!(comms.uart, "Writing HTTP Request").unwrap();
//
//         control.gpio_set(0, false).await;
//
//         // let mut client = HttpClient::new(&socket, StaticDns); // Types implementing embedded-nal-async
//         // let mut rx_buf = [0; 4096];
//         // let response = client
//         //     .request(Method::POST, &url)
//         //     .await
//         //     .unwrap()
//         //     .body(b"PING")
//         //     .content_type(ContentType::TextPlain)
//         //     .send(&mut rx_buf)
//         //     .await
//         //     .unwrap();
//
//         let mut message: Message = Message {
//             current_len: 0,
//             message: &mut [0; 128],
//         };
//         writeln!(comms.uart, "Initiating read-write").unwrap();
//         loop {
//             // request.write(&mut buf);
//
//             let message_content: &str = "{ \"message\": \"Hello\" }";
//             let message_content_bytes: usize = message_content.len();
//
//             if let Err(e) = write!(message, "POST /details HTTP/1.1\r\nHost:192.168.137.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}\r\n", message_content_bytes, message_content) {
//                 writeln!(comms.uart, "Message write error: {:?}", e).unwrap();
//                 continue;
//             }
//             // let message: &str = core::format_args!(
//             //     "POST /details HTTP/1.1\r\nHost:192.168.137.1\r\nContent-Type: application/json\r\nContent-Length: 22\r\n\r\n{}\r\n", message_content
//             // );
//             // .unwrap(); //"POST /details/ HTTP/1.1\r\nHost:192.168.137.1\r\n\r\n";
//             // writeln!(comms.uart, "{}", message.message).unwrap();
//             let to_write: &[u8] = &message.message;
//
//             let test_str: &str = core::str::from_utf8(to_write).unwrap();
//             writeln!(comms.uart, "Message: {}", test_str).unwrap();
//
//             match embedded_io_async::Write::write_all(&mut socket, to_write).await {
//                 Ok(()) => {}
//                 Err(e) => {
//                     writeln!(comms.uart, "write error: {:?}", e).unwrap();
//                     comms.delay(2000);
//                     break;
//                 }
//             };
//
//             comms.delay(1000);
//
//             let bytes_read = match socket.read(&mut buf).await {
//                 Ok(0) => {
//                     writeln!(comms.uart, "read EOF").unwrap();
//                     break;
//                 }
//                 Ok(n) => n,
//                 Err(e) => {
//                     writeln!(comms.uart, "read error: {:?}", e).unwrap();
//                     break;
//                 }
//             };
//
//             writeln!(
//                 comms.uart,
//                 "rxd {}",
//                 core::str::from_utf8(&buf[..bytes_read]).unwrap()
//             )
//             .unwrap();
//
//             message.reset();
//         }
//     }
// }

#[entry]
fn main() -> ! {
    let embassy_peripherals = embassy_rp::init(Default::default());
    let mut peripherals = pac::Peripherals::take().expect("Cannot take peripherals");

    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog: Watchdog = Watchdog::new(peripherals.WATCHDOG);
    let sio: Sio = Sio::new(peripherals.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    const EXTERNAL_XTAL_FREQ_HZ: u32 = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        EXTERNAL_XTAL_FREQ_HZ,
        peripherals.XOSC,
        peripherals.CLOCKS,
        peripherals.PLL_SYS,
        peripherals.PLL_USB,
        &mut peripherals.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let delay: Delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

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

        // let spi_cs: SpiCsPin = pins.wl_cs.into_function();

        // // TODO should be high from the beginning :-(
        // spi_cs.into_push_pull_output_in_state(gpio::PinState::High);

        // let mut spi_clk: SpiClkPin = pins
        //     .voltage_monitor_wl_clk
        //     .into_push_pull_output_in_state(gpio::PinState::Low);

        // let spi_mosi_miso: SpiMosiMisoPin = pins.wl_d.into_function();
        // spi_mosi_miso.into_push_pull_output_in_state(gpio::PinState::Low);

        // let pwr: PowerPin = pins.wl_on.into_push_pull_output();

        // let mut pio = Pio::new(p.PIO0, Irqs);
        // let spi = PioSpi::new(
        //     &mut pio.common,
        //     pio.sm0,
        //     pio.irq0,
        //     cs,
        //     p.PIN_24,
        //     p.PIN_29,
        //     p.DMA_CH0,
        // );

        // let bus = GSpi::new(spi_clk, spi_mosi_miso);
        // let spi: ExclusiveDevice<GSpi, SpiCsPin, embedded_hal_bus::spi::NoDelay> =
        //     ExclusiveDevice::new_no_delay(bus, spi_cs);

        executor.run(|spawner: Spawner| {
            spawner
                .spawn(wifi_blinky(
                    spawner,
                    i2c_machine_shared,
                    state,
                    embassy_peripherals,
                ))
                .unwrap();
        })
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

    // const DO_DISTANCE: bool = true;
    // const DELAY: u16 = 40;

    // if DO_DISTANCE {
    //     let mut comms = i2c_machine_shared.borrow_mut();
    //     let distance_sensor: PiicoDevVL53L1X = PiicoDevVL53L1X::new(None, &mut comms);
    //     let mut led: PiicoDevRGB = PiicoDevRGB::new(&mut comms);
    //     let mut buzzer: PiicoDevBuzzer = PiicoDevBuzzer::new(&mut comms, Some(BuzzerVolume::Low));
    //     led.set_brightness(16, &mut comms).unwrap();

    //     let mut has_played: bool = false;

    //     loop {
    //         let distance_reading_mm: u16 = distance_sensor.read(&mut comms).unwrap();
    //         comms.delay(DELAY as u32);
    //         if writeln!(comms.uart, "Distance: {}mm", distance_reading_mm).is_err() {
    //             // Wiring probably isn't set up correctly
    //         }

    //         let mut note: Note = Note::A3;
    //         // Green light
    //         if distance_reading_mm < 190 {
    //             led.set_pixel(0, (0, 255, 0));
    //             note = Note::A4;
    //         } else {
    //             led.set_pixel(0, (0, 0, 0));
    //         }

    //         // Yellow light
    //         if distance_reading_mm < 120 {
    //             led.set_pixel(1, (255, 255, 0));
    //             note = Note::A5;
    //         } else {
    //             led.set_pixel(1, (0, 0, 0))
    //         }

    //         // Red light
    //         if distance_reading_mm < 60 {
    //             led.set_pixel(2, (255, 0, 0));
    //             note = Note::A6;
    //             if !has_played {
    //                 writeln!(comms.uart, "WUNESCAEB").unwrap();
    //                 // buzzer.play_song(&HARMONY, &mut comms);
    //                 has_played = true;
    //             }
    //         } else {
    //             led.set_pixel(2, (0, 0, 0));
    //         }

    //         if distance_reading_mm >= 190 {
    //             led.clear(&mut comms).unwrap();
    //         } else {
    //             led.show(&mut comms).unwrap();
    //             // Note is guaranteed to not be null in this flow
    //             // buzzer.tone(&note, DELAY, &mut comms).unwrap();
    //         }
    //     }
    // }

    // let mut count: u8 = 0;
    // let mut i = i2c_machine_shared.borrow_mut();
    // loop {
    //     count += 1;

    //     writeln!(i.uart, "FINALLY GOT SERIAL COMMUNICATIONS {}", count).unwrap();
    //     i.delay(1000);
    // }

    loop {}
}

// End of file
