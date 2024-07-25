use core::cell::RefCell;

use rp_pico as bsp;

use bsp::hal::i2c::Error;
use embedded_hal::i2c::I2c;

use crate::i2c::I2CHandler;

// Peripheral address of the PiicoDev RGB device
const BASE_ADDR: u8 = 0x1E;
const _DEV_ID: u8 = 0x84;
const _REG_DEV_ID: u8 = 0x00;
const _REG_FIRM_VER: u8 = 0x01;
// Address of the LED
const REG_CTRL: u8 = 0x03;
const _REG_CLEAR: u8 = 0x04;
const _REG_I2C_ADDR: u8 = 0x05;
// Address of the brightness controller
const REG_BRIGHT: u8 = 0x06;
// Address of where to send LED colour colours
const REG_LED_VALS: u8 = 0x07;

// Red-Green-Blue properties
pub type RGB = (u8, u8, u8);

pub struct PiicoDevRGB<'i2c> {
    addr: u8,
    led: [RGB; 3],
    bright: u8,
    i2c: &'i2c RefCell<I2CHandler>,
}

impl<'i2c> PiicoDevRGB<'i2c> {
    pub fn new(i2c: &'i2c RefCell<I2CHandler>) -> Self {
        Self {
            addr: BASE_ADDR,
            led: [(0, 0, 0), (0, 0, 0), (0, 0, 0)],
            bright: 40,
            i2c,
        }
    }

    pub fn set_pixel(&mut self, n: usize, c: RGB) {
        self.led[n] = c;
    }

    // fn set_i2c_addr(&mut self, new_addr: u8) -> Result<(), Error> {
    //     i2c.write(self.addr, &[REG_I2C_ADDR, new_addr])
    // }

    pub fn show(&mut self) -> Result<(), Error> {
        let mut i2c = self.i2c.borrow_mut();

        let buffer = [
            REG_LED_VALS,
            self.led[0].0,
            self.led[0].1,
            self.led[0].2,
            self.led[1].0,
            self.led[1].1,
            self.led[1].2,
            self.led[2].0,
            self.led[2].1,
            self.led[2].2,
        ];

        i2c.write(self.addr, &buffer)
    }

    // pub fn clear(&mut self) -> Result<(), Error> {
    //     let mut i2c = self.i2c.borrow_mut();
    //
    //     i2c.write(self.addr, &[REG_CLEAR, 0x01])?;
    //     self.led = [(0, 0, 0), (0, 0, 0), (0, 0, 0)];
    //
    //     Ok(())
    // }

    // pub fn fill(&mut self, c: u8) -> Result<(), Error> {
    //     for i in 0..self.led.len() {
    //         self.led[i] = (c, c, c);
    //     }
    //
    //     self.show()
    // }

    pub fn set_brightness(&mut self, x: u8) -> Result<(), Error> {
        let mut i2c = self.i2c.borrow_mut();

        self.bright = x;
        i2c.write(self.addr, &[REG_BRIGHT, self.bright])
    }

    pub fn power_led(&mut self, state: bool) -> Result<(), Error> {
        let mut i2c = self.i2c.borrow_mut();

        let state_value: u8 = match state {
            true => 1,
            false => 0,
        };

        i2c.write(self.addr, &[REG_CTRL, state_value])
    }
}
