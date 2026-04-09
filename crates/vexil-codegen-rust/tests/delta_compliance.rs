//! Delta encoding compliance tests.
//!
//! These tests validate that BitWriter produces correct bytes for delta-encoded
//! sequences. Since we can't easily run generated encoder code in integration
//! tests, we simulate the delta computation manually using BitWriter.

use vexil_runtime::BitWriter;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn delta_u32_two_frames() {
    // Frame 1: delta from 0 to 100 = 100
    let mut w = BitWriter::new();
    w.write_u32(100_u32.wrapping_sub(0));
    w.flush_to_byte_boundary();
    let frame1 = hex(&w.finish());
    assert_eq!(frame1, "64000000");

    // Frame 2: delta from 100 to 110 = 10
    let mut w = BitWriter::new();
    w.write_u32(110_u32.wrapping_sub(100));
    w.flush_to_byte_boundary();
    let frame2 = hex(&w.finish());
    assert_eq!(frame2, "0a000000");
}

#[test]
fn delta_i64_three_frames() {
    let values: Vec<i64> = vec![1000, 2000, 2005];
    let expected = ["e803000000000000", "e803000000000000", "0500000000000000"];
    let mut prev: i64 = 0;
    for (i, val) in values.iter().enumerate() {
        let delta = val - prev;
        let mut w = BitWriter::new();
        w.write_i64(delta);
        w.flush_to_byte_boundary();
        assert_eq!(hex(&w.finish()), expected[i], "frame {} mismatch", i + 1);
        prev = *val;
    }
}

#[test]
fn delta_mixed_message() {
    // Frame 1: ts delta=1000, label="hello", count delta=50
    let mut w = BitWriter::new();
    w.write_i64(1000); // ts: 0->1000
    w.write_string("hello");
    w.write_u32(50); // count: 0->50
    w.flush_to_byte_boundary();
    assert_eq!(hex(&w.finish()), "e8030000000000000568656c6c6f32000000");

    // Frame 2: ts delta=1000, label="hello", count delta=5
    let mut w = BitWriter::new();
    w.write_i64(1000); // 2000-1000
    w.write_string("hello");
    w.write_u32(5); // 55-50
    w.flush_to_byte_boundary();
    assert_eq!(hex(&w.finish()), "e8030000000000000568656c6c6f05000000");
}

#[test]
fn delta_reset() {
    // Frame 1: 0->100 = 100
    let mut w = BitWriter::new();
    w.write_u32(100);
    w.flush_to_byte_boundary();
    assert_eq!(hex(&w.finish()), "64000000");

    // Frame 2: 100->150 = 50
    let mut w = BitWriter::new();
    w.write_u32(50);
    w.flush_to_byte_boundary();
    assert_eq!(hex(&w.finish()), "32000000");

    // After reset: 0->100 = 100 (same as frame 1)
    let mut w = BitWriter::new();
    w.write_u32(100);
    w.flush_to_byte_boundary();
    assert_eq!(hex(&w.finish()), "64000000");
}
