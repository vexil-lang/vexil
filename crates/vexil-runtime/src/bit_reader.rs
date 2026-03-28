use crate::error::DecodeError;
use crate::{MAX_BYTES_LENGTH, MAX_RECURSION_DEPTH};

/// A cursor over a byte slice that reads fields LSB-first at the bit level.
///
/// Created with [`BitReader::new`], consumed with `read_*` methods. Tracks
/// a byte position and a sub-byte bit offset, plus a recursion depth counter
/// for safely decoding recursive types.
///
/// Sub-byte reads pull individual bits from the current byte. Multi-byte reads
/// (e.g. [`read_u16`](Self::read_u16)) first align to the next byte boundary,
/// then interpret the bytes as little-endian.
pub struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_offset: u8,
    recursion_depth: u32,
}

impl<'a> BitReader<'a> {
    /// Create a new `BitReader` over the given byte slice.
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_offset: 0,
            recursion_depth: 0,
        }
    }

    /// Read `count` bits LSB-first into a u64.
    pub fn read_bits(&mut self, count: u8) -> Result<u64, DecodeError> {
        let mut result: u64 = 0;
        for i in 0..count {
            if self.byte_pos >= self.data.len() {
                return Err(DecodeError::UnexpectedEof);
            }
            let bit = (self.data[self.byte_pos] >> self.bit_offset) & 1;
            result |= u64::from(bit) << i;
            self.bit_offset += 1;
            if self.bit_offset == 8 {
                self.byte_pos += 1;
                self.bit_offset = 0;
            }
        }
        Ok(result)
    }

    /// Read a single bit as bool.
    pub fn read_bool(&mut self) -> Result<bool, DecodeError> {
        Ok(self.read_bits(1)? != 0)
    }

    /// Advance to the next byte boundary, discarding any remaining bits in the current byte.
    /// Infallible.
    pub fn flush_to_byte_boundary(&mut self) {
        if self.bit_offset > 0 {
            self.byte_pos += 1;
            self.bit_offset = 0;
        }
    }

    /// Remaining bytes from byte_pos.
    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.byte_pos)
    }

    /// Read a `u8`, aligning to a byte boundary first.
    pub fn read_u8(&mut self) -> Result<u8, DecodeError> {
        self.flush_to_byte_boundary();
        if self.remaining() < 1 {
            return Err(DecodeError::UnexpectedEof);
        }
        let v = self.data[self.byte_pos];
        self.byte_pos += 1;
        Ok(v)
    }

    /// Read a little-endian `u16`, aligning to a byte boundary first.
    pub fn read_u16(&mut self) -> Result<u16, DecodeError> {
        self.flush_to_byte_boundary();
        if self.remaining() < 2 {
            return Err(DecodeError::UnexpectedEof);
        }
        let bytes: [u8; 2] = self.data[self.byte_pos..self.byte_pos + 2]
            .try_into()
            .unwrap();
        self.byte_pos += 2;
        Ok(u16::from_le_bytes(bytes))
    }

    /// Read a little-endian `u32`, aligning to a byte boundary first.
    pub fn read_u32(&mut self) -> Result<u32, DecodeError> {
        self.flush_to_byte_boundary();
        if self.remaining() < 4 {
            return Err(DecodeError::UnexpectedEof);
        }
        let bytes: [u8; 4] = self.data[self.byte_pos..self.byte_pos + 4]
            .try_into()
            .unwrap();
        self.byte_pos += 4;
        Ok(u32::from_le_bytes(bytes))
    }

    /// Read a little-endian `u64`, aligning to a byte boundary first.
    pub fn read_u64(&mut self) -> Result<u64, DecodeError> {
        self.flush_to_byte_boundary();
        if self.remaining() < 8 {
            return Err(DecodeError::UnexpectedEof);
        }
        let bytes: [u8; 8] = self.data[self.byte_pos..self.byte_pos + 8]
            .try_into()
            .unwrap();
        self.byte_pos += 8;
        Ok(u64::from_le_bytes(bytes))
    }

    /// Read an `i8`, aligning to a byte boundary first.
    pub fn read_i8(&mut self) -> Result<i8, DecodeError> {
        self.flush_to_byte_boundary();
        if self.remaining() < 1 {
            return Err(DecodeError::UnexpectedEof);
        }
        let bytes: [u8; 1] = [self.data[self.byte_pos]];
        self.byte_pos += 1;
        Ok(i8::from_le_bytes(bytes))
    }

    /// Read a little-endian `i16`, aligning to a byte boundary first.
    pub fn read_i16(&mut self) -> Result<i16, DecodeError> {
        self.flush_to_byte_boundary();
        if self.remaining() < 2 {
            return Err(DecodeError::UnexpectedEof);
        }
        let bytes: [u8; 2] = self.data[self.byte_pos..self.byte_pos + 2]
            .try_into()
            .unwrap();
        self.byte_pos += 2;
        Ok(i16::from_le_bytes(bytes))
    }

    /// Read a little-endian `i32`, aligning to a byte boundary first.
    pub fn read_i32(&mut self) -> Result<i32, DecodeError> {
        self.flush_to_byte_boundary();
        if self.remaining() < 4 {
            return Err(DecodeError::UnexpectedEof);
        }
        let bytes: [u8; 4] = self.data[self.byte_pos..self.byte_pos + 4]
            .try_into()
            .unwrap();
        self.byte_pos += 4;
        Ok(i32::from_le_bytes(bytes))
    }

    /// Read a little-endian `i64`, aligning to a byte boundary first.
    pub fn read_i64(&mut self) -> Result<i64, DecodeError> {
        self.flush_to_byte_boundary();
        if self.remaining() < 8 {
            return Err(DecodeError::UnexpectedEof);
        }
        let bytes: [u8; 8] = self.data[self.byte_pos..self.byte_pos + 8]
            .try_into()
            .unwrap();
        self.byte_pos += 8;
        Ok(i64::from_le_bytes(bytes))
    }

    /// Read a little-endian `f32`, aligning to a byte boundary first.
    pub fn read_f32(&mut self) -> Result<f32, DecodeError> {
        self.flush_to_byte_boundary();
        if self.remaining() < 4 {
            return Err(DecodeError::UnexpectedEof);
        }
        let bytes: [u8; 4] = self.data[self.byte_pos..self.byte_pos + 4]
            .try_into()
            .unwrap();
        self.byte_pos += 4;
        Ok(f32::from_le_bytes(bytes))
    }

    /// Read a little-endian `f64`, aligning to a byte boundary first.
    pub fn read_f64(&mut self) -> Result<f64, DecodeError> {
        self.flush_to_byte_boundary();
        if self.remaining() < 8 {
            return Err(DecodeError::UnexpectedEof);
        }
        let bytes: [u8; 8] = self.data[self.byte_pos..self.byte_pos + 8]
            .try_into()
            .unwrap();
        self.byte_pos += 8;
        Ok(f64::from_le_bytes(bytes))
    }

    /// Read a LEB128-encoded u64, consuming at most `max_bytes` bytes.
    pub fn read_leb128(&mut self, max_bytes: u8) -> Result<u64, DecodeError> {
        self.flush_to_byte_boundary();
        let (value, consumed) = crate::leb128::decode(&self.data[self.byte_pos..], max_bytes)?;
        self.byte_pos += consumed;
        Ok(value)
    }

    /// Read a ZigZag + LEB128 encoded signed integer.
    pub fn read_zigzag(&mut self, _type_bits: u8, max_bytes: u8) -> Result<i64, DecodeError> {
        let raw = self.read_leb128(max_bytes)?;
        Ok(crate::zigzag::zigzag_decode(raw))
    }

    /// Read a length-prefixed UTF-8 string.
    pub fn read_string(&mut self) -> Result<String, DecodeError> {
        self.flush_to_byte_boundary();
        let len = self.read_leb128(crate::MAX_LENGTH_PREFIX_BYTES)?;
        if len > MAX_BYTES_LENGTH {
            return Err(DecodeError::LimitExceeded {
                field: "string",
                limit: MAX_BYTES_LENGTH,
                actual: len,
            });
        }
        let len = len as usize;
        if self.remaining() < len {
            return Err(DecodeError::UnexpectedEof);
        }
        let bytes = self.data[self.byte_pos..self.byte_pos + len].to_vec();
        self.byte_pos += len;
        String::from_utf8(bytes).map_err(|_| DecodeError::InvalidUtf8)
    }

    /// Read a length-prefixed byte vector.
    pub fn read_bytes(&mut self) -> Result<Vec<u8>, DecodeError> {
        self.flush_to_byte_boundary();
        let len = self.read_leb128(crate::MAX_LENGTH_PREFIX_BYTES)?;
        if len > MAX_BYTES_LENGTH {
            return Err(DecodeError::LimitExceeded {
                field: "bytes",
                limit: MAX_BYTES_LENGTH,
                actual: len,
            });
        }
        let len = len as usize;
        if self.remaining() < len {
            return Err(DecodeError::UnexpectedEof);
        }
        let bytes = self.data[self.byte_pos..self.byte_pos + len].to_vec();
        self.byte_pos += len;
        Ok(bytes)
    }

    /// Read exactly `len` raw bytes with no length prefix.
    pub fn read_raw_bytes(&mut self, len: usize) -> Result<Vec<u8>, DecodeError> {
        self.flush_to_byte_boundary();
        if self.remaining() < len {
            return Err(DecodeError::UnexpectedEof);
        }
        let bytes = self.data[self.byte_pos..self.byte_pos + len].to_vec();
        self.byte_pos += len;
        Ok(bytes)
    }

    /// Read all remaining bytes from the current position to the end.
    /// Flushes to byte boundary first. Returns an empty Vec if no bytes remain.
    pub fn read_remaining(&mut self) -> Vec<u8> {
        self.flush_to_byte_boundary();
        let remaining = self.data.len().saturating_sub(self.byte_pos);
        if remaining == 0 {
            return Vec::new();
        }
        let result = self.data[self.byte_pos..].to_vec();
        self.byte_pos = self.data.len();
        result
    }

    /// Increment recursion depth; return error if limit exceeded.
    pub fn enter_recursive(&mut self) -> Result<(), DecodeError> {
        self.recursion_depth += 1;
        if self.recursion_depth > MAX_RECURSION_DEPTH {
            return Err(DecodeError::RecursionLimitExceeded);
        }
        Ok(())
    }

    /// Decrement recursion depth.
    pub fn leave_recursive(&mut self) {
        self.recursion_depth = self.recursion_depth.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BitWriter;

    #[test]
    fn read_single_bit() {
        let mut r = BitReader::new(&[0x01]);
        assert!(r.read_bool().unwrap());
    }

    #[test]
    fn round_trip_sub_byte() {
        let mut w = BitWriter::new();
        w.write_bits(5, 3);
        w.write_bits(19, 5);
        w.write_bits(42, 6);
        let buf = w.finish();
        let mut r = BitReader::new(&buf);
        assert_eq!(r.read_bits(3).unwrap(), 5);
        assert_eq!(r.read_bits(5).unwrap(), 19);
        assert_eq!(r.read_bits(6).unwrap(), 42);
    }

    #[test]
    fn round_trip_u16() {
        let mut w = BitWriter::new();
        w.write_u16(0x1234);
        let b = w.finish();
        assert_eq!(BitReader::new(&b).read_u16().unwrap(), 0x1234);
    }

    #[test]
    fn round_trip_i32_neg() {
        let mut w = BitWriter::new();
        w.write_i32(-42);
        let b = w.finish();
        assert_eq!(BitReader::new(&b).read_i32().unwrap(), -42);
    }

    #[test]
    fn round_trip_f32() {
        let mut w = BitWriter::new();
        w.write_f32(std::f32::consts::PI);
        let b = w.finish();
        assert_eq!(BitReader::new(&b).read_f32().unwrap(), std::f32::consts::PI);
    }

    #[test]
    fn round_trip_f64_nan() {
        let mut w = BitWriter::new();
        w.write_f64(f64::NAN);
        let b = w.finish();
        let v = BitReader::new(&b).read_f64().unwrap();
        assert!(v.is_nan());
        assert_eq!(v.to_bits(), 0x7FF8000000000000);
    }

    #[test]
    fn round_trip_string() {
        let mut w = BitWriter::new();
        w.write_string("hello");
        let b = w.finish();
        assert_eq!(BitReader::new(&b).read_string().unwrap(), "hello");
    }

    #[test]
    fn round_trip_leb128() {
        let mut w = BitWriter::new();
        w.write_leb128(300);
        let b = w.finish();
        assert_eq!(BitReader::new(&b).read_leb128(4).unwrap(), 300);
    }

    #[test]
    fn round_trip_zigzag() {
        let mut w = BitWriter::new();
        w.write_zigzag(-42, 64);
        let b = w.finish();
        assert_eq!(BitReader::new(&b).read_zigzag(64, 10).unwrap(), -42);
    }

    #[test]
    fn unexpected_eof() {
        assert_eq!(
            BitReader::new(&[]).read_u8().unwrap_err(),
            DecodeError::UnexpectedEof
        );
    }

    #[test]
    fn invalid_utf8() {
        let mut w = BitWriter::new();
        w.write_leb128(2);
        w.write_raw_bytes(&[0xFF, 0xFE]);
        let b = w.finish();
        assert_eq!(
            BitReader::new(&b).read_string().unwrap_err(),
            DecodeError::InvalidUtf8
        );
    }

    #[test]
    fn recursion_depth_limit() {
        let mut r = BitReader::new(&[]);
        for _ in 0..64 {
            r.enter_recursive().unwrap();
        }
        assert_eq!(
            r.enter_recursive().unwrap_err(),
            DecodeError::RecursionLimitExceeded
        );
    }

    #[test]
    fn recursion_depth_leave() {
        let mut r = BitReader::new(&[]);
        for _ in 0..64 {
            r.enter_recursive().unwrap();
        }
        r.leave_recursive();
        r.enter_recursive().unwrap();
    }

    #[test]
    fn trailing_bytes_not_rejected() {
        // Simulate v2-encoded message read by v1 decoder:
        // v2 wrote u32(42) + u16(99), v1 only reads u32(42)
        let data = [0x2a, 0x00, 0x00, 0x00, 0x63, 0x00];
        let mut r = BitReader::new(&data);
        let x = r.read_u32().unwrap();
        assert_eq!(x, 42);
        r.flush_to_byte_boundary();
        // Remaining bytes (0x63, 0x00) must not cause error.
        // BitReader can be dropped with unread data — no panic.
    }

    #[test]
    fn read_remaining_after_partial_decode() {
        let data = [0x2a, 0x00, 0x00, 0x00, 0x63, 0x00];
        let mut r = BitReader::new(&data);
        let _x = r.read_u32().unwrap();
        let remaining = r.read_remaining();
        assert_eq!(remaining, vec![0x63, 0x00]);
    }

    #[test]
    fn read_remaining_when_fully_consumed() {
        let data = [0x2a, 0x00, 0x00, 0x00];
        let mut r = BitReader::new(&data);
        let _x = r.read_u32().unwrap();
        let remaining = r.read_remaining();
        assert!(remaining.is_empty());
    }

    #[test]
    fn read_remaining_from_start() {
        let data = [0x01, 0x02, 0x03];
        let mut r = BitReader::new(&data);
        let remaining = r.read_remaining();
        assert_eq!(remaining, vec![0x01, 0x02, 0x03]);
    }

    #[test]
    fn flush_reader() {
        let mut w = BitWriter::new();
        w.write_bits(0b101, 3);
        w.flush_to_byte_boundary();
        w.write_u8(0xAB);
        let b = w.finish();
        let mut r = BitReader::new(&b);
        assert_eq!(r.read_bits(3).unwrap(), 0b101);
        r.flush_to_byte_boundary();
        assert_eq!(r.read_u8().unwrap(), 0xAB);
    }
}
