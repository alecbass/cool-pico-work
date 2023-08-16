use crate::{
    byte_reader::ByteReader,
    piicodev_unified::{I2CBase, I2CUnifiedMachine},
};
use core::{
    cell::{RefCell, RefMut},
    fmt::Write,
};
use defmt::{debug, info};
use rp_pico::hal::i2c;

const VL51L1X_DEFAULT_CONFIGURATION: &[u8] = &[
    0x00, // Register padding
    0x2D, // Register
    0x00, // 0x2d : set bit 2 and 5 to 1 for fast plus mode (1MHz I2C), else don't touch */
    0x00, // 0x2e : bit 0 if I2C pulled up at 1.8V, else set bit 0 to 1 (pull up at AVDD) */
    0x00, // 0x2f : bit 0 if GPIO pulled up at 1.8V, else set bit 0 to 1 (pull up at AVDD) */
    0x01, // 0x30 : set bit 4 to 0 for active high interrupt and 1 for active low (bits 3:0 must be 0x1), use SetInterruptPolarity() */
    0x02, // 0x31 : bit 1 = interrupt depending on the polarity, use CheckForDataReady() */
    0x00, // 0x32 : not user-modifiable (NUM)*/
    0x02, // 0x33 : NUM */
    0x08, // 0x34 : NUM */
    0x00, // 0x35 : NUM */
    0x08, // 0x36 : NUM */
    0x10, // 0x37 : NUM */
    0x01, // 0x38 : NUM */
    0x01, // 0x39 : NUM */
    0x00, // 0x3a : NUM */
    0x00, // 0x3b : NUM */
    0x00, // 0x3c : NUM */
    0x00, // 0x3d : NUM */
    0xff, // 0x3e : NUM */
    0x00, // 0x3f : NUM */
    0x0F, // 0x40 : NUM */
    0x00, // 0x41 : NUM */
    0x00, // 0x42 : NUM */
    0x00, // 0x43 : NUM */
    0x00, // 0x44 : NUM */
    0x00, // 0x45 : NUM */
    0x20, // 0x46 : interrupt configuration 0->level low detection, 1-> level high, 2-> Out of window, 3->In window, 0x20-> New sample ready , TBC */
    0x0b, // 0x47 : NUM */
    0x00, // 0x48 : NUM */
    0x00, // 0x49 : NUM */
    0x02, // 0x4a : NUM */
    0x0a, // 0x4b : NUM */
    0x21, // 0x4c : NUM */
    0x00, // 0x4d : NUM */
    0x00, // 0x4e : NUM */
    0x05, // 0x4f : NUM */
    0x00, // 0x50 : NUM */
    0x00, // 0x51 : NUM */
    0x00, // 0x52 : NUM */
    0x00, // 0x53 : NUM */
    0xc8, // 0x54 : NUM */
    0x00, // 0x55 : NUM */
    0x00, // 0x56 : NUM */
    0x38, // 0x57 : NUM */
    0xff, // 0x58 : NUM */
    0x01, // 0x59 : NUM */
    0x00, // 0x5a : NUM */
    0x08, // 0x5b : NUM */
    0x00, // 0x5c : NUM */
    0x00, // 0x5d : NUM */
    0x01, // 0x5e : NUM */
    0xdb, // 0x5f : NUM */
    0x0f, // 0x60 : NUM */
    0x01, // 0x61 : NUM */
    0xf1, // 0x62 : NUM */
    0x0d, // 0x63 : NUM */
    0x01, // 0x64 : Sigma threshold MSB (mm in 14.2 format for MSB+LSB), use SetSigmaThreshold(), default value 90 mm  */
    0x68, // 0x65 : Sigma threshold LSB */
    0x00, // 0x66 : Min count Rate MSB (MCPS in 9.7 format for MSB+LSB), use SetSignalThreshold() */
    0x80, // 0x67 : Min count Rate LSB */
    0x08, // 0x68 : NUM */
    0xb8, // 0x69 : NUM */
    0x00, // 0x6a : NUM */
    0x00, // 0x6b : NUM */
    0x00, // 0x6c : Intermeasurement period MSB, 32 bits register, use SetIntermeasurementInMs() */
    0x00, // 0x6d : Intermeasurement period */
    0x0f, // 0x6e : Intermeasurement period */
    0x89, // 0x6f : Intermeasurement period LSB */
    0x00, // 0x70 : NUM */
    0x00, // 0x71 : NUM */
    0x00, // 0x72 : distance threshold high MSB (in mm, MSB+LSB), use SetD:tanceThreshold() */
    0x00, // 0x73 : distance threshold high LSB */
    0x00, // 0x74 : distance threshold low MSB ( in mm, MSB+LSB), use SetD:tanceThreshold() */
    0x00, // 0x75 : distance threshold low LSB */
    0x00, // 0x76 : NUM */
    0x01, // 0x77 : NUM */
    0x0f, // 0x78 : NUM */
    0x0d, // 0x79 : NUM */
    0x0e, // 0x7a : NUM */
    0x0e, // 0x7b : NUM */
    0x00, // 0x7c : NUM */
    0x00, // 0x7d : NUM */
    0x02, // 0x7e : NUM */
    0xc7, // 0x7f : ROI center, use SetROI() */
    0xff, // 0x80 : XY ROI (X=Width, Y=Height), use SetROI() */
    0x9B, // 0x81 : NUM */
    0x00, // 0x82 : NUM */
    0x00, // 0x83 : NUM */
    0x00, // 0x84 : NUM */
    0x01, // 0x85 : NUM */
    0x01, // 0x86 : clear interrupt, use ClearInterrupt() */
    0x40, // 0x87 : start ranging, use StartRanging() or StopRanging(), If you want an automatic start after VL53L1X_init() call, put 0x40 in location 0x87 */
];

