use core::fmt::Write;

use cortex_m::delay::Delay;
use embedded_hal::i2c::I2c;
use rp_pico::hal::{i2c, I2C};

use crate::i2c::I2CHandler;
use crate::piicodev_rfid::{
    constants::{TAG_CMD_ANTCOL2, TAG_CMD_ANTCOL3},
    types::{TagId, TagType},
};
use crate::uart::Uart;

use super::constants::{
    CMD_CALC_CRC, CMD_IDLE, CMD_MF_AUTHENT, CMD_SOFT_RESET, CMD_TRANCEIVE, ERR, I2C_ADDRESS,
    NOTAGERR, OK, REG_BIT_FRAMING, REG_COMMAND, REG_COM_IRQ, REG_COM_I_EN, REG_CONTROL,
    REG_CRC_RESULT_LSB, REG_CRC_RESULT_MSB, REG_DIV_IRQ, REG_DIV_I_EN, REG_ERROR, REG_FIFO_DATA,
    REG_FIFO_LEVEL, REG_MODE, REG_TX_ASK, REG_TX_CONTROL, REG_T_MODE, REG_T_PRESCALER,
    REG_T_RELOAD_HI, REG_T_RELOAD_LO, TAG_CMD_ANTCOL1, TAG_CMD_REQIDL,
};

pub struct PiicoDevRfid {
    i2c: I2CHandler,
}

impl PiicoDevRfid {
    pub fn new(i2c: I2CHandler) -> Self {
        Self { i2c }
    }

    pub fn init(&mut self, delay: &mut Delay) -> Result<(), i2c::Error> {
        self.reset()?;
        delay.delay_ms(50);

        let address = I2C_ADDRESS;
        self.i2c.write(address, &[REG_T_MODE, 0x80])?;
        self.i2c.write(address, &[REG_T_PRESCALER, 0xA9])?;
        self.i2c.write(address, &[REG_T_RELOAD_HI, 0x03])?;
        self.i2c.write(address, &[REG_T_RELOAD_LO, 0xE8])?;
        self.i2c.write(address, &[REG_TX_ASK, 0x40])?;
        self.i2c.write(address, &[REG_MODE, 0x3D])?;
        self.i2c.write(address, &[REG_DIV_I_EN, 0x80])?; // CMOS Logic for IRQ pin
        self.i2c.write(address, &[REG_COM_I_EN, 0x20])?; // Allows the receiver interrupt request (RxIRq bit) to be propagated to pin IRQ
        self.antenna_on()
    }

    fn read_reg_byte(&mut self, register: u8) -> Result<u8, i2c::Error> {
        let mut read_buffer = [0; 1];

        let address = I2C_ADDRESS;
        self.i2c.write(address, &[register])?;
        self.i2c.read(address, &mut read_buffer)?;
        // self.i2c
        //     .write_read(address, &[register], &mut read_buffer)?;

        Ok(read_buffer[0])
    }

    /// I2C write to FIFO buffer
    fn write_to_fifo(&mut self, reg: u8, value: &[u8]) -> Result<(), i2c::Error> {
        let address = I2C_ADDRESS;
        self.i2c.write(address, &[reg, value[0]])
    }

    fn set_register_flags(&mut self, register: u8, mask: u8) -> Result<(), i2c::Error> {
        let address = I2C_ADDRESS;
        let current_value = self.read_reg_byte(register)?;

        self.i2c.write(address, &[register, current_value | mask])
    }

    fn clear_register_flags(&mut self, register: u8, mask: u8) -> Result<(), i2c::Error> {
        let address = I2C_ADDRESS;
        let current_value = self.read_reg_byte(register)?;
        self.i2c
            .write(address, &[register, current_value & (!mask)])
    }

    /// Resets the RFID module
    pub fn reset(&mut self) -> Result<(), i2c::Error> {
        let address = I2C_ADDRESS;

        self.i2c.write(address, &[REG_COMMAND, CMD_SOFT_RESET])
    }

