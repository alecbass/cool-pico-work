mod constants;
mod reading;

use core::fmt::Write;

use bsp::hal::i2c::Error;
use cortex_m::delay::Delay;
use embedded_hal::i2c::I2c;
use rp_pico as bsp;

use crate::{
    i2c::I2CHandler,
    piicodev_qmc6310::constants::{ADDRESS_XOUT, ADDRESS_YOUT, ADDRESS_ZOUT},
    uart::Uart,
};

use self::constants::{
    ADDRESS_CONTROL1, ADDRESS_CONTROL2, ADDRESS_SIGN, ADDRESS_STATUS, BIT_ODR, BIT_OSR1, BIT_OSR2,
    BIT_RANGE, I2C_ADDRESS,
};
use self::reading::MagnetometerReading;

#[derive(Copy, Clone)]
pub enum GaussRange {
    Gauss3000,
    Gauss1200,
    Gauss800,
    Gauss200,
}

impl Into<f32> for GaussRange {
    fn into(self) -> f32 {
        match self {
            GaussRange::Gauss3000 => 1e-3,
            GaussRange::Gauss1200 => 4e-4,
            GaussRange::Gauss800 => 2.6666667e-4,
            GaussRange::Gauss200 => 6.6666667e-5,
        }
    }
}

// range_gauss = {3000:1e-3, 1200:4e-4, 800:2.6666667e-4, 200:6.6666667e-5} # Maps the range (key) to sensitivity (lsb/gauss)
// range_microtesla = {3000:1e-1, 1200:4e-2, 800:2.6666667e-2, 200:6.6666667e-3} # Maps the range (key) to sensitivity (lsb/microtesla)
#[derive(Copy, Clone)]
enum MicroteslaRange {
    Microtesla3000,
    Microtesla1200,
    Microtesla800,
    Microtesla200,
}

impl From<&GaussRange> for MicroteslaRange {
    fn from(range: &GaussRange) -> Self {
        match range {
            GaussRange::Gauss3000 => MicroteslaRange::Microtesla3000,
            GaussRange::Gauss1200 => MicroteslaRange::Microtesla1200,
            GaussRange::Gauss800 => MicroteslaRange::Microtesla800,
            GaussRange::Gauss200 => MicroteslaRange::Microtesla200,
        }
    }
}

impl Into<f32> for MicroteslaRange {
    fn into(self) -> f32 {
        match self {
            MicroteslaRange::Microtesla3000 => 1e-1,
            MicroteslaRange::Microtesla1200 => 4e-2,
            MicroteslaRange::Microtesla800 => 2.6666667e-2,
            MicroteslaRange::Microtesla200 => 6.6666667e-3,
        }
    }
}

/// Reads an individual bit from a byte
fn read_bit(byte: u8, bit_index: u8) -> u8 {
    // Copied from https://users.rust-lang.org/t/extracting-bits-from-bytes/77110/3
    (byte >> bit_index) & 1
}

/// Sets an individual bit of a byte
fn set_bit(x: u8, n: u8) -> u8 {
    x | (1 << n)
}

/// Sets an individual bit of a byte to 0
fn clear_bit(x: u8, n: u8) -> u8 {
    x & !(1 << n)
}

/// Writes a bit to a byte
fn write_bit(x: u8, n: u8, b: u8) -> u8 {
    if b == 0 {
        return clear_bit(x, n);
    }

    set_bit(x, n)
}

fn write_crumb(x: u8, n: u8, c: u8) -> u8 {
    let x = write_bit(x, n, read_bit(c, 0));

    write_bit(x, n + 1, read_bit(c, 1))
}

fn convert_angle_to_positive(angle: f32) -> f32 {
    if angle >= 360.0 {
        return angle - 360.0;
    }

    if angle < 0.0 {
        return angle + 360.0;
    }

    angle
}

