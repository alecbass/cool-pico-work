use rp_pico as bsp;

use bsp::hal::gpio::{FunctionUart, Pin, PinId, PullNone};
use bsp::hal::uart::{Enabled, UartPeripheral};
use bsp::pac::UART0;

pub type UartPins<T1: PinId, T2: PinId> = (
    Pin<T1, FunctionUart, PullNone>,
    Pin<T2, FunctionUart, PullNone>,
);

/// Alias the type for our UART to make things clearer.
pub type Uart<T1: PinId, T2: PinId> = UartPeripheral<Enabled, UART0, UartPins<T1, T2>>;
