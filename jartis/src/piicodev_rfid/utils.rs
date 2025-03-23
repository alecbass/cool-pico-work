/// Finds the "length" of a given buffer, assuming that the index of the first zero value is where
/// the length read ends
/// params:
/// * buffer The buffer to read the length of
pub fn get_array_length(buffer: &[u8]) -> usize {
    let length = buffer.len();

    for (index, value) in buffer.iter().enumerate() {
        if *value == 0 {
            return index;
        }
    }

    length
}
