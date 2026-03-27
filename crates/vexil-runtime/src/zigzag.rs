/// Encode a signed integer using ZigZag encoding.
///
/// Maps signed values to unsigned: 0 -> 0, -1 -> 1, 1 -> 2, -2 -> 3, etc.
/// `type_bits` is the bit width of the source type (e.g. 32 for `i32`, 64 for `i64`).
pub fn zigzag_encode(n: i64, type_bits: u8) -> u64 {
    ((n << 1) ^ (n >> (u32::from(type_bits) - 1))) as u64
}

/// Decode a ZigZag-encoded unsigned integer back to its signed value.
pub fn zigzag_decode(n: u64) -> i64 {
    ((n >> 1) as i64) ^ -((n & 1) as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_mapping() {
        assert_eq!(zigzag_encode(0, 64), 0);
        assert_eq!(zigzag_encode(-1, 64), 1);
        assert_eq!(zigzag_encode(1, 64), 2);
        assert_eq!(zigzag_encode(-2, 64), 3);
        assert_eq!(zigzag_encode(2, 64), 4);
        assert_eq!(zigzag_encode(i64::MIN, 64), u64::MAX);
        assert_eq!(zigzag_encode(i64::MAX, 64), u64::MAX - 1);
    }

    #[test]
    fn round_trip_i32_range() {
        for &v in &[0i64, 1, -1, 127, -128, i32::MIN as i64, i32::MAX as i64] {
            let encoded = zigzag_encode(v, 32);
            let decoded = zigzag_decode(encoded);
            assert_eq!(decoded, v, "round-trip failed for {v}");
        }
    }

    #[test]
    fn encode_32bit_width() {
        assert_eq!(zigzag_encode(-1, 32), 1);
        assert_eq!(zigzag_encode(1, 32), 2);
    }
}
