use core::cell::RefCell;

use cortex_m::delay::Delay;
use embedded_hal::i2c::I2c;
use libm::powf;

use rp_pico as bsp;

use bsp::hal::i2c::Error;

use crate::i2c::I2CHandler;

use super::reading::AtmosphericReading;

const BASE_ADDR: u8 = 0x77;

/** A tuple representing temperature, pressure and humidity readings */

pub struct PiicoDevBME280<'i2c, 'delay> {
    addr: u8,
    i2c: &'i2c RefCell<I2CHandler>,
    delay: &'delay RefCell<Delay>,
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

///
/// Writes and reads to a given address's register as an unsigned 16-bit avlue
///
fn write_reg_16(addr: u8, register: u16, i2c: &mut I2CHandler) -> Result<u16, Error> {
    let register_bytes = register.to_le_bytes();
    let mut value_buffer = [0; 2];
    i2c.write_read(
        addr,
        &[register_bytes[0], register_bytes[1]],
        &mut value_buffer,
    )?;

    let value = u16::from_le_bytes(value_buffer);
    Ok(value)
}

fn write_reg_8(addr: u8, register: u8, i2c: &mut I2CHandler) -> Result<u8, Error> {
    let mut value_buffer = [0; 1];
    i2c.write_read(addr, &[register], &mut value_buffer)?;

    let value = value_buffer[0];
    Ok(value)
}

impl<'i2c, 'delay> PiicoDevBME280<'i2c, 'delay> {
    pub fn new(i2c: &'i2c RefCell<I2CHandler>, delay: &'delay RefCell<Delay>) -> Self {
        let addr = BASE_ADDR;

        Self {
            addr,
            i2c,
            delay,
            t_mode: 0,
            p_mode: 0,
            h_mode: 0,
            iir: 0,
            t1: 0,
            t2: 0,
            t3: 0,
            p1: 0,
            p2: 0,
            p3: 0,
            p4: 0,
            p5: 0,
            p6: 0,
            p7: 0,
            p8: 0,
            p9: 0,
            h1: 0,
            h2: 0,
            h3: 0,
            h4: 0,
            h5: 0,
            h6: 0,
        }
    }

    pub fn init(&mut self) -> Result<(), Error> {
        fn short(num: i64) -> i64 {
            if num > 32767 {
                return num - 65537;
            }

            num
        }

        let addr = self.addr;
        let mut i2c = self.i2c.borrow_mut();
        let mut delay = self.delay.borrow_mut();

        // NOTE: This can be set up to be dynamic

        let t_mode = 2 as i64;
        let p_mode = 5 as i64;
        let h_mode = 1 as i64;
        let iir = 1 as i64;

        // The Piicodev libraries expect Python 32-bit integers, so while these number casts
        // seem inefficient, it's to mimic the expected behaviour

        // Read 16 bits from register 0x88
        // let t1 = write_reg_16(addr, 0x88, &mut i2c)? as i64;
        let t1 = write_reg_16(addr, 0x88, &mut i2c)? as i64;
        let t2 = write_reg_16(addr, 0x8A, &mut i2c)? as i64;
        let t3 = write_reg_16(addr, 0x8C, &mut i2c)? as i64;

        let p1 = write_reg_16(addr, 0x8E, &mut i2c)? as i64;
        let p2 = short(write_reg_16(addr, 0x90, &mut i2c)? as i64);
        let p3 = short(write_reg_16(addr, 0x92, &mut i2c)? as i64);
        let p4 = short(write_reg_16(addr, 0x94, &mut i2c)? as i64);
        let p5 = short(write_reg_16(addr, 0x96, &mut i2c)? as i64);
        let p6 = short(write_reg_16(addr, 0x98, &mut i2c)? as i64);
        let p7 = short(write_reg_16(addr, 0x9A, &mut i2c)? as i64);
        let p8 = short(write_reg_16(addr, 0x9C, &mut i2c)? as i64);
        let p9 = short(write_reg_16(addr, 0x9E, &mut i2c)? as i64);

        let h1 = write_reg_8(addr, 0xA1, &mut i2c)? as i64;
        let h2 = write_reg_16(addr, 0xE1, &mut i2c)? as i64;
        let h3 = write_reg_8(addr, 0xE3, &mut i2c)? as i64;
        let a = write_reg_8(addr, 0xE5, &mut i2c)? as i64;
        let h4 = ((write_reg_8(addr, 0xE4, &mut i2c)? as i64) << 4) + (a % 16);
        let h5 = ((write_reg_8(addr, 0xE6, &mut i2c)? as i64) << 4) + (a >> 4);
        let mut h6 = write_reg_8(addr, 0xE7, &mut i2c)? as i64;

        if h6 > 127 {
            h6 -= 256;
        }

        i2c.write(addr, &[0xF2, h_mode as u8])?;
        delay.delay_ms(2);
        i2c.write(addr, &[0xF4, 0x24])?;
        delay.delay_ms(2);
        i2c.write(addr, &[0xF5, (iir as u8) << 2])?;

        // Update configuration
        self.t_mode = t_mode;
        self.p_mode = p_mode;
        self.h_mode = h_mode;
        self.iir = iir;
        self.t1 = t1;
        self.t2 = t2;
        self.t3 = t3;
        self.p1 = p1;
        self.p2 = p2;
        self.p3 = p3;
        self.p4 = p4;
        self.p5 = p5;
        self.p6 = p6;
        self.p7 = p7;
        self.p8 = p8;
        self.p9 = p9;
        self.h1 = h1;
        self.h2 = h2;
        self.h3 = h3;
        self.h4 = h4;
        self.h5 = h5;
        self.h6 = h6;

        Ok(())
    }

