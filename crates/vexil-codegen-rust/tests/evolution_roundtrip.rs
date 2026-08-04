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

fn decode_v2(bytes: &[u8]) -> Result<(u32, u16), vexil_runtime::DecodeError> {
    let mut r = BitReader::new(bytes);
    let x = r.read_u32()?;
    let y = r.read_u16()?;
    Ok((x, y))
}

#[test]
fn v1_encode_v2_decode_rejects_missing_appended_field() {
    let bytes = encode_v1(42);
    assert_eq!(hex(&bytes), "2a000000");
    assert!(decode_v2(&bytes).is_err());
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
fn appended_nested_field_is_indistinguishable_from_parent_field() {
    let mut w = BitWriter::new();
    w.write_u8(1); // InnerV1.x
    w.write_u8(2); // Outer.z
    let bytes = w.finish();
    assert_eq!(hex(&bytes), "0102");

    let mut r = BitReader::new(&bytes);
    assert_eq!(r.read_u8().unwrap(), 1); // InnerV2.x
    assert_eq!(r.read_u8().unwrap(), 2); // Misread as InnerV2.y
    assert!(r.read_u8().is_err()); // Outer.z has been consumed
}

#[test]
fn appended_zero_sub_byte_field_matches_old_padding() {
    let mut v1 = BitWriter::new();
    v1.write_bits(5, 4);
    let v1_bytes = v1.finish();

    let mut v2 = BitWriter::new();
    v2.write_bits(5, 4);
    v2.write_bits(0, 4);
    let v2_bytes = v2.finish();

    assert_eq!(hex(&v1_bytes), "05");
    assert_eq!(v1_bytes, v2_bytes);
    let mut r = BitReader::new(&v1_bytes);
    assert_eq!(r.read_bits(4).unwrap(), 5);
    assert_eq!(r.read_bits(4).unwrap(), 0);
}

#[test]
fn v2_roundtrip() {
    let bytes = encode_v2(42, 99);
    let (x, y) = decode_v2(&bytes).unwrap();
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
