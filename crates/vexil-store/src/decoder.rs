use std::collections::BTreeMap;

use vexil_lang::ast::{PrimitiveType, SemanticType};
use vexil_lang::ir::{ConfigDef, EnumDef, FlagsDef, MessageDef, UnionDef};
use vexil_lang::{CompiledSchema, ResolvedType, TypeDef, TypeRegistry};
use vexil_runtime::BitReader;

use crate::error::StoreDecodeError;
use crate::Value;

/// Decode bitpack bytes into a `Value` using the given schema and type name.
pub fn decode(
    bytes: &[u8],
    type_name: &str,
    schema: &CompiledSchema,
) -> Result<Value, StoreDecodeError> {
    let type_id =
        schema
            .registry
            .lookup(type_name)
            .ok_or_else(|| StoreDecodeError::TypeNotFound {
                type_name: type_name.to_string(),
            })?;
    let type_def = schema
        .registry
        .get(type_id)
        .ok_or(StoreDecodeError::UnknownTypeId)?;

    let mut reader = BitReader::new(bytes);
    decode_type_def(type_def, &schema.registry, &mut reader)
}

fn decode_type_def(
    type_def: &TypeDef,
    registry: &TypeRegistry,
    r: &mut BitReader<'_>,
) -> Result<Value, StoreDecodeError> {
    match type_def {
        TypeDef::Message(msg) => decode_message(msg, registry, r),
        TypeDef::Enum(e) => decode_enum(e, r),
        TypeDef::Flags(f) => decode_flags(f, r),
        TypeDef::Union(u) => decode_union(u, registry, r),
        TypeDef::Newtype(nt) => decode_resolved(&nt.terminal_type, registry, r),
        TypeDef::Config(cfg) => decode_config(cfg, registry, r),
        _ => Err(StoreDecodeError::TypeMismatch {
            context: "type_def".to_string(),
            expected: "known type kind".to_string(),
        }),
    }
}

fn decode_resolved(
    ty: &ResolvedType,
    registry: &TypeRegistry,
    r: &mut BitReader<'_>,
) -> Result<Value, StoreDecodeError> {
    match ty {
        ResolvedType::Primitive(p) => decode_primitive(*p, r),
        ResolvedType::SubByte(sbt) => {
            let value = r.read_bits(sbt.bits)?;
            Ok(Value::Bits {
                value,
                width: sbt.bits,
            })
        }
        ResolvedType::Semantic(s) => decode_semantic(*s, r),
        ResolvedType::Named(type_id) => {
            let td = registry
                .get(*type_id)
                .ok_or(StoreDecodeError::UnknownTypeId)?;
            decode_type_def(td, registry, r)
        }
        ResolvedType::Optional(inner) => {
            if r.read_bool()? {
                Ok(Value::Some(Box::new(decode_resolved(inner, registry, r)?)))
            } else {
                Ok(Value::None)
            }
        }
        ResolvedType::Array(elem) => {
            let count = r.read_leb128(4)? as usize;
            let mut items = Vec::with_capacity(count.min(1024));
            for _ in 0..count {
                items.push(decode_resolved(elem, registry, r)?);
            }
            Ok(Value::Array(items))
        }
        ResolvedType::Map(key, val) => {
            let count = r.read_leb128(4)? as usize;
            let mut entries = Vec::with_capacity(count.min(1024));
            for _ in 0..count {
                let k = decode_resolved(key, registry, r)?;
                let v = decode_resolved(val, registry, r)?;
                entries.push((k, v));
            }
            Ok(Value::Map(entries))
        }
        ResolvedType::Result(ok_ty, err_ty) => {
            if r.read_bool()? {
                Ok(Value::Ok(Box::new(decode_resolved(ok_ty, registry, r)?)))
            } else {
                Ok(Value::Err(Box::new(decode_resolved(err_ty, registry, r)?)))
            }
        }
        _ => Err(StoreDecodeError::TypeMismatch {
            context: "resolved".to_string(),
            expected: format!("{ty:?}"),
        }),
    }
}

