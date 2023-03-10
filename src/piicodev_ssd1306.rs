use super::piicodev_unified::I2CUnifiedMachine;
use crate::piicodev_unified::{HardwareArgs, I2CBase};
use defmt::*;
use libm::{cosf, sinf};
use rp_pico::hal::i2c;

const BASE_ADDR: u8 = 0x3C;
const _SET_CONTRAST: u8 = 0x81;
const _SET_ENTIRE_ON: u8 = 0xA4;
const _SET_NORM_INV: u8 = 0xA6;
const _SET_DISP: u8 = 0xAE;
const _SET_MEM_ADDR: u8 = 0x20;
const _SET_COL_ADDR: u8 = 0x21;
const _SET_PAGE_ADDR: u8 = 0x22;
const _SET_DISP_START_LINE: u8 = 0x40;
const _SET_SEG_REMAP: u8 = 0xA0;
const _SET_MUX_RATIO: u8 = 0xA8;
const _SET_IREF_SELECT: u8 = 0xAD;
const _SET_COM_OUT_DIR: u8 = 0xC0;
const _SET_DISP_OFFSET: u8 = 0xD3;
const _SET_COM_PIN_CFG: u8 = 0xDA;
const _SET_DISP_CLK_DIV: u8 = 0xD5;
const _SET_PRECHARGE: u8 = 0xD9;
const _SET_VCOM_DESEL: u8 = 0xDB;
const _SET_CHARGE_PUMP: u8 = 0x8D;
const WIDTH: u8 = 128;
const HEIGHT: u8 = 64;

const BUFFER_SIZE: usize = (WIDTH as usize * HEIGHT as usize) / 2; // 4096
const PAGES: u8 = HEIGHT / 8;

#[derive(PartialEq, Clone, Copy)]
pub enum OLEDColour {
    BLACK = 0,
    WHITE = 1,
}

impl Into<u8> for OLEDColour {
    fn into(self) -> u8 {
        match self {
            Self::BLACK => 0,
            Self::WHITE => 1,
        }
    }
}

pub struct PiicoDevSSD1306<'a> {
    addr: u8,
    pub i2c: &'a mut I2CUnifiedMachine,
    buffer: [u8; BUFFER_SIZE],
}

impl<'a> PiicoDevSSD1306<'a> {
    fn init_display(&mut self) {
        for cmd in [
            _SET_DISP, // display off
            // address setting
            _SET_MEM_ADDR,
            0x00, // horizontal
            // resolution and layout
            _SET_DISP_START_LINE,  // start at line 0
            _SET_SEG_REMAP | 0x01, // column addr 127 mapped to SEG0
            _SET_MUX_RATIO,
            HEIGHT - 1,
            _SET_COM_OUT_DIR | 0x08, // scan from COM[N] to COM0
            _SET_DISP_OFFSET,
            0x00,
            _SET_COM_PIN_CFG,
            0x12,
            // timing and driving scheme
            _SET_DISP_CLK_DIV,
            0x80,
            _SET_PRECHARGE,
            0xF1,
            _SET_VCOM_DESEL,
            0x30, // 0.83*Vcc
            // display
            _SET_CONTRAST,
            0xFF,           // maximum
            _SET_ENTIRE_ON, // output follows RAM contents
            _SET_NORM_INV,  // not inverted
            _SET_IREF_SELECT,
            0x30, // enable internal IREF during display on
            // charge pump
            _SET_CHARGE_PUMP,
            0x14,
            _SET_DISP | 0x01, // display on
        ] {
            self.write_cmd(cmd).unwrap();
        }
    }

    pub fn new(i2c: &'a mut I2CUnifiedMachine) -> Self {
        let mut oled = Self {
            addr: BASE_ADDR,
            i2c,
            buffer: [0; BUFFER_SIZE],
        };
        oled.init_display();
        oled
    }

