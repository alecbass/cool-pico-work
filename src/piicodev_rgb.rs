use crate::piicodev_unified::{I2CBase, I2CUnifiedMachine};
use rp_pico::hal::i2c;

const BASE_ADDR: u8 = 0x1E; // 0x08;
const DEV_ID: u8 = 0x84;
const REG_DEV_ID: u8 = 0x00;
const REG_FIRM_VER: u8 = 0x01;
const REG_CTRL: u8 = 0x03;
const REG_CLEAR: u8 = 0x04;
const REG_I2C_ADDR: u8 = 0x05;
const REG_BRIGHT: u8 = 0x06;
const REG_LED_VALS: u8 = 0x07;

fn wheel(h: u8, s: u8, v: u8) -> (u8, u8, u8) {
    if s == 0 {
        let v: u8 = v * 255;
        return (v, v, v);
    }

    let i: u8 = h * 6;
    let f: u8 = (h * 6) - i;
    let p: u8 = 255 * (v * (1 - s));
    let q: u8 = 255 * (v * (1 - s * f));
    let t: u8 = 255 * (v * (1 - s * (1 - f)));

    let v = v * 255;
    let i: u8 = i % 6;

    match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        5 => (v, p, q),
        _ => panic!("Cannot find rule"),
    }
}

// Colour properties. Not sure what they stand for
pub type PQV = (u8, u8, u8);

pub struct PiicoDevRGB {
    addr: u8,
    led: [PQV; 3],
    bright: u8,
}

impl PiicoDevRGB {
    pub fn new(comms: &mut I2CUnifiedMachine) -> Self {
        let mut rgb = Self {
            addr: BASE_ADDR,
            led: [(0, 0, 0), (0, 0, 0), (0, 0, 0)],
            bright: 40,
        };
        rgb.set_brightness(rgb.bright, comms).unwrap();
        rgb.show(comms).unwrap();
        rgb
    }

    pub fn set_pixel(&mut self, n: usize, c: PQV) {
        self.led[n] = c;
    }

    pub fn set_i2c_addr(
        &mut self,
        new_addr: u8,
        comms: &mut I2CUnifiedMachine,
    ) -> Result<(), i2c::Error> {
        let result = comms.write(self.addr, &[REG_I2C_ADDR, new_addr]);
        self.addr = new_addr;
        result
    }

    pub fn show(&mut self, comms: &mut I2CUnifiedMachine) -> Result<(), i2c::Error> {
        let buffer: [u8; 10] = [
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

        comms.write(self.addr, &buffer)
    }

    pub fn clear(&mut self, comms: &mut I2CUnifiedMachine) -> Result<(), i2c::Error> {
        let result = comms.write(self.addr, &[REG_CLEAR, 0x01]);
        self.led = [(0, 0, 0), (0, 0, 0), (0, 0, 0)];
        result
    }

    pub fn fill(&mut self, c: u8, comms: &mut I2CUnifiedMachine) -> Result<(), i2c::Error> {
        for i in 0..self.led.len() {
            self.led[i] = (c, c, c);
        }
        self.show(comms)
    }

    pub fn set_brightness(
        &mut self,
        x: u8,
        comms: &mut I2CUnifiedMachine,
    ) -> Result<(), i2c::Error> {
        self.bright = x;
        let result = comms.write(self.addr, &[REG_BRIGHT, self.bright]);
        comms.delay(1);

        result
    }

    pub fn power_led(
        &mut self,
        state: bool,
        comms: &mut I2CUnifiedMachine,
    ) -> Result<(), i2c::Error> {
        let state_value: u8 = match state {
            true => 1,
            false => 0,
        };
        let result = comms.write(self.addr, &[REG_CTRL, state_value]);
        comms.delay(1);

        result
    }
}
