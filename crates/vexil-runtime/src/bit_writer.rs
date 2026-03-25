pub struct BitWriter {
    buf: Vec<u8>,
    current_byte: u8,
    bit_offset: u8,
}

impl BitWriter {
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            current_byte: 0,
            bit_offset: 0,
        }
    }

    /// Internal: align to a byte boundary without the "empty = zero byte" rule.
    /// Used before multi-byte writes to ensure alignment.
    fn align(&mut self) {
        if self.bit_offset > 0 {
            self.buf.push(self.current_byte);
            self.current_byte = 0;
            self.bit_offset = 0;
        }
    }

    /// Write `count` bits from `value`, LSB first.
    pub fn write_bits(&mut self, value: u64, count: u8) {
        let mut v = value;
        for _ in 0..count {
            let bit = (v & 1) as u8;
            self.current_byte |= bit << self.bit_offset;
            self.bit_offset += 1;
            if self.bit_offset == 8 {
                self.buf.push(self.current_byte);
                self.current_byte = 0;
                self.bit_offset = 0;
            }
            v >>= 1;
        }
    }

    /// Write a single boolean as 1 bit.
    pub fn write_bool(&mut self, v: bool) {
        self.write_bits(v as u64, 1);
    }

    /// Flush any partial byte to the buffer.
    ///
    /// Special case per spec §4.1: if nothing has been written at all
    /// (bit_offset == 0 AND buf is empty), push a zero byte anyway.
    /// If bit_offset == 0 and buf is non-empty, this is a no-op.
    pub fn flush_to_byte_boundary(&mut self) {
        if self.bit_offset == 0 {
            if self.buf.is_empty() {
                self.buf.push(0x00);
            }
            // else: already aligned and something was written — no-op
        } else {
            self.buf.push(self.current_byte);
            self.current_byte = 0;
            self.bit_offset = 0;
        }
    }

    pub fn write_u8(&mut self, v: u8) {
        self.align();
        self.buf.push(v);
    }

    pub fn write_u16(&mut self, v: u16) {
        self.align();
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_u32(&mut self, v: u32) {
        self.align();
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_u64(&mut self, v: u64) {
        self.align();
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_i8(&mut self, v: i8) {
        self.align();
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_i16(&mut self, v: i16) {
        self.align();
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_i32(&mut self, v: i32) {
        self.align();
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_i64(&mut self, v: i64) {
        self.align();
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// Write an f32, canonicalizing NaN to 0x7FC00000.
    pub fn write_f32(&mut self, v: f32) {
        self.align();
        let bits: u32 = if v.is_nan() {
            0x7FC00000u32
        } else {
            v.to_bits()
        };
        self.buf.extend_from_slice(&bits.to_le_bytes());
    }

    /// Write an f64, canonicalizing NaN to 0x7FF8000000000000.
    pub fn write_f64(&mut self, v: f64) {
        self.align();
        let bits: u64 = if v.is_nan() {
            0x7FF8000000000000u64
        } else {
            v.to_bits()
        };
        self.buf.extend_from_slice(&bits.to_le_bytes());
    }

    /// Write a LEB128-encoded unsigned integer.
    pub fn write_leb128(&mut self, v: u64) {
        self.align();
        crate::leb128::encode(&mut self.buf, v);
    }

    /// Write a ZigZag + LEB128 encoded signed integer.
    pub fn write_zigzag(&mut self, v: i64, type_bits: u8) {
        let encoded = crate::zigzag::zigzag_encode(v, type_bits);
        self.write_leb128(encoded);
    }

    /// Write a UTF-8 string with a LEB128 length prefix.
    pub fn write_string(&mut self, s: &str) {
        self.align();
        crate::leb128::encode(&mut self.buf, s.len() as u64);
        self.buf.extend_from_slice(s.as_bytes());
    }

    /// Write a byte slice with a LEB128 length prefix.
    pub fn write_bytes(&mut self, data: &[u8]) {
        self.align();
        crate::leb128::encode(&mut self.buf, data.len() as u64);
        self.buf.extend_from_slice(data);
    }

    /// Write raw bytes with no length prefix.
    pub fn write_raw_bytes(&mut self, data: &[u8]) {
        self.align();
        self.buf.extend_from_slice(data);
    }

    /// Flush any partial byte and return the finished buffer.
    pub fn finish(mut self) -> Vec<u8> {
        self.flush_to_byte_boundary();
        self.buf
    }
}

impl Default for BitWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_single_bit_true() {
        let mut w = BitWriter::new();
        w.write_bool(true);
        assert_eq!(w.finish(), [0x01]);
    }

    #[test]
    fn write_single_bit_false() {
        let mut w = BitWriter::new();
        w.write_bool(false);
        assert_eq!(w.finish(), [0x00]);
    }

    #[test]
    fn write_bits_lsb_first() {
        let mut w = BitWriter::new();
        w.write_bits(5, 3); // 101
        w.write_bits(19, 5); // 10011
                             // LSB-first: byte = 10011_101 = 0x9D
        assert_eq!(w.finish(), [0x9D]);
    }

    #[test]
    fn write_bits_cross_byte_boundary() {
        let mut w = BitWriter::new();
        w.write_bits(5, 3);
        w.write_bits(19, 5);
        w.write_bits(42, 6); // 101010
                             // Byte 0: 0x9D, Byte 1: 00_101010 = 0x2A
        assert_eq!(w.finish(), [0x9D, 0x2A]);
    }

    #[test]
    fn flush_to_byte_boundary_pads_zeros() {
        let mut w = BitWriter::new();
        w.write_bits(0b101, 3);
        w.flush_to_byte_boundary();
        w.write_bits(0xFF, 8);
        assert_eq!(w.finish(), [0x05, 0xFF]);
    }

    #[test]
    fn write_u8_flushes_first() {
        let mut w = BitWriter::new();
        w.write_bool(true);
        w.write_u8(0xAB);
        assert_eq!(w.finish(), [0x01, 0xAB]);
    }

    #[test]
    fn write_u16_le() {
        let mut w = BitWriter::new();
        w.write_u16(0x0102);
        assert_eq!(w.finish(), [0x02, 0x01]);
    }

    #[test]
    fn write_u32_le() {
        let mut w = BitWriter::new();
        w.write_u32(0x01020304);
        assert_eq!(w.finish(), [0x04, 0x03, 0x02, 0x01]);
    }

    #[test]
    fn write_i16_negative() {
        let mut w = BitWriter::new();
        w.write_i16(-1);
        assert_eq!(w.finish(), [0xFF, 0xFF]);
    }

    #[test]
    fn write_f32_nan_canonicalized() {
        let mut w = BitWriter::new();
        w.write_f32(f32::NAN);
        assert_eq!(w.finish(), [0x00, 0x00, 0xC0, 0x7F]);
    }

    #[test]
    fn write_f64_nan_canonicalized() {
        let mut w = BitWriter::new();
        w.write_f64(f64::NAN);
        assert_eq!(w.finish(), 0x7FF8000000000000u64.to_le_bytes());
    }

    #[test]
    fn write_f32_negative_zero_preserved() {
        let mut w = BitWriter::new();
        w.write_f32(-0.0f32);
        let buf = w.finish();
        assert_eq!(buf, (-0.0f32).to_le_bytes());
        assert_ne!(buf, 0.0f32.to_le_bytes());
    }

    #[test]
    fn write_leb128_test() {
        let mut w = BitWriter::new();
        w.write_leb128(300);
        assert_eq!(w.finish(), [0xAC, 0x02]);
    }

    #[test]
    fn write_zigzag_neg1() {
        let mut w = BitWriter::new();
        w.write_zigzag(-1, 64);
        assert_eq!(w.finish(), [0x01]);
    }

    #[test]
    fn write_string_test() {
        let mut w = BitWriter::new();
        w.write_string("hi");
        assert_eq!(w.finish(), [0x02, 0x68, 0x69]);
    }

    #[test]
    fn write_bytes_test() {
        let mut w = BitWriter::new();
        w.write_bytes(&[0xDE, 0xAD]);
        assert_eq!(w.finish(), [0x02, 0xDE, 0xAD]);
    }

    #[test]
    fn write_raw_bytes_test() {
        let mut w = BitWriter::new();
        w.write_raw_bytes(&[0xCA, 0xFE]);
        assert_eq!(w.finish(), [0xCA, 0xFE]);
    }

    #[test]
    fn empty_flush_produces_zero_byte() {
        let mut w = BitWriter::new();
        w.flush_to_byte_boundary();
        assert_eq!(w.finish(), [0x00]);
    }
}
