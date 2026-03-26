/// Bitpack correctness tests.
///
/// These tests target the specific failure modes that simple roundtrip tests miss:
/// integer boundary values, sub-byte packing, ZigZag signed encoding, LEB128
/// multi-byte varints, flags edge cases, and untested type kinds (map, result).
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

fn roundtrip(value: Value, type_name: &str, schema: &vexil_lang::CompiledSchema) -> Value {
    let bytes = encode(&value, type_name, schema)
        .unwrap_or_else(|e| panic!("encode failed for {type_name}: {e:?}"));
    decode(&bytes, type_name, schema)
        .unwrap_or_else(|e| panic!("decode failed for {type_name}: {e:?}"))
}

// ── Integer boundaries ──────────────────────────────────────────────────────

#[test]
fn integer_boundaries_unsigned() {
    let schema = compile(
        r#"
        namespace test.bc.uint
        message Uints {
            a @0 : u8
            b @1 : u16
            c @2 : u32
            d @3 : u64
        }
    "#,
    );

    for (a, b, c, d) in [
        (0u8, 0u16, 0u32, 0u64),
        (1, 1, 1, 1),
        (u8::MAX, u16::MAX, u32::MAX, u64::MAX),
    ] {
        let mut fields = BTreeMap::new();
        fields.insert("a".to_string(), Value::U8(a));
        fields.insert("b".to_string(), Value::U16(b));
        fields.insert("c".to_string(), Value::U32(c));
        fields.insert("d".to_string(), Value::U64(d));
        let v = Value::Message(fields);
        assert_eq!(
            roundtrip(v.clone(), "Uints", &schema),
            v,
            "u={a}/{b}/{c}/{d}"
        );
    }
}

#[test]
fn integer_boundaries_signed() {
    let schema = compile(
        r#"
        namespace test.bc.sint
        message Sints {
            a @0 : i8
            b @1 : i16
            c @2 : i32
            d @3 : i64
        }
    "#,
    );

    for (a, b, c, d) in [
        (0i8, 0i16, 0i32, 0i64),
        (1, 1, 1, 1),
        (-1, -1, -1, -1),
        (i8::MIN, i16::MIN, i32::MIN, i64::MIN),
        (i8::MAX, i16::MAX, i32::MAX, i64::MAX),
    ] {
        let mut fields = BTreeMap::new();
        fields.insert("a".to_string(), Value::I8(a));
        fields.insert("b".to_string(), Value::I16(b));
        fields.insert("c".to_string(), Value::I32(c));
        fields.insert("d".to_string(), Value::I64(d));
        let v = Value::Message(fields);
        assert_eq!(
            roundtrip(v.clone(), "Sints", &schema),
            v,
            "s={a}/{b}/{c}/{d}"
        );
    }
}

// ── Float special values ─────────────────────────────────────────────────────

#[test]
fn float_special_values() {
    let schema = compile(
        r#"
        namespace test.bc.float
        message Floats {
            a @0 : f32
            b @1 : f64
        }
    "#,
    );

    for (a, b) in [
        (0.0f32, 0.0f64),
        (1.0, 1.0),
        (-1.0, -1.0),
        (f32::MAX, f64::MAX),
        (f32::MIN_POSITIVE, f64::MIN_POSITIVE),
        (f32::NEG_INFINITY, f64::NEG_INFINITY),
        (f32::INFINITY, f64::INFINITY),
    ] {
        let mut fields = BTreeMap::new();
        fields.insert("a".to_string(), Value::F32(a));
        fields.insert("b".to_string(), Value::F64(b));
        let v = Value::Message(fields);
        assert_eq!(roundtrip(v.clone(), "Floats", &schema), v, "f={a}/{b}");
    }

    // NaN: equality doesn't hold; check it survives encode/decode and is still NaN
    let mut fields = BTreeMap::new();
    fields.insert("a".to_string(), Value::F32(f32::NAN));
    fields.insert("b".to_string(), Value::F64(f64::NAN));
    let bytes = encode(&Value::Message(fields), "Floats", &schema).unwrap();
    let decoded = decode(&bytes, "Floats", &schema).unwrap();
    if let Value::Message(m) = decoded {
        assert!(matches!(m["a"], Value::F32(v) if v.is_nan()));
        assert!(matches!(m["b"], Value::F64(v) if v.is_nan()));
    } else {
        panic!("expected Message");
    }
}

