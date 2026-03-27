use crate::error::DecodeError;

/// Encode `value` as an unsigned LEB128 varint, appending bytes to `buf`.
pub fn encode(buf: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
}

/// Decode an unsigned LEB128 varint from `data`, consuming at most `max_bytes`.
///
/// Returns `(value, bytes_consumed)` on success. Returns
/// [`DecodeError::InvalidVarint`] for overlong encodings or if the varint
/// exceeds `max_bytes`, and [`DecodeError::UnexpectedEof`] if the input ends
/// before a terminating byte.
pub fn decode(data: &[u8], max_bytes: u8) -> Result<(u64, usize), DecodeError> {
    let mut result: u64 = 0;
    let mut shift: u32 = 0;

    for (i, &byte) in data.iter().enumerate() {
        if i >= max_bytes as usize {
            return Err(DecodeError::InvalidVarint);
        }
        result |= u64::from(byte & 0x7F) << shift;
        shift += 7;

        if byte & 0x80 == 0 {
            // Reject overlong: if not first byte and byte is 0
            if i > 0 && byte == 0 {
                return Err(DecodeError::InvalidVarint);
            }
            return Ok((result, i + 1));
        }
    }
    Err(DecodeError::UnexpectedEof)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_zero() {
        let mut b = Vec::new();
        encode(&mut b, 0);
        assert_eq!(b, [0x00]);
    }

    #[test]
    fn encode_127() {
        let mut b = Vec::new();
        encode(&mut b, 127);
        assert_eq!(b, [0x7F]);
    }

    #[test]
    fn encode_128() {
        let mut b = Vec::new();
        encode(&mut b, 128);
        assert_eq!(b, [0x80, 0x01]);
    }

    #[test]
    fn encode_300() {
        let mut b = Vec::new();
        encode(&mut b, 300);
        assert_eq!(b, [0xAC, 0x02]);
    }

    #[test]
    fn round_trip_max_u64() {
        let mut buf = Vec::new();
        encode(&mut buf, u64::MAX);
        let (val, consumed) = decode(&buf, 10).unwrap();
        assert_eq!(val, u64::MAX);
        assert_eq!(consumed, 10);
    }

    #[test]
    fn decode_max_4_bytes_limit() {
        let mut buf = Vec::new();
        encode(&mut buf, (1 << 28) - 1);
        assert!(buf.len() <= 4);
        let (val, _) = decode(&buf, 4).unwrap();
        assert_eq!(val, (1 << 28) - 1);
    }

    #[test]
    fn decode_exceeds_max_bytes() {
        let mut buf = Vec::new();
        encode(&mut buf, 1 << 28);
        assert!(decode(&buf, 4).is_err());
    }

    #[test]
    fn reject_overlong_encoding() {
        let buf = [0x80, 0x00];
        assert!(decode(&buf, 10).is_err());
    }
}
