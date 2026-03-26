use vexil_lang::ast::{PrimitiveType, SemanticType};
use vexil_lang::ir::{ConfigDef, EnumDef, FlagsDef, MessageDef, UnionDef};
use vexil_lang::{CompiledSchema, ResolvedType, TypeDef, TypeRegistry};
use vexil_runtime::BitWriter;

use crate::error::StoreEncodeError;
use crate::Value;

/// Encode a `Value` as bitpack bytes using the given schema and type name.
pub fn encode(
    value: &Value,
    type_name: &str,
    schema: &CompiledSchema,
) -> Result<Vec<u8>, StoreEncodeError> {
    let type_id =
        schema
            .registry
            .lookup(type_name)
            .ok_or_else(|| StoreEncodeError::TypeNotFound {
                type_name: type_name.to_string(),
            })?;
    let type_def = schema
        .registry
        .get(type_id)
        .ok_or(StoreEncodeError::UnknownTypeId)?;

    let mut writer = BitWriter::new();
    encode_type_def(value, type_def, &schema.registry, &mut writer)?;
    Ok(writer.finish())
}

fn encode_type_def(
    value: &Value,
    type_def: &TypeDef,
    registry: &TypeRegistry,
    w: &mut BitWriter,
) -> Result<(), StoreEncodeError> {
    match type_def {
        TypeDef::Message(msg) => encode_message(value, msg, registry, w),
        TypeDef::Enum(e) => encode_enum(value, e, w),
        TypeDef::Flags(f) => encode_flags(value, f, w),
        TypeDef::Union(u) => encode_union(value, u, registry, w),
        TypeDef::Newtype(nt) => encode_resolved(value, &nt.terminal_type, registry, w),
        TypeDef::Config(cfg) => encode_config(value, cfg, registry, w),
        _ => Err(StoreEncodeError::TypeMismatch {
            expected: "known type kind".to_string(),
            actual: value_type_name(value).to_string(),
        }),
    }
}

fn encode_resolved(
    value: &Value,
    ty: &ResolvedType,
    registry: &TypeRegistry,
    w: &mut BitWriter,
) -> Result<(), StoreEncodeError> {
    match ty {
        ResolvedType::Primitive(p) => encode_primitive(value, *p, w),
        ResolvedType::SubByte(sbt) => encode_sub_byte(value, sbt.bits, sbt.signed, w),
        ResolvedType::Semantic(s) => encode_semantic(value, *s, w),
        ResolvedType::Named(type_id) => {
            let td = registry
                .get(*type_id)
                .ok_or(StoreEncodeError::UnknownTypeId)?;
            encode_type_def(value, td, registry, w)
        }
        ResolvedType::Optional(inner) => encode_optional(value, inner, registry, w),
        ResolvedType::Array(elem) => encode_array(value, elem, registry, w),
        ResolvedType::Map(key, val) => encode_map(value, key, val, registry, w),
        ResolvedType::Result(ok_ty, err_ty) => encode_result(value, ok_ty, err_ty, registry, w),
        _ => Err(StoreEncodeError::TypeMismatch {
            expected: format!("{ty:?}"),
            actual: value_type_name(value).to_string(),
        }),
    }
}

fn encode_primitive(
    value: &Value,
    prim: PrimitiveType,
    w: &mut BitWriter,
) -> Result<(), StoreEncodeError> {
    match (value, prim) {
        (Value::Bool(v), PrimitiveType::Bool) => {
            w.write_bool(*v);
            Ok(())
        }
        (Value::U8(v), PrimitiveType::U8) => {
            w.write_u8(*v);
            Ok(())
        }
        (Value::U16(v), PrimitiveType::U16) => {
            w.write_u16(*v);
            Ok(())
        }
        (Value::U32(v), PrimitiveType::U32) => {
            w.write_u32(*v);
            Ok(())
        }
        (Value::U64(v), PrimitiveType::U64) => {
            w.write_u64(*v);
            Ok(())
        }
        (Value::I8(v), PrimitiveType::I8) => {
            w.write_i8(*v);
            Ok(())
        }
        (Value::I16(v), PrimitiveType::I16) => {
            w.write_i16(*v);
            Ok(())
        }
        (Value::I32(v), PrimitiveType::I32) => {
            w.write_i32(*v);
            Ok(())
        }
        (Value::I64(v), PrimitiveType::I64) => {
            w.write_i64(*v);
            Ok(())
        }
        (Value::F32(v), PrimitiveType::F32) => {
            w.write_f32(*v);
            Ok(())
        }
        (Value::F64(v), PrimitiveType::F64) => {
            w.write_f64(*v);
            Ok(())
        }
        _ => Err(StoreEncodeError::TypeMismatch {
            expected: format!("{prim:?}"),
            actual: value_type_name(value).to_string(),
        }),
    }
}

