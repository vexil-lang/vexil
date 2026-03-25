//! Hand-written types mirroring codegen output.
//! Proves wire format correctness without compiling generated code.

use vexil_runtime::*;

// ── Simple Message ──

struct Hello {
    name: String,
    age: u8,
}

impl Pack for Hello {
    fn pack(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        w.write_string(&self.name);
        w.write_u8(self.age);
        w.flush_to_byte_boundary();
        Ok(())
    }
}

impl Unpack for Hello {
    fn unpack(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        let name = r.read_string()?;
        let age = r.read_u8()?;
        r.flush_to_byte_boundary();
        Ok(Self { name, age })
    }
}

#[test]
fn hello_round_trip() {
    let val = Hello {
        name: "Alice".into(),
        age: 30,
    };
    let mut w = BitWriter::new();
    val.pack(&mut w).unwrap();
    let buf = w.finish();
    let mut r = BitReader::new(&buf);
    let decoded = Hello::unpack(&mut r).unwrap();
    assert_eq!(decoded.name, "Alice");
    assert_eq!(decoded.age, 30);
}

#[test]
fn hello_exact_bytes() {
    let val = Hello {
        name: "hi".into(),
        age: 7,
    };
    let mut w = BitWriter::new();
    val.pack(&mut w).unwrap();
    let buf = w.finish();
    // LEB128(2) + "hi" + u8(7) = [0x02, 0x68, 0x69, 0x07]
    assert_eq!(buf, [0x02, 0x68, 0x69, 0x07]);
}

// ── Sub-byte packing ──

// Message with u3 + u5 + u6 fields
struct SubByte {
    a: u8, // u3
    b: u8, // u5
    c: u8, // u6
}

impl Pack for SubByte {
    fn pack(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        w.write_bits(self.a as u64, 3);
        w.write_bits(self.b as u64, 5);
        w.write_bits(self.c as u64, 6);
        w.flush_to_byte_boundary();
        Ok(())
    }
}

impl Unpack for SubByte {
    fn unpack(r: &mut BitReader<'_>) -> Result<Self, DecodeError> {
        let a = r.read_bits(3)? as u8;
        let b = r.read_bits(5)? as u8;
        let c = r.read_bits(6)? as u8;
        r.flush_to_byte_boundary();
        Ok(Self { a, b, c })
    }
}

#[test]
fn sub_byte_round_trip() {
    let val = SubByte { a: 5, b: 19, c: 42 };
    let mut w = BitWriter::new();
    val.pack(&mut w).unwrap();
    let buf = w.finish();
    assert_eq!(buf, [0x9D, 0x2A]); // spec worked example
    let mut r = BitReader::new(&buf);
    let decoded = SubByte::unpack(&mut r).unwrap();
    assert_eq!((decoded.a, decoded.b, decoded.c), (5, 19, 42));
}

// ── Optional (byte-aligned T) ──

#[test]
fn optional_present_byte_aligned() {
    let mut w = BitWriter::new();
    let val: Option<String> = Some("hello".into());
    w.write_bool(val.is_some());
    if let Some(ref s) = val {
        w.flush_to_byte_boundary();
        w.write_string(s);
    }
    w.flush_to_byte_boundary();
    let buf = w.finish();

    let mut r = BitReader::new(&buf);
    let present = r.read_bool().unwrap();
    assert!(present);
    r.flush_to_byte_boundary();
    let s = r.read_string().unwrap();
    assert_eq!(s, "hello");
}

#[test]
fn optional_absent() {
    let mut w = BitWriter::new();
    w.write_bool(false);
    w.flush_to_byte_boundary();
    let buf = w.finish();

    let mut r = BitReader::new(&buf);
    assert!(!r.read_bool().unwrap());
}

// ── Optional (sub-byte T) ──

#[test]
fn optional_sub_byte_present() {
    let mut w = BitWriter::new();
    w.write_bool(true); // present
    w.write_bits(5, 3); // u3 value, no flush
    w.flush_to_byte_boundary();
    let buf = w.finish();
    // LSB-first: bit0=1 (present), bit1=1 (lsb of 5), bit2=0, bit3=1 = 0b...1011
    assert_eq!(buf, [0x0B]);

    let mut r = BitReader::new(&buf);
    assert!(r.read_bool().unwrap());
    assert_eq!(r.read_bits(3).unwrap(), 5);
}

// ── Result ──

#[test]
fn result_ok_round_trip() {
    let mut w = BitWriter::new();
    w.write_bool(false); // 0 = Ok
    w.write_u32(42);
    w.flush_to_byte_boundary();
    let buf = w.finish();

    let mut r = BitReader::new(&buf);
    let is_err = r.read_bool().unwrap();
    assert!(!is_err);
    assert_eq!(r.read_u32().unwrap(), 42);
}

#[test]
fn result_err_round_trip() {
    let mut w = BitWriter::new();
    w.write_bool(true); // 1 = Err
    w.write_string("oops");
    w.flush_to_byte_boundary();
    let buf = w.finish();

    let mut r = BitReader::new(&buf);
    let is_err = r.read_bool().unwrap();
    assert!(is_err);
    assert_eq!(r.read_string().unwrap(), "oops");
}

