use crate::{
    piicodev_unified::{HardwareArgs, I2CBase, I2CUnifiedMachine},
    utils::create_buffer,
};
use rp_pico::hal::i2c;

pub struct PiicoDevBME280 {
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
    h1: u8,
    h2: u16,
    h3: u8,
    h4: u8,
    h5: u8,
    h6: u8,
}

impl PiicoDevBME280 {
    pub fn new(args: HardwareArgs) -> Self {
        let i2c: I2CUnifiedMachine = I2CUnifiedMachine::new(args);

        let t_mode: u8 = 2;
        let p_mode: u8 = 5;
        let h_mode: u8 = 1;
        let iir: u8 = 1;

        let t1 = Self::read_16(0x88, i2c).unwrap();
        let t2 = Self::read_16(0x8A, i2c).unwrap();
        let t3 = Self::read_16(0x8C, i2c).unwrap();

        let p1 = Self::read_16(0x8E, i2c).unwrap();
        let p2 = Self::read_16(0x90, i2c).unwrap();
        let p3 = Self::read_16(0x92, i2c).unwrap();
        let p4 = Self::read_16(0x94, i2c).unwrap();
        let p5 = Self::read_16(0x96, i2c).unwrap();
        let p6 = Self::read_16(0x98, i2c).unwrap();
        let p7 = Self::read_16(0x9A, i2c).unwrap();
        let p8 = Self::read_16(0x9C, i2c).unwrap();
        let p9 = Self::read_16(0x9E, i2c).unwrap();

        let h1 = Self::read_8(0xE5, i2c).unwrap();
        let h2 = Self::read_16(0xE1, i2c).unwrap();
        let h3 = Self::read_8(0xE3, i2c).unwrap();
        let h4 = Self::read_8(0xE4, i2c).unwrap();
        let h5 = Self::read_8(0xE6, i2c).unwrap();
        let h6 = Self::read_8(0xE7, i2c).unwrap();

        i2c.write(i2c.addr, &[0xF2, h_mode]).unwrap();
        i2c.delay(2);
        i2c.write(i2c.addr, &[0xF4, 0x24]).unwrap();
        i2c.delay(2);
        i2c.write(i2c.addr, &[0xF5, iir << 2]).unwrap();

        Self {
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

    pub(crate) fn read_8(reg: u8, i2c: I2CUnifiedMachine) -> Result<u8, i2c::Error> {
        let mut buffer: [u8; 16] = create_buffer();

        i2c.write(i2c.addr, &[reg]).unwrap();

        match i2c.read(i2c.addr, &mut buffer) {
            Ok(()) => Ok(buffer[0]),
            Err(e) => Err(e),
        }
    }

    pub(crate) fn read_16(reg: u8, i2c: I2CUnifiedMachine) -> Result<u16, i2c::Error> {
        let mut buffer: [u8; 16] = create_buffer();

        i2c.write(i2c.addr, &[reg]).unwrap();

        match i2c.read(i2c.addr, &mut buffer) {
            Ok(()) => Ok(buffer[0] as u16 + buffer[1] as u16 * 256),
            Err(e) => Err(e),
        }
    }
}
