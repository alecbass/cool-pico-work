use core::cell::RefCell;

use cortex_m::delay::Delay;
use rp_pico as bsp;

use bsp::hal::i2c::Error;
use embedded_hal::i2c::I2c;

use crate::i2c::I2CHandler;

use super::notes::{note_to_frequency, Note};

const BASE_ADDR: u8 = 0x5C;
const _DEV_ID: u8 = 0x51;
const REG_DEV_ID: u8 = 0x11;
const REG_STATUS: u8 = 0x01;
const REG_FIRM_MAJ: u8 = 0x02;
const REG_FIRM_MIN: u8 = 0x03;
const REG_I2C_ADDR: u8 = 0x04;
const REG_TONE: u8 = 0x05;
const REG_VOLUME: u8 = 0x06;
const REG_LED: u8 = 0x07;

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

pub struct PiicoDevBuzzer<'i2c, 'delay> {
    addr: u8,
    i2c: &'i2c RefCell<I2CHandler>,
    delay: &'delay RefCell<Delay>,
}

impl<'i2c, 'delay> PiicoDevBuzzer<'i2c, 'delay> {
    pub fn new(i2c: &'i2c RefCell<I2CHandler>, delay: &'delay RefCell<Delay>) -> Self {
        let addr = BASE_ADDR;

        Self { addr, i2c, delay }
    }

    pub fn init(&mut self) -> Result<(), Error> {
        let mut i2c = self.i2c.borrow_mut();

        i2c.write(BASE_ADDR, &[REG_LED, 0x01])
    }

    pub fn tone(&mut self, note: &Note, dur: u16) -> Result<(), Error> {
        let mut i2c = self.i2c.borrow_mut();

        // Using u16 as the buzzer module requires 2 big-endian bytes to be passed in as payload
        let freq = note_to_frequency(note) as u16;
        let frequency: &[u8] = &freq.to_be_bytes();
        let duration: &[u8] = &dur.to_be_bytes();

        // [address, frequency1, frequency2, duration1, duration2]
        let payload: [u8; 5] = [
            REG_TONE,
            frequency[0],
            frequency[1],
            duration[0],
            duration[1],
        ];

        i2c.write(BASE_ADDR, &payload)
    }

    pub fn volume(&mut self, vol: BuzzerVolume) -> Result<(), Error> {
        let mut i2c = self.i2c.borrow_mut();

        i2c.write(self.addr, &[REG_VOLUME, vol.into()])
    }

    // pub fn read_firmware(&mut self) -> Result<[u8; 2], Error> {
    //     let mut i2c = self.i2c.borrow_mut();
    //
    //     let mut v: [u8; 2] = [0, 0];
    //     i2c.read(self.addr, &mut v).map(|()| v)
    // }
    //
    // pub fn read_status(&mut self) -> Result<[u8; 1], Error> {
    //     let mut i2c = self.i2c.borrow_mut();
    //
    //     let mut status: [u8; 1] = [REG_STATUS];
    //     i2c.read(self.addr, &mut status).map(|()| status)
    // }
    //
    // pub fn read_id(&mut self) -> Result<u8, Error> {
    //     let mut i2c = self.i2c.borrow_mut();
    //
    //     let mut id_buffer: [u8; 1] = [REG_DEV_ID];
    //     i2c.read(self.addr, &mut id_buffer).map(|()| id_buffer[0])
    // }
    //
    // pub fn power_led(&mut self, on: bool) -> Result<(), Error> {
    //     let mut i2c = self.i2c.borrow_mut();
    //
    //     i2c.write(
    //         self.addr,
    //         &[
    //             REG_LED,
    //             match on {
    //                 false => 0,
    //                 true => 1,
    //             },
    //         ],
    //     )
    // }

    pub fn play_song(&mut self, notes: &[(Note, u16)]) -> Result<(), Error> {
        let mut delay = self.delay.borrow_mut();

        for (tone, duration) in notes {
            let note_duration = *duration / 4;

            self.tone(tone, note_duration)?;

            delay.delay_ms(note_duration as u32)
        }

        Ok(())
    }
}
