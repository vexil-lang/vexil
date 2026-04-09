//! Tests for deterministic map key ordering across all valid key types.
//!
//! Per the Vexil spec §3.4, map keys MUST be sorted in ascending canonical order:
//! - Integer types: ascending numeric (signed use signed comparison)
//! - string, bytes: ascending lexicographic
//! - uuid: ascending lexicographic (16-byte big-endian)
//! - enum: ascending by ordinal value
//! - flags: ascending by bit value (1 << ordinal)
//! - newtypes: follow the inner type sort order

use std::collections::BTreeMap;
use vexil_lang::diagnostic::Severity;
use vexil_store::{encode, Value};

fn compile_schema(source: &str) -> vexil_lang::CompiledSchema {
    let result = vexil_lang::compile(source);
    let has_errors = result
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error);
    assert!(!has_errors, "schema errors: {:?}", result.diagnostics);
    result.compiled.expect("schema should compile")
}

/// Helper: encode a map with entries in given order, return the bytes
fn encode_map_value(entries: Vec<(Value, Value)>, schema: &vexil_lang::CompiledSchema) -> Vec<u8> {
    let mut fields = BTreeMap::new();
    fields.insert("data".to_string(), Value::Map(entries));
    let value = Value::Message(fields);
    encode(&value, "TestMsg", schema).expect("encode should succeed")
}

// ============================================================================
// Integer key types
// ============================================================================

#[test]
fn map_key_order_u8_deterministic() {
    let schema = compile_schema(
        r#"
        namespace test.map_order
        message TestMsg {
            data @0 : map<u8, string>
        }
    "#,
    );

    // Entries in reverse order: 5, 3, 1
    let entries = vec![
        (Value::U8(5), Value::String("five".to_string())),
        (Value::U8(3), Value::String("three".to_string())),
        (Value::U8(1), Value::String("one".to_string())),
    ];
    let bytes1 = encode_map_value(entries.clone(), &schema);

    // Entries in different order: 3, 1, 5
    let entries2 = vec![
        (Value::U8(3), Value::String("three".to_string())),
        (Value::U8(1), Value::String("one".to_string())),
        (Value::U8(5), Value::String("five".to_string())),
    ];
    let bytes2 = encode_map_value(entries2, &schema);

    // Both should produce identical bytes (sorted order: 1, 3, 5)
    assert_eq!(
        bytes1, bytes2,
        "map encoding must be deterministic regardless of insertion order"
    );
}

#[test]
fn map_key_order_u64_deterministic() {
    let schema = compile_schema(
        r#"
        namespace test.map_order
        message TestMsg {
            data @0 : map<u64, string>
        }
    "#,
    );

    let entries = vec![
        (Value::U64(u64::MAX), Value::String("max".to_string())),
        (Value::U64(0), Value::String("zero".to_string())),
        (Value::U64(1000), Value::String("thousand".to_string())),
    ];
    let bytes1 = encode_map_value(entries.clone(), &schema);

    let entries2 = vec![
        (Value::U64(0), Value::String("zero".to_string())),
        (Value::U64(u64::MAX), Value::String("max".to_string())),
        (Value::U64(1000), Value::String("thousand".to_string())),
    ];
    let bytes2 = encode_map_value(entries2, &schema);

    assert_eq!(bytes1, bytes2, "u64 map keys must sort ascending");
}

