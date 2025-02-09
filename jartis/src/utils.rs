const BUFFER_LIMIT: usize = 16;

pub fn create_buffer() -> [u8; BUFFER_LIMIT] {
    [0; BUFFER_LIMIT]
}
