use core::fmt::Write;

use cortex_m::delay::Delay;
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embedded_hal::digital::OutputPin;
use fugit::RateExtU32;
use jartis::uart::{Uart, UartPins};
use rp_pico::Pins;
use rp_pico::hal::Clock;
use rp_pico::hal::clocks::ClocksManager;
use rp_pico::hal::gpio::bank0::Gpio23;
use rp_pico::hal::gpio::{FunctionSioOutput, Pin, PullDown, PullType};
use rp_pico::hal::gpio::{
    FunctionUart, PullNone,
    bank0::{Gpio0, Gpio1},
};
use rp_pico::hal::uart::UartPeripheral;
use rp_pico::hal::uart::{DataBits, StopBits, UartConfig};
use rp_pico::pac::{RESETS, UART0};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<
        'static,
        Pin<Gpio23, FunctionSioOutput, PullDown>,
        PioSpi<'static, PIO0, 0, DMA_CH0>,
    >,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
pub async fn wireless_main(
    spawner: Spawner,
    uart_device: UART0,
    mut resets: RESETS,
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

    let mut uart: Uart<Gpio0, Gpio1> = UartPeripheral::new(uart_device, uart_pins, &mut resets)
        .enable(
            UartConfig::new(9600_u32.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    writeln!(uart, "hiiii yeah").unwrap();

    let fw = include_bytes!("../../../cyw43/43439A0.bin");
    let clm = include_bytes!("../../../cyw43/43439A0_clm.bin");

    // writeln!(uart, "{fw:?}").unwrap();
    // writeln!(uart, "{clm:?}").unwrap();

    for i in 0..5 {
        writeln!(uart, "hiiii {i}").unwrap();
    }

    let p = embassy_rp::init(Default::default());
    // let pwr = Output::new(p.PIN_23, Level::Low); // embassy
    let mut pwr = pins.b_power_save.into_push_pull_output(); // rp-pico
    pwr.set_low().unwrap();
    let cs = Output::new(p.PIN_25, Level::High); // embassy
    // let cs = pins.led.into_push_pull_output().set_high(); // rp-pico
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    writeln!(uart, "again!!!!").unwrap();

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (_net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    unwrap!(spawner.spawn(cyw43_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    loop {
        writeln!(uart, "hiiiiee").unwrap();
        delay.delay_ms(250);
        // info!("led on!");
        // control.gpio_set(0, true).await;
        // Timer::after(delay).await;
        //
        // info!("led off!");
        // control.gpio_set(0, false).await;
        // Timer::after(delay).await;
    }
}