fn decode_primitive(prim: PrimitiveType, r: &mut BitReader<'_>) -> Result<Value, StoreDecodeError> {
    Ok(match prim {
        PrimitiveType::Bool => Value::Bool(r.read_bool()?),
        PrimitiveType::U8 => Value::U8(r.read_u8()?),
        PrimitiveType::U16 => Value::U16(r.read_u16()?),
        PrimitiveType::U32 => Value::U32(r.read_u32()?),
        PrimitiveType::U64 => Value::U64(r.read_u64()?),
        PrimitiveType::I8 => Value::I8(r.read_i8()?),
        PrimitiveType::I16 => Value::I16(r.read_i16()?),
        PrimitiveType::I32 => Value::I32(r.read_i32()?),
        PrimitiveType::I64 => Value::I64(r.read_i64()?),
        PrimitiveType::F32 => Value::F32(r.read_f32()?),
        PrimitiveType::F64 => Value::F64(r.read_f64()?),
        PrimitiveType::Void => Value::Bool(false),
    })
}

fn decode_semantic(sem: SemanticType, r: &mut BitReader<'_>) -> Result<Value, StoreDecodeError> {
    Ok(match sem {
        SemanticType::String => Value::String(r.read_string()?),
        SemanticType::Bytes => Value::Bytes(r.read_bytes()?),
        SemanticType::Rgb => {
            let bytes = r.read_raw_bytes(3)?;
            Value::Rgb([bytes[0], bytes[1], bytes[2]])
        }
        SemanticType::Uuid => {
            let bytes = r.read_raw_bytes(16)?;
            let mut arr = [0u8; 16];
            arr.copy_from_slice(&bytes);
            Value::Uuid(arr)
        }
        SemanticType::Timestamp => Value::Timestamp(r.read_i64()?),
        SemanticType::Hash => {
            let bytes = r.read_raw_bytes(32)?;
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            Value::Hash(arr)
        }
    })
}

fn decode_message(
    msg: &MessageDef,
    registry: &TypeRegistry,
    r: &mut BitReader<'_>,
) -> Result<Value, StoreDecodeError> {
    let mut fields = BTreeMap::new();
    let mut sorted: Vec<_> = msg.fields.iter().collect();
    sorted.sort_by_key(|f| f.ordinal);

    for field_def in sorted {
        let value = decode_resolved(&field_def.resolved_type, registry, r)?;
        fields.insert(field_def.name.to_string(), value);
    }
    Ok(Value::Message(fields))
}

fn decode_enum(enum_def: &EnumDef, r: &mut BitReader<'_>) -> Result<Value, StoreDecodeError> {
    let ordinal = r.read_bits(enum_def.wire_bits)?;
    let variant = enum_def
        .variants
        .iter()
        .find(|v| v.ordinal as u64 == ordinal)
        .ok_or_else(|| StoreDecodeError::UnknownVariant {
            type_name: enum_def.name.to_string(),
            ordinal,
        })?;
    Ok(Value::Enum(variant.name.to_string()))
}

fn decode_flags(flags_def: &FlagsDef, r: &mut BitReader<'_>) -> Result<Value, StoreDecodeError> {
    let raw_bits: u64 = match flags_def.wire_bytes {
        1 => r.read_u8()? as u64,
        2 => r.read_u16()? as u64,
        4 => r.read_u32()? as u64,
        8 => r.read_u64()?,
        _ => {
            return Err(StoreDecodeError::TypeMismatch {
                context: "flags".to_string(),
                expected: "wire_bytes 1/2/4/8".to_string(),
            })
        }
    };

    let mut names = Vec::new();
    for bit_def in &flags_def.bits {
        // Note: field is .bit (u32), not .ordinal
        if raw_bits & (1u64 << bit_def.bit) != 0 {
            names.push(bit_def.name.to_string());
        }
    }
    Ok(Value::Flags(names))
}

