mod constants;
mod reading;

use bsp::hal::i2c::Error;
use embedded_hal::i2c::I2c;
use rp_pico as bsp;

use crate::{
    i2c::I2CHandler,
    piicodev_qmc6310::constants::{ADDRESS_XOUT, ADDRESS_YOUT, ADDRESS_ZOUT},
};

use self::constants::{
    ADDRESS_CONTROL1, ADDRESS_SIGN, ADDRESS_STATUS, BIT_ODR, BIT_OSR1, BIT_OSR2, I2C_ADDRESS,
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

fn read_bit(x: u8, n: u8) -> u8 {
    let is_n_not_zero = (n != 0) as u8;
    x & 1 << is_n_not_zero
}

fn set_bit(x: u8, n: u8) -> u8 {
    x | (1 << n)
}

fn clear_bit(x: u8, n: u8) -> u8 {
    x & !(1 << n)
}

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
    declination: u8,
    data: [u8; 64], // Meant to be a Python dictionary
}

impl PiicoDevQMC6310 {
    pub fn new(addr: Option<u8>, range: Option<GaussRange>) -> Self {
        let addr = addr.unwrap_or(I2C_ADDRESS);
        let odr = 3;
        let calibration_file = "calibration.cal";
        let suppress_warnings = false;
        let cr1 = 0x00;
        let cr2 = 0x00;
        let osr1 = 0;
        let osr2 = 3;
        let range = range.unwrap_or(GaussRange::Gauss3000);
        let sensitivity = MicroteslaRange::from(&range);
        let x_offset = 0;
        let y_offset = 0;
        let z_offset = 0;
        let declination = 0;
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

        Ok(())
    }

    fn set_mode(&mut self, mode: u8, i2c: &mut I2CHandler) -> Result<(), Error> {
        self.cr1 = write_crumb(self.cr1, 0, mode);
        i2c.write(self.addr, &[self.cr1])
    }

    fn set_output_data_rate(&mut self, odr: u8, i2c: &mut I2CHandler) -> Result<(), Error> {
        self.cr1 = write_crumb(self.cr1, BIT_ODR, odr);

        // self.i2c.writeto_mem(self.addr, _ADDRESS_CONTROL1, bytes([self._CR1]))
        i2c.write(self.addr, &[ADDRESS_CONTROL1, self.cr1])
    }

    fn set_oversampling_ratio(&mut self, osr1: u8, i2c: &mut I2CHandler) -> Result<(), Error> {
        self.cr1 = write_crumb(self.cr1, BIT_OSR1, osr1);
        // self.i2c.writeto_mem(self.addr, _ADDRESS_CONTROL1, bytes([self._CR1]))
        i2c.write(self.addr, &[ADDRESS_CONTROL1, self.cr1])
    }

    fn set_oversampling_rate(&mut self, osr2: u8, i2c: &mut I2CHandler) -> Result<(), Error> {
        self.cr1 = write_crumb(self.cr1, BIT_OSR2, osr2);
        // self.i2c.writeto_mem(self.addr, _ADDRESS_CONTROL1, bytes([self._CR1]))
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
        self.cr2 = write_crumb(self.cr2, BIT_OSR2, range_bit);

        i2c.write(self.addr, &[ADDRESS_CONTROL1, self.cr2])
    }

    fn set_sign(&mut self, sign: u8, i2c: &mut I2CHandler) -> Result<(), Error> {
        i2c.write(self.addr, &[ADDRESS_SIGN, sign])
    }

    fn get_control_registers(&mut self, i2c: &mut I2CHandler) -> Result<[u8; 2], Error> {
        let mut buffer = [0; 2];
        i2c.write_read(self.addr, &[ADDRESS_CONTROL1], &mut buffer)?;

        Ok(buffer)
    }

    fn get_status_ready(&self, status: u8) -> u8 {
        read_bit(status, 0)
    }

    fn get_status_overflow(&self, status: u8) -> u8 {
        read_bit(status, 1)
    }

