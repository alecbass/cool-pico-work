use super::piicodev_unified::I2CUnifiedMachine;
use crate::piicodev_unified::{HardwareArgs, I2CBase};
use defmt::*;
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

const BUFFER_SIZE: usize = WIDTH as usize * HEIGHT as usize;
const PAGES: u8 = HEIGHT / 8;

#[derive(PartialEq)]
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

pub struct PiicoDevSSD1306 {
    pub i2c: I2CUnifiedMachine,
    buffer: [u8; BUFFER_SIZE],
}

impl PiicoDevSSD1306 {
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

    pub fn new(args: HardwareArgs) -> Self {
        // TODO: Find fixed address
        let i2c = I2CUnifiedMachine::new(args, Some(BASE_ADDR));

        let mut oled = Self {
            i2c,
            buffer: [0; BUFFER_SIZE],
        };
        oled.init_display();
        oled
    }

    pub(self) fn write_cmd(&mut self, command: u8) -> Result<(), i2c::Error> {
        debug!("Writing cmd {}", command);
        self.i2c.write(self.i2c.addr, &[0x80, command])
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
        self.i2c.write(self.i2c.addr, &self.buffer)
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

    pub fn fill_rect(&mut self, x: u8, y: u8, colour: OLEDColour) {
        let x_coord: u32 = x as u32;
        let mut y_coord: u32 = y as u32;
        let mut height: u8 = HEIGHT;
        let width = WIDTH as u32;
        let stride = WIDTH as u32;
        while height > 0 {
            let index: u32 = (y_coord >> 3) * stride + x_coord;
            let offset: u8 = y & 0x07;
            for ww in 0..width {
                self.buffer[(index + ww) as usize] = (self.buffer[(index + ww) as usize]
                    & !(0x01 << offset))
                    | ((u8::from(colour != OLEDColour::BLACK)) << offset);

                debug!("Buffer: {:?}", self.buffer);
            }

            y_coord += 1;
            height -= 1;
        }
    }

    pub fn fill(&mut self, colour: OLEDColour) {
        self.fill_rect(0, 0, colour);
    }

    pub fn circ(&self, x: u8, y: u8, r: u8) {
        let t: u8 = 1;
        let c: u8 = 1;
    }
}