    /// Communicates with the tag
    fn to_card(
        &mut self,
        cmd: u8,
        send: &[u8],
        uart: &mut Uart,
    ) -> Result<(u8, [u8; 1024], usize), i2c::Error> {
        let mut recv = [0; 1024];
        let mut wait_irq = 0;
        let mut irq_en = 0;
        let mut bits: usize = 0;
        let mut n: usize = 0;
        let mut status = ERR;

        if cmd == CMD_MF_AUTHENT {
            irq_en = 0x12;
            wait_irq = 0x10;
        } else if cmd == CMD_TRANCEIVE {
            irq_en = 0x77;
            wait_irq = 0x30;
        }

        let address = I2C_ADDRESS;
        self.i2c.write(address, &[REG_COMMAND, CMD_IDLE])?; // Stop any active command.
        self.i2c.write(address, &[REG_COM_IRQ, 0x7F])?; // Clear all seven interrupt request bits
        self.set_register_flags(REG_FIFO_LEVEL, 0x80)?; // FlushBuffer = 1, FIFO initialization
        self.write_to_fifo(REG_FIFO_DATA, send)?; // Write to the FIFO

        if cmd == CMD_TRANCEIVE {
            self.set_register_flags(REG_BIT_FRAMING, 0x00)?; // This starts the transceive operation
        }

        self.i2c.write(address, &[REG_COMMAND, cmd])?;

        if cmd == CMD_TRANCEIVE {
            self.set_register_flags(REG_BIT_FRAMING, 0x80)?; // This starts the transceive operation
        }

        let mut i = 20000; // 2000

        loop {
            n = self.read_reg_byte(REG_COM_IRQ)? as usize;
            i -= 1;

            if n & wait_irq != 0 {
                break;
            }

            if (n & 0x01) == 0x01 {
                break;
            }

            if i == 0 {
                break;
            }
        }

        self.clear_register_flags(REG_BIT_FRAMING, 0x80)?;

        if i > 0 {
            let read = self.read_reg_byte(REG_ERROR)?;

            if (read & 0x1B) == 0x00 {
                status = OK;

                if (n & irq_en & 0x01) == 0x01 {
                    status = NOTAGERR;
                } else if cmd == CMD_TRANCEIVE {
                    n = self.read_reg_byte(REG_FIFO_LEVEL)?.into();
                    let lbits: usize = (self.read_reg_byte(REG_CONTROL)? as usize) & 0x07;

                    if lbits != 0 {
                        bits = (n - 1) * 8 + lbits
                    } else {
                        bits = n * 8
                    }

                    if n == 0 {
                        n = 1
                    } else if n > 16 {
                        n = 16
                    }

                    for i in 0..n {
                        let read = self.read_reg_byte(REG_FIFO_DATA)?;
                        recv[i] = read;
                        // recv.append(self.read_reg_byte(_REG_FIFO_DATA)?)
                    }
                }
            } else {
                status = ERR;
            }
        }

        Ok((status, recv, bits))
    }

    /// Use the co-processor on the RFID module to obtain CRC
    pub fn crc(&mut self, data: &[u8]) -> Result<[u8; 2], i2c::Error> {
        let address = I2C_ADDRESS;
        self.i2c.write(address, &[REG_COMMAND, CMD_IDLE])?;
        self.clear_register_flags(REG_DIV_IRQ, 0x04)?;
        self.set_register_flags(REG_FIFO_LEVEL, 0x80)?;

        for c in data {
            self.i2c.write(REG_FIFO_DATA, &[*c])?;
        }

        self.i2c.write(REG_COMMAND, &[CMD_CALC_CRC])?;

        let mut i: u8 = 0xFF;
        loop {
            let n = self.read_reg_byte(REG_DIV_IRQ)?;
            i -= 1;
            if !((i != 0) && (n & 0x04) == 0) {
                break;
            }
        }

        self.i2c.write(REG_COMMAND, &[CMD_IDLE])?;
        Ok([
            self.read_reg_byte(REG_CRC_RESULT_LSB)?,
            self.read_reg_byte(REG_CRC_RESULT_MSB)?,
        ])
    }

    /// Invites tag in state IDLE to go to READY
    fn request(&mut self, mode: u8, uart: &mut Uart) -> Result<(u8, usize), i2c::Error> {
        let address = I2C_ADDRESS;
        self.i2c.write(address, &[REG_BIT_FRAMING, 0x07])?;
        let (mut stat, _recv, bits) = self.to_card(CMD_TRANCEIVE, &[mode], uart)?;

        if (stat != OK) | (bits != 0x10) {
            stat = ERR
        }

        Ok((stat, bits))
    }

    /// Perform anticollision check
    fn anti_collision_check(
        &mut self,
        anti_col_n: u8,
        uart: &mut Uart,
    ) -> Result<(u8, [u8; 1024]), i2c::Error> {
        let mut ser_chk = 0;
        let ser = [anti_col_n, 0x20];

        self.i2c.write(REG_BIT_FRAMING, &[0x00])?;

        let (mut stat, recv, _bits) = self.to_card(CMD_TRANCEIVE, &ser, uart)?;

        if stat == OK {
            if recv.len() == 5 {
                for i in 0..4 {
                    ser_chk = ser_chk ^ recv[i];
                }
                if ser_chk != recv[4] {
                    stat = ERR;
                }
            } else {
                stat = ERR;
            }
        }

        Ok((stat, recv))
    }

