use rp_pico as bsp;

use bsp::hal::gpio::bank0::{Gpio0, Gpio1};
use bsp::hal::gpio::{FunctionUart, Pin, PullNone};
use bsp::hal::uart::{Enabled, UartPeripheral};
use bsp::pac::UART0;

pub type UartPins = (
    Pin<Gpio0, FunctionUart, PullNone>,
    Pin<Gpio1, FunctionUart, PullNone>,
);

/// Alias the type for our UART to make things clearer.
pub type Uart = UartPeripheral<Enabled, UART0, UartPins>;