#[test]
fn map_key_order_i8_signed_comparison() {
    let schema = compile_schema(
        r#"
        namespace test.map_order
        message TestMsg {
            data @0 : map<i8, string>
        }
    "#,
    );

    // Mix of positive and negative: signed comparison should put negatives first
    let entries = vec![
        (Value::I8(127), Value::String("max".to_string())),
        (Value::I8(-128), Value::String("min".to_string())),
        (Value::I8(0), Value::String("zero".to_string())),
        (Value::I8(-1), Value::String("neg_one".to_string())),
        (Value::I8(1), Value::String("one".to_string())),
    ];
    let bytes1 = encode_map_value(entries.clone(), &schema);

    let entries2 = vec![
        (Value::I8(1), Value::String("one".to_string())),
        (Value::I8(-128), Value::String("min".to_string())),
        (Value::I8(127), Value::String("max".to_string())),
        (Value::I8(0), Value::String("zero".to_string())),
        (Value::I8(-1), Value::String("neg_one".to_string())),
    ];
    let bytes2 = encode_map_value(entries2, &schema);

    assert_eq!(bytes1, bytes2, "i8 map keys must use signed comparison");
}

#[test]
fn map_key_order_i64_signed_comparison() {
    let schema = compile_schema(
        r#"
        namespace test.map_order
        message TestMsg {
            data @0 : map<i64, string>
        }
    "#,
    );

    let entries = vec![
        (Value::I64(i64::MAX), Value::String("max".to_string())),
        (Value::I64(i64::MIN), Value::String("min".to_string())),
        (Value::I64(-1), Value::String("neg_one".to_string())),
        (Value::I64(0), Value::String("zero".to_string())),
        (Value::I64(1), Value::String("one".to_string())),
    ];
    let bytes1 = encode_map_value(entries.clone(), &schema);

    let entries2 = vec![
        (Value::I64(0), Value::String("zero".to_string())),
        (Value::I64(i64::MIN), Value::String("min".to_string())),
        (Value::I64(1), Value::String("one".to_string())),
        (Value::I64(i64::MAX), Value::String("max".to_string())),
        (Value::I64(-1), Value::String("neg_one".to_string())),
    ];
    let bytes2 = encode_map_value(entries2, &schema);

    assert_eq!(
        bytes1, bytes2,
        "i64 map keys must use signed comparison (negatives before positives)"
    );
}

#[test]
fn map_key_order_fixed32_signed_comparison() {
    let schema = compile_schema(
        r#"
        namespace test.map_order
        message TestMsg {
            data @0 : map<fixed32, string>
        }
    "#,
    );

    let entries = vec![
        (Value::Fixed32(i32::MAX), Value::String("max".to_string())),
        (Value::Fixed32(i32::MIN), Value::String("min".to_string())),
        (Value::Fixed32(-1), Value::String("neg_one".to_string())),
        (Value::Fixed32(0), Value::String("zero".to_string())),
        (Value::Fixed32(1), Value::String("one".to_string())),
    ];
    let bytes1 = encode_map_value(entries.clone(), &schema);

    let entries2 = vec![
        (Value::Fixed32(0), Value::String("zero".to_string())),
        (Value::Fixed32(i32::MIN), Value::String("min".to_string())),
        (Value::Fixed32(1), Value::String("one".to_string())),
        (Value::Fixed32(i32::MAX), Value::String("max".to_string())),
        (Value::Fixed32(-1), Value::String("neg_one".to_string())),
    ];
    let bytes2 = encode_map_value(entries2, &schema);

    assert_eq!(
        bytes1, bytes2,
        "fixed32 map keys must use signed comparison (negatives before positives)"
    );
}

#[test]
fn map_key_order_bool_deterministic() {
    let schema = compile_schema(
        r#"
        namespace test.map_order
        message TestMsg {
            data @0 : map<bool, string>
        }
    "#,
    );

    // false (0) should come before true (1)
    let entries = vec![
        (Value::Bool(true), Value::String("true_val".to_string())),
        (Value::Bool(false), Value::String("false_val".to_string())),
    ];
    let bytes1 = encode_map_value(entries.clone(), &schema);

    let entries2 = vec![
        (Value::Bool(false), Value::String("false_val".to_string())),
        (Value::Bool(true), Value::String("true_val".to_string())),
    ];
    let bytes2 = encode_map_value(entries2, &schema);

    assert_eq!(bytes1, bytes2, "bool map keys must sort false before true");
}