fn decode_union(
    union_def: &UnionDef,
    registry: &TypeRegistry,
    r: &mut BitReader<'_>,
) -> Result<Value, StoreDecodeError> {
    let discriminant = r.read_leb128(4)?;

    let variant_def = union_def
        .variants
        .iter()
        .find(|v| v.ordinal as u64 == discriminant)
        .ok_or_else(|| StoreDecodeError::UnknownUnionDiscriminant {
            type_name: union_def.name.to_string(),
            discriminant,
        })?;

    // Payload is length-prefixed bytes
    let payload = r.read_bytes()?;
    let mut inner_reader = BitReader::new(&payload);

    let mut fields = BTreeMap::new();
    let mut sorted: Vec<_> = variant_def.fields.iter().collect();
    sorted.sort_by_key(|f| f.ordinal);

    for field_def in sorted {
        let value = decode_resolved(&field_def.resolved_type, registry, &mut inner_reader)?;
        fields.insert(field_def.name.to_string(), value);
    }

    Ok(Value::Union {
        variant: variant_def.name.to_string(),
        fields,
    })
}

fn decode_config(
    config: &ConfigDef,
    registry: &TypeRegistry,
    r: &mut BitReader<'_>,
) -> Result<Value, StoreDecodeError> {
    let mut fields = BTreeMap::new();
    for field_def in &config.fields {
        let value = decode_resolved(&field_def.resolved_type, registry, r)?;
        fields.insert(field_def.name.to_string(), value);
    }
    Ok(Value::Message(fields))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use vexil_lang::diagnostic::Severity;

    use crate::encoder::encode;

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
    fn decode_simple_message_roundtrip() {
        let schema = compile_schema(
            r#"
            namespace test.decode
            message Point {
                x @0 : u32
                y @1 : u32
            }
        "#,
        );

        let mut fields = BTreeMap::new();
        fields.insert("x".to_string(), Value::U32(42));
        fields.insert("y".to_string(), Value::U32(99));
        let original = Value::Message(fields);

        let bytes = encode(&original, "Point", &schema).unwrap();
        let decoded = decode(&bytes, "Point", &schema).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn decode_enum_roundtrip() {
        let schema = compile_schema(
            r#"
            namespace test.decode.enum
            enum Color { Red @0 Green @1 Blue @2 }
            message Pixel { color @0 : Color }
        "#,
        );

        let mut fields = BTreeMap::new();
        fields.insert("color".to_string(), Value::Enum("Green".to_string()));
        let original = Value::Message(fields);

        let bytes = encode(&original, "Pixel", &schema).unwrap();
        let decoded = decode(&bytes, "Pixel", &schema).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn decode_optional_roundtrip() {
        let schema = compile_schema(
            r#"
            namespace test.decode.opt
            message Named { name @0 : optional<string> }
        "#,
        );

        // Some case
        let mut fields = BTreeMap::new();
        fields.insert(
            "name".to_string(),
            Value::Some(Box::new(Value::String("hello".to_string()))),
        );
        let original = Value::Message(fields);
        let bytes = encode(&original, "Named", &schema).unwrap();
        let decoded = decode(&bytes, "Named", &schema).unwrap();
        assert_eq!(original, decoded);

        // None case
        let mut fields = BTreeMap::new();
        fields.insert("name".to_string(), Value::None);
        let original = Value::Message(fields);
        let bytes = encode(&original, "Named", &schema).unwrap();
        let decoded = decode(&bytes, "Named", &schema).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn decode_array_roundtrip() {
        let schema = compile_schema(
            r#"
            namespace test.decode.arr
            message Numbers { values @0 : array<u32> }
        "#,
        );

        let mut fields = BTreeMap::new();
        fields.insert(
            "values".to_string(),
            Value::Array(vec![Value::U32(1), Value::U32(2), Value::U32(3)]),
        );
        let original = Value::Message(fields);

        let bytes = encode(&original, "Numbers", &schema).unwrap();
        let decoded = decode(&bytes, "Numbers", &schema).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn decode_unknown_type_errors() {
        let schema = compile_schema(
            r#"
            namespace test.decode.err
            message Foo { x @0 : u32 }
        "#,
        );

        let result = decode(&[], "Bar", &schema);
        assert!(result.is_err());
    }
}
