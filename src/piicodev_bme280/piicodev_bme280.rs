use crate::{
    byte_reader::ByteReader,
    piicodev_unified::{I2CBase, I2CUnifiedMachine},
};
use core::cell::{RefCell, RefMut};
use libm::powf;

const BASE_ADDR: u8 = 0x77;

/** A tuple representing temperature, pressure and humidity readings */
type TempPresHumi = (i64, i64, i64);

pub struct PiicoDevBME280<'a> {
    addr: u8,
    i2c: &'a RefCell<I2CUnifiedMachine>,
    t_mode: i64,
    p_mode: i64,
    h_mode: i64,
    iir: i64,
    t1: i64,
    t2: i64,
    t3: i64,
    p1: i64,
    p2: i64,
    p3: i64,
    p4: i64,
    p5: i64,
    p6: i64,
    p7: i64,
    p8: i64,
    p9: i64,
    h1: i64,
    h2: i64,
    h3: i64,
    h4: i64,
    h5: i64,
    h6: i64,
}

impl<'a> PiicoDevBME280<'a> {
    pub fn new(i2c_cell: &'a RefCell<I2CUnifiedMachine>) -> Self {
        fn short(num: i64) -> i64 {
            if num > 32767 {
                return num - 65537;
            }

            num
        }

        let mut i2c: RefMut<I2CUnifiedMachine> = i2c_cell.borrow_mut();

        // NOTE: This can be set up to be dynamic
        let addr: u8 = BASE_ADDR;

        let t_mode: i64 = 2;
        let p_mode: i64 = 5;
        let h_mode: i64 = 1;
        let iir: i64 = 1;

        // The Piicodev libraries expect Python 32-bit integers, so while these number casts
        // seem inefficient, it's to mimic the expected behaviour
        let t1: i64 = Self::read_16(addr, 0x88, &mut i2c).unwrap() as i64;
        let t2: i64 = Self::read_16(addr, 0x8A, &mut i2c).unwrap() as i64;
        let t3: i64 = Self::read_16(addr, 0x8C, &mut i2c).unwrap() as i64;

        let p1: i64 = Self::read_16(addr, 0x8E, &mut i2c).unwrap() as i64;
        let p2: i64 = short(Self::read_16(addr, 0x90, &mut i2c).unwrap() as i64);
        let p3: i64 = short(Self::read_16(addr, 0x92, &mut i2c).unwrap() as i64);
        let p4: i64 = short(Self::read_16(addr, 0x94, &mut i2c).unwrap() as i64);
        let p5: i64 = short(Self::read_16(addr, 0x96, &mut i2c).unwrap() as i64);
        let p6: i64 = short(Self::read_16(addr, 0x98, &mut i2c).unwrap() as i64);
        let p7: i64 = short(Self::read_16(addr, 0x9A, &mut i2c).unwrap() as i64);
        let p8: i64 = short(Self::read_16(addr, 0x9C, &mut i2c).unwrap() as i64);
        let p9: i64 = short(Self::read_16(addr, 0x9E, &mut i2c).unwrap() as i64);

        let h1: i64 = Self::read_8(addr, 0xA1, &mut i2c).unwrap() as i64;
        let h2: i64 = Self::read_16(addr, 0xE1, &mut i2c).unwrap() as i64;
        let h3: i64 = Self::read_8(addr, 0xE3, &mut i2c).unwrap() as i64;
        let a: i64 = Self::read_8(addr, 0xE5, &mut i2c).unwrap() as i64;
        let h4: i64 = ((Self::read_8(addr, 0xE4, &mut i2c).unwrap() as i64) << 4) + (a % 16);
        let h5: i64 = ((Self::read_8(addr, 0xE6, &mut i2c).unwrap() as i64) << 4) + (a >> 4);
        let mut h6: i64 = Self::read_8(addr, 0xE7, &mut i2c).unwrap() as i64;

        if h6 > 127 {
            h6 -= 256;
        }

        i2c.write(addr, &[0xF2, h_mode as u8]).unwrap();
        i2c.delay(2);
        i2c.write(addr, &[0xF4, 0x24]).unwrap();
        i2c.delay(2);
        i2c.write(addr, &[0xF5, (iir as u8) << 2]).unwrap();

        Self {
            addr,
            i2c: i2c_cell,
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

    pub fn read_raw_data(&mut self) -> TempPresHumi {
        let mut i2c: RefMut<I2CUnifiedMachine> = self.i2c.borrow_mut();

        // Trigger the module to take a measurement
        // The PiicoDev _write8 method just wraps bytes into buffers and writes them
        // self._write8(0xF4, (self.p_mode << 5 | self.t_mode << 2 | 1))
        i2c.write(
            self.addr,
            &[
                0xF4,
                ((self.p_mode << 5) as u8 | (self.t_mode << 2) as u8 | 1),
            ],
        )
        .unwrap();

        const LOW_AMOUNTS: [i64; 5] = [1, 2, 3, 4, 5];
        let mut sleep_time: u32 = 1250;

        if LOW_AMOUNTS.contains(&self.t_mode) {
            sleep_time += 2300 * (1 << self.t_mode);
        }

        if LOW_AMOUNTS.contains(&self.p_mode) {
            sleep_time += 575 + (2300 * (1 << self.p_mode));
        }

        if LOW_AMOUNTS.contains(&self.h_mode) {
            sleep_time += 575 + (2300 * (1 << self.h_mode))
        }

        i2c.delay(1 + sleep_time / 1000);

        while (Self::read_16(self.addr, 0xF3, &mut i2c).unwrap() & 0x08) != 0 {
            i2c.delay(1);
        }

        // Calculate pressure
        let raw_p: i32 = i32::from_be_bytes([
            0,
            Self::read_8(self.addr, 0xF7, &mut i2c).unwrap(),
            Self::read_8(self.addr, 0xF8, &mut i2c).unwrap(),
            Self::read_8(self.addr, 0xF9, &mut i2c).unwrap(),
        ]) >> 4;

        // Calculate temperature
        let raw_t: i32 = i32::from_be_bytes([
            0,
            Self::read_8(self.addr, 0xFA, &mut i2c).unwrap(),
            Self::read_8(self.addr, 0xFB, &mut i2c).unwrap(),
            Self::read_8(self.addr, 0xFC, &mut i2c).unwrap(),
        ]) >> 4;

        // Calculate humidity
        let raw_h: i32 = i32::from_be_bytes([
            0,
            0,
            Self::read_8(self.addr, 0xFD, &mut i2c).unwrap(),
            Self::read_8(self.addr, 0xFE, &mut i2c).unwrap(),
        ]);

        (raw_t as i64, raw_p as i64, raw_h as i64)
    }

    pub fn read_compensated_data(&mut self) -> TempPresHumi {
        let (raw_t, raw_p, raw_h) = self.read_raw_data();

        let mut var1: i64 = ((raw_t >> 3) - (self.t1 << 1)) * (self.t2 >> 11);
        let mut var2: i64 = (raw_t >> 4) - self.t1;
        var2 = var2 * ((raw_t >> 4) - self.t1);
        var2 = ((var2 >> 12) * self.t3) >> 14;
        let t_fine: i64 = var1 + var2;

        let temp: i64 = (t_fine * 5 + 128) >> 8;
        var1 = t_fine - 128000;
        var2 = var1 * var1 * self.p6;
        var2 = var2 + ((var1 * self.p5) << 17);
        var2 = var2 + (self.p4 << 35);
        var1 = ((var1 * var1 * self.p3) >> 8) + ((var1 * self.p2) << 12);
        var1 = (((1 << 47) + var1) * self.p1) >> 33;

        let pres: i64 = if var1 == 0 {
            0
        } else {
            let p: i64 = (((1048576 - raw_p) << 31) - var2) * 3125 / var1;
            var1 = (self.p9 * (p >> 13) * (p >> 13)) >> 25;
            var2 = (self.p8 * p) >> 19;
            ((p + var1 + var2) >> 8) + (self.p7 << 4)
        };

        let mut h: i64 = t_fine - 76800;
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

        let humi: i64 = h >> 12;
        (temp, pres, humi)
    }

    pub fn values(&mut self) -> (f32, f32, f32) {
        let (temp, pres, humi) = self.read_compensated_data();
        (
            ((temp as f32) / 100.0),
            ((pres as f32) / 256.0),
            ((humi as f32) / 1024.0),
        )
    }

    pub fn pressure_precision(&mut self) -> (f32, i64) {
        let p: i64 = self.read_compensated_data().1;
        let pi: f32 = (p / 256) as f32;
        let pd: i64 = (p % 256) / 256;
        (pi, pd)
    }

    pub fn altitude(&mut self, pressure_sea_level: Option<f32>) -> f32 {
        const SEA_LEVEL_PRESSURE: f32 = 1013.25;
        let (pi, pd) = self.pressure_precision();

        44330.0
            * (1.0
                - powf(
                    ((pi + pd as f32) / 100.0) / pressure_sea_level.unwrap_or(SEA_LEVEL_PRESSURE),
                    1.0 / 5.255,
                ))
    }
}

impl<'a> ByteReader for PiicoDevBME280<'a> {}
