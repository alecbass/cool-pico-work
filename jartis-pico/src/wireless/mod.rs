use core::fmt::Write;

use cortex_m::delay::Delay;
use fugit::RateExtU32;
use jartis::uart::{Uart, UartPins};
use rp_pico::Pins;
use rp_pico::hal::Clock;
use rp_pico::hal::clocks::ClocksManager;
use rp_pico::hal::gpio::{
    FunctionUart, PullNone,
    bank0::{Gpio0, Gpio1},
};
use rp_pico::hal::uart::UartPeripheral;
use rp_pico::hal::uart::{DataBits, StopBits, UartConfig};
use rp_pico::pac::{RESETS, UART0};

#[link(name = "jartis", kind = "static")]
unsafe extern "C" {
    fn connectToWifi() -> i32;
}

pub fn wireless_main(
    uart_device: UART0,
    resets: &mut RESETS,
    clocks: ClocksManager,
    pins: Pins,
    mut delay: Delay,
) {
    let uart_pins: UartPins<Gpio0, Gpio1> = (
        // UART TX (characters sent from RP2040) on pin 1 (GPIO0)
        pins.gpio0.reconfigure::<FunctionUart, PullNone>(),
        // UART RX (characters received by RP2040) on pin 2 (GPIO1)
        pins.gpio1.reconfigure::<FunctionUart, PullNone>(),
    );

    let mut uart: Uart<Gpio0, Gpio1> = UartPeripheral::new(uart_device, uart_pins, resets)
        .enable(
            UartConfig::new(9600_u32.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    unsafe {
        loop {
            writeln!(uart, "Connecting to wifi").unwrap();
            // delay.delay_ms(200);

            let r = connectToWifi();
            writeln!(uart, "Connect to wifi result: {r}").unwrap();
        }
    }

    // Configure CYW43 pins using rp2040_hal
    // let pwr = pins.gpio23.into_push_pull_output();
    // let cs = pins.gpio29.into_function::<FunctionPio0>();
    // let dio = pins.gpio24.into_function::<FunctionPio0>();
    // let clk = pins.gpio25.into_function::<FunctionPio0>();
    //
    // let spi = PioSpi::new(
    //     &mut pio.common,
    //     pio.sm0,
    //     DEFAULT_CLOCK_DIVIDER,
    //     pio.irq0,
    //     cs,
    //     p.PIN_24,
    //     p.PIN_29,
    //     p.DMA_CH0,
    // );
    //
    // let (_net_device, mut control, runner) = cyw43::new(state, pwr, spi, firmware);
}
