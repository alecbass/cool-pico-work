use crate::piicodev_unified::{I2CBase, I2CUnifiedMachine};
use core::cell::RefMut;
use rp_pico::hal::i2c;

pub trait ByteReader {
    fn read_8(addr: u8, reg: u8, i2c: &mut RefMut<I2CUnifiedMachine>) -> Result<u8, i2c::Error> {
        let mut buffer: [u8; 1] = [0; 1];

        i2c.write(addr, &[reg])?;

        match i2c.read(addr, &mut buffer) {
            Ok(()) => Ok(buffer[0]),
            Err(e) => Err(e),
        }
    }

    fn read_16(addr: u8, reg: u16, i2c: &mut RefMut<I2CUnifiedMachine>) -> Result<u16, i2c::Error> {
        let mut buffer: [u8; 2] = [0; 2];

        let reg_bytes: [u8; 2] = reg.to_le_bytes();
        i2c.write(addr, &[reg_bytes[0], reg_bytes[1]]).unwrap();

        match i2c.read(addr, &mut buffer) {
            Ok(()) => Ok(u16::from_le_bytes([buffer[0], buffer[1]])),
            Err(e) => Err(e),
        }
    }

    fn write_reg_8_bit(
        addr: u8,
        reg: u8,
        value: u8,
        i2c: &mut RefMut<I2CUnifiedMachine>,
    ) -> Result<(), i2c::Error> {
        i2c.write(addr, &[reg, value])
    }

    fn write_reg_16_bit(
        addr: u8,
        reg: u8,
        value: u16,
        i2c: &mut RefMut<I2CUnifiedMachine>,
    ) -> Result<(), i2c::Error> {
        // Turn a 16-bit value into two bytes
        let value: [u8; 2] = value.to_le_bytes();
        i2c.write(addr, &[reg, value[0], value[1]])
    }
}