///
/// PiicoDev magnometer
///
/// Implementation modified from https://github.com/CoreElectronics/CE-PiicoDev-QMC6310-MicroPython-Module/blob/main/PiicoDev_QMC6310.py
///
pub struct PiicoDevQMC6310 {
    addr: u8,
    odr: u8,
    calibration_file: &'static str,
    suppress_warnings: bool,
    cr1: u8,
    cr2: u8,
    osr1: u8,
    osr2: u8,
    range: GaussRange,
    sensitivity: MicroteslaRange,
    x_offset: u16,
    y_offset: u16,
    z_offset: u16,
    declination: f32,
    data: [u8; 64], // Meant to be a Python dictionary
}

impl PiicoDevQMC6310 {
    pub fn new(addr: Option<u8>, range: Option<GaussRange>, declination: f32) -> Self {
        let addr = addr.unwrap_or(I2C_ADDRESS);
        let odr = 3;
        let osr1 = 0;
        let osr2 = 3;
        let calibration_file = "calibration.cal";
        let suppress_warnings = false;
        let cr1 = 0x00;
        let cr2 = 0x00;
        let range = range.unwrap_or(GaussRange::Gauss3000);
        let sensitivity = MicroteslaRange::from(&range);
        let x_offset = 0;
        let y_offset = 0;
        let z_offset = 0;
        let data = [0; 64];

        Self {
            addr,
            odr,
            calibration_file,
            suppress_warnings,
            cr1,
            cr2,
            osr1,
            osr2,
            range,
            sensitivity,
            x_offset,
            y_offset,
            z_offset,
            declination,
            data,
        }
    }

    pub fn init(&mut self, i2c: &mut I2CHandler) -> Result<(), Error> {
        let sign_x = 0;
        let sign_y = 1;
        let sign_z = 1;
        let sign = sign_x + sign_y * 2 + sign_z * 4;

        self.set_mode(1, i2c)?;
        self.set_output_data_rate(self.odr, i2c)?;
        self.set_oversampling_ratio(self.osr1, i2c)?;
        self.set_oversampling_rate(self.osr2, i2c)?;
        self.set_range(self.range, i2c)?;
        self.set_sign(sign, i2c)?;
        self.load_calibration();

        Ok(())
    }

    fn set_mode(&mut self, mode: u8, i2c: &mut I2CHandler) -> Result<(), Error> {
        self.cr1 = write_crumb(self.cr1, 0, mode);
        i2c.write(self.addr, &[ADDRESS_CONTROL1, self.cr1])
    }

    fn set_output_data_rate(&mut self, odr: u8, i2c: &mut I2CHandler) -> Result<(), Error> {
        self.cr1 = write_crumb(self.cr1, BIT_ODR, odr);
        i2c.write(self.addr, &[ADDRESS_CONTROL1, self.cr1])
    }

    fn set_oversampling_ratio(&mut self, osr1: u8, i2c: &mut I2CHandler) -> Result<(), Error> {
        self.cr1 = write_crumb(self.cr1, BIT_OSR1, osr1);
        i2c.write(self.addr, &[ADDRESS_CONTROL1, self.cr1])
    }

    fn set_oversampling_rate(&mut self, osr2: u8, i2c: &mut I2CHandler) -> Result<(), Error> {
        self.cr1 = write_crumb(self.cr1, BIT_OSR2, osr2);
        i2c.write(self.addr, &[ADDRESS_CONTROL1, self.cr1])
    }

    fn set_range(&mut self, range: GaussRange, i2c: &mut I2CHandler) -> Result<(), Error> {
        let range_bit = match range {
            GaussRange::Gauss3000 => 0,
            GaussRange::Gauss1200 => 1,
            GaussRange::Gauss800 => 2,
            GaussRange::Gauss200 => 3,
        };

        self.range = range;
        self.sensitivity = MicroteslaRange::from(&range);
        self.cr2 = write_crumb(self.cr2, BIT_RANGE, range_bit);

        i2c.write(self.addr, &[ADDRESS_CONTROL2, self.cr2])
    }

    fn set_sign(&mut self, sign: u8, i2c: &mut I2CHandler) -> Result<(), Error> {
        i2c.write(self.addr, &[ADDRESS_SIGN, sign])
    }

