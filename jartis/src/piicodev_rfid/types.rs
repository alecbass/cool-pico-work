#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RfidStatus {
    Ok,
    NoTag,
    Error,
}

#[derive(Debug)]
pub enum TagType {
    Classic,
    NTag,
    Unknown,
}

#[derive(Debug)]
pub struct TagId {
    pub success: bool,
    pub id_integers: [u8; 16],
    pub id_formatted: [u8; 64],
    pub tag_type: TagType,
}

impl Default for TagId {
    fn default() -> Self {
        Self {
            success: false,
            id_integers: [0; 16],
            id_formatted: [0; 64],
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
