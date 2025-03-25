use core::fmt::Write;

use cortex_m::delay::Delay;
use embedded_hal::i2c::I2c;

use super::{
    constants::{
        CMD_CALC_CRC, CMD_IDLE, CMD_MF_AUTHENT, CMD_SOFT_RESET, CMD_TRANCEIVE, I2C_ADDRESS,
        REG_BIT_FRAMING, REG_COMMAND, REG_COM_IRQ, REG_COM_I_EN, REG_CONTROL, REG_CRC_RESULT_LSB,
        REG_CRC_RESULT_MSB, REG_DIV_IRQ, REG_DIV_I_EN, REG_ERROR, REG_FIFO_DATA, REG_FIFO_LEVEL,
        REG_MODE, REG_TX_ASK, REG_TX_CONTROL, REG_T_MODE, REG_T_PRESCALER, REG_T_RELOAD_HI,
        REG_T_RELOAD_LO, TAG_CMD_ANTCOL1, TAG_CMD_REQIDL,
    },
    utils::get_array_length,
};
use crate::{
    piicodev_rfid::{
        constants::{TAG_CMD_ANTCOL2, TAG_CMD_ANTCOL3},
        types::{RfidStatus, TagDetectResult, TagId, TagType},
    },
    Uart,
};

/// The length of internal read buffer arrays
const READ_BUFFER_LENGTH: usize = 32;

pub struct PiicoDevRfid<I2C> {
    i2c: I2C,
    uart: Uart,
    delay: Delay,
}

