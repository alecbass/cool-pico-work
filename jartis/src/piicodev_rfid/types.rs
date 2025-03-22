#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RfidStatus {
    Ok,
    NoTag,
    Error,
}

pub enum TagType {
    Classic,
    NTag,
    Unknown,
}

pub struct TagId {
    pub success: bool,
    pub id_integers: [u8; 1024],
    pub id_formatted: [u8; 1024],
    pub tag_type: TagType,
}
