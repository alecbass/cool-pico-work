use crate::piicodev_unified::I2CBase;
use crate::piicodev_unified::I2CUnifiedMachine;
use rp_pico::hal::i2c;

use super::notes::{note_to_frequency, Note};

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
    addr: u8,
}

impl PiicoDevBuzzer {
    pub fn new(comms: &mut I2CUnifiedMachine, volume: Option<BuzzerVolume>) -> Self {
        comms.write(_BASE_ADDR, &[_REG_LED, 0x01]).unwrap();

        let mut buzzer = Self { addr: _BASE_ADDR };
        buzzer
            .volume(volume.unwrap_or(BuzzerVolume::High), comms)
            .expect("Failed to initialise PiicoDevBuzzer");
        buzzer
    }

    pub fn tone(
        &mut self,
        note: &Note,
        dur: u16,
        comms: &mut I2CUnifiedMachine,
    ) -> Result<(), i2c::Error> {
        // Using u16 as the buzzer module requires 2 big-endian bytes to be passed in as payload
        let freq: u16 = note_to_frequency(note) as u16;
        let frequency: &[u8] = &freq.to_be_bytes();
        let duration: &[u8] = &dur.to_be_bytes();

        // [address, frequency1, frequency2, duration1, duration2]
        let payload: [u8; 5] = [
            _REG_TONE,
            frequency[0],
            frequency[1],
            duration[0],
            duration[1],
        ];

        comms.write(_BASE_ADDR, &payload)
    }

    pub fn volume(
        &mut self,
        vol: BuzzerVolume,
        comms: &mut I2CUnifiedMachine,
    ) -> Result<(), i2c::Error> {
        comms.write(self.addr, &[_REG_VOLUME, vol.into()])
    }

    pub fn read_firmware(&mut self, comms: &mut I2CUnifiedMachine) -> Result<[u8; 2], i2c::Error> {
        let mut v: [u8; 2] = [0, 0];
        comms.read(self.addr, &mut v).map(|()| v)
    }

    pub fn read_status(&mut self, comms: &mut I2CUnifiedMachine) -> Result<[u8; 1], i2c::Error> {
        let mut status: [u8; 1] = [_REG_STATUS];
        comms.read(self.addr, &mut status).map(|()| status)
    }

    pub fn read_id(&mut self, comms: &mut I2CUnifiedMachine) -> Result<u8, i2c::Error> {
        let mut id_buffer: [u8; 1] = [_REG_DEV_ID];
        comms.read(self.addr, &mut id_buffer).map(|()| id_buffer[0])
    }

    pub fn power_led(&mut self, on: bool, comms: &mut I2CUnifiedMachine) -> Result<(), i2c::Error> {
        comms.write(
            self.addr,
            &[
                _REG_LED,
                match on {
                    false => 0,
                    true => 1,
                },
            ],
        )
    }

    pub fn play_song(&mut self, notes: &[(Note, u16)], comms: &mut I2CUnifiedMachine) {
        let mut is_led_on = true;
        for (tone, duration) in notes {
            self.power_led(is_led_on, comms).unwrap();
            is_led_on = !is_led_on;

            self.tone(tone, duration / 4, comms).unwrap();
            comms.delay((duration / 4) as u32);
        }
    }
}