#[test]
fn map_key_order_fixed64_signed_comparison() {
    let schema = compile_schema(
        r#"
        namespace test.map_order
        message TestMsg {
            data @0 : map<fixed64, string>
        }
    "#,
    );

    let entries = vec![
        (Value::Fixed64(i64::MAX), Value::String("max".to_string())),
        (Value::Fixed64(i64::MIN), Value::String("min".to_string())),
        (Value::Fixed64(-1), Value::String("neg_one".to_string())),
        (Value::Fixed64(0), Value::String("zero".to_string())),
        (Value::Fixed64(1), Value::String("one".to_string())),
    ];
    let bytes1 = encode_map_value(entries.clone(), &schema);

    let entries2 = vec![
        (Value::Fixed64(0), Value::String("zero".to_string())),
        (Value::Fixed64(i64::MIN), Value::String("min".to_string())),
        (Value::Fixed64(1), Value::String("one".to_string())),
        (Value::Fixed64(i64::MAX), Value::String("max".to_string())),
        (Value::Fixed64(-1), Value::String("neg_one".to_string())),
    ];
    let bytes2 = encode_map_value(entries2, &schema);

    assert_eq!(
        bytes1, bytes2,
        "fixed64 map keys must use signed comparison (negatives before positives)"
    );
}

// ============================================================================
// String and bytes key types
// ============================================================================

#[test]
fn map_key_order_string_lexicographic() {
    let schema = compile_schema(
        r#"
        namespace test.map_order
        message TestMsg {
            data @0 : map<string, u32>
        }
    "#,
    );

    // String comparison is lexicographic by UTF-8 bytes
    let entries = vec![
        (Value::String("zebra".to_string()), Value::U32(1)),
        (Value::String("apple".to_string()), Value::U32(2)),
        (Value::String("mango".to_string()), Value::U32(3)),
    ];
    let bytes1 = encode_map_value(entries.clone(), &schema);

    let entries2 = vec![
        (Value::String("mango".to_string()), Value::U32(3)),
        (Value::String("apple".to_string()), Value::U32(2)),
        (Value::String("zebra".to_string()), Value::U32(1)),
    ];
    let bytes2 = encode_map_value(entries2, &schema);

    assert_eq!(
        bytes1, bytes2,
        "string map keys must sort lexicographically (apple, mango, zebra)"
    );
}

#[test]
fn map_key_order_bytes_lexicographic() {
    let schema = compile_schema(
        r#"
        namespace test.map_order
        message TestMsg {
            data @0 : map<bytes, u32>
        }
    "#,
    );

    let entries = vec![
        (Value::Bytes(vec![0xFF, 0xFF]), Value::U32(1)),
        (Value::Bytes(vec![0x00, 0x00]), Value::U32(2)),
        (Value::Bytes(vec![0x80, 0x00]), Value::U32(3)),
    ];
    let bytes1 = encode_map_value(entries.clone(), &schema);

    let entries2 = vec![
        (Value::Bytes(vec![0x80, 0x00]), Value::U32(3)),
        (Value::Bytes(vec![0x00, 0x00]), Value::U32(2)),
        (Value::Bytes(vec![0xFF, 0xFF]), Value::U32(1)),
    ];
    let bytes2 = encode_map_value(entries2, &schema);

    assert_eq!(
        bytes1, bytes2,
        "bytes map keys must sort lexicographically (00 00, 80 00, FF FF)"
    );
}

