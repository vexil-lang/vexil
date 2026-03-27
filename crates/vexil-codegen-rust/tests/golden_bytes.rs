//! Golden byte vector compliance tests.
//!
//! Validates that BitWriter produces bytes matching compliance/vectors/*.json.

use vexil_runtime::BitWriter;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// --- Primitives ---

#[test]
fn verify_bool_false() {
    let mut w = BitWriter::new();
    w.write_bool(false);
    w.flush_to_byte_boundary();
    assert_eq!(hex(&w.finish()), "00");
}

#[test]
fn verify_bool_true() {
    let mut w = BitWriter::new();
    w.write_bool(true);
    w.flush_to_byte_boundary();
    assert_eq!(hex(&w.finish()), "01");
}

#[test]
fn verify_u8_zero() {
    let mut w = BitWriter::new();
    w.write_u8(0);
    assert_eq!(hex(&w.finish()), "00");
}

#[test]
fn verify_u8_max() {
    let mut w = BitWriter::new();
    w.write_u8(255);
    assert_eq!(hex(&w.finish()), "ff");
}

#[test]
fn verify_u16_le() {
    let mut w = BitWriter::new();
    w.write_u16(258);
    assert_eq!(hex(&w.finish()), "0201");
}

#[test]
fn verify_u32_le() {
    let mut w = BitWriter::new();
    w.write_u32(305419896);
    assert_eq!(hex(&w.finish()), "78563412");
}

#[test]
fn verify_i32_negative() {
    let mut w = BitWriter::new();
    w.write_i32(-1);
    assert_eq!(hex(&w.finish()), "ffffffff");
}

#[test]
fn verify_f32_nan_canonical() {
    let mut w = BitWriter::new();
    w.write_f32(f32::NAN);
    assert_eq!(hex(&w.finish()), "0000c07f");
}

#[test]
fn verify_f64_negative_zero() {
    let mut w = BitWriter::new();
    w.write_f64(-0.0_f64);
    assert_eq!(hex(&w.finish()), "0000000000000080");
}

#[test]
fn verify_string_hello() {
    let mut w = BitWriter::new();
    w.write_string("hello");
    assert_eq!(hex(&w.finish()), "0568656c6c6f");
}

#[test]
fn verify_string_empty() {
    let mut w = BitWriter::new();
    w.write_string("");
    assert_eq!(hex(&w.finish()), "00");
}

// --- Sub-byte ---

#[test]
fn verify_u3_u5_packed() {
    let mut w = BitWriter::new();
    w.write_bits(5, 3);
    w.write_bits(18, 5);
    w.flush_to_byte_boundary();
    assert_eq!(hex(&w.finish()), "95");
}

#[test]
fn verify_u3_u5_u6_cross_byte() {
    let mut w = BitWriter::new();
    w.write_bits(7, 3);
    w.write_bits(31, 5);
    w.write_bits(63, 6);
    w.flush_to_byte_boundary();
    // LSB-first: byte 0 = 3+5 bits = 0xFF, byte 1 = 6 bits = 0b00111111 = 0x3F
    assert_eq!(hex(&w.finish()), "ff3f");
}

// --- Messages ---

#[test]
fn verify_empty_message() {
    // Per spec §4.1, an empty message flushes to a single zero byte.
    let w = BitWriter::new();
    assert_eq!(hex(&w.finish()), "00");
}

#[test]
fn verify_two_u32_fields() {
    let mut w = BitWriter::new();
    w.write_u32(1);
    w.write_u32(2);
    assert_eq!(hex(&w.finish()), "0100000002000000");
}

#[test]
fn verify_mixed_bool_u16_string() {
    let mut w = BitWriter::new();
    w.write_bool(true);
    w.flush_to_byte_boundary();
    w.write_u16(42);
    w.write_string("test");
    assert_eq!(hex(&w.finish()), "012a000474657374");
}

// --- Optionals ---

#[test]
fn verify_optional_none() {
    let mut w = BitWriter::new();
    w.write_bool(false);
    w.flush_to_byte_boundary();
    assert_eq!(hex(&w.finish()), "00");
}

#[test]
fn verify_optional_some_u32() {
    let mut w = BitWriter::new();
    w.write_bool(true);
    w.flush_to_byte_boundary();
    w.write_u32(42);
    assert_eq!(hex(&w.finish()), "012a000000");
}

// --- Arrays ---

#[test]
fn verify_array_empty() {
    let mut w = BitWriter::new();
    w.write_leb128(0);
    assert_eq!(hex(&w.finish()), "00");
}

#[test]
fn verify_array_three_u32() {
    let mut w = BitWriter::new();
    w.write_leb128(3);
    w.write_u32(1);
    w.write_u32(2);
    w.write_u32(3);
    assert_eq!(hex(&w.finish()), "03010000000200000003000000");
}