    fn read(&mut self, raw: bool, i2c: &mut I2CHandler) -> Result<(f32, f32, f32), Error> {
        let nan = (f32::NAN, f32::NAN, f32::NAN);

        // Create a buffer to hold one bit
        let mut buffer = [0; 1];

        let status_result = i2c.write_read(self.addr, &[ADDRESS_STATUS], &mut buffer);

        if status_result.is_err() {
            return Ok(nan);
        }

        let status = buffer[0];

        // Re-initialise the buffer to hold two bits
        let mut buffer = [0; 2];

        let is_status_ready = self.get_status_ready(status) != 0;

        if !is_status_ready {
            return Ok(nan);
        }

        // Read x
        i2c.write_read(self.addr, &[ADDRESS_XOUT], &mut buffer)?;
        let x_from_buffer = u16::from_le_bytes([buffer[0], buffer[1]]);

        // Read y
        i2c.write_read(self.addr, &[ADDRESS_YOUT], &mut buffer)?;
        let y_from_buffer = u16::from_le_bytes([buffer[0], buffer[1]]);

        // Read z
        i2c.write_read(self.addr, &[ADDRESS_ZOUT], &mut buffer)?;
        let z_from_buffer = u16::from_le_bytes([buffer[0], buffer[1]]);

        let is_status_overflow = self.get_status_overflow(status) != 0;

        if is_status_overflow {
            return Ok(nan);
        }

        fn calculate_value(raw_value: u16, offset: u16) -> u16 {
            let mut value = raw_value;
            if value >= 0x8000 {
                value = !((65535 - raw_value) + 1);
            }

            value - offset
        }

        let x = calculate_value(x_from_buffer, self.x_offset) as f32;
        let y = calculate_value(y_from_buffer, self.y_offset) as f32;
        let z = calculate_value(z_from_buffer, self.z_offset) as f32;

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

    pub fn read_polar(&mut self, i2c: &mut I2CHandler) -> Result<MagnetometerReading, Error> {
        const PI: f32 = 3.14159265358979323846;

        let (x, y, z) = self.read(false, i2c)?;

        let angle = (libm::atan2f(x, y) / PI) * 180.0 + self.declination as f32;
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
}

//     def readMagnitude(self):
//         return self.readPolar()['uT']
//
//     def readHeading(self):
//         return self.readPolar()['polar']
//
//     def setDeclination(self, dec):
//         self.declination = dec
//
//     def calibrate(self, enable_logging=False):
//         try:
//             self.setOutputDataRate(3)
//         except Exception as e:
//             print(i2c_err_str.format(self.addr))
//             raise e
//         x_min = 65535
//         x_max = -65535
//         y_min = 65535
//         y_max = -65535
//         z_min = 65535
//         z_max = -65535
//         log = ''
//         print('*** Calibrating.\n    Slowly rotate your sensor until the bar is full')
//         print('[          ]', end='')
//         range = 1000
//         i = 0
//         x=0;y=0;z=0;
//         a=0.5 # EMA filter weight
//         while i < range:
//             i += 1
//             sleep_ms(5)
//             d = self.read(raw=True)
//             x = a*d['x'] + (1-a)*x # EMA filter
//             y = a*d['y'] + (1-a)*y
//             z = a*d['z'] + (1-a)*z
//             if x < x_min: x_min = x; i=0
//             if x > x_max: x_max = x; i=0
//             if y < y_min: y_min = y; i=0
//             if y > y_max: y_max = y; i=0
//             if z < z_min: z_min = z; i=0
//             if z > z_max: z_max = z; i=0
//             j = round(10*i/range);
//             print( '\015[' + int(j)*'*' + int(10-j)*' ' + ']', end='') # print a progress bar
//             if enable_logging:
//                 log = log + (str(d['x']) + ',' + str(d['y']) + ',' + str(d['z']) + '\n')
//         self.setOutputDataRate(self.odr) # set the output data rate back to the user selected rate
//         self.x_offset = (x_max + x_min) / 2
//         self.y_offset = (y_max + y_min) / 2
//         self.z_offset = (z_max + z_min) / 2
//         f = open(self.calibrationFile, "w")
//         f.write('x_min:\n' + str(x_min) + '\nx_max:\n' + str(x_max) + '\ny_min:\n' + str(y_min) + '\ny_max:\n' + str(y_max) + '\nz_min\n' + str(z_min) + '\nz_max:\n' + str(z_max) + '\nx_offset:\n')
//         f.write(str(self.x_offset) + '\ny_offset:\n' + str(self.y_offset) + '\nz_offset:\n' + str(self.z_offset))
//         f.close()
//         if enable_logging:
//             flog = open("calibration.log", "w")
//             flog.write(log)
//             flog.close
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