fn encode_sub_byte(
    value: &Value,
    bits: u8,
    signed: bool,
    w: &mut BitWriter,
) -> Result<(), StoreEncodeError> {
    let mask: u64 = if bits == 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    };

    if signed {
        // iN: two's complement in exactly N bits (spec §3.2).
        // Accept any signed integer Value; mask to the low N bits.
        let raw: u64 = match value {
            Value::I8(v) => (*v as i64 as u64) & mask,
            Value::I16(v) => (*v as i64 as u64) & mask,
            Value::I32(v) => (*v as i64 as u64) & mask,
            Value::I64(v) => (*v as u64) & mask,
            _ => {
                return Err(StoreEncodeError::TypeMismatch {
                    expected: format!("i{bits}"),
                    actual: value_type_name(value).to_string(),
                })
            }
        };
        w.write_bits(raw, bits);
        return Ok(());
    }

    // uN: unsigned N bits.
    match value {
        Value::Bits { value: v, width } => {
            if *width != bits {
                return Err(StoreEncodeError::Overflow {
                    value: v.to_string(),
                    bits,
                });
            }
            if *v > mask {
                return Err(StoreEncodeError::Overflow {
                    value: v.to_string(),
                    bits,
                });
            }
            w.write_bits(*v, bits);
            Ok(())
        }
        Value::U8(v) => {
            w.write_bits(*v as u64, bits);
            Ok(())
        }
        Value::U16(v) => {
            w.write_bits(*v as u64, bits);
            Ok(())
        }
        Value::U32(v) => {
            w.write_bits(*v as u64, bits);
            Ok(())
        }
        Value::U64(v) => {
            w.write_bits(*v, bits);
            Ok(())
        }
        _ => Err(StoreEncodeError::TypeMismatch {
            expected: format!("u{bits}"),
            actual: value_type_name(value).to_string(),
        }),
    }
}

fn encode_semantic(
    value: &Value,
    sem: SemanticType,
    w: &mut BitWriter,
) -> Result<(), StoreEncodeError> {
    match (value, sem) {
        (Value::String(s), SemanticType::String) => {
            w.write_string(s);
            Ok(())
        }
        (Value::Bytes(b), SemanticType::Bytes) => {
            w.write_bytes(b);
            Ok(())
        }
        (Value::Rgb(rgb), SemanticType::Rgb) => {
            w.write_raw_bytes(rgb);
            Ok(())
        }
        (Value::Uuid(uuid), SemanticType::Uuid) => {
            w.write_raw_bytes(uuid);
            Ok(())
        }
        (Value::Timestamp(ts), SemanticType::Timestamp) => {
            w.write_i64(*ts);
            Ok(())
        }
        (Value::Hash(h), SemanticType::Hash) => {
            w.write_raw_bytes(h);
            Ok(())
        }
        _ => Err(StoreEncodeError::TypeMismatch {
            expected: format!("{sem:?}"),
            actual: value_type_name(value).to_string(),
        }),
    }
}

fn encode_optional(
    value: &Value,
    inner_ty: &ResolvedType,
    registry: &TypeRegistry,
    w: &mut BitWriter,
) -> Result<(), StoreEncodeError> {
    match value {
        Value::None => {
            w.write_bool(false);
            Ok(())
        }
        Value::Some(inner) => {
            w.write_bool(true);
            encode_resolved(inner, inner_ty, registry, w)
        }
        other => {
            w.write_bool(true);
            encode_resolved(other, inner_ty, registry, w)
        }
    }
}

fn encode_array(
    value: &Value,
    elem_ty: &ResolvedType,
    registry: &TypeRegistry,
    w: &mut BitWriter,
) -> Result<(), StoreEncodeError> {
    let items = match value {
        Value::Array(items) => items,
        _ => {
            return Err(StoreEncodeError::TypeMismatch {
                expected: "array".to_string(),
                actual: value_type_name(value).to_string(),
            })
        }
    };
    w.write_leb128(items.len() as u64);
    for item in items {
        encode_resolved(item, elem_ty, registry, w)?;
    }
    Ok(())
}

fn encode_map(
    value: &Value,
    key_ty: &ResolvedType,
    val_ty: &ResolvedType,
    registry: &TypeRegistry,
    w: &mut BitWriter,
) -> Result<(), StoreEncodeError> {
    let entries = match value {
        Value::Map(entries) => entries,
        _ => {
            return Err(StoreEncodeError::TypeMismatch {
                expected: "map".to_string(),
                actual: value_type_name(value).to_string(),
            })
        }
    };
    w.write_leb128(entries.len() as u64);
    for (k, v) in entries {
        encode_resolved(k, key_ty, registry, w)?;
        encode_resolved(v, val_ty, registry, w)?;
    }
    Ok(())
}