// const VL51L1X_DEFAULT_CONFIGURATION: &[u8] = &[
//     0x00, // Padding
//     0x2D, // Register
//     0x00, 0x2d, // set bit 2 and 5 to 1 for fast plus mode (1MHz I2C), else don't touch */
//     0x00, 0x2e, // bit 0 if I2C pulled up at 1.8V, else set bit 0 to 1 (pull up at AVDD) */
//     0x00, 0x2f, // bit 0 if GPIO pulled up at 1.8V, else set bit 0 to 1 (pull up at AVDD) */
//     0x01,
//     0x30, // set bit 4 to 0 for active high interrupt and 1 for active low (bits 3:0 must be 0x1), use SetInterruptPolarity() */
//     0x02, 0x31, // bit 1 = interrupt depending on the polarity, use CheckForDataReady() */
//     0x00, 0x32, // not user-modifiable (NUM)*/
//     0x02, 0x33, // NUM */
//     0x08, 0x34, // NUM */
//     0x00, 0x35, // NUM */
//     0x08, 0x36, // NUM */
//     0x10, 0x37, // NUM */
//     0x01, 0x38, // NUM */
//     0x01, 0x39, // NUM */
//     0x00, 0x3a, // NUM */
//     0x00, 0x3b, // NUM */
//     0x00, 0x3c, // NUM */
//     0x00, 0x3d, // NUM */
//     0xff, 0x3e, // NUM */
//     0x00, 0x3f, // NUM */
//     0x0F, 0x40, // NUM */
//     0x00, 0x41, // NUM */
//     0x00, 0x42, // NUM */
//     0x00, 0x43, // NUM */
//     0x00, 0x44, // NUM */
//     0x00, 0x45, // NUM */
//     0x20,
//     0x46, // interrupt configuration 0->level low detection, 1-> level high, 2-> Out of window, 3->In window, 0x20-> New sample ready , TBC */
//     0x0b, 0x47, // NUM */
//     0x00, 0x48, // NUM */
//     0x00, 0x49, // NUM */
//     0x02, 0x4a, // NUM */
//     0x0a, 0x4b, // NUM */
//     0x21, 0x4c, // NUM */
//     0x00, 0x4d, // NUM */
//     0x00, 0x4e, // NUM */
//     0x05, 0x4f, // NUM */
//     0x00, 0x50, // NUM */
//     0x00, 0x51, // NUM */
//     0x00, 0x52, // NUM */
//     0x00, 0x53, // NUM */
//     0xc8, 0x54, // NUM */
//     0x00, 0x55, // NUM */
//     0x00, 0x56, // NUM */
//     0x38, 0x57, // NUM */
//     0xff, 0x58, // NUM */
//     0x01, 0x59, // NUM */
//     0x00, 0x5a, // NUM */
//     0x08, 0x5b, // NUM */
//     0x00, 0x5c, // NUM */
//     0x00, 0x5d, // NUM */
//     0x01, 0x5e, // NUM */
//     0xdb, 0x5f, // NUM */
//     0x0f, 0x60, // NUM */
//     0x01, 0x61, // NUM */
//     0xf1, 0x62, // NUM */
//     0x0d, 0x63, // NUM */
//     0x01,
//     0x64, // Sigma threshold MSB (mm in 14.2 format for MSB+LSB), use SetSigmaThreshold(), default value 90 mm  */
//     0x68, 0x65, // Sigma threshold LSB */
//     0x00,
//     0x66, // Min count Rate MSB (MCPS in 9.7 format for MSB+LSB), use SetSignalThreshold() */
//     0x80, 0x67, // Min count Rate LSB */
//     0x08, 0x68, // NUM */
//     0xb8, 0x69, // NUM */
//     0x00, 0x6a, // NUM */
//     0x00, 0x6b, // NUM */
//     0x00,
//     0x6c, // Intermeasurement period MSB, 32 bits register, use SetIntermeasurementInMs() */
//     0x00, 0x6d, // Intermeasurement period */
//     0x0f, 0x6e, // Intermeasurement period */
//     0x89, 0x6f, // Intermeasurement period LSB */
//     0x00, 0x70, // NUM */
//     0x00, 0x71, // NUM */
//     0x00, 0x72, // distance threshold high MSB (in mm, MSB+LSB), use SetD:tanceThreshold() */
//     0x00, 0x73, // distance threshold high LSB */
//     0x00, 0x74, // distance threshold low MSB ( in mm, MSB+LSB), use SetD:tanceThreshold() */
//     0x00, 0x75, // distance threshold low LSB */
//     0x00, 0x76, // NUM */
//     0x01, 0x77, // NUM */
//     0x0f, 0x78, // NUM */
//     0x0d, 0x79, // NUM */
//     0x0e, 0x7a, // NUM */
//     0x0e, 0x7b, // NUM */
//     0x00, 0x7c, // NUM */
//     0x00, 0x7d, // NUM */
//     0x02, 0x7e, // NUM */
//     0xc7, 0x7f, // ROI center, use SetROI() */
//     0xff, 0x80, // XY ROI (X=Width, Y=Height), use SetROI() */
//     0x9B, 0x81, // NUM */
//     0x00, 0x82, // NUM */
//     0x00, 0x83, // NUM */
//     0x00, 0x84, // NUM */
//     0x01, 0x85, // NUM */
//     0x01, 0x86, // clear interrupt, use ClearInterrupt() */
//     0x40,
//     0x87, // start ranging, use StartRanging() or StopRanging(), If you want an automatic start after VL53L1X_init() call, put 0x40 in location 0x87 */
// ];

