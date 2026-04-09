/// Wire format golden tests.
///
/// These tests commit the exact byte sequences that Vexil encoding must produce.
/// They run on every supported platform and architecture to guarantee that:
///
///   1. Encoding always produces the same bytes regardless of platform.
///   2. Decoding those bytes always recovers the same values regardless of platform.
///
/// If a test fails on a specific OS/arch but not others, it indicates a
/// platform-specific encoding bug (e.g., endianness assumption, integer representation).
///
/// To regenerate golden bytes after a deliberate wire-format change (requires RFC):
///   UPDATE_WIRE_GOLDEN=1 cargo test -p vexil-store wire_golden
///
/// Regeneration will print the new byte sequences; copy them into the constants below.
use std::collections::BTreeMap;
use vexil_lang::diagnostic::Severity;
use vexil_store::{decode, encode, Value};

fn compile(source: &str) -> vexil_lang::CompiledSchema {
    let result = vexil_lang::compile(source);
    let has_errors = result
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error);
    assert!(!has_errors, "schema errors: {:?}", result.diagnostics);
    result.compiled.expect("schema should compile")
}

/// Assert that encoding `value` produces exactly `golden_bytes`, and that
/// decoding `golden_bytes` produces exactly `value`. Both directions must match
/// on every platform for the wire format to be portable.
fn assert_wire_golden(
    value: Value,
    type_name: &str,
    schema: &vexil_lang::CompiledSchema,
    golden_bytes: &[u8],
    label: &str,
) {
    // Check encode → bytes
    let encoded = encode(&value, type_name, schema)
        .unwrap_or_else(|e| panic!("[{label}] encode failed: {e:?}"));

    if std::env::var("UPDATE_WIRE_GOLDEN").is_ok() {
        println!("[{label}] golden bytes: {:?}", encoded);
    } else {
        assert_eq!(
            encoded, golden_bytes,
            "[{label}] encoded bytes differ from golden — platform-specific encoding bug?"
        );
    }

    // Check decode → value
    let decoded = decode(golden_bytes, type_name, schema)
        .unwrap_or_else(|e| panic!("[{label}] decode of golden bytes failed: {e:?}"));
    assert_eq!(
        decoded, value,
        "[{label}] decoded value from golden bytes differs — platform-specific decoding bug?"
    );
}

// ── Golden: scalar primitives ────────────────────────────────────────────────
//
// Schema: message Scalars { a @0: u8, b @1: u16, c @2: u32, d @3: u64 }
//
// Wire layout (all little-endian, each after byte-align):
//   u8  = 1 byte
//   u16 = 2 bytes LE
//   u32 = 4 bytes LE
//   u64 = 8 bytes LE

#[test]
fn golden_unsigned_max_values() {
    let schema = compile(
        r#"
        namespace test.gld.umax
        message Scalars { a @0: u8  b @1: u16  c @2: u32  d @3: u64 }
    "#,
    );
    let mut fields = BTreeMap::new();
    fields.insert("a".to_string(), Value::U8(u8::MAX));
    fields.insert("b".to_string(), Value::U16(u16::MAX));
    fields.insert("c".to_string(), Value::U32(u32::MAX));
    fields.insert("d".to_string(), Value::U64(u64::MAX));
    let value = Value::Message(fields);

    // u8::MAX=0xFF, u16::MAX=0xFFFF (LE), u32::MAX=0xFFFFFFFF (LE), u64::MAX=0xFFFFFFFFFFFFFFFF (LE)
    #[rustfmt::skip]
    let golden: &[u8] = &[
        0xFF,                               // u8::MAX
        0xFF, 0xFF,                         // u16::MAX LE
        0xFF, 0xFF, 0xFF, 0xFF,             // u32::MAX LE
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, // u64::MAX LE
    ];
    assert_wire_golden(value, "Scalars", &schema, golden, "unsigned_max");
}

