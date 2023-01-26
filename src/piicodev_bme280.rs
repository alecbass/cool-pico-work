//
// GOING TO REVISIT THIS LATER WHEN I CAN DEBUG
//

use crate::{
    piicodev_unified::{HardwareArgs, I2CBase, I2CUnifiedMachine},
    utils::create_buffer,
};
use rp_pico::hal::i2c;

pub struct PiicoDevBME280 {
    i2c: I2CUnifiedMachine,
    t_mode: u8,
    p_mode: u8,
    h_mode: u8,
    iir: u8,
    t1: u16,
    t2: u16,
    t3: u16,
    p1: u16,
    p2: u16,
    p3: u16,
    p4: u16,
    p5: u16,
    p6: u16,
    p7: u16,
    p8: u16,
    p9: u16,
    h1: u16,
    h2: u16,
    h3: u16,
    h4: u16,
    h5: u16,
    h6: u16,
}

impl PiicoDevBME280 {
    pub fn new(args: HardwareArgs) -> Self {
        let mut i2c: I2CUnifiedMachine = I2CUnifiedMachine::new(args);

        let t_mode: u8 = 2;
        let p_mode: u8 = 5;
        let h_mode: u8 = 1;
        let iir: u8 = 1;

        let t1 = Self::read_16(0x88, &mut i2c).unwrap();
        let t2 = Self::read_16(0x8A, &mut i2c).unwrap();
        let t3 = Self::read_16(0x8C, &mut i2c).unwrap();

        let p1 = Self::read_16(0x8E, &mut i2c).unwrap();
        let p2 = Self::read_16(0x90, &mut i2c).unwrap();
        let p3 = Self::read_16(0x92, &mut i2c).unwrap();
        let p4 = Self::read_16(0x94, &mut i2c).unwrap();
        let p5 = Self::read_16(0x96, &mut i2c).unwrap();
        let p6 = Self::read_16(0x98, &mut i2c).unwrap();
        let p7 = Self::read_16(0x9A, &mut i2c).unwrap();
        let p8 = Self::read_16(0x9C, &mut i2c).unwrap();
        let p9 = Self::read_16(0x9E, &mut i2c).unwrap();

        let h1 = Self::read_8(0xE5, &mut i2c).unwrap() as u16;
        let h2 = Self::read_16(0xE1, &mut i2c).unwrap();
        let h3 = Self::read_8(0xE3, &mut i2c).unwrap() as u16;
        let h4 = Self::read_8(0xE4, &mut i2c).unwrap() as u16;
        let h5 = Self::read_8(0xE6, &mut i2c).unwrap() as u16;
        let h6 = Self::read_8(0xE7, &mut i2c).unwrap() as u16;

        i2c.write(i2c.addr, &[0xF2, h_mode]).unwrap();
        i2c.delay(2);
        i2c.write(i2c.addr, &[0xF4, 0x24]).unwrap();
        i2c.delay(2);
        i2c.write(i2c.addr, &[0xF5, iir << 2]).unwrap();

        Self {
            i2c,
            t_mode,
            p_mode,
            h_mode,
            iir,
            t1,
            t2,
            t3,
            p1,
            p2,
            p3,
            p4,
            p5,
            p6,
            p7,
            p8,
            p9,
            h1,
            h2,
            h3,
            h4,
            h5,
            h6,
        }
    }

    pub(self) fn read_8(reg: u8, i2c: &mut I2CUnifiedMachine) -> Result<u8, i2c::Error> {
        let mut buffer: [u8; 16] = create_buffer();

        i2c.write(i2c.addr, &[reg]).unwrap();

        match i2c.read(i2c.addr, &mut buffer) {
            Ok(()) => Ok(buffer[0]),
            Err(e) => Err(e),
        }
    }

    pub(self) fn read_16(reg: u8, i2c: &mut I2CUnifiedMachine) -> Result<u16, i2c::Error> {
        let mut buffer: [u8; 16] = create_buffer();

        i2c.write(i2c.addr, &[reg]).unwrap();

        match i2c.read(i2c.addr, &mut buffer) {
            Ok(()) => Ok(buffer[0] as u16 + buffer[1] as u16 * 256),
            Err(e) => Err(e),
        }
    }