#[test]
fn map_key_order_uuid_lexicographic() {
    let schema = compile_schema(
        r#"
        namespace test.map_order
        message TestMsg {
            data @0 : map<uuid, u32>
        }
    "#,
    );

    let entries = vec![
        (
            Value::Uuid([0xFF; 16]), // all 0xFF (highest)
            Value::U32(1),
        ),
        (
            Value::Uuid([0x00; 16]), // all 0x00 (lowest)
            Value::U32(2),
        ),
        (
            Value::Uuid([0x80; 16]), // all 0x80 (middle)
            Value::U32(3),
        ),
    ];
    let bytes1 = encode_map_value(entries.clone(), &schema);

    let entries2 = vec![
        (Value::Uuid([0x80; 16]), Value::U32(3)),
        (Value::Uuid([0x00; 16]), Value::U32(2)),
        (Value::Uuid([0xFF; 16]), Value::U32(1)),
    ];
    let bytes2 = encode_map_value(entries2, &schema);

    assert_eq!(
        bytes1, bytes2,
        "uuid map keys must sort lexicographically by 16-byte big-endian representation"
    );
}

// ============================================================================
// Enum key type
// ============================================================================

#[test]
fn map_key_order_enum_by_ordinal() {
    let schema = compile_schema(
        r#"
        namespace test.map_order
        enum Status {
            Pending   @5
            Active    @1
            Completed @10
            Failed    @2
        }
        message TestMsg {
            data @0 : map<Status, string>
        }
    "#,
    );

    // Enum variants should sort by their ordinal values: 1(Active), 2(Failed), 5(Pending), 10(Completed)
    let entries = vec![
        (
            Value::Enum("Completed".to_string()),
            Value::String("done".to_string()),
        ),
        (
            Value::Enum("Pending".to_string()),
            Value::String("waiting".to_string()),
        ),
        (
            Value::Enum("Active".to_string()),
            Value::String("running".to_string()),
        ),
        (
            Value::Enum("Failed".to_string()),
            Value::String("error".to_string()),
        ),
    ];
    let bytes1 = encode_map_value(entries.clone(), &schema);

    let entries2 = vec![
        (
            Value::Enum("Active".to_string()),
            Value::String("running".to_string()),
        ),
        (
            Value::Enum("Failed".to_string()),
            Value::String("error".to_string()),
        ),
        (
            Value::Enum("Pending".to_string()),
            Value::String("waiting".to_string()),
        ),
        (
            Value::Enum("Completed".to_string()),
            Value::String("done".to_string()),
        ),
    ];
    let bytes2 = encode_map_value(entries2, &schema);

    assert_eq!(
        bytes1, bytes2,
        "enum map keys must sort by ordinal value, not variant name"
    );
}

// ============================================================================
// Flags key type
// ============================================================================

#[test]
fn map_key_order_flags_by_bit_value() {
    let schema = compile_schema(
        r#"
        namespace test.map_order
        flags Permissions {
            Read    @0  # bit value: 1
            Write   @1  # bit value: 2
            Execute @2  # bit value: 4
            Delete  @3  # bit value: 8
        }
        message TestMsg {
            data @0 : map<Permissions, string>
        }
    "#,
    );

    // Flags should sort by bit value: Read(1) < Write(2) < Execute(4) < Delete(8)
    let entries = vec![
        (
            Value::Flags(vec!["Delete".to_string()]),
            Value::String("del".to_string()),
        ),
        (
            Value::Flags(vec!["Read".to_string()]),
            Value::String("read".to_string()),
        ),
        (
            Value::Flags(vec!["Execute".to_string()]),
            Value::String("exec".to_string()),
        ),
        (
            Value::Flags(vec!["Write".to_string()]),
            Value::String("write".to_string()),
        ),
    ];
    let bytes1 = encode_map_value(entries.clone(), &schema);

    let entries2 = vec![
        (
            Value::Flags(vec!["Read".to_string()]),
            Value::String("read".to_string()),
        ),
        (
            Value::Flags(vec!["Write".to_string()]),
            Value::String("write".to_string()),
        ),
        (
            Value::Flags(vec!["Execute".to_string()]),
            Value::String("exec".to_string()),
        ),
        (
            Value::Flags(vec!["Delete".to_string()]),
            Value::String("del".to_string()),
        ),
    ];
    let bytes2 = encode_map_value(entries2, &schema);

    assert_eq!(
        bytes1, bytes2,
        "flags map keys must sort by bit value (1 << ordinal), not bit name"
    );
}