    fn get_control_registers(&mut self, i2c: &mut I2CHandler) -> Result<[u8; 2], Error> {
        let mut buffer = [0; 2];
        i2c.write_read(self.addr, &[ADDRESS_CONTROL1], &mut buffer)?;

        Ok(buffer)
    }

    fn get_status_ready(&self, status: u8) -> bool {
        read_bit(status, 0) != 0
    }

    fn get_status_overflow(&self, status: u8) -> bool {
        read_bit(status, 1) != 0
    }

    fn read(&mut self, raw: bool, i2c: &mut I2CHandler) -> Result<(f32, f32, f32), Error> {
        fn calculate_value(raw_value: u16, offset: u16) -> f32 {
            let mut value = raw_value as f32;
            let offset = offset as f32;

            if raw_value >= 0x8000 {
                value = !((65535 - raw_value) + 1) as f32;
            }

            value - offset
        }

        const NAN: (f32, f32, f32) = (f32::NAN, f32::NAN, f32::NAN);

        // Create a buffer to hold one bit
        let mut buffer = [0; 1];

        let status_result = i2c.write_read(self.addr, &[ADDRESS_STATUS], &mut buffer);

        if status_result.is_err() {
            return Ok(NAN);
        }

        let status = buffer[0];

        let is_status_ready = self.get_status_ready(status);

        if !is_status_ready {
            return Ok(NAN);
        }

        // Re-initialise the buffer to hold two bits
        let mut buffer = [0; 2];

        // Read x
        i2c.write_read(self.addr, &[ADDRESS_XOUT], &mut buffer)?;
        let x_from_buffer = u16::from_le_bytes(buffer);

        // Read y
        i2c.write_read(self.addr, &[ADDRESS_YOUT], &mut buffer)?;
        let y_from_buffer = u16::from_le_bytes(buffer);

        // Read z
        i2c.write_read(self.addr, &[ADDRESS_ZOUT], &mut buffer)?;
        let z_from_buffer = u16::from_le_bytes(buffer);

        let is_status_overflow = self.get_status_overflow(status);

        if is_status_overflow {
            return Ok(NAN);
        }

        let x = calculate_value(x_from_buffer, self.x_offset);
        let y = calculate_value(y_from_buffer, self.y_offset);
        let z = calculate_value(z_from_buffer, self.z_offset);

        let sensitivity: f32 = self.sensitivity.into();

        if !raw {
            let x = x * sensitivity;
            let y = y * sensitivity;
            let z = z * sensitivity;

            let sample = (x, y, z);

            return Ok(sample);
        }

        let sample = (x, y, z);

        Ok(sample)
    }

    pub fn read_polar(
        &mut self,
        i2c: &mut I2CHandler,
        uart: &mut Uart,
    ) -> Result<MagnetometerReading, Error> {
        const PI: f32 = 3.14159265358979323846;

        let (x, y, z) = self.read(false, i2c)?;
        // writeln!(uart, "{} {} {}", x, y, z).unwrap();

        let angle = (libm::atan2f(x, -y) / PI) * 180.0 + self.declination;
        // writeln!(uart, "{}", angle).unwrap();
        let angle = convert_angle_to_positive(angle);

        let magnitude = libm::sqrtf(x * x + y * y + z * z);
        let gauss = magnitude * 100.0;

        let reading = MagnetometerReading {
            polar: angle,
            gauss,
            magnitude,
        };

        Ok(reading)
    }

