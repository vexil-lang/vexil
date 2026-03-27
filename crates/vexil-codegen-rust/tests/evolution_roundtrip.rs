//! Schema evolution roundtrip tests.

use vexil_runtime::{BitReader, BitWriter};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn encode_v1(x: u32) -> Vec<u8> {
    let mut w = BitWriter::new();
    w.write_u32(x);
    w.finish()
}

fn encode_v2(x: u32, y: u16) -> Vec<u8> {
    let mut w = BitWriter::new();
    w.write_u32(x);
    w.write_u16(y);
    w.finish()
}

fn decode_v1(bytes: &[u8]) -> u32 {
    let mut r = BitReader::new(bytes);
    r.read_u32().unwrap()
}

fn decode_v2(bytes: &[u8]) -> (u32, u16) {
    let mut r = BitReader::new(bytes);
    let x = r.read_u32().unwrap();
    // Simulate forward-compatible decode: if there aren't enough bytes for the
    // new field, fall back to a default.  `remaining()` is private on BitReader,
    // so we check the slice length vs what we've already consumed (4 bytes for u32).
    let y = if bytes.len() >= 4 + 2 {
        r.read_u16().unwrap()
    } else {
        0
    };
    (x, y)
}

#[test]
fn v1_encode_v2_decode_field_gets_default() {
    let bytes = encode_v1(42);
    assert_eq!(hex(&bytes), "2a000000");
    let (x, y) = decode_v2(&bytes);
    assert_eq!(x, 42);
    assert_eq!(y, 0);
}

#[test]
fn v2_encode_v1_decode_trailing_ignored() {
    let bytes = encode_v2(42, 99);
    assert_eq!(hex(&bytes), "2a0000006300");
    let x = decode_v1(&bytes);
    assert_eq!(x, 42);
}

#[test]
fn v1_v2_prefix_bit_identical() {
    let v1_bytes = encode_v1(42);
    let v2_bytes = encode_v2(42, 99);
    assert_eq!(&v1_bytes[..4], &v2_bytes[..4]);
}

#[test]
fn v2_roundtrip() {
    let bytes = encode_v2(42, 99);
    let (x, y) = decode_v2(&bytes);
    assert_eq!(x, 42);
    assert_eq!(y, 99);
}

#[test]
fn deprecated_field_still_encodes() {
    let mut w = BitWriter::new();
    w.write_string("current");
    w.write_string("old");
    w.write_u32(30);
    let bytes = w.finish();

    let mut r = BitReader::new(&bytes);
    assert_eq!(r.read_string().unwrap(), "current");
    assert_eq!(r.read_string().unwrap(), "old");
    assert_eq!(r.read_u32().unwrap(), 30);
}

#[test]
fn required_to_optional_is_breaking() {
    let mut w1 = BitWriter::new();
    w1.write_u32(42);
    w1.write_string("test");
    let v1_bytes = w1.finish();

    let mut w2 = BitWriter::new();
    w2.write_bool(true);
    w2.flush_to_byte_boundary();
    w2.write_u32(42);
    w2.write_string("test");
    let v2_bytes = w2.finish();

    assert_ne!(v1_bytes, v2_bytes);
    assert_ne!(v1_bytes[0], v2_bytes[0]);
}
