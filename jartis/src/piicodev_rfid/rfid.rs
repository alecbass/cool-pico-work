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
        // Wait for completion
        let mut i = 20000; // Timeout counter
        let mut n = 0;

        while i > 0 {
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

                if n & irq_en & 0x01 != 0 {
                    stat = RfidStatus::NoTag;
                } else if cmd == CMD_TRANCEIVE {
                    let n = self.read_reg_byte(REG_FIFO_LEVEL)?;
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

                    for i in 0..read_count {
                        let val = self.read_reg_byte(REG_FIFO_DATA)?;
                        recv[i as usize] = val;
                    }
                }
            } else {
                stat = RfidStatus::Error;
            }
        }

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

            // Check if the tag is a classic RFID tag
            let possible_fifth_element = recv.get(5);
            let is_classic_tag = length == 4
                && possible_fifth_element.is_some()
                && *possible_fifth_element.unwrap() == 0;

            // Check if hte tag is an NTag
            let is_ntag = length == 5;

            if !is_classic_tag && !is_ntag {
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

        // Only send as many bytes as we should for the calcuation (should be 7)
        let buffer_length_for_crc = get_array_length(&buf);
        let p_out = self.calculate_crc(&buf[0..buffer_length_for_crc])?;
        buf[ser_num_length + 2] = p_out[0];
        buf[ser_num_length + 3] = p_out[1];

        // Only send the real data
        let data = &buf[0..ser_num_length + 4];
        let (status, _back_data, back_len) = self.to_card(CMD_TRANCEIVE, data)?;

        if status == RfidStatus::Ok && back_len == 0x18 {
            return Ok(true);
        }

        Ok(false)
    }

    // Read tag ID
    fn read_tag_id_private(&mut self) -> Result<TagId, I2C::Error> {
        let mut result = TagId::default();
        let mut valid_uid: [u8; 16] = [0; 16];

        let (status, mut uid) = self.anticoll(TAG_CMD_ANTCOL1)?;

        if status != RfidStatus::Ok {
            return Ok(result);
        }

        if !self.select_tag(&uid, TAG_CMD_ANTCOL1)? {
            writeln!(self.uart, "Could not select tag").unwrap();
            return Ok(result);
        }

        let uid_length = get_array_length(&uid);
        let mut valid_uid_index = 0;

        if uid_length > 0 && uid[0] == 0x88 {
            // NTAG
            for i in 1..4 {
                if i < uid_length {
                    valid_uid[valid_uid_index] = uid[i];
                    valid_uid_index += 1;
                }
            }

            let (status, mut inner_uid) = self.anticoll(TAG_CMD_ANTCOL2)?;
            if status != RfidStatus::Ok {
                return Ok(result);
            }

            // Once again, calculate the length of actually read values rather than the lenght of
            // the underlying buffer
            let uid_length = get_array_length(&inner_uid);

            let rtn = self.select_tag(&inner_uid[0..uid_length], TAG_CMD_ANTCOL2)?;
            if !rtn {
                return Ok(result);
            }

            if uid_length > 0 && inner_uid[0] == 0x88 {
                for i in 1..4 {
                    if i < uid_length {
                        valid_uid[valid_uid_index] = inner_uid[i];
                        valid_uid_index += 1;
                    }
                }

                let (status, innerer_uid) = self.anticoll(TAG_CMD_ANTCOL3)?;
                if status != RfidStatus::Ok {
                    return Ok(result);
                }

                // Get around Python re-assigning to the same variable in a destructure
                inner_uid = innerer_uid;
            }

            // Get around Python re-assigning to the same variable in a destructure
            uid = inner_uid;
        }

        for i in 0..5 {
            if i < uid_length {
                valid_uid[valid_uid_index] = uid[i];
                valid_uid_index += 1;
            }
        }

        let valid_uid_length = valid_uid_index;

        let tag_type = if valid_uid_length <= 5 {
            TagType::Classic
        } else {
            TagType::NTag
        };

        // Create result
        result.success = true;
        result.id_integers = valid_uid;
        result.id_length = valid_uid_length;
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

        if !detection.present {
            return Ok(TagId::default());
        }

        self.read_tag_id_private()
    }

    /// Wrapper for readTagID
    pub fn is_tag_present(&mut self) -> Result<bool, I2C::Error> {
        Ok(self.read_tag_id()?.success)
    }

    /// Required for Classic Tag only - Select a specific tag for reading & writing
    fn classic_select_tag(&mut self, ser: &[u8]) -> Result<RfidStatus, I2C::Error> {
        let buf: [u8; 7] = [0x93, 0x70, ser[0], ser[1], ser[2], ser[3], ser[4]];
        let p_out = self.calculate_crc(&buf)?;

        let mut crc_buf: [u8; 9] = [0; 9];
        for (i, byte) in buf.into_iter().enumerate() {
            crc_buf[i] = byte;
        }
        crc_buf[7] = p_out[0];
        crc_buf[8] = p_out[1];

        let (stat, _recv, bits) = self.to_card(CMD_TRANCEIVE, &crc_buf)?;

        if stat == RfidStatus::Ok && bits == 0x18 {
            return Ok(stat);
        }

        Ok(RfidStatus::Error)
    }

    // # Required for Classic Tag only - Authenticate the address in memory
    // def _classicAuth(self, mode, addr, sect, ser):
    //     return self._tocard(_CMD_MF_AUTHENT, [mode, addr] + sect + ser[:4])[0]
    //
    // # Required for Classic Tag only - Turn off crypto
    // def _classicStopCrypto(self):
    //     self._cflags(_REG_STATUS_2, 0x08)
    ///
    /// PiicoDev expansion
    ///
    ///

    /// Read a register from NTAG or Classic
    fn read(&mut self, addr: u8) -> Result<Option<[u8; READ_BUFFER_LENGTH]>, I2C::Error> {
        let mut data: [u8; 4] = [0x30, addr, 0, 0];
        let p_out = self.calculate_crc(&data[0..1])?;
        data[2] = p_out[0];
        data[3] = p_out[1];
        let (stat, recv, _) = self.to_card(CMD_TRANCEIVE, &data)?;

        if stat != RfidStatus::Ok {
            return Ok(None);
        }

        Ok(Some(recv))
    }

    /// Write to an NTAG page
    fn write_page_ntag(
        &mut self,
        page: u8,
        data: &[u8],
        data_length: usize,
    ) -> Result<RfidStatus, I2C::Error> {
        let mut buf: [u8; 16] = [0; 16];
        buf[0] = 0xA2;
        buf[1] = page;

        for (i, &byte) in data.iter().enumerate().take(data_length.saturating_sub(1)) {
            buf[i + 2] = byte;
        }

        // Append the CRC calculation after the data
        let p_out = self.calculate_crc(&buf[0..data_length + 2])?;
        buf[data_length + 2] = p_out[0];
        buf[data_length + 3] = p_out[1];

        // The length of provided data plus the two prepended and two appended elements
        let total_buf_length = data_length + 4;
        writeln!(
            self.uart,
            "Write page buf: {buf:?} total_buf_length: {total_buf_length:?}"
        )
        .unwrap();

        let (stat, _recv, _bits) = self.to_card(CMD_TRANCEIVE, &buf[0..total_buf_length])?;
        Ok(stat)
    }

    /// Writes a number to NTAG
    /// Slot must be >> SLOT_NO_MIN && <= SNOT_NO_MAX (0 and 35)
    fn write_number_to_ntag(&mut self, bytes_number: &[u8], slot: u8) -> Result<bool, I2C::Error> {
        // assert slot >= _SLOT_NO_MIN and slot <=_SLOT_NO_MAX, 'Slot must be between 0 and 35'
        let page_adr_min = 4;
        let stat = self.write_page_ntag(page_adr_min + slot, bytes_number, size_of::<i64>())?;

        let tag_write_success = stat == RfidStatus::Ok;
        Ok(tag_write_success)
    }

    /// Writes a number to the tag
    pub fn write_number(&mut self, number: i64, slot: u8) -> Result<bool, I2C::Error> {
        let mut success = false;
        let bytearray_number = i64::to_le_bytes(number);

        let mut read_tag_id_result = TagId::default();
        while !read_tag_id_result.success {
            read_tag_id_result = self.read_tag_id()?;
        }

        if read_tag_id_result.success && read_tag_id_result.tag_type == TagType::NTag {
            while !success {
                success = self.write_number_to_ntag(&bytearray_number, slot)?;
                writeln!(self.uart, "Success: {success:?}").unwrap();
            }
        }

        // TODO: Classic tags

        writeln!(self.uart, "Write number: {bytearray_number:?}").unwrap();

        Ok(success)
    }

    /// Reads a number from the tag
    pub fn read_number(&mut self, slot: u8) -> Result<i64, I2C::Error> {
        let mut bytearray_number: Option<[u8; READ_BUFFER_LENGTH]> = None;
        let mut read_tag_id_result = self.read_tag_id()?;
        while !read_tag_id_result.success {
            read_tag_id_result = self.read_tag_id()?;
        }

        if read_tag_id_result.tag_type == TagType::NTag {
            let page_address = 4;
            bytearray_number = self.read(page_address + slot)?;
        }

        // TODO: Handle classic tags

        let Some(bytearray_number) = bytearray_number else {
            return Ok(0);
        };

        let bytes: [u8; 8] = [
            bytearray_number[0],
            bytearray_number[1],
            bytearray_number[2],
            bytearray_number[3],
            bytearray_number[4],
            bytearray_number[5],
            bytearray_number[6],
            bytearray_number[7],
        ];

        let number = i64::from_le_bytes(bytes);
        writeln!(self.uart, "Read bytearray_number: {bytearray_number:?}").unwrap();
        writeln!(self.uart, "Read number: {number}").unwrap();
        Ok(number)
    }
}