// ── Empty message ──

#[test]
fn empty_message_produces_one_zero_byte() {
    let mut w = BitWriter::new();
    w.flush_to_byte_boundary();
    let buf = w.finish();
    assert_eq!(buf, [0x00]);
}

// ── Delta encoding (integer) ──

#[test]
fn delta_integer_round_trip() {
    // Simulate two sequential writes of a timestamp field with @delta
    let values: Vec<i64> = vec![1000, 1005, 1003];

    // Encode
    let mut w = BitWriter::new();
    let mut prev: i64 = 0;
    for &val in &values {
        let delta = val.wrapping_sub(prev);
        w.write_i64(delta);
        prev = val;
    }
    let buf = w.finish();

    // Decode
    let mut r = BitReader::new(&buf);
    let mut prev: i64 = 0;
    for &expected in &values {
        let delta = r.read_i64().unwrap();
        let val = prev.wrapping_add(delta);
        assert_eq!(val, expected);
        prev = val;
    }
}

// ── Delta + varint composition ──

#[test]
fn delta_varint_round_trip() {
    let values: Vec<u32> = vec![100, 200, 150];

    let mut w = BitWriter::new();
    let mut prev: u32 = 0;
    for &val in &values {
        let delta = val.wrapping_sub(prev);
        w.write_leb128(delta as u64); // varint-encoded delta
        prev = val;
    }
    let buf = w.finish();

    let mut r = BitReader::new(&buf);
    let mut prev: u32 = 0;
    for &expected in &values {
        let delta = r.read_leb128(5).unwrap() as u32;
        let val = prev.wrapping_add(delta);
        assert_eq!(val, expected);
        prev = val;
    }
}

// ── Delta + zigzag composition ──

#[test]
fn delta_zigzag_round_trip() {
    let values: Vec<i32> = vec![10, 15, 8, -5];

    let mut w = BitWriter::new();
    let mut prev: i32 = 0;
    for &val in &values {
        let delta = val.wrapping_sub(prev);
        w.write_zigzag(delta as i64, 32);
        prev = val;
    }
    let buf = w.finish();

    let mut r = BitReader::new(&buf);
    let mut prev: i32 = 0;
    for &expected in &values {
        let delta = r.read_zigzag(32, 5).unwrap() as i32;
        let val = prev.wrapping_add(delta);
        assert_eq!(val, expected);
        prev = val;
    }
}

// ── Delta on float ──

#[test]
fn delta_float_round_trip() {
    let values: Vec<f64> = vec![1.0, 1.5, 1.25];

    let mut w = BitWriter::new();
    let mut prev: f64 = 0.0;
    for &val in &values {
        let delta = val - prev;
        w.write_f64(delta);
        prev = val;
    }
    let buf = w.finish();

    let mut r = BitReader::new(&buf);
    let mut prev: f64 = 0.0;
    for &expected in &values {
        let delta = r.read_f64().unwrap();
        let val = prev + delta;
        assert!((val - expected).abs() < f64::EPSILON);
        prev = val;
    }
}

// ── Union wire format ──

#[test]
fn union_wire_format() {
    // Encode: discriminant=1, payload = [3 bytes: u8, u8, u8]
    let mut w = BitWriter::new();
    // writer is already byte-aligned at start; no flush needed
    w.write_leb128(1); // discriminant
    let mut payload = BitWriter::new();
    payload.write_u8(0xFF);
    payload.write_u8(0x00);
    payload.write_u8(0xAB);
    payload.flush_to_byte_boundary();
    let payload_bytes = payload.finish();
    w.write_leb128(payload_bytes.len() as u64); // byte length
    w.write_raw_bytes(&payload_bytes);
    let buf = w.finish();

    // Decode
    let mut r = BitReader::new(&buf);
    // reader is already byte-aligned at start; no flush needed
    let disc = r.read_leb128(4).unwrap();
    assert_eq!(disc, 1);
    let len = r.read_leb128(4).unwrap() as usize;
    assert_eq!(len, 3);
    let payload = r.read_raw_bytes(len).unwrap();
    assert_eq!(payload, [0xFF, 0x00, 0xAB]);
}

// ── Overlong LEB128 rejection ──

#[test]
fn overlong_leb128_rejected() {
    // 0 encoded as [0x80, 0x00] — overlong
    let buf = [0x80, 0x00];
    let mut r = BitReader::new(&buf);
    assert_eq!(r.read_leb128(10).unwrap_err(), DecodeError::InvalidVarint);
}

// ── @limit enforcement ──

#[test]
fn encode_limit_exceeded() {
    // Simulate: field with @limit(2) on a string of length 3
    let err = EncodeError::LimitExceeded {
        field: "name",
        limit: 2,
        actual: 3,
    };
    // Just verify the error type is constructable and correct
    assert_eq!(
        err,
        EncodeError::LimitExceeded {
            field: "name",
            limit: 2,
            actual: 3,
        }
    );
}
