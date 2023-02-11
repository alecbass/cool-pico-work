use super::piicodev_unified::{HardwareArgs, I2CUnifiedMachine};
use crate::piicodev_unified::I2CBase;
use rp_pico::hal::i2c;

const _BASE_ADDR: u8 = 0x5C;
const _DEV_ID: u8 = 0x51;
const _REG_DEV_ID: u8 = 0x11;
const _REG_STATUS: u8 = 0x01;
const _REG_FIRM_MAJ: u8 = 0x02;
const _REG_FIRM_MIN: u8 = 0x03;
const _REG_I2C_ADDR: u8 = 0x04;
const _REG_TONE: u8 = 0x05;
const _REG_VOLUME: u8 = 0x06;
const _REG_LED: u8 = 0x07;

#[derive(Clone, Copy)]
pub enum BuzzerVolume {
    Low = 0,
    Medium = 1,
    High = 2,
}

impl Into<u8> for BuzzerVolume {
    fn into(self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Medium => 1,
            Self::High => 2,
        }
    }
}

pub struct PiicoDevBuzzer {
    i2c: I2CUnifiedMachine,
}

impl PiicoDevBuzzer {
    pub fn new(args: HardwareArgs, volume: Option<BuzzerVolume>) -> Self {
        let mut i2c = I2CUnifiedMachine::new(args, Some(_BASE_ADDR));
        i2c.write(i2c.addr, &[_REG_LED, 0x01]).unwrap();

        let mut buzzer = Self { i2c };
        buzzer
            .volume(volume.unwrap_or(BuzzerVolume::High))
            .expect("Failed to initialise PiicoDevBuzzer");
        buzzer
    }

    pub fn tone(&mut self, freq: u32, dur: u8) -> Result<(), i2c::Error> {
        let frequency: &[u8] = &freq.to_be_bytes();

        // NOTE: Not sure if three writes will work
        self.i2c
            .write(self.i2c.addr, &[_REG_TONE, freq as u8 + dur])?;
        self.i2c.write(self.i2c.addr, frequency)?;
        self.i2c.write(self.i2c.addr, &[dur])
    }

    pub fn volume(&mut self, vol: BuzzerVolume) -> Result<(), i2c::Error> {
        self.i2c.write(self.i2c.addr, &[_REG_VOLUME, vol.into()])
    }

    pub fn read_firmware(&mut self) -> Result<[u8; 2], i2c::Error> {
        let mut v: [u8; 2] = [0, 0];
        self.i2c.read(self.i2c.addr, &mut v).map(|()| v)
    }

    pub fn read_status(&mut self) -> Result<[u8; 1], i2c::Error> {
        let mut status: [u8; 1] = [_REG_STATUS];
        self.i2c.read(self.i2c.addr, &mut status).map(|()| status)
    }

    pub fn read_id(&mut self) -> Result<u8, i2c::Error> {
        let mut id_buffer: [u8; 1] = [_REG_DEV_ID];
        self.i2c
            .read(self.i2c.addr, &mut id_buffer)
            .map(|()| id_buffer[0])
    }

    pub fn power_led(&mut self, on: bool) -> Result<(), i2c::Error> {
        self.i2c.write(
            self.i2c.addr,
            &[
                _REG_LED,
                match on {
                    false => 0,
                    true => 1,
                },
            ],
        )
    }
}
