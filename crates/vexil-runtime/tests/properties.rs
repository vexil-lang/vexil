use proptest::prelude::*;
use vexil_runtime::{BitReader, BitWriter};

// --- BitWriter/BitReader roundtrip properties ---

proptest! {
    /// Any u64 value written with write_u64 must read back identically.
    #[test]
    fn roundtrip_u64(val in any::<u64>()) {
        let mut w = BitWriter::new();
        w.write_u64(val);
        let bytes = w.finish();

        let mut r = BitReader::new(&bytes);
        prop_assert_eq!(r.read_u64().unwrap(), val);
    }

    /// Any u32 value written with write_u32 must read back identically.
    #[test]
    fn roundtrip_u32(val in any::<u32>()) {
        let mut w = BitWriter::new();
        w.write_u32(val);
        let bytes = w.finish();

        let mut r = BitReader::new(&bytes);
        prop_assert_eq!(r.read_u32().unwrap(), val);
    }

    /// Any u16 value written with write_u16 must read back identically.
    #[test]
    fn roundtrip_u16(val in any::<u16>()) {
        let mut w = BitWriter::new();
        w.write_u16(val);
        let bytes = w.finish();

        let mut r = BitReader::new(&bytes);
        prop_assert_eq!(r.read_u16().unwrap(), val);
    }

    /// Any u8 value written with write_u8 must read back identically.
    #[test]
    fn roundtrip_u8(val in any::<u8>()) {
        let mut w = BitWriter::new();
        w.write_u8(val);
        let bytes = w.finish();

        let mut r = BitReader::new(&bytes);
        prop_assert_eq!(r.read_u8().unwrap(), val);
    }

    /// Any i64 value written with write_i64 must read back identically.
    #[test]
    fn roundtrip_i64(val in any::<i64>()) {
        let mut w = BitWriter::new();
        w.write_i64(val);
        let bytes = w.finish();

        let mut r = BitReader::new(&bytes);
        prop_assert_eq!(r.read_i64().unwrap(), val);
    }

    /// Any i32 value written with write_i32 must read back identically.
    #[test]
    fn roundtrip_i32(val in any::<i32>()) {
        let mut w = BitWriter::new();
        w.write_i32(val);
        let bytes = w.finish();

        let mut r = BitReader::new(&bytes);
        prop_assert_eq!(r.read_i32().unwrap(), val);
    }

    /// Any f32 value (including NaN, inf, -0) roundtrips exactly.
    #[test]
    fn roundtrip_f32(val in any::<f32>()) {
        let mut w = BitWriter::new();
        w.write_f32(val);
        let bytes = w.finish();

        let mut r = BitReader::new(&bytes);
        let decoded = r.read_f32().unwrap();
        // Compare bit patterns to handle NaN correctly
        prop_assert_eq!(val.to_bits(), decoded.to_bits());
    }

    /// Any f64 value (including NaN, inf, -0) roundtrips exactly.
    #[test]
    fn roundtrip_f64(val in any::<f64>()) {
        let mut w = BitWriter::new();
        w.write_f64(val);
        let bytes = w.finish();

        let mut r = BitReader::new(&bytes);
        let decoded = r.read_f64().unwrap();
        prop_assert_eq!(val.to_bits(), decoded.to_bits());
    }

    /// Any bool value roundtrips.
    #[test]
    fn roundtrip_bool(val in any::<bool>()) {
        let mut w = BitWriter::new();
        w.write_bool(val);
        w.flush_to_byte_boundary();
        let bytes = w.finish();

        let mut r = BitReader::new(&bytes);
        prop_assert_eq!(r.read_bool().unwrap(), val);
    }

    /// Sub-byte fields: any value fitting in `bits` bits roundtrips.
    /// We test bits 1..=8 to cover all sub-byte types.
    #[test]
    fn roundtrip_sub_byte(bits in 1u8..=8, val in any::<u64>()) {
        let mask = if bits == 64 { u64::MAX } else { (1u64 << bits) - 1 };
        let masked_val = val & mask;

        let mut w = BitWriter::new();
        w.write_bits(masked_val, bits);
        w.flush_to_byte_boundary();
        let bytes = w.finish();

        let mut r = BitReader::new(&bytes);
        prop_assert_eq!(r.read_bits(bits).unwrap(), masked_val);
    }

    /// Multiple sub-byte fields packed together roundtrip correctly.
    #[test]
    fn roundtrip_packed_fields(
        a in 0u8..=7u8,
        b in 0u8..=31u8,
        c in 0u8..=15u8,
    ) {
        let mut w = BitWriter::new();
        w.write_bits(a as u64, 3);
        w.write_bits(b as u64, 5);
        w.write_bits(c as u64, 4);
        w.flush_to_byte_boundary();
        let bytes = w.finish();

        let mut r = BitReader::new(&bytes);
        prop_assert_eq!(r.read_bits(3).unwrap(), a as u64);
        prop_assert_eq!(r.read_bits(5).unwrap(), b as u64);
        prop_assert_eq!(r.read_bits(4).unwrap(), c as u64);
    }

    /// String roundtrip: any valid UTF-8 string up to 1024 chars.
    #[test]
    fn roundtrip_string(val in "[ -~]{0,1024}") {
        let mut w = BitWriter::new();
        w.write_string(&val);
        let bytes = w.finish();

        let mut r = BitReader::new(&bytes);
        prop_assert_eq!(r.read_string().unwrap(), val);
    }

    /// String roundtrip zero-copy: read_string_ref returns the same string.
    #[test]
    fn roundtrip_string_ref(val in "[ -~]{0,1024}") {
        let mut w = BitWriter::new();
        w.write_string(&val);
        let bytes = w.finish();

        let mut r = BitReader::new(&bytes);
        prop_assert_eq!(r.read_string_ref().unwrap(), val);
    }

    /// Bytes roundtrip: any byte sequence up to 1024 bytes.
    #[test]
    fn roundtrip_bytes(val in prop::collection::vec(any::<u8>(), 0..=1024)) {
        let mut w = BitWriter::new();
        w.write_bytes(&val);
        let bytes = w.finish();

        let mut r = BitReader::new(&bytes);
        prop_assert_eq!(r.read_bytes().unwrap(), val);
    }

    /// LEB128 roundtrip: any value up to 2^35.
    #[test]
    fn roundtrip_leb128(val in 0u64..(1u64 << 35)) {
        let mut w = BitWriter::new();
        w.write_leb128(val);
        let bytes = w.finish();

        let mut r = BitReader::new(&bytes);
        prop_assert_eq!(r.read_leb128(10).unwrap(), val);
    }

    /// BitWriter with_capacity + reset: encoding same data after reset
    /// produces identical bytes.
    #[test]
    fn writer_reset_reuse(val in any::<u32>()) {
        let mut w = BitWriter::with_capacity(64);

        w.write_u32(val);
        let bytes1 = w.finish();

        let mut w = BitWriter::with_capacity(64);
        w.write_u32(val + 1);
        w.reset();
        w.write_u32(val);
        let bytes2 = w.finish();

        prop_assert_eq!(bytes1, bytes2);
    }
}