    fn read_raw_data(&mut self) -> Result<(i64, i64, i64), Error> {
        let mut i2c = self.i2c.borrow_mut();
        let mut delay = self.delay.borrow_mut();

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

        delay.delay_ms(1 + sleep_time / 1000);

        while (write_reg_16(self.addr, 0xF3, &mut i2c).unwrap() & 0x08) != 0 {
            delay.delay_ms(1);
        }

        // Calculate pressure
        let raw_p = i32::from_be_bytes([
            0,
            write_reg_8(self.addr, 0xF7, &mut i2c)?,
            write_reg_8(self.addr, 0xF8, &mut i2c)?,
            write_reg_8(self.addr, 0xF9, &mut i2c)?,
        ]) >> 4;

        // Calculate temperature
        let raw_t = i32::from_be_bytes([
            0,
            write_reg_8(self.addr, 0xFA, &mut i2c)?,
            write_reg_8(self.addr, 0xFB, &mut i2c)?,
            write_reg_8(self.addr, 0xFC, &mut i2c)?,
        ]) >> 4;

        // Calculate humidity
        let raw_h = i32::from_be_bytes([
            0,
            0,
            write_reg_8(self.addr, 0xFD, &mut i2c)?,
            write_reg_8(self.addr, 0xFE, &mut i2c)?,
        ]);

        Ok((raw_t as i64, raw_p as i64, raw_h as i64))
    }

    fn read_compensated_data(&mut self) -> Result<(i64, i64, i64), Error> {
        let (raw_t, raw_p, raw_h) = self.read_raw_data()?;

        let mut var1 = ((raw_t >> 3) - (self.t1 << 1)) * (self.t2 >> 11);
        let mut var2 = (raw_t >> 4) - self.t1;
        var2 = var2 * ((raw_t >> 4) - self.t1);
        var2 = ((var2 >> 12) * self.t3) >> 14;
        let t_fine = var1 + var2;

        let temp = (t_fine * 5 + 128) >> 8;
        var1 = t_fine - 128000;
        var2 = var1 * var1 * self.p6;
        var2 = var2 + ((var1 * self.p5) << 17);
        var2 = var2 + (self.p4 << 35);
        var1 = ((var1 * var1 * self.p3) >> 8) + ((var1 * self.p2) << 12);
        var1 = (((1 << 47) + var1) * self.p1) >> 33;

        let pres = if var1 == 0 {
            0
        } else {
            let p = (((1048576 - raw_p) << 31) - var2) * 3125 / var1;
            var1 = (self.p9 * (p >> 13) * (p >> 13)) >> 25;
            var2 = (self.p8 * p) >> 19;
            ((p + var1 + var2) >> 8) + (self.p7 << 4)
        };

        let mut h = t_fine - 76800;
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

        let humi = h >> 12;

        Ok((temp, pres, humi))
    }

    pub fn values(&mut self) -> Result<AtmosphericReading, Error> {
        let (temp, pres, humi) = self.read_compensated_data()?;

        let temperature = temp as f32 / 100.0;
        let pressure = pres as f32 / 256.0;
        let humidity = humi as f32 / 1024.0;

        let reading = AtmosphericReading {
            temperature,
            pressure,
            humidity,
            altitude: 0.0,
        };

        Ok(reading)
    }

    fn pressure_precision(&mut self) -> Result<(f32, i64), Error> {
        let p = self.read_compensated_data()?.1;
        let pi = (p / 256) as f32;
        let pd = (p % 256) / 256;

        Ok((pi, pd))
    }

    pub fn altitude(&mut self, pressure_sea_level: Option<f32>) -> Result<f32, Error> {
        const SEA_LEVEL_PRESSURE: f32 = 1013.25;
        let (pi, pd) = self.pressure_precision()?;

        let altitude = 44330.0
            * (1.0
                - powf(
                    ((pi + pd as f32) / 100.0) / pressure_sea_level.unwrap_or(SEA_LEVEL_PRESSURE),
                    1.0 / 5.255,
                ));

        Ok(altitude)
    }
}