    pub fn calibrate(
        &mut self,
        enable_logging: bool,
        i2c: &mut I2CHandler,
        uart: &mut Uart,
        delay: &mut Delay,
    ) -> Result<(), Error> {
        self.set_output_data_rate(3, i2c)?;

        let mut x_min = 65535.0;
        let mut x_max = -65535.0;
        let mut y_min = 65535.0;
        let mut y_max = -65535.0;
        let mut z_min = 65535.0;
        let mut z_max = -65535.0;

        // log = ''
        writeln!(
            uart,
            "*** Calibrating.\n    Slowly rotate your sensor until the bar is full"
        )
        .unwrap();
        write!(uart, "[          ]").unwrap();
        let range = 1000;
        let mut i = 0;

        let mut x = 0.0;
        let mut y = 0.0;
        let mut z = 0.0;

        // EMA filter weight
        let a = 0.5;

        while i < range {
            i += 1;
            delay.delay_ms(5);

            let (polar, gauss, magnitude) = self.read(true, i2c)?;

            x = a * polar + (1.0 - a) * x;
            y = a * gauss + (1.0 - a) * y;
            z = a * magnitude + (1.0 - a) * z;

            if x < x_min {
                x_min = x;
                i = 0;
            }

            if x > x_max {
                x_max = x;
                i = 0;
            }

            if y < y_min {
                y_min = y;
                i = 0;
            }

            if y > y_max {
                y_max = y;
                i = 0;
            }

            if z < z_min {
                z_min = z;
                i = 0;
            }

            if z > z_max {
                z_max = z;
                i = 0;
            }

            let j = 10 * i / range;

            write!(uart, "\015[").unwrap();

            for _ in 0..j {
                write!(uart, "*").unwrap();
            }

            for _ in 0..10 - j {
                write!(uart, " ").unwrap();
            }

            writeln!(uart, "]").unwrap();

            if enable_logging {
                let _ = writeln!(uart, "x: {}, y: {}, z: {}", x, y, z);
            }
        }

        // set the output data rate back to the user selected rate
        self.set_output_data_rate(self.odr, i2c)?;

        let x_offset = (x_max + x_min) as u16 / 2;
        let y_offset = (y_max + y_min) as u16 / 2;
        let z_offset = (z_max + z_min) as u16 / 2;

        write!(
            uart,
            "x_min:\n{}\nx_max:\n{}\ny_min:\n{}\ny_max:\n{}\nz_min:\n{}\nz_max:\n{}\nx_offset:\n{}\ny_offset:\n{}\nz_offset:\n{}",
            x_min, x_max, y_min, y_max, z_min, z_max, x_offset, y_offset, z_offset
        )
        .unwrap();

        // f = open(self.calibrationFile, "w")
        // f.write('x_min:\n' + str(x_min) + '\nx_max:\n' + str(x_max) + '\ny_min:\n' + str(y_min) + '\ny_max:\n' + str(y_max) + '\nz_min\n' + str(z_min) + '\nz_max:\n' + str(z_max) + '\nx_offset:\n')
        // f.write(str(self.x_offset) + '\ny_offset:\n' + str(self.y_offset) + '\nz_offset:\n' + str(self.z_offset))
        // f.close()
        // if enable_logging:
        //     flog = open("calibration.log", "w")
        //     flog.write(log)
        //     flog.close

        Ok(())
    }

    fn load_calibration(&mut self) {
        // Harcoded from a previous test
        // let x_min = 0.9501879;
        // let x_max = 65525.46;
        // let y_min = 3.8511074;
        // let y_max = 65533.85;
        // let z_min = 2.381914;
        // let z_max = 65522.266;
        const X_OFFSET: u16 = 32717;
        const Y_OFFSET: u16 = 32765;
        const Z_OFFSET: u16 = 32714;

        self.x_offset = X_OFFSET;
        self.y_offset = Y_OFFSET;
        self.z_offset = Z_OFFSET;
    }
}

//     def readMagnitude(self):
//         return self.readPolar()['uT']
//
//     def readHeading(self):
//         return self.readPolar()['polar']
//

//

//
//     def loadCalibration(self):
//         try:
//             f = open(self.calibrationFile, "r")
//             for i in range(13): f.readline()
//             self.x_offset = float(f.readline())
//             f.readline()
//             self.y_offset = float(f.readline())
//             f.readline()
//             self.z_offset = float(f.readline())
//             sleep_ms(5)
//         except:
//             if not self.suppress_warnings:
//                 print("No calibration file found. Run 'calibrate()' for best results.  Visit https://piico.dev/p15 for more info.")
//             sleep_ms(1000)