const BASE_ADDR: u8 = 0x29;
// Used for the read() method
const READ_BUFFER_SIZE: usize = 17;

pub struct PiicoDevVL53L1X<'a> {
    pub addr: u8,
    i2c: &'a RefCell<I2CUnifiedMachine>,
}

impl<'a> PiicoDevVL53L1X<'a> {
    pub fn new(addr: Option<u8>, i2c: &'a RefCell<I2CUnifiedMachine>) -> Self {
        let addr: u8 = addr.unwrap_or(BASE_ADDR);
        let mut i2c_mut: RefMut<I2CUnifiedMachine> = i2c.borrow_mut();

        // NOTE: The Python library has a check for compat_ind >= 1 here. I don't know what it does

        let sensor: Self = Self { addr, i2c };

        let s = sensor.reset(&mut i2c_mut);
        if s.is_err() {
            panic!("non")
        }

        i2c_mut.delay(1);
        let model_id: u16 = sensor.read_model_id(&mut i2c_mut).unwrap_or(0xEACC);
        writeln!(i2c_mut.uart, "Model ID: {}", model_id).unwrap();

        // Write default configuration
        // Python: self.i2c.writeto_mem(self.addr, 0x2D, VL51L1X_DEFAULT_CONFIGURATION, addrsize=16)

        // let mut index: usize = 0;
        // let mut config_bytes: [u8; 182] = [0; VL51L1X_DEFAULT_CONFIGURATION.len() * 2]; // VL51L1X_DEFAULT_CONFIGURATION.clone();
        // for byte in VL51L1X_DEFAULT_CONFIGURATION {
        //     config_bytes[index] = 0x00;
        //     config_bytes[index + 1] = *byte;
        //     index += 2;
        // }
        // i2c_mut.write(addr, &config_bytes).unwrap();

        i2c_mut.write(addr, VL51L1X_DEFAULT_CONFIGURATION).unwrap();
        i2c_mut.delay(100);

        // The API triggers this change in VL53L1_init_and_start_range() once a
        // measurement is started; assumes MM1 and MM2 are disabled
        i2c_mut.write(addr, &[0x0022]).unwrap();
        Self::write_reg_16_bit(
            addr,
            0x001E,
            Self::read_16(addr, 0x0022, &mut i2c_mut).unwrap() * 4,
            &mut i2c_mut,
        )
        .unwrap();
        i2c_mut.delay(200);

        sensor
    }

