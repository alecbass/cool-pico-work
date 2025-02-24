pub enum TagType {
    Classic,
    NTag,
}

pub struct TagId {
    pub success: bool,
    pub id_integers: [u8; 1024],
    pub id_formatted: [u8; 1024],
    pub tag_type: TagType,
}
