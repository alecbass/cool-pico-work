use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::mono_font::ascii::{FONT_6X10, FONT_6X12};
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Primitive, PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::text::{Alignment, Text};
use embedded_hal::i2c::I2c;
use rp_pico::hal::i2c::Error;
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306};

use crate::i2c::I2CHandler;

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

type Ssd1306OledDisplay =
    Ssd1306<I2CInterface<I2CHandler>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>;

pub struct PiicoDevSSD1306 {
    display: Ssd1306OledDisplay,
}

impl PiicoDevSSD1306 {
    pub fn new(mut i2c: I2CHandler) -> Self {
        // Initialise the OLED display
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
            Self::write_cmd(&mut i2c, cmd).unwrap();
        }

        // Convert the I2C into an OLED display
        let interface = I2CDisplayInterface::new(i2c);
        let display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();

        Self { display }
    }

    /// Writes a given byte as an I2C command to the OLED display
    fn write_cmd(i2c: &mut I2CHandler, command: u8) -> Result<(), Error> {
        i2c.write(BASE_ADDR, &[0x80, command])
    }

    pub fn set_white_background(&mut self) -> Result<(), ()> {
        let white_rectangle_style = PrimitiveStyleBuilder::new()
            .fill_color(BinaryColor::On)
            .stroke_color(BinaryColor::On)
            .stroke_width(3)
            .build();

        Rectangle::new(Point::zero(), Size::new(128, 64))
            .into_styled(white_rectangle_style)
            .draw(&mut self.display)
            .map_err(|_e| ())?;

        self.display.flush().map_err(|_e| ())
    }

    pub fn write_text(&mut self, text: &str) -> Result<(), ()> {
        self.display.clear(BinaryColor::On).map_err(|_e| ())?;

        let black_text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X12)
            .text_color(BinaryColor::Off)
            .build();

        Text::with_alignment(text, Point::new(64, 8), black_text_style, Alignment::Center)
            .draw(&mut self.display)
            .map_err(|_e| ())?;

        self.display.flush().map_err(|_e| ())
    }

    pub fn reset(&mut self) -> Result<(), ()> {
        self.display.clear(BinaryColor::On).map_err(|_e| ())?;
        self.display.flush().map_err(|_e| ())
    }
}