fn encode_result(
    value: &Value,
    ok_ty: &ResolvedType,
    err_ty: &ResolvedType,
    registry: &TypeRegistry,
    w: &mut BitWriter,
) -> Result<(), StoreEncodeError> {
    match value {
        Value::Ok(inner) => {
            w.write_bool(true);
            encode_resolved(inner, ok_ty, registry, w)
        }
        Value::Err(inner) => {
            w.write_bool(false);
            encode_resolved(inner, err_ty, registry, w)
        }
        _ => Err(StoreEncodeError::TypeMismatch {
            expected: "ok(...) or err(...)".to_string(),
            actual: value_type_name(value).to_string(),
        }),
    }
}

fn encode_message(
    value: &Value,
    msg: &MessageDef,
    registry: &TypeRegistry,
    w: &mut BitWriter,
) -> Result<(), StoreEncodeError> {
    let fields_map = match value {
        Value::Message(fields) => fields,
        _ => {
            return Err(StoreEncodeError::TypeMismatch {
                expected: format!("message `{}`", msg.name),
                actual: value_type_name(value).to_string(),
            })
        }
    };

    let mut sorted_fields: Vec<_> = msg.fields.iter().collect();
    sorted_fields.sort_by_key(|f| f.ordinal);

    for field_def in sorted_fields {
        let field_value = fields_map
            .get(field_def.name.as_str())
            .cloned()
            .unwrap_or_else(|| Value::default_for_type(&field_def.resolved_type, registry));
        encode_resolved(&field_value, &field_def.resolved_type, registry, w)?;
    }
    Ok(())
}

fn encode_enum(
    value: &Value,
    enum_def: &EnumDef,
    w: &mut BitWriter,
) -> Result<(), StoreEncodeError> {
    let variant_name = match value {
        Value::Enum(name) => name,
        _ => {
            return Err(StoreEncodeError::TypeMismatch {
                expected: format!("enum `{}`", enum_def.name),
                actual: value_type_name(value).to_string(),
            })
        }
    };

    let variant = enum_def
        .variants
        .iter()
        .find(|v| v.name.as_str() == variant_name)
        .ok_or_else(|| StoreEncodeError::UnknownVariant {
            type_name: enum_def.name.to_string(),
            variant: variant_name.clone(),
        })?;

    w.write_bits(variant.ordinal as u64, enum_def.wire_bits);
    Ok(())
}

fn encode_flags(
    value: &Value,
    flags_def: &FlagsDef,
    w: &mut BitWriter,
) -> Result<(), StoreEncodeError> {
    let flag_names = match value {
        Value::Flags(names) => names,
        _ => {
            return Err(StoreEncodeError::TypeMismatch {
                expected: format!("flags `{}`", flags_def.name),
                actual: value_type_name(value).to_string(),
            })
        }
    };

    let mut bits: u64 = 0;
    for name in flag_names {
        // Note: FlagsBitDef field is `.bit` (u32), not `.ordinal`
        let bit_def = flags_def
            .bits
            .iter()
            .find(|b| b.name.as_str() == name.as_str())
            .ok_or_else(|| StoreEncodeError::UnknownVariant {
                type_name: flags_def.name.to_string(),
                variant: name.clone(),
            })?;
        bits |= 1u64 << bit_def.bit;
    }

    match flags_def.wire_bytes {
        1 => w.write_u8(bits as u8),
        2 => w.write_u16(bits as u16),
        4 => w.write_u32(bits as u32),
        8 => w.write_u64(bits),
        _ => {
            return Err(StoreEncodeError::TypeMismatch {
                expected: "flags wire_bytes 1/2/4/8".to_string(),
                actual: flags_def.wire_bytes.to_string(),
            })
        }
    }
    Ok(())
}

fn encode_union(
    value: &Value,
    union_def: &UnionDef,
    registry: &TypeRegistry,
    w: &mut BitWriter,
) -> Result<(), StoreEncodeError> {
    let (variant_name, fields_map) = match value {
        Value::Union { variant, fields } => (variant, fields),
        _ => {
            return Err(StoreEncodeError::TypeMismatch {
                expected: format!("union `{}`", union_def.name),
                actual: value_type_name(value).to_string(),
            })
        }
    };

    let variant_def = union_def
        .variants
        .iter()
        .find(|v| v.name.as_str() == variant_name.as_str())
        .ok_or_else(|| StoreEncodeError::UnknownVariant {
            type_name: union_def.name.to_string(),
            variant: variant_name.clone(),
        })?;

    w.write_leb128(variant_def.ordinal as u64);

    let mut inner_writer = BitWriter::new();
    let mut sorted_fields: Vec<_> = variant_def.fields.iter().collect();
    sorted_fields.sort_by_key(|f| f.ordinal);

    for field_def in sorted_fields {
        let field_value = fields_map
            .get(field_def.name.as_str())
            .cloned()
            .unwrap_or_else(|| Value::default_for_type(&field_def.resolved_type, registry));
        encode_resolved(
            &field_value,
            &field_def.resolved_type,
            registry,
            &mut inner_writer,
        )?;
    }

    let payload = inner_writer.finish();
    w.write_bytes(&payload);
    Ok(())
}