// ── Sub-byte types ───────────────────────────────────────────────────────────

#[test]
fn sub_byte_all_widths_roundtrip() {
    // Test each sub-byte width at 0, 1, and max value.
    for bits in [1u8, 2, 3, 4, 5, 6, 7] {
        let type_str = format!("u{bits}");
        let schema_src = format!(
            r#"
            namespace test.bc.subbyte{bits}
            message W {{ v @0 : {type_str} }}
        "#
        );
        let schema = compile(&schema_src);
        let max_val: u64 = (1u64 << bits) - 1;

        for &raw in &[0u64, 1, max_val] {
            let mut fields = BTreeMap::new();
            fields.insert(
                "v".to_string(),
                Value::Bits {
                    value: raw,
                    width: bits,
                },
            );
            let v = Value::Message(fields);
            assert_eq!(roundtrip(v.clone(), "W", &schema), v, "u{bits} value={raw}");
        }
    }
}

#[test]
fn sub_byte_overflow_rejected() {
    let schema = compile(
        r#"
        namespace test.bc.overflow
        message W { v @0 : u3 }
    "#,
    );
    let mut fields = BTreeMap::new();
    fields.insert("v".to_string(), Value::Bits { value: 8, width: 3 }); // 8 > max(u3)=7
    let result = encode(&Value::Message(fields), "W", &schema);
    assert!(result.is_err(), "overflow of u3 should be rejected");
}

#[test]
fn sub_byte_width_mismatch_rejected() {
    let schema = compile(
        r#"
        namespace test.bc.mismatch
        message W { v @0 : u3 }
    "#,
    );
    let mut fields = BTreeMap::new();
    fields.insert("v".to_string(), Value::Bits { value: 1, width: 4 }); // width=4 ≠ schema u3
    let result = encode(&Value::Message(fields), "W", &schema);
    assert!(result.is_err(), "width mismatch should be rejected");
}

/// Verifies LSB-first packing: u1=1 + u7=63 must pack into exactly 1 byte = 0x7F.
#[test]
fn multi_field_bit_packing_lsb_order() {
    let schema = compile(
        r#"
        namespace test.bc.pack
        message Packed {
            lo @0 : u1
            hi @1 : u7
        }
    "#,
    );

    // lo=1 occupies bit 0; hi=63 occupies bits 1-7.
    // LSB-first: byte = 1 | (63 << 1) = 1 | 126 = 127 = 0x7F
    let mut fields = BTreeMap::new();
    fields.insert("lo".to_string(), Value::Bits { value: 1, width: 1 });
    fields.insert(
        "hi".to_string(),
        Value::Bits {
            value: 63,
            width: 7,
        },
    );
    let v = Value::Message(fields);

    let bytes = encode(&v, "Packed", &schema).unwrap();
    assert_eq!(bytes, vec![0x7F], "LSB-first u1+u7 must be 0x7F");
    assert_eq!(decode(&bytes, "Packed", &schema).unwrap(), v);
}

