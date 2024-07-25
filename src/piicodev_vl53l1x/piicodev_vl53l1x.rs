use core::{borrow::BorrowMut, cell::RefCell};

use cortex_m::delay::Delay;
use embedded_hal::i2c::I2c;
use rp_pico::hal::i2c::Error;

use crate::{i2c::I2CHandler, uart::Uart};

use super::constants::VL51L1X_DEFAULT_CONFIGURATION;

const _DEFAULT_MODEL_ID: u16 = 0xEACC;

// Device address
const BASE_ADDR: u8 = 0x29;

// Used for the read() method
const READ_BUFFER_SIZE: usize = 17;

pub struct PiicoDevVL53L1X<'i2c, 'uart, 'delay> {
    pub addr: u8,
    i2c: &'i2c RefCell<I2CHandler>,
    uart: &'uart RefCell<Uart>,
    delay: &'delay RefCell<Delay>,
}

impl<'i2c, 'uart, 'delay> PiicoDevVL53L1X<'i2c, 'uart, 'delay> {
    pub fn new(
        addr: Option<u8>,
        i2c: &'i2c RefCell<I2CHandler>,
        uart: &'uart RefCell<Uart>,
        delay: &'delay RefCell<Delay>,
    ) -> Self {
        let addr = addr.unwrap_or(BASE_ADDR);

        Self {
            addr,
            i2c,
            uart,
            delay,
        }
    }

    pub fn init(&mut self) -> Result<(), Error> {
        // NOTE: The Python library has a check for compat_ind >= 1 here. I don't know what it does
        self.reset()?;

        let mut i2c = self.i2c.borrow_mut();
        let mut delay = self.delay.borrow_mut();

        delay.delay_ms(1);

        // let model_id = self.read_model_id().unwrap_or(0xEACC);

        // Write default configuration
        // Python: i2c.writeto_mem(self.addr, 0x2D, VL51L1X_DEFAULT_CONFIGURATION, addrsize=16)

        // let mut index: usize = 0;
        // let mut config_bytes: [u8; 182] = [0; VL51L1X_DEFAULT_CONFIGURATION.len() * 2]; // VL51L1X_DEFAULT_CONFIGURATION.clone();
        // for byte in VL51L1X_DEFAULT_CONFIGURATION {
        //     config_bytes[index] = 0x00;
        //     config_bytes[index + 1] = *byte;
        //     index += 2;
        // }
        // i2c_mut.write(addr, &config_bytes).unwrap();

        i2c.write(self.addr, VL51L1X_DEFAULT_CONFIGURATION)?;
        delay.delay_ms(100);

        // The API triggers this change in VL53L1_init_and_start_range() once a
        // measurement is started; assumes MM1 and MM2 are disabled
        i2c.write(self.addr, &[0x0022])?;

        // Write 16 bits
        let mut read_buffer: [u8; 2] = [0; 2];
        i2c.read(self.addr, &mut read_buffer)?;

        let value = u16::from_be_bytes(read_buffer) * 4;

        let address_bytes = [0x00, 0x1E];
        let value_bytes = value.to_be_bytes();

        i2c.write(
            self.addr,
            &[
                address_bytes[0],
                address_bytes[1],
                value_bytes[0],
                value_bytes[1],
            ],
        )

        // Self::write_reg_16_bit(
        //     self.addr,
        //     0x001E,
        //     Self::read_16(addr, 0x0022, comms).unwrap() * 4,
        //     comms,
        // )
        // .unwrap();
    }

    // fn read_model_id(&mut self) -> Result<u16, Error> {
    //     i2c.write(self.addr, &[0x01, 0x0F])?;
    //
    //     let mut buffer = [0; 2];
    //
    //     match i2c.read(self.addr, &mut buffer) {
    //         Ok(()) => Ok(u16::from_le_bytes([buffer[0], buffer[1]])),
    //         Err(e) => Err(e),
    //     }
    //
    //     // Self::read_16(self.addr, 0x010F, i2c_mut)
    // }

    fn reset(&mut self) -> Result<(), Error> {
        let mut i2c = self.i2c.borrow_mut();

        // Self::write_reg_8_bit(self.addr, 0x0000, 0x00, i2c_mut)?;
        i2c.write(self.addr, &[0x00, 0x00, 0x00])?;
        // i2c.delay(100);
        i2c.write(self.addr, &[0x00, 0x00, 0x01])
        // Self::write_reg_8_bit(self.addr, 0x0000, 0x01, i2c_mut)
    }

    fn read_17_bytes(&mut self, reg: u16) -> Result<[u8; READ_BUFFER_SIZE], Error> {
        let mut i2c = self.i2c.borrow_mut();

        let reg_bytes: [u8; 2] = reg.to_be_bytes();

        i2c.write(self.addr, &reg_bytes).unwrap();

        let mut buffer: [u8; READ_BUFFER_SIZE] = [0; READ_BUFFER_SIZE];

        i2c.read(self.addr, &mut buffer)?;

        Ok(buffer)
    }

    pub fn read(&mut self) -> Result<u16, Error> {
        let data: [u8; READ_BUFFER_SIZE] = self.read_17_bytes(0x0089)?;
        let _range_status: u8 = data[0];
        let _report_status: u8 = data[1];
        let _stream_count: u8 = data[2];
        let _dss_actual_effective_spads_sd0: u16 = u16::from_le_bytes([data[3], data[4]]);
        let _peak_signal_count_rate_mcps_sd0: u16 = u16::from_le_bytes([data[5], data[6]]);
        let _ambient_count_rate_mcps_sd0: u16 = u16::from_le_bytes([data[7], data[8]]);

        let _sigma_sd0: u16 = u16::from_le_bytes([data[9], data[10]]);
        let _phase_sd0: u16 = u16::from_le_bytes([data[11], data[12]]);
        let final_crosstalk_corrected_range_mm_sd0: u16 = u16::from_le_bytes([data[13], data[14]]);
        let _peak_signal_count_rate_crosstalk_corrected_mcps_sd0: u16 =
            u16::from_le_bytes([data[15], data[16]]);

        // Roughly accurate to mm
        let distance_mm: u16 = final_crosstalk_corrected_range_mm_sd0 / 100 / 2;
        //status = None
        //if range_status in (17, 2, 1, 3):
        //status = "HardwareFail"
        //elif range_status == 13:
        //status = "MinRangeFail"
        //elif range_status == 18:
        //status = "SynchronizationInt"
        //elif range_status == 5:
        //status = "OutOfBoundsFail"
        //elif range_status == 4:
        //status = "SignalFail"
        //elif range_status == 6:
        //status = "SignalFail"
        //elif range_status == 7:
        //status = "WrapTargetFail"
        //elif range_status == 12:
        //status = "XtalkSignalFail"
        //elif range_status == 8:
        //status = "RangeValidMinRangeClipped"
        //elif range_status == 9:
        //if stream_count == 0:
        //status = "RangeValidNoWrapCheckFail"
        //else:
        //status = "OK"

        Ok(distance_mm)
    }
}
