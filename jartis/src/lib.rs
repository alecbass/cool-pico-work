//! Jartis library
#![no_std]

use core::cell::RefCell;
use core::fmt::Write;

use cortex_m::delay::Delay;
use defmt::*;
use defmt_rtt as _;
use embedded_graphics::mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::text::{Alignment, Text};
use embedded_hal::digital::InputPin;
use embedded_hal::pwm::SetDutyCycle;
use fugit::RateExtU32;
use panic_probe as _;

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use rp_pico as bsp;
// use sparkfun_pro_micro_rp2040 as bsp;

use bsp::entry;
use bsp::hal::clocks::{init_clocks_and_plls, Clock};
use bsp::hal::gpio::{FunctionI2C, FunctionUart, PullNone, PullUp};
use bsp::hal::i2c::I2C;
use bsp::hal::pac;
use bsp::hal::pwm::Slices;
use bsp::hal::sio::Sio;
use bsp::hal::uart::{DataBits, StopBits, UartConfig, UartPeripheral};
use bsp::hal::watchdog::Watchdog;
use bsp::hal::Timer;
use bsp::Pins;
use servo::Servo;
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

use i2c::I2CHandler;
use piicodev_bme280::piicodev_bme280::PiicoDevBME280;
use piicodev_buzzer::notes::HARMONY;
use piicodev_buzzer::piicodev_buzzer::{BuzzerVolume, PiicoDevBuzzer};
use piicodev_qmc6310::{GaussRange, PiicoDevQMC6310};
use piicodev_rgb::piicodev_rgb::PiicoDevRGB;
use piicodev_ssd1306::{OLEDColour, PiicoDevSSD1306};
use piicodev_vl53l1x::piicodev_vl53l1x::PiicoDevVL53L1X;
use uart::{Uart, UartPins};

pub mod i2c;
pub mod piicodev_bme280;
pub mod piicodev_buzzer;
pub mod piicodev_qmc6310;
pub mod piicodev_rgb;
pub mod piicodev_ssd1306;
pub mod piicodev_vl53l1x;
pub mod servo;
pub mod uart;