fn encode_config(
    value: &Value,
    config: &ConfigDef,
    registry: &TypeRegistry,
    w: &mut BitWriter,
) -> Result<(), StoreEncodeError> {
    let fields_map = match value {
        Value::Message(fields) => fields,
        _ => {
            return Err(StoreEncodeError::TypeMismatch {
                expected: format!("config `{}`", config.name),
                actual: value_type_name(value).to_string(),
            })
        }
    };

    for field_def in &config.fields {
        let field_value = fields_map
            .get(field_def.name.as_str())
            .cloned()
            .unwrap_or_else(|| Value::default_for_type(&field_def.resolved_type, registry));
        encode_resolved(&field_value, &field_def.resolved_type, registry, w)?;
    }
    Ok(())
}

fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Bool(_) => "bool",
        Value::U8(_) => "u8",
        Value::U16(_) => "u16",
        Value::U32(_) => "u32",
        Value::U64(_) => "u64",
        Value::I8(_) => "i8",
        Value::I16(_) => "i16",
        Value::I32(_) => "i32",
        Value::I64(_) => "i64",
        Value::F32(_) => "f32",
        Value::F64(_) => "f64",
        Value::Bits { .. } => "bits",
        Value::String(_) => "string",
        Value::Bytes(_) => "bytes",
        Value::Rgb(_) => "rgb",
        Value::Uuid(_) => "uuid",
        Value::Timestamp(_) => "timestamp",
        Value::Hash(_) => "hash",
        Value::None => "none",
        Value::Some(_) => "some",
        Value::Array(_) => "array",
        Value::Map(_) => "map",
        Value::Ok(_) => "ok",
        Value::Err(_) => "err",
        Value::Message(_) => "message",
        Value::Enum(_) => "enum",
        Value::Flags(_) => "flags",
        Value::Union { .. } => "union",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use vexil_lang::diagnostic::Severity;

    fn compile_schema(source: &str) -> CompiledSchema {
        let result = vexil_lang::compile(source);
        let has_errors = result
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error);
        assert!(!has_errors, "schema errors: {:?}", result.diagnostics);
        result.compiled.expect("schema should compile")
    }

    #[test]
    fn encode_simple_message() {
        let schema = compile_schema(
            r#"
            namespace test.simple
            message Point {
                x @0 : u32
                y @1 : u32
            }
        "#,
        );

        let mut fields = BTreeMap::new();
        fields.insert("x".to_string(), Value::U32(10));
        fields.insert("y".to_string(), Value::U32(20));
        let value = Value::Message(fields);

        let bytes = encode(&value, "Point", &schema).unwrap();
        assert_eq!(bytes.len(), 8);
        assert_eq!(&bytes[0..4], &10u32.to_le_bytes());
        assert_eq!(&bytes[4..8], &20u32.to_le_bytes());
    }

    #[test]
    fn encode_enum() {
        let schema = compile_schema(
            r#"
            namespace test.enums
            enum Direction {
                North @0
                South @1
                East @2
                West @3
            }
            message Move {
                dir @0 : Direction
            }
        "#,
        );

        let mut fields = BTreeMap::new();
        fields.insert("dir".to_string(), Value::Enum("East".to_string()));
        let value = Value::Message(fields);

        let bytes = encode(&value, "Move", &schema).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn encode_optional_present() {
        let schema = compile_schema(
            r#"
            namespace test.opt
            message Named {
                name @0 : optional<string>
            }
        "#,
        );

        let mut fields = BTreeMap::new();
        fields.insert(
            "name".to_string(),
            Value::Some(Box::new(Value::String("hello".to_string()))),
        );
        let value = Value::Message(fields);

        let bytes = encode(&value, "Named", &schema).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn encode_optional_absent() {
        let schema = compile_schema(
            r#"
            namespace test.opt2
            message Named {
                name @0 : optional<string>
            }
        "#,
        );

        let mut fields = BTreeMap::new();
        fields.insert("name".to_string(), Value::None);
        let value = Value::Message(fields);

        let bytes = encode(&value, "Named", &schema).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn encode_unknown_type_errors() {
        let schema = compile_schema(
            r#"
            namespace test.err
            message Foo { x @0 : u32 }
        "#,
        );

        let value = Value::Message(BTreeMap::new());
        let result = encode(&value, "Bar", &schema);
        assert!(result.is_err());
    }
}