    fn read_model_id(&self, i2c_mut: &mut RefMut<I2CUnifiedMachine>) -> Result<u16, i2c::Error> {
        i2c_mut.write(self.addr, &[0x01, 0x0F])?;

        let mut buffer: [u8; 2] = [0; 2];

        match i2c_mut.read(self.addr, &mut buffer) {
            Ok(()) => Ok(u16::from_le_bytes([buffer[0], buffer[1]])),
            Err(e) => Err(e),
        }

        // Self::read_16(self.addr, 0x010F, i2c_mut)
    }

    fn reset(&self, i2c_mut: &mut RefMut<I2CUnifiedMachine>) -> Result<(), i2c::Error> {
        // Self::write_reg_8_bit(self.addr, 0x0000, 0x00, i2c_mut)?;
        i2c_mut.write(self.addr, &[0x00, 0x00, 0x00])?;
        i2c_mut.delay(100);
        i2c_mut.write(self.addr, &[0x00, 0x00, 0x01])
        // Self::write_reg_8_bit(self.addr, 0x0000, 0x01, i2c_mut)
    }

    fn read_17_bytes(
        &self,
        reg: u16,
        comms: &mut RefMut<I2CUnifiedMachine>,
    ) -> Result<[u8; READ_BUFFER_SIZE], i2c::Error> {
        let reg_bytes: [u8; 2] = reg.to_be_bytes();

        comms
            .write(self.addr, &[reg_bytes[0], reg_bytes[1]])
            .unwrap();

        let mut buffer: [u8; READ_BUFFER_SIZE] = [0; READ_BUFFER_SIZE];

        comms.read(self.addr, &mut buffer)?;

        Ok(buffer)
    }

    pub fn read(&self, comms: &mut RefMut<I2CUnifiedMachine>) -> Result<u16, i2c::Error> {
        let data: [u8; READ_BUFFER_SIZE] = self.read_17_bytes(0x0089, comms)?;
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
        let distance_cm: u16 = final_crosstalk_corrected_range_mm_sd0 / 1000 / 2;
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

        Ok(distance_cm)
    }
}

impl<'a> ByteReader for PiicoDevVL53L1X<'a> {}