    /// Select the desired tag
    fn select_tag(
        &mut self,
        ser_num: &[u8],
        anti_col_n: u8,
        uart: &mut Uart,
    ) -> Result<u8, i2c::Error> {
        let mut buf = [0; 64];

        buf[0] = anti_col_n;
        buf[1] = 0x70;

        let mut index: usize = 2;

        for i in ser_num {
            buf[*i as usize] = *i;
            index = *i as usize;
        }

        let p_out = self.crc(&buf)?;
        buf[index] = p_out[0];
        buf[index + 1] = p_out[1];

        let (status, _back_data, back_len) = self.to_card(0x0C, &buf, uart)?;
        if status == OK && back_len == 0x18 {
            return Ok(1);
        }

        Ok(0)
    }

    /// Returns detailed information about the tag
    fn read_tag_id_private(&mut self, uart: &mut Uart) -> Result<TagId, i2c::Error> {
        let result = TagId {
            success: false,
            id_integers: [0; 1024],
            id_formatted: [0; 1024],
            tag_type: TagType::Classic,
        };
        let mut valid_uid = [0; 1024];
        let (status, uid) = self.anti_collision_check(TAG_CMD_ANTCOL1, uart)?;

        if status != OK {
            return Ok(result);
        }

        if self.select_tag(&uid, TAG_CMD_ANTCOL1, uart)? == 0 {
            return Ok(result);
        }

        if uid[0] == 0x88 {
            // NTAG
            for i in 1..4 {
                valid_uid[i - 1] = uid[i];
            }

            let (status, uid) = self.anti_collision_check(TAG_CMD_ANTCOL2, uart)?;

            if status != OK {
                return Ok(result);
            }

            let rtn = self.select_tag(&uid, TAG_CMD_ANTCOL2, uart)?;

            if rtn == 0 {
                return Ok(result);
            }

            // Now check again if uid[0] is 0x88
            if uid[0] == 0x88 {
                for i in 1..4 {
                    valid_uid[i + 4] = uid[i];
                }

                let (status, _uid) = self.anti_collision_check(TAG_CMD_ANTCOL3, uart)?;

                if status != OK {
                    return Ok(result);
                }
            }
        }

        for i in 0..5 {
            valid_uid[i + 8] = uid[i];
        }

        // Format the ID into a string
        let id_formatted = [0; 1024];

        // TODO: Format this into a string
        let uid_length = uid.len();
        // for i in 0..uid_length {
        //     if i > 0 {
        //         id_formatted
        //     }
        // }

        // id = valid_uid[:len(valid_uid)-1]
        // for i in range(0,len(id)):
        //     if i > 0:
        //         id_formatted = id_formatted + ':'
        //     if id[i] < 16:
        //         id_formatted = id_formatted + '0'
        //     id_formatted = id_formatted + hex(id[i])[2:]
        //
        let tag_type = match uid_length {
            4 => TagType::Classic,
            _ => TagType::NTag,
        };

        Ok(TagId {
            success: true,
            id_integers: uid,
            id_formatted,
            tag_type,
        })
    }

    /// Detect the presence of a tag
    fn _detect_tag(&mut self, uart: &mut Uart) -> Result<(bool, usize), i2c::Error> {
        let (stat, atqa) = self.request(TAG_CMD_REQIDL, uart)?;
        let present = stat == OK;

        Ok((present, atqa))
    }

    // Turns the antenna on
    fn antenna_on(&mut self) -> Result<(), i2c::Error> {
        let read = self.read_reg_byte(REG_TX_CONTROL)?;

        if !(read & 0x03) != 0 {
            return self.set_register_flags(REG_TX_CONTROL, 0x83);
        }

        Ok(())
    }

    /// Turns the antenna off
    fn anntenna_off(&mut self) -> Result<(), i2c::Error> {
        let read = self.read_reg_byte(REG_TX_CONTROL)?;

        if !(read & 0x03) == 0 {
            return self.clear_register_flags(REG_TX_CONTROL, b'\x03');
        }

        Ok(())
    }

    ///
    /// Public methods
    ///

    /// Stand-alone function that puts the tag into the correct state
    /// Returns detailed information about the tag
    pub fn read_tag_id(&mut self, uart: &mut Uart) -> Result<TagId, i2c::Error> {
        let (mut present, _) = self._detect_tag(uart)?;
        if !present {
            // Try again, the card may not be in the correct state
            (present, _) = self._detect_tag(uart)?;
        }

        if !present {
            return Ok(TagId {
                success: false,
                id_integers: [0; 1024],
                id_formatted: [0; 1024],
                tag_type: TagType::Classic,
            });
        }

        self.read_tag_id_private(uart)
    }

    /// Wrapper for readTagID
    pub fn is_tag_present(&mut self, uart: &mut Uart) -> Result<bool, i2c::Error> {
        Ok(self.read_tag_id(uart)?.success)
    }
}
