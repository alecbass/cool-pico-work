use rp_pico as bsp;

use bsp::hal::gpio::{FunctionUart, Pin, PullNone};
use bsp::hal::uart::{Enabled, UartPeripheral};
use bsp::pac::UART0;

pub type UartPins<T1: PinId, T2: PinId> = (
    Pin<T1, FunctionUart, PullNone>,
    Pin<T2, FunctionUart, PullNone>,
);

/// Alias the type for our UART to make things clearer.
pub type Uart<T1: PinId, T2: PinId> = UartPeripheral<Enabled, UART0, UartPins<T1, T2>>;

use rp_pico::hal::gpio::{Function, PinId, PullType};

struct MyDevice<I, F, T>
where
    I: PinId,
    F: Function,
    T: PullType,
{
    pin: Pin<I, F, T>,
}

impl<I, F, T> MyDevice<I, F, T>
where
    I: PinId,
    F: Function,
    T: PullType,
{
    pub fn new(pin: Pin<I, F, T>) -> Self {
        Self { pin }
    }

    // Methods that use the pin
    pub fn do_something(&mut self) {
        // Pin operations here
    }
}
