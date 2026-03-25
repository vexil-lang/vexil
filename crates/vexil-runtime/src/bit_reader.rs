#[allow(dead_code)]
pub struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_offset: u8,
    recursion_depth: u32,
}