#[test]
fn multi_field_three_sub_byte_fields() {
    // u3 + u5 = 8 bits exactly; u3=7 (0b111) + u5=31 (0b11111) = 0b11111_111 = 0xFF
    let schema = compile(
        r#"
        namespace test.bc.pack2
        message Packed2 {
            a @0 : u3
            b @1 : u5
        }
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
    let v = Value::Message(fields);

    let bytes = encode(&v, "Packed2", &schema).unwrap();
    assert_eq!(bytes, vec![0xFF], "u3=7 + u5=31 must be 0xFF");
    assert_eq!(decode(&bytes, "Packed2", &schema).unwrap(), v);
}

// ── Flags edge cases ─────────────────────────────────────────────────────────

#[test]
fn flags_none_set() {
    let schema = compile(
        r#"
        namespace test.bc.flags
        flags Perms { Read @0  Write @1  Exec @2 }
        message Msg { p @0 : Perms }
    "#,
    );
    let mut fields = BTreeMap::new();
    fields.insert("p".to_string(), Value::Flags(vec![]));
    let v = Value::Message(fields);
    assert_eq!(roundtrip(v.clone(), "Msg", &schema), v);
}

#[test]
fn flags_all_set() {
    let schema = compile(
        r#"
        namespace test.bc.flags2
        flags Perms { Read @0  Write @1  Exec @2 }
        message Msg { p @0 : Perms }
    "#,
    );
    let mut fields = BTreeMap::new();
    fields.insert(
        "p".to_string(),
        Value::Flags(vec![
            "Read".to_string(),
            "Write".to_string(),
            "Exec".to_string(),
        ]),
    );
    let v = Value::Message(fields);
    assert_eq!(roundtrip(v.clone(), "Msg", &schema), v);
}

#[test]
fn flags_single_bit_each() {
    let schema = compile(
        r#"
        namespace test.bc.flags3
        flags Bits8 {
            B0 @0  B1 @1  B2 @2  B3 @3
            B4 @4  B5 @5  B6 @6  B7 @7
        }
        message Msg { v @0 : Bits8 }
    "#,
    );
    for i in 0..8usize {
        let flag = format!("B{i}");
        let mut fields = BTreeMap::new();
        fields.insert("v".to_string(), Value::Flags(vec![flag.clone()]));
        let v = Value::Message(fields);
        assert_eq!(roundtrip(v.clone(), "Msg", &schema), v, "single flag B{i}");
    }
}

// ── LEB128 multi-byte varints ────────────────────────────────────────────────

#[test]
fn leb128_large_array_forces_multi_byte_length() {
    let schema = compile(
        r#"
        namespace test.bc.leb
        message Container { items @0 : array<u8> }
    "#,
    );

    // 128 elements → LEB128 length requires 2 bytes (0x80 0x01)
    let items: Vec<Value> = (0u8..=127).map(Value::U8).collect();
    let mut fields = BTreeMap::new();
    fields.insert("items".to_string(), Value::Array(items));
    let v = Value::Message(fields);
    assert_eq!(roundtrip(v.clone(), "Container", &schema), v);
}

#[test]
fn leb128_very_large_array() {
    let schema = compile(
        r#"
        namespace test.bc.leb2
        message Container { items @0 : array<u8> }
    "#,
    );

    // 300 elements → LEB128 length requires 2 bytes
    let items: Vec<Value> = (0u16..300).map(|i| Value::U8(i as u8)).collect();
    let mut fields = BTreeMap::new();
    fields.insert("items".to_string(), Value::Array(items));
    let v = Value::Message(fields);
    assert_eq!(roundtrip(v.clone(), "Container", &schema), v);
}

// ── Map type ─────────────────────────────────────────────────────────────────

#[test]
fn map_empty_roundtrip() {
    let schema = compile(
        r#"
        namespace test.bc.map
        message M { entries @0 : map<string, u32> }
    "#,
    );
    let mut fields = BTreeMap::new();
    fields.insert("entries".to_string(), Value::Map(vec![]));
    let v = Value::Message(fields);
    assert_eq!(roundtrip(v.clone(), "M", &schema), v);
}

#[test]
fn map_multiple_entries_roundtrip() {
    let schema = compile(
        r#"
        namespace test.bc.map2
        message M { entries @0 : map<string, u32> }
    "#,
    );
    let mut fields = BTreeMap::new();
    fields.insert(
        "entries".to_string(),
        Value::Map(vec![
            (Value::String("a".to_string()), Value::U32(1)),
            (Value::String("b".to_string()), Value::U32(2)),
            (Value::String("c".to_string()), Value::U32(u32::MAX)),
        ]),
    );
    let v = Value::Message(fields);
    assert_eq!(roundtrip(v.clone(), "M", &schema), v);
}

// ── Result type ──────────────────────────────────────────────────────────────

#[test]
fn result_ok_roundtrip() {
    let schema = compile(
        r#"
        namespace test.bc.result
        message R { v @0 : result<u32, string> }
    "#,
    );
    let mut fields = BTreeMap::new();
    fields.insert("v".to_string(), Value::Ok(Box::new(Value::U32(42))));
    let v = Value::Message(fields);
    assert_eq!(roundtrip(v.clone(), "R", &schema), v);
}

#[test]
fn result_err_roundtrip() {
    let schema = compile(
        r#"
        namespace test.bc.result2
        message R { v @0 : result<u32, string> }
    "#,
    );
    let mut fields = BTreeMap::new();
    fields.insert(
        "v".to_string(),
        Value::Err(Box::new(Value::String("oops".to_string()))),
    );
    let v = Value::Message(fields);
    assert_eq!(roundtrip(v.clone(), "R", &schema), v);
}

// ── Semantic types ───────────────────────────────────────────────────────────

#[test]
fn semantic_types_roundtrip() {
    let schema = compile(
        r#"
        namespace test.bc.sem
        message Sem {
            s   @0 : string
            b   @1 : bytes
            rgb @2 : rgb
            uid @3 : uuid
            ts  @4 : timestamp
            h   @5 : hash
        }
    "#,
    );

    let mut fields = BTreeMap::new();
    fields.insert("s".to_string(), Value::String("hello ☃ world".to_string()));
    fields.insert("b".to_string(), Value::Bytes(vec![0, 1, 2, 255]));
    fields.insert("rgb".to_string(), Value::Rgb([255, 128, 0]));
    fields.insert("uid".to_string(), Value::Uuid([1u8; 16]));
    fields.insert("ts".to_string(), Value::Timestamp(i64::MIN));
    fields.insert("h".to_string(), Value::Hash([0xABu8; 32]));
    let v = Value::Message(fields);
    assert_eq!(roundtrip(v.clone(), "Sem", &schema), v);
}

#[test]
fn string_empty_and_large() {
    let schema = compile(
        r#"
        namespace test.bc.str
        message S { v @0 : string }
    "#,
    );

    // Empty string
    let empty = Value::Message({
        let mut m = BTreeMap::new();
        m.insert("v".to_string(), Value::String(String::new()));
        m
    });
    assert_eq!(roundtrip(empty.clone(), "S", &schema), empty);

    // Large string forces multi-byte LEB128 for length prefix
    let large = Value::Message({
        let mut m = BTreeMap::new();
        m.insert("v".to_string(), Value::String("x".repeat(200)));
        m
    });
    assert_eq!(roundtrip(large.clone(), "S", &schema), large);
}

// ── Nested messages ──────────────────────────────────────────────────────────

#[test]
fn nested_message_roundtrip() {
    let schema = compile(
        r#"
        namespace test.bc.nested
        message Inner { x @0 : u32  y @1 : u32 }
        message Outer { a @0 : Inner  b @1 : optional<Inner> }
    "#,
    );

    let inner = || {
        let mut m = BTreeMap::new();
        m.insert("x".to_string(), Value::U32(u32::MAX));
        m.insert("y".to_string(), Value::U32(0));
        Value::Message(m)
    };

    let mut fields = BTreeMap::new();
    fields.insert("a".to_string(), inner());
    fields.insert("b".to_string(), Value::Some(Box::new(inner())));
    let v = Value::Message(fields);
    assert_eq!(roundtrip(v.clone(), "Outer", &schema), v);
}

// ── Union multi-field variant ────────────────────────────────────────────────

#[test]
fn union_large_discriminant_leb128() {
    // Build a union with enough variants that a discriminant ≥ 128 is reachable.
    let variants: String = (0..=130)
        .map(|i| format!("V{i} @{i} {{ val @0 : u8 }}"))
        .collect::<Vec<_>>()
        .join("\n");
    let schema_src = format!(
        r#"
        namespace test.bc.union_leb
        union BigUnion {{
            {variants}
        }}
        message M {{ v @0 : BigUnion }}
    "#
    );
    let schema = compile(&schema_src);

    // Variant 130 has discriminant 130 ≥ 128 → needs 2-byte LEB128
    let mut vfields = BTreeMap::new();
    vfields.insert("val".to_string(), Value::U8(42));
    let mut fields = BTreeMap::new();
    fields.insert(
        "v".to_string(),
        Value::Union {
            variant: "V130".to_string(),
            fields: vfields,
        },
    );
    let v = Value::Message(fields);
    assert_eq!(roundtrip(v.clone(), "M", &schema), v);
}