    pub fn read_raw_data(&mut self) -> (u16, u16, u16) {
        let low_amounts: [u8; 5] = [1, 2, 3, 4, 5];
        let mut sleep_time: u32 = 1250;

        if low_amounts.contains(&self.t_mode) {
            sleep_time += 2300 * (1 << self.t_mode);
        }

        if low_amounts.contains(&self.p_mode) {
            sleep_time += 575 + (2300 * (1 << self.p_mode));
        }

        if low_amounts.contains(&self.h_mode) {
            sleep_time += 575 + (2300 * (1 << self.h_mode))
        }

        self.i2c.delay(1 + sleep_time / 1000);

        while (Self::read_16(0xF3, &mut self.i2c).unwrap() & 0x08) != 0 {
            self.i2c.delay(1);
        }

        let raw_p: u16 = ((Self::read_8(0xF7, &mut self.i2c).unwrap() << 16) as u16
            | (Self::read_8(0xF8, &mut self.i2c).unwrap() << 8) as u16
            | Self::read_8(0xF9, &mut self.i2c).unwrap() as u16)
            >> 4;
        let raw_t: u16 = ((Self::read_8(0xFA, &mut self.i2c).unwrap() << 16) as u16
            | (Self::read_8(0xFB, &mut self.i2c).unwrap() << 8) as u16
            | Self::read_8(0xFC, &mut self.i2c).unwrap() as u16)
            >> 4;
        let raw_h: u16 = (Self::read_8(0xFD, &mut self.i2c).unwrap() << 8) as u16
            | Self::read_8(0xFE, &mut self.i2c).unwrap() as u16;

        (raw_p, raw_t, raw_h)
    }

    pub fn read_compensated_data(&mut self) -> (u32, u32, u32) {
        let (raw_t, raw_p, raw_h) = self.read_raw_data();

        let mut var1: u16 = ((raw_t >> 3) - (self.t1 << 1)) * (self.t2 >> 11);
        let mut var2: u16 = (raw_t >> 4) - self.t1;
        var2 = var2 * ((raw_t >> 4) - self.t1);
        var2 = ((var2 >> 12) * self.t3) >> 14;
        let t_fine: u16 = var1 + var2;

        let temp = (t_fine * 5 + 128) >> 8;
        var1 = t_fine - 128000;
        var2 = var1 * var1 * self.p6;
        var2 = var2 + ((var1 * self.p5) << 17);
        var2 = var2 + (self.p4 << 35);
        var1 = ((var1 * var1 * self.p3) >> 8) + ((var1 * self.p2) << 12);
        var1 = (((1 << 47) + var1) * self.p1) >> 33;

        let pres: u32 = if var1 == 0 {
            0
        } else {
            let p: u32 = (((1048576 - raw_p) << 31) - var2) * 3125;
            var1 = (self.p9 * (p >> 13) * (p >> 13)) >> 25;
            var2 = (self.p8 * p) >> 19;
            ((p + var1 + var2) >> 8) + (self.p7 << 4)
        };

        let mut h: u16 = t_fine - 76800;
        h = ((((raw_h << 14) - (self.h4 << 20) - (self.h5 * h)) + 16384) >> 15)
            * (((((((h * self.h6) >> 10) * (((h * self.h3) >> 11) + 32768)) >> 10) + 2097152)
                * self.h2
                + 8192)
                >> 14);
        h = h - (((((h >> 15) * (h >> 15)) >> 7) * self.h1) >> 4);
        if h < 0 {
            h = 0;
        }

        if h > 419430400 {
            h = 419430400;
        }

        let humi: u16 = h >> 12;
        (temp, pres, humi)
    }

    pub fn values(&mut self) -> (u16, u16, u16) {
        let (temp, pres, humi) = self.read_compensated_data();
        (temp / 100, pres / 256, humi / 1024)
    }

    pub fn pressure_precision(&mut self) -> (f32, u16) {
        let p: u16 = self.read_compensated_data().1;
        let pi: f32 = (p / 256) as f32;
        let pd: u16 = (p % 256) / 256;
        (pi, pd)
    }

    pub fn altitude(&mut self, pressure_sea_level: Option<f32>) -> f32 {
        /** Bad method for exponentiation */
        fn power_float(val: f32, amount: f32) -> f32 {
            let mut result: f32 = 1.0;

            for i in 1..amount as u32 {
                result *= val;
            }

            result
        }

        let (pi, pd) = self.pressure_precision();
        let inner: f32 = ((pi + pd as f32) / 100.0) / pressure_sea_level.unwrap_or(1013.25);
        44330.0 * (1.0 - power_float(inner, 1.0 / 5.255))
    }
}
