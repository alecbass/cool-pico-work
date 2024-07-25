use rp_pico as bsp;

use bsp::hal::gpio::bank0::{Gpio8, Gpio9};
use bsp::hal::gpio::{FunctionI2C, Pin, PullUp};
use bsp::hal::i2c::I2C;
use bsp::pac::I2C0;

pub type I2CHandler = I2C<
    I2C0,
    (
        Pin<Gpio8, FunctionI2C, PullUp>,
        Pin<Gpio9, FunctionI2C, PullUp>,
    ),
>;
