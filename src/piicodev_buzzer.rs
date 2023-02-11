use super::piicodev_unified::{HardwareArgs, I2CUnifiedMachine};

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

pub enum BuzzerVolume {
    Low = 0,
    Medium = 1,
    High = 2,
}

pub struct PiicoDevBuzzer {
    i2c: I2CUnifiedMachine,
}

impl PiicoDevBuzzer {
    pub fn new(args: HardwareArgs) -> Self {
        let i2c = I2CUnifiedMachine::new(args, Some(_BASE_ADDR));

        Self { i2c }
    }

    pub fn tone(&self, freq: u32, dur: u32) {}

    pub fn volume(&self, vol: BuzzerVolume) {}

    pub fn read_firmware(&self) {}

    pub fn read_status(&self) {}

    pub fn read_id(&self) {}

    pub fn power_led(&self) {}
}
