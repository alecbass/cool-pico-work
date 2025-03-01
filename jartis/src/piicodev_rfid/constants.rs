#![allow(unused)]

pub const I2C_ADDRESS: u8 = 0x2C;
pub const REG_COMMAND: u8 = 0x01;
pub const REG_COM_I_EN: u8 = 0x02;
pub const REG_DIV_I_EN: u8 = 0x03;
pub const REG_COM_IRQ: u8 = 0x04;
pub const REG_DIV_IRQ: u8 = 0x05;
pub const REG_ERROR: u8 = 0x06;
pub const REG_STATUS_1: u8 = 0x07;
pub const REG_STATUS_2: u8 = 0x08;
pub const REG_FIFO_DATA: u8 = 0x09;
pub const REG_FIFO_LEVEL: u8 = 0x0A;
pub const REG_CONTROL: u8 = 0x0C;
pub const REG_BIT_FRAMING: u8 = 0x0D;
pub const REG_MODE: u8 = 0x11;
pub const REG_TX_CONTROL: u8 = 0x14;
pub const REG_TX_ASK: u8 = 0x15;
pub const REG_CRC_RESULT_MSB: u8 = 0x21;
pub const REG_CRC_RESULT_LSB: u8 = 0x22;
pub const REG_T_MODE: u8 = 0x2A;
pub const REG_T_PRESCALER: u8 = 0x2B;
pub const REG_T_RELOAD_HI: u8 = 0x2C;
pub const REG_T_RELOAD_LO: u8 = 0x2D;
pub const REG_AUTO_TEST: u8 = 0x36;
pub const REG_VERSION: u8 = 0x37;
pub const CMD_IDLE: u8 = 0x00;
pub const CMD_CALC_CRC: u8 = 0x03;
pub const CMD_TRANCEIVE: u8 = 0x0C;
pub const CMD_MF_AUTHENT: u8 = 0x0E;
pub const CMD_SOFT_RESET: u8 = 0x0F;

// RFID Tag (Proximity Integrated Circuit Card)
pub const TAG_CMD_REQIDL: u8 = 0x26;
pub const TAG_CMD_REQALL: u8 = 0x52;
pub const TAG_CMD_ANTCOL1: u8 = 0x93;
pub const TAG_CMD_ANTCOL2: u8 = 0x95;
pub const TAG_CMD_ANTCOL3: u8 = 0x97;

// Classic
pub const TAG_AUTH_KEY_A: u8 = 0x60;
pub const CLASSIC_KEY: [u8; 6] = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];

// Tag detection statuses
pub const OK: u8 = 1;
pub const NOTAGERR: u8 = 2;
pub const ERR: u8 = 3;