impl<I2C> PiicoDevRfid<I2C>
where
    I2C: I2c,
{
    pub fn new(i2c: I2C, uart: Uart, delay: Delay) -> Self {
        Self { i2c, uart, delay }
    }

    pub fn init(&mut self) -> Result<(), I2C::Error> {
        self.reset()?;
        self.delay.delay_ms(50);

        self.write_reg_byte(REG_T_MODE, 0x80)?;
        self.write_reg_byte(REG_T_PRESCALER, 0xA9)?;
        self.write_reg_byte(REG_T_RELOAD_HI, 0x03)?;
        self.write_reg_byte(REG_T_RELOAD_LO, 0xE8)?;
        self.write_reg_byte(REG_TX_ASK, 0x40)?;
        self.write_reg_byte(REG_MODE, 0x3D)?;
        self.write_reg_byte(REG_DIV_I_EN, 0x80)?; // CMOS Logic for IRQ pin
        self.write_reg_byte(REG_COM_I_EN, 0x20)?; // Allows the receiver interrupt request (RxIRq bit) to be propagated to pin IRQ
        self.antenna_on()
    }

    /// Writes a byte to the register, based on the Arduino library at https://github.com/MakerSpaceLeiden/rfid/blob/master/src/MFRC522_i2c.cpp
    fn write_reg_byte(&mut self, register: u8, byte: u8) -> Result<(), I2C::Error> {
        let address = I2C_ADDRESS;
        self.i2c.write(address, &[register, byte])
    }

    /// Reads a byte from the register, based on the Adruino library at https://github.com/MakerSpaceLeiden/rfid/blob/master/src/MFRC522_i2c.cpp
    fn read_reg_byte(&mut self, register: u8) -> Result<u8, I2C::Error> {
        let address = I2C_ADDRESS;
        let mut read_buffer: [u8; 1] = [0; 1];

        self.i2c
            .write_read(address, &[register], &mut read_buffer)?;

        Ok(read_buffer[0])
    }

    /// I2C write to FIFO buffer
    fn write_to_fifo(&mut self, register: u8, value: &[u8]) -> Result<(), I2C::Error> {
        let address = I2C_ADDRESS;
        let mut buffer = [0; READ_BUFFER_LENGTH];
        buffer[0] = register;

        for (i, &byte) in value.iter().enumerate() {
            let index = i + 1;

            if index < buffer.len() {
                buffer[index] = byte;
            }
        }

        // Write the register, plus the length of the provided array
        let value_to_write = &buffer[0..value.len() + 1];
        self.i2c.write(address, value_to_write)
    }

    fn set_register_flags(&mut self, register: u8, mask: u8) -> Result<(), I2C::Error> {
        let current_value = self.read_reg_byte(register)?;
        self.write_reg_byte(register, current_value | mask)
    }

    fn clear_register_flags(&mut self, register: u8, mask: u8) -> Result<(), I2C::Error> {
        let current_value = self.read_reg_byte(register)?;
        self.write_reg_byte(register, current_value & (!mask))
    }

    /// Resets the RFID module
    pub fn reset(&mut self) -> Result<(), I2C::Error> {
        self.write_reg_byte(REG_COMMAND, CMD_SOFT_RESET)
    }

    // Communication with the tag
    fn to_card(
        &mut self,
        cmd: u8,
        send: &[u8],
    ) -> Result<(RfidStatus, [u8; READ_BUFFER_LENGTH], u16), I2C::Error> {
        let mut recv: [u8; READ_BUFFER_LENGTH] = [0; READ_BUFFER_LENGTH];
        let mut bits: u16 = 0;
        let mut stat = RfidStatus::Error;

        let (irq_en, wait_irq) = match cmd {
            CMD_MF_AUTHENT => (0x12, 0x10),
            CMD_TRANCEIVE => (0x77, 0x30),
            _ => (0x00, 0x00),
        };

        // Stop any active command
        self.write_reg_byte(REG_COMMAND, CMD_IDLE)?;
        // Clear all interrupt request bits
        self.write_reg_byte(REG_COM_IRQ, 0x7F)?;
        // FlushBuffer = 1, FIFO initialization
        self.set_register_flags(REG_FIFO_LEVEL, 0x80)?;
        // Write to the FIFO
        self.delay.delay_ms(10);
        self.write_to_fifo(REG_FIFO_DATA, send)?;

        if cmd == CMD_TRANCEIVE {
            // This starts the transceive operation
            self.clear_register_flags(REG_BIT_FRAMING, 0x80)?;
        }

        self.write_reg_byte(REG_COMMAND, cmd)?;

        if cmd == CMD_TRANCEIVE {
            // This starts the transceive operation
            self.set_register_flags(REG_BIT_FRAMING, 0x80)?;
        }

        self.delay.delay_ms(10);
        writeln!(self.uart, "Doing calculation").unwrap();
        // Wait for completion
        let mut i = 20000; // Timeout counter
        let mut n = 0;

        writeln!(self.uart, "Reading").unwrap();
        while i > 0 {
            // self.delay.delay_ms(1);
            n = self.read_reg_byte(REG_COM_IRQ)?;
            i -= 1;
            if n & wait_irq != 0 {
                break;
            }
            if n & 0x01 != 0 {
                break;
            }
        }

        // Stop the transceive operation
        // writeln!(self.uart, "Clearing register flags").unwrap();
        self.clear_register_flags(REG_BIT_FRAMING, 0x80)?;

        if i > 0 {
            let error_status = self.read_reg_byte(REG_ERROR)?;
            if (error_status & 0x1B) == 0x00 {
                stat = RfidStatus::Ok;

                if send.len() == 9 {
                    writeln!(
                        self.uart,
                        "EEEEE n: {n} irq_en: {irq_en}     expected: {}",
                        n & irq_en & 0x01
                    )
                    .unwrap();
                }

                if n & irq_en & 0x01 != 0 {
                    stat = RfidStatus::NoTag;
                } else if cmd == CMD_TRANCEIVE {
                    let n = self.read_reg_byte(REG_FIFO_LEVEL)?;
                    writeln!(self.uart, "to_card n: {n}").unwrap();
                    let lbits = self.read_reg_byte(REG_CONTROL)? & 0x07;

                    if lbits != 0 {
                        bits = ((n - 1) as u16) * 8 + lbits as u16;
                    } else {
                        bits = (n as u16) * 8;
                    }

                    let read_count = if n == 0 {
                        1
                    } else if n > 16 {
                        16
                    } else {
                        n
                    };

                    writeln!(self.uart, "Read count: {read_count}   n: {n}").unwrap();

                    for i in 0..read_count {
                        let val = self.read_reg_byte(REG_FIFO_DATA)?;
                        recv[i as usize] = val;
                    }
                }
            } else {
                stat = RfidStatus::Error;
            }
        }
        writeln!(self.uart, "Reading with i: {i}   stat: {stat:?}").unwrap();

        Ok((stat, recv, bits))
    }

    // Calculate CRC using the coprocessor
    fn calculate_crc(&mut self, data: &[u8]) -> Result<[u8; 2], I2C::Error> {
        self.write_reg_byte(REG_COMMAND, CMD_IDLE)?;
        self.clear_register_flags(REG_DIV_IRQ, 0x04)?;
        self.set_register_flags(REG_FIFO_LEVEL, 0x80)?;

        for &byte in data {
            self.write_reg_byte(REG_FIFO_DATA, byte)?;
        }

        self.write_reg_byte(REG_COMMAND, CMD_CALC_CRC)?;

        // Wait for CRC calculation to complete
        let mut i: u8 = 0xFF;
        loop {
            let n = self.read_reg_byte(REG_DIV_IRQ)?;
            i -= 1;
            if !((i != 0) && !(n & 0x04 != 0)) {
                break;
            }
        }

        self.write_reg_byte(REG_COMMAND, CMD_IDLE)?;

        let result_lsb = self.read_reg_byte(REG_CRC_RESULT_LSB)?;
        let result_msb = self.read_reg_byte(REG_CRC_RESULT_MSB)?;

        Ok([result_lsb, result_msb])
    }

    // Request tag to go to READY state
    fn request(&mut self, mode: u8) -> Result<(RfidStatus, u16), I2C::Error> {
        self.write_reg_byte(REG_BIT_FRAMING, 0x07)?;
        let (stat, _recv, bits) = self.to_card(CMD_TRANCEIVE, &[mode])?;

        if stat != RfidStatus::Ok || bits != 0x10 {
            return Ok((RfidStatus::Error, bits));
        }

        Ok((stat, bits))
    }

    // Perform anticollision check
    fn anticoll(
        &mut self,
        anticol_n: u8,
    ) -> Result<(RfidStatus, [u8; READ_BUFFER_LENGTH]), I2C::Error> {
        let ser: [u8; 2] = [anticol_n, 0x20];
        self.write_reg_byte(REG_BIT_FRAMING, 0x00)?;

        let (stat, recv, _bits) = self.to_card(CMD_TRANCEIVE, &ser)?;

        if stat == RfidStatus::Ok {
            let length = get_array_length(&recv);

            if length != 5 {
                return Ok((RfidStatus::Error, recv));
            }

            let mut ser_chk = 0;
            for &byte in recv.iter().take(4) {
                ser_chk ^= byte;
            }

            if ser_chk != recv[4] {
                writeln!(self.uart, "Unexpected ser_chk: {ser_chk:?} {:?}", recv[4]).unwrap();
                return Ok((RfidStatus::Error, recv));
            }
        }

        Ok((stat, recv))
    }

    /// Select the desired tag
    fn select_tag(&mut self, ser_num: &[u8], anti_col_n: u8) -> Result<bool, I2C::Error> {
        let mut buf = [0; READ_BUFFER_LENGTH];
        buf[0] = anti_col_n;
        buf[1] = 0x70;

        // ser_num has a length of 32 (the array's length), so only assume that the first zero is
        // when the array stopped being read
        let ser_num_length = get_array_length(ser_num);

        for i in 0..ser_num_length {
            buf[i + 2] = ser_num[i];
        }

        writeln!(self.uart, "ser_num: {ser_num:?}").unwrap();
        writeln!(self.uart, "Data pre-CRC: {buf:?}").unwrap();

        let p_out = self.calculate_crc(&buf)?;
        buf[ser_num_length + 2] = p_out[0];
        buf[ser_num_length + 3] = p_out[1];

        writeln!(self.uart, "Data post-CRC: {buf:?}").unwrap();

        // Only send the real data
        let data = &buf[0..ser_num_length + 4];
        writeln!(self.uart, "CRC data: {data:?}   length: {}", data.len()).unwrap();

        let (status, _back_data, back_len) = self.to_card(CMD_TRANCEIVE, data)?;
        writeln!(
            self.uart,
            "select tag to_card status: {status:?} {back_len}"
        )
        .unwrap();

        if status == RfidStatus::Ok && back_len == 0x18 {
            return Ok(true);
        }

        Ok(false)
    }

    // Read tag ID
    fn read_tag_id_private(&mut self) -> Result<TagId, I2C::Error> {
        let mut result = TagId::default();
        let mut valid_uid: [u8; 16] = [0; 16];

        let (status, uid) = self.anticoll(TAG_CMD_ANTCOL1)?;

        if status != RfidStatus::Ok {
            return Ok(result);
        }

        if !self.select_tag(&uid, TAG_CMD_ANTCOL1)? {
            writeln!(self.uart, "Could not select tag").unwrap();
            return Ok(result);
        }

        writeln!(self.uart, "Got here: {status:?} {uid:?}").unwrap();

        let mut valid_uid_index = 1;

        if uid.len() > 0 && uid[0] == 0x88 {
            // NTAG
            for i in 1..4 {
                if i < uid.len() {
                    valid_uid[valid_uid_index] = uid[i];
                    valid_uid_index += 1;
                }
            }

            let (status, uid) = self.anticoll(TAG_CMD_ANTCOL2)?;
            if status != RfidStatus::Ok {
                return Ok(result);
            }

            let rtn = self.select_tag(&uid, TAG_CMD_ANTCOL2)?;
            if !rtn {
                return Ok(result);
            }

            if uid.len() > 0 && uid[0] == 0x88 {
                for i in 1..4 {
                    if i < uid.len() {
                        valid_uid[valid_uid_index] = uid[i];
                        valid_uid_index += 1;
                    }
                }

                let (status, uid) = self.anticoll(TAG_CMD_ANTCOL3)?;
                if status != RfidStatus::Ok {
                    return Ok(result);
                }
            }
        }

        for i in 0..5 {
            if i < uid.len() {
                valid_uid[valid_uid_index] = uid[i];
                valid_uid_index += 1;
            }
        }

        // Format ID
        let id = valid_uid.iter().take(valid_uid.len().saturating_sub(1));
        let mut id_formatted: [u8; 64] = [0; 64];
        let mut id_formatted_index = 0;

        for (i, &byte) in id.enumerate() {
            if i > 0 {
                id_formatted[id_formatted_index] = b':';
                id_formatted_index += 1;
            }

            if byte < 16 {
                id_formatted[id_formatted_index] = b'0';
                id_formatted_index += 1;
            }

            id_formatted[id_formatted_index] = byte;
            id_formatted_index += 1;
        }

        let tag_type = if valid_uid.len() <= 5 {
            TagType::Classic
        } else {
            TagType::NTag
        };

        // Create result
        result.success = true;
        result.id_integers = valid_uid;
        result.id_formatted = id_formatted;
        result.tag_type = tag_type;

        Ok(result)
    }

    /// Detect the presence of a tag
    fn detect_tag(&mut self) -> Result<TagDetectResult, I2C::Error> {
        let (stat, atqa) = self.request(TAG_CMD_REQIDL)?;
        let present = stat == RfidStatus::Ok;

        Ok(TagDetectResult { present, atqa })
    }

    // Turns the antenna on
    fn antenna_on(&mut self) -> Result<(), I2C::Error> {
        let read = self.read_reg_byte(REG_TX_CONTROL)?;

        if read & 0x03 == 0 {
            return self.set_register_flags(REG_TX_CONTROL, 0x83);
        }

        Ok(())
    }

    /// Turns the antenna off
    fn anntenna_off(&mut self) -> Result<(), I2C::Error> {
        let read = self.read_reg_byte(REG_TX_CONTROL)?;

        if read & 0x03 != 0 {
            return self.clear_register_flags(REG_TX_CONTROL, b'\x03');
        }

        Ok(())
    }

    ///
    /// Public methods
    ///

    /// Stand-alone function that puts the tag into the correct state
    /// Returns detailed information about the tag
    pub fn read_tag_id(&mut self) -> Result<TagId, I2C::Error> {
        let mut detection = self.detect_tag()?;
        if !detection.present {
            // Try again, the card may not be in the correct state
            detection = self.detect_tag()?;
        }

        writeln!(self.uart, "Tag detected: {}", detection.present);

        if !detection.present {
            return Ok(TagId::default());
        }

        let result = self.read_tag_id_private();
        self.reset()?;
        panic!("Exiting for tests' sake");

        // if let Ok(ref result) = result {
        //     writeln!(self.uart, "Tag details: {result:?}").unwrap();
        //     if let Ok(id) = core::str::from_utf8(&result.id_formatted) {
        //         writeln!(self.uart, "ID: {id}").unwrap();
        //     } else {
        //         writeln!(self.uart, "Invalid ID").unwrap();
        //     }
        // }

        result
    }

    /// Wrapper for readTagID
    pub fn is_tag_present(&mut self) -> Result<bool, I2C::Error> {
        Ok(self.read_tag_id()?.success)
    }
}