#[test]
fn map_key_order_flags_combined_by_sum() {
    let schema = compile_schema(
        r#"
        namespace test.map_order
        flags Perms {
            A @0  # bit 1
            B @1  # bit 2
            C @2  # bit 4
        }
        message TestMsg {
            data @0 : map<Perms, string>
        }
    "#,
    );

    // Combined flags sort by sum of bit values: A(1) < B(2) < A+B(3) < C(4)
    let entries = vec![
        (
            Value::Flags(vec!["C".to_string()]), // 4
            Value::String("c".to_string()),
        ),
        (
            Value::Flags(vec!["A".to_string(), "B".to_string()]), // 1+2=3
            Value::String("ab".to_string()),
        ),
        (
            Value::Flags(vec!["B".to_string()]), // 2
            Value::String("b".to_string()),
        ),
        (
            Value::Flags(vec!["A".to_string()]), // 1
            Value::String("a".to_string()),
        ),
    ];
    let bytes1 = encode_map_value(entries.clone(), &schema);

    let entries2 = vec![
        (
            Value::Flags(vec!["A".to_string()]), // 1
            Value::String("a".to_string()),
        ),
        (
            Value::Flags(vec!["B".to_string()]), // 2
            Value::String("b".to_string()),
        ),
        (
            Value::Flags(vec!["A".to_string(), "B".to_string()]), // 3
            Value::String("ab".to_string()),
        ),
        (
            Value::Flags(vec!["C".to_string()]), // 4
            Value::String("c".to_string()),
        ),
    ];
    let bytes2 = encode_map_value(entries2, &schema);

    assert_eq!(
        bytes1, bytes2,
        "combined flags must sort by sum of bit values (1, 2, 3, 4)"
    );
}

// ============================================================================
// Newtype key type
// ============================================================================

#[test]
fn map_key_order_newtype_follows_inner_type() {
    let schema = compile_schema(
        r#"
        namespace test.map_order
        newtype UserId : u32
        newtype Label : string
        
        message TestMsg {
            id_map   @0 : map<UserId, string>
            label_map @1 : map<Label, u32>
        }
    "#,
    );

    // Test UserId (u32 newtype) - should sort as u32
    let entries_id = vec![
        (Value::U32(100), Value::String("hundred".to_string())),
        (Value::U32(1), Value::String("one".to_string())),
        (Value::U32(50), Value::String("fifty".to_string())),
    ];

    let entries_id2 = vec![
        (Value::U32(1), Value::String("one".to_string())),
        (Value::U32(50), Value::String("fifty".to_string())),
        (Value::U32(100), Value::String("hundred".to_string())),
    ];

    // Test Label (string newtype) - should sort as string
    let entries_label = vec![
        (Value::String("zebra".to_string()), Value::U32(1)),
        (Value::String("apple".to_string()), Value::U32(2)),
    ];

    let entries_label2 = vec![
        (Value::String("apple".to_string()), Value::U32(2)),
        (Value::String("zebra".to_string()), Value::U32(1)),
    ];

    let mut fields1 = BTreeMap::new();
    fields1.insert("id_map".to_string(), Value::Map(entries_id));
    fields1.insert("label_map".to_string(), Value::Map(entries_label));
    let value1 = Value::Message(fields1);
    let bytes1 = encode(&value1, "TestMsg", &schema).expect("encode should succeed");

    let mut fields2 = BTreeMap::new();
    fields2.insert("id_map".to_string(), Value::Map(entries_id2));
    fields2.insert("label_map".to_string(), Value::Map(entries_label2));
    let value2 = Value::Message(fields2);
    let bytes2 = encode(&value2, "TestMsg", &schema).expect("encode should succeed");

    assert_eq!(
        bytes1, bytes2,
        "newtype map keys must follow the sort order of their inner type"
    );
}
