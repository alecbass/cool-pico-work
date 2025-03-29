use core::fmt::Write;

use heapless::String;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RfidStatus {
    Ok,
    NoTag,
    Error,
}

#[derive(Debug, PartialEq)]
pub enum TagType {
    Classic,
    NTag,
    Unknown,
}

#[derive(Debug)]
pub struct TagId {
    pub success: bool,
    pub id_integers: [u8; 16],
    /// The length of real data in id_integers
    pub id_length: usize,
    pub tag_type: TagType,
}

impl TagId {
    /// Turns the internal integers into a string for display purposes
    /// * Can fail of appending to the internal id_formatted string fails, or if the write! command
    /// to the hexadecimal buffer fails
    pub fn get_formatted_id(&self) -> Result<String<64>, ()> {
        let mut id_formatted: String<64> = String::new();
        let mut hex_buffer: String<2> = String::new();

        for (i, &byte) in self
            .id_integers
            .iter()
            .take(self.id_length.saturating_sub(1))
            .enumerate()
        {
            if i > 0 {
                id_formatted.push(':')?;
            }

            // NOTE: Removed a check for byte < 16 here

            // Format the byte into hexadecimal
            hex_buffer.clear();
            write!(hex_buffer, "{byte:02x}").map_err(|_e| ())?;

            // Add these characters into the string
            id_formatted.push_str(&hex_buffer)?;
        }

        Ok(id_formatted)
    }
}

impl Default for TagId {
    fn default() -> Self {
        Self {
            success: false,
            id_integers: [0; 16],
            id_length: 0,
            tag_type: TagType::Unknown,
        }
    }
}

// Tag detection result
#[derive(Debug)]
pub struct TagDetectResult {
    pub present: bool,
    pub atqa: u16,
}
