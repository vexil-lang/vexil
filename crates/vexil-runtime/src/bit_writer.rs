#[allow(dead_code)]
pub struct BitWriter {
    buf: Vec<u8>,
    current_byte: u8,
    bit_offset: u8,
}