#[test]
fn golden_unsigned_zero_values() {
    let schema = compile(
        r#"
        namespace test.gld.uzero
        message Scalars { a @0: u8  b @1: u16  c @2: u32  d @3: u64 }
    "#,
    );
    let mut fields = BTreeMap::new();
    fields.insert("a".to_string(), Value::U8(0));
    fields.insert("b".to_string(), Value::U16(0));
    fields.insert("c".to_string(), Value::U32(0));
    fields.insert("d".to_string(), Value::U64(0));
    let value = Value::Message(fields);

    #[rustfmt::skip]
    let golden: &[u8] = &[
        0x00,
        0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    assert_wire_golden(value, "Scalars", &schema, golden, "unsigned_zero");
}

#[test]
fn golden_signed_boundary_values() {
    // Signed integers use raw two's-complement little-endian (NOT ZigZag).
    let schema = compile(
        r#"
        namespace test.gld.sint
        message Sints { a @0: i8  b @1: i16  c @2: i32  d @3: i64 }
    "#,
    );
    let mut fields = BTreeMap::new();
    fields.insert("a".to_string(), Value::I8(-1));
    fields.insert("b".to_string(), Value::I16(-1));
    fields.insert("c".to_string(), Value::I32(-1));
    fields.insert("d".to_string(), Value::I64(-1));
    let value = Value::Message(fields);

    // -1 in two's-complement is all-ones for each width
    #[rustfmt::skip]
    let golden: &[u8] = &[
        0xFF,
        0xFF, 0xFF,
        0xFF, 0xFF, 0xFF, 0xFF,
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    ];
    assert_wire_golden(value, "Sints", &schema, golden, "signed_neg1");
}

#[test]
fn golden_signed_min_values() {
    let schema = compile(
        r#"
        namespace test.gld.smin
        message Sints { a @0: i8  b @1: i16  c @2: i32  d @3: i64 }
    "#,
    );
    let mut fields = BTreeMap::new();
    fields.insert("a".to_string(), Value::I8(i8::MIN));
    fields.insert("b".to_string(), Value::I16(i16::MIN));
    fields.insert("c".to_string(), Value::I32(i32::MIN));
    fields.insert("d".to_string(), Value::I64(i64::MIN));
    let value = Value::Message(fields);

    // MIN = 0x80 sign bit only for each width, LE
    #[rustfmt::skip]
    let golden: &[u8] = &[
        0x80,
        0x00, 0x80,
        0x00, 0x00, 0x00, 0x80,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80,
    ];
    assert_wire_golden(value, "Sints", &schema, golden, "signed_min");
}

// ── Golden: little-endian byte order of a known u32 ─────────────────────────

#[test]
fn golden_u32_known_value_byte_order() {
    // 0x01020304 in LE should be [0x04, 0x03, 0x02, 0x01]
    let schema = compile(
        r#"
        namespace test.gld.order
        message W { v @0: u32 }
    "#,
    );
    let mut fields = BTreeMap::new();
    fields.insert("v".to_string(), Value::U32(0x01020304));
    assert_wire_golden(
        Value::Message(fields),
        "W",
        &schema,
        &[0x04, 0x03, 0x02, 0x01],
        "u32_byte_order",
    );
}

// ── Golden: sub-byte packing (LSB-first) ────────────────────────────────────

#[test]
fn golden_sub_byte_lsb_first_packing() {
    // u1=1 at bit0, u7=63 at bits1-7: byte = 0b0_111_1111 | 0b0_000_0001 = 0x7F
    // Note: 63 = 0b0111111, so bit6=0 means bit7 of the output byte is 0.
    let schema = compile(
        r#"
        namespace test.gld.subbyte
        message Packed { lo @0: u1  hi @1: u7 }
    "#,
    );
    let mut fields = BTreeMap::new();
    fields.insert("lo".to_string(), Value::Bits { value: 1, width: 1 });
    fields.insert(
        "hi".to_string(),
        Value::Bits {
            value: 63,
            width: 7,
        },
    );
    assert_wire_golden(
        Value::Message(fields),
        "Packed",
        &schema,
        &[0x7F],
        "sub_byte_lsb",
    );
}

#[test]
fn golden_sub_byte_max_u3_u5() {
    // u3=7 (0b111), u5=31 (0b11111): all bits set → 0xFF
    let schema = compile(
        r#"
        namespace test.gld.u3u5
        message Packed { a @0: u3  b @1: u5 }
    "#,
    );
    let mut fields = BTreeMap::new();
    fields.insert("a".to_string(), Value::Bits { value: 7, width: 3 });
    fields.insert(
        "b".to_string(),
        Value::Bits {
            value: 31,
            width: 5,
        },
    );
    assert_wire_golden(
        Value::Message(fields),
        "Packed",
        &schema,
        &[0xFF],
        "u3_u5_max",
    );
}

#[test]
fn golden_sub_byte_zero_fields() {
    // u1=0 + u7=0: byte = 0x00
    let schema = compile(
        r#"
        namespace test.gld.subbyte0
        message Packed { lo @0: u1  hi @1: u7 }
    "#,
    );
    let mut fields = BTreeMap::new();
    fields.insert("lo".to_string(), Value::Bits { value: 0, width: 1 });
    fields.insert("hi".to_string(), Value::Bits { value: 0, width: 7 });
    assert_wire_golden(
        Value::Message(fields),
        "Packed",
        &schema,
        &[0x00],
        "sub_byte_zero",
    );
}

// ── Golden: LEB128 encoding ──────────────────────────────────────────────────

#[test]
fn golden_leb128_array_length_one_byte() {
    // Array of 5 u8 items: LEB128(5) = [0x05], then 5 bytes
    let schema = compile(
        r#"
        namespace test.gld.leb1
        message C { items @0: array<u8> }
    "#,
    );
    let items = vec![
        Value::U8(1),
        Value::U8(2),
        Value::U8(3),
        Value::U8(4),
        Value::U8(5),
    ];
    let mut fields = BTreeMap::new();
    fields.insert("items".to_string(), Value::Array(items));
    assert_wire_golden(
        Value::Message(fields),
        "C",
        &schema,
        &[0x05, 0x01, 0x02, 0x03, 0x04, 0x05],
        "leb128_1byte_len",
    );
}

#[test]
fn golden_leb128_array_length_two_bytes() {
    // Array of 128 items → LEB128(128) = [0x80, 0x01], then 128 bytes of 0x00
    let schema = compile(
        r#"
        namespace test.gld.leb2
        message C { items @0: array<u8> }
    "#,
    );
    let items: Vec<Value> = (0..128u8).map(|_| Value::U8(0)).collect();
    let mut fields = BTreeMap::new();
    fields.insert("items".to_string(), Value::Array(items));

    let mut golden = vec![0x80u8, 0x01]; // LEB128(128)
    golden.extend(std::iter::repeat_n(0x00u8, 128));

    assert_wire_golden(
        Value::Message(fields),
        "C",
        &schema,
        &golden,
        "leb128_2byte_len",
    );
}

#[test]
fn golden_leb128_string_length() {
    // string "hello": LEB128(5) = [0x05], then 0x68 0x65 0x6C 0x6C 0x6F
    let schema = compile(
        r#"
        namespace test.gld.str
        message S { v @0: string }
    "#,
    );
    let mut fields = BTreeMap::new();
    fields.insert("v".to_string(), Value::String("hello".to_string()));
    assert_wire_golden(
        Value::Message(fields),
        "S",
        &schema,
        &[0x05, 0x68, 0x65, 0x6C, 0x6C, 0x6F],
        "leb128_string",
    );
}

// ── Golden: float canonical encoding ────────────────────────────────────────

#[test]
fn golden_float_known_values() {
    // f32: 1.0 = 0x3F800000 LE = [0x00, 0x00, 0x80, 0x3F]
    // f64: 1.0 = 0x3FF0000000000000 LE = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F]
    let schema = compile(
        r#"
        namespace test.gld.float
        message Floats { a @0: f32  b @1: f64 }
    "#,
    );
    let mut fields = BTreeMap::new();
    fields.insert("a".to_string(), Value::F32(1.0));
    fields.insert("b".to_string(), Value::F64(1.0));
    #[rustfmt::skip]
    let golden: &[u8] = &[
        0x00, 0x00, 0x80, 0x3F,                         // f32(1.0) LE bits
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, // f64(1.0) LE bits
    ];
    assert_wire_golden(
        Value::Message(fields),
        "Floats",
        &schema,
        golden,
        "float_1_0",
    );
}

#[test]
fn golden_float_nan_canonicalized() {
    // NaN must be canonicalized: f32 → 0x7FC00000, f64 → 0x7FF8000000000000
    let schema = compile(
        r#"
        namespace test.gld.nan
        message Floats { a @0: f32  b @1: f64 }
    "#,
    );
    let mut fields = BTreeMap::new();
    fields.insert("a".to_string(), Value::F32(f32::NAN));
    fields.insert("b".to_string(), Value::F64(f64::NAN));
    // Canonical NaN: f32=0x7FC00000 LE, f64=0x7FF8000000000000 LE
    #[rustfmt::skip]
    let golden: &[u8] = &[
        0x00, 0x00, 0xC0, 0x7F,                         // canonical f32 NaN
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF8, 0x7F, // canonical f64 NaN
    ];
    // Encoding NaN from any platform must produce the canonical form.
    let encoded = encode(&Value::Message(fields), "Floats", &schema).unwrap();
    assert_eq!(
        encoded, golden,
        "NaN must canonicalize to a platform-independent bit pattern"
    );

    // Decoding the canonical bytes must produce a NaN.
    let decoded = decode(golden, "Floats", &schema).unwrap();
    if let Value::Message(m) = decoded {
        assert!(
            matches!(m["a"], Value::F32(v) if v.is_nan()),
            "f32 should be NaN"
        );
        assert!(
            matches!(m["b"], Value::F64(v) if v.is_nan()),
            "f64 should be NaN"
        );
    } else {
        panic!("expected Message");
    }
}

// ── Golden: optional flag bit ────────────────────────────────────────────────

#[test]
fn golden_optional_none() {
    // optional<u32> absent: 1-bit flag=0, padded to byte = [0x00]
    let schema = compile(
        r#"
        namespace test.gld.opt
        message M { v @0: optional<u32> }
    "#,
    );
    let mut fields = BTreeMap::new();
    fields.insert("v".to_string(), Value::None);
    assert_wire_golden(
        Value::Message(fields),
        "M",
        &schema,
        &[0x00],
        "optional_none",
    );
}

#[test]
fn golden_optional_some() {
    // optional<u32> present: 1-bit flag=1 (bit0), then u32=1 LE at next byte boundary
    // After flag bit, bit_offset=1. write_u32 calls align() → pushes byte [0x01], then [0x01,0x00,0x00,0x00]
    let schema = compile(
        r#"
        namespace test.gld.opt2
        message M { v @0: optional<u32> }
    "#,
    );
    let mut fields = BTreeMap::new();
    fields.insert("v".to_string(), Value::Some(Box::new(Value::U32(1))));
    // flag=1 → partial byte 0x01 flushed by align(), then u32=1 LE
    let golden: &[u8] = &[0x01, 0x01, 0x00, 0x00, 0x00];
    assert_wire_golden(
        Value::Message(fields),
        "M",
        &schema,
        golden,
        "optional_some",
    );
}

// ── Golden: bool ─────────────────────────────────────────────────────────────

#[test]
fn golden_bool_values() {
    let schema = compile(
        r#"
        namespace test.gld.bool
        message M { a @0: bool  b @1: bool }
    "#,
    );
    // true=bit1, false=bit0 → byte 0b00000001 = 0x01
    let mut t = BTreeMap::new();
    t.insert("a".to_string(), Value::Bool(true));
    t.insert("b".to_string(), Value::Bool(false));
    assert_wire_golden(Value::Message(t), "M", &schema, &[0x01], "bool_true_false");

    // false=bit0, true=bit1 → byte 0b00000010 = 0x02
    let mut f = BTreeMap::new();
    f.insert("a".to_string(), Value::Bool(false));
    f.insert("b".to_string(), Value::Bool(true));
    assert_wire_golden(Value::Message(f), "M", &schema, &[0x02], "bool_false_true");
}