    pub(self) fn write_cmd(&mut self, command: u8) -> Result<(), i2c::Error> {
        debug!("Writing cmd {}", command);
        self.i2c.write(self.addr, &[0x80, command])
    }

    pub fn show(&mut self) -> Result<(), i2c::Error> {
        let x0 = 0;
        let x1 = WIDTH - 1;
        self.write_cmd(_SET_COL_ADDR)?;
        self.write_cmd(x0)?;
        self.write_cmd(x1)?;
        self.write_cmd(_SET_PAGE_ADDR)?;
        self.write_cmd(0)?;
        self.write_cmd(PAGES - 1)?;

        // write_data replacement
        self.buffer[0] = 0x40;
        self.i2c.write(self.addr, &self.buffer)
    }

    pub fn power_off(&mut self) -> Result<(), i2c::Error> {
        self.write_cmd(_SET_DISP)
    }

    pub fn power_on(&mut self) -> Result<(), i2c::Error> {
        self.write_cmd(_SET_DISP | 0x01)
    }

    pub fn set_contrast(&mut self, contrast: u8) -> Result<(), i2c::Error> {
        self.write_cmd(_SET_CONTRAST)?;
        self.write_cmd(contrast)
    }

    pub fn invert(&mut self, invert: u8) -> Result<(), i2c::Error> {
        self.write_cmd(_SET_NORM_INV | (invert & 1))
    }

    pub fn rotate(&mut self, rotate: u8) -> Result<(), i2c::Error> {
        self.write_cmd(_SET_COM_OUT_DIR | ((rotate & 1) << 3))?;
        self.write_cmd(_SET_SEG_REMAP | (rotate & 1))
    }

    pub fn pixel(&mut self, x: u8, y: u8, colour: OLEDColour) {
        fn set_pixel(buffer: &mut [u8], stride: u8, x: u8, y: u8, colour: OLEDColour) {
            let x_coord: u32 = x as u32;
            let y_coord: u32 = y as u32;
            let index: usize = ((y_coord >> 3) * stride as u32 + x_coord) as usize;
            let offset: u32 = y_coord & 0x07;

            let new: u8 = (buffer[index] & !(0x01 << offset))
                | ((u8::from(colour != OLEDColour::BLACK)) << offset);
            buffer[index] = new;
        }

        // let index: usize = (x + y) as usize;
        // self.buffer[index] = colour;
        set_pixel(&mut self.buffer, WIDTH, x, y, colour)
    }

    pub fn fill_rect(&mut self, x: u8, y: u8, x_offset: u8, y_offset: u8, colour: OLEDColour) {
        for x_coord in 0..x {
            for y_coord in 0..y {
                self.pixel(x_coord + x_offset, y_coord + y_offset, colour);
            }
        }
    }

    pub fn fill(&mut self, colour: OLEDColour) {
        const FULL_HEIGHT: u8 = 128;
        const FULL_WIDTH: u8 = 255;
        self.fill_rect(FULL_HEIGHT, FULL_WIDTH, 0, 0, colour);
    }

    pub fn arc(&mut self, x: u8, y: u8, r: u8, start_angle: u8, end_angle: u8) {
        let t: u8 = 0;

        let test: u8 = r * (1 - t) - 1;
        debug!("Hello {}", test);
        let x: f32 = x as f32;
        let y: f32 = y as f32;

        debug!("Hello {}", test);
        for i in (r * (1 - t) - 1)..r {
            for ta in start_angle..end_angle {
                let x: u8 = (i as f32 * (cosf((ta as f64).to_radians() as f32) + x)) as u8;
                let y: u8 = (i as f32 * (sinf((ta as f64).to_radians() as f32) + y)) as u8;
                self.pixel(x, y, OLEDColour::WHITE);
            }
        }
    }

    pub fn circ(&self, x: u8, y: u8, r: u8) {
        let t: u8 = 1;
        let c: u8 = 1;
    }
}
