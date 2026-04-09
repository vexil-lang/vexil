use vexil_lang::ast::{PrimitiveType, SemanticType};
use vexil_lang::ir::{ConfigDef, EnumDef, FlagsDef, MessageDef, UnionDef};
use vexil_lang::{CompiledSchema, ResolvedType, TypeDef, TypeRegistry};

use crate::error::VxError;
use crate::Value;

/// Validate a `Value` against the named type in the given schema.
///
/// Returns `Ok(())` if the value is valid, or `Err(errors)` containing all
/// validation errors found. Missing fields are NOT an error (they use defaults).
pub fn validate(
    value: &Value,
    type_name: &str,
    schema: &CompiledSchema,
) -> Result<(), Vec<VxError>> {
    let type_id = schema.registry.lookup(type_name).ok_or_else(|| {
        vec![VxError::UnknownType {
            namespace: schema
                .namespace
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("."),
            type_name: type_name.to_string(),
        }]
    })?;
    let type_def = schema.registry.get(type_id).ok_or_else(|| {
        vec![VxError::UnknownType {
            namespace: String::new(),
            type_name: type_name.to_string(),
        }]
    })?;

    let mut errors = Vec::new();
    validate_type_def(value, type_def, &schema.registry, &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_type_def(
    value: &Value,
    type_def: &TypeDef,
    registry: &TypeRegistry,
    errors: &mut Vec<VxError>,
) {
    match type_def {
        TypeDef::Message(msg) => validate_message(value, msg, registry, errors),
        TypeDef::Enum(e) => validate_enum(value, e, errors),
        TypeDef::Flags(f) => validate_flags(value, f, errors),
        TypeDef::Union(u) => validate_union(value, u, registry, errors),
        TypeDef::Newtype(nt) => validate_resolved(value, &nt.terminal_type, registry, errors),
        TypeDef::Config(cfg) => validate_config(value, cfg, registry, errors),
        _ => {} // Non-exhaustive: ignore unknown type kinds
    }
}

fn validate_config(
    value: &Value,
    cfg: &ConfigDef,
    registry: &TypeRegistry,
    errors: &mut Vec<VxError>,
) {
    match value {
        Value::Message(fields) => {
            for field_name in fields.keys() {
                if !cfg.fields.iter().any(|f| f.name.as_str() == field_name) {
                    errors.push(VxError::UnknownField {
                        type_name: cfg.name.to_string(),
                        field: field_name.clone(),
                    });
                }
            }
            for field_def in &cfg.fields {
                if let Some(v) = fields.get(field_def.name.as_str()) {
                    validate_resolved(v, &field_def.resolved_type, registry, errors);
                }
                // Missing fields: not an error
            }
        }
        _ => errors.push(VxError::TypeMismatch {
            expected: format!("config `{}`", cfg.name),
            actual: value_type_name(value).to_string(),
        }),
    }
}

fn validate_resolved(
    value: &Value,
    ty: &ResolvedType,
    registry: &TypeRegistry,
    errors: &mut Vec<VxError>,
) {
    match ty {
        ResolvedType::Primitive(p) => validate_primitive(value, *p, errors),
        ResolvedType::SubByte(sbt) => {
            if let Value::Bits { value: v, width } = value {
                if *width != sbt.bits {
                    errors.push(VxError::TypeMismatch {
                        expected: format!("bits({})", sbt.bits),
                        actual: format!("bits({width})"),
                    });
                }
                let max = if sbt.bits == 64 {
                    u64::MAX
                } else {
                    (1u64 << sbt.bits) - 1
                };
                if *v > max {
                    errors.push(VxError::Overflow {
                        value: v.to_string(),
                        ty: format!("u{}", sbt.bits),
                    });
                }
            }
            // Non-Bits values for sub-byte: allow (same leniency as encoder)
        }
        ResolvedType::Semantic(s) => validate_semantic(value, *s, errors),
        ResolvedType::Named(type_id) => {
            if let Some(td) = registry.get(*type_id) {
                validate_type_def(value, td, registry, errors);
            }
        }
        ResolvedType::Optional(inner) => match value {
            Value::None => {}
            Value::Some(v) => validate_resolved(v, inner, registry, errors),
            v => validate_resolved(v, inner, registry, errors), // implicit Some
        },
        ResolvedType::Array(elem) => match value {
            Value::Array(items) => {
                for item in items {
                    validate_resolved(item, elem, registry, errors);
                }
            }
            _ => errors.push(VxError::TypeMismatch {
                expected: "array".to_string(),
                actual: value_type_name(value).to_string(),
            }),
        },
        ResolvedType::Map(key_ty, val_ty) => match value {
            Value::Map(entries) => {
                for (k, v) in entries {
                    validate_resolved(k, key_ty, registry, errors);
                    validate_resolved(v, val_ty, registry, errors);
                }
            }
            _ => errors.push(VxError::TypeMismatch {
                expected: "map".to_string(),
                actual: value_type_name(value).to_string(),
            }),
        },
        ResolvedType::Result(ok_ty, err_ty) => match value {
            Value::Ok(v) => validate_resolved(v, ok_ty, registry, errors),
            Value::Err(v) => validate_resolved(v, err_ty, registry, errors),
            _ => errors.push(VxError::TypeMismatch {
                expected: "ok or err".to_string(),
                actual: value_type_name(value).to_string(),
            }),
        },
        _ => {} // Non-exhaustive: ignore
    }
}

fn validate_primitive(value: &Value, prim: PrimitiveType, errors: &mut Vec<VxError>) {
    let ok = matches!(
        (value, prim),
        (Value::Bool(_), PrimitiveType::Bool)
            | (Value::U8(_), PrimitiveType::U8)
            | (Value::U16(_), PrimitiveType::U16)
            | (Value::U32(_), PrimitiveType::U32)
            | (Value::U64(_), PrimitiveType::U64)
            | (Value::I8(_), PrimitiveType::I8)
            | (Value::I16(_), PrimitiveType::I16)
            | (Value::I32(_), PrimitiveType::I32)
            | (Value::I64(_), PrimitiveType::I64)
            | (Value::F32(_), PrimitiveType::F32)
            | (Value::F64(_), PrimitiveType::F64)
            | (Value::Fixed32(_), PrimitiveType::Fixed32)
            | (Value::Fixed64(_), PrimitiveType::Fixed64)
    );
    if !ok {
        errors.push(VxError::TypeMismatch {
            expected: format!("{prim:?}"),
            actual: value_type_name(value).to_string(),
        });
    }
}

fn validate_semantic(value: &Value, sem: SemanticType, errors: &mut Vec<VxError>) {
    let ok = matches!(
        (value, sem),
        (Value::String(_), SemanticType::String)
            | (Value::Bytes(_), SemanticType::Bytes)
            | (Value::Rgb(_), SemanticType::Rgb)
            | (Value::Uuid(_), SemanticType::Uuid)
            | (Value::Timestamp(_), SemanticType::Timestamp)
            | (Value::Hash(_), SemanticType::Hash)
    );
    if !ok {
        errors.push(VxError::TypeMismatch {
            expected: format!("{sem:?}"),
            actual: value_type_name(value).to_string(),
        });
    }
}

fn validate_message(
    value: &Value,
    msg: &MessageDef,
    registry: &TypeRegistry,
    errors: &mut Vec<VxError>,
) {
    match value {
        Value::Message(fields) => {
            // Check for extra/unknown fields
            for field_name in fields.keys() {
                if !msg.fields.iter().any(|f| f.name.as_str() == field_name) {
                    errors.push(VxError::UnknownField {
                        type_name: msg.name.to_string(),
                        field: field_name.clone(),
                    });
                }
            }
            // Validate present fields
            for field_def in &msg.fields {
                if let Some(v) = fields.get(field_def.name.as_str()) {
                    validate_resolved(v, &field_def.resolved_type, registry, errors);
                }
                // Missing fields: not an error
            }
        }
        _ => errors.push(VxError::TypeMismatch {
            expected: format!("message `{}`", msg.name),
            actual: value_type_name(value).to_string(),
        }),
    }
}

fn validate_enum(value: &Value, enum_def: &EnumDef, errors: &mut Vec<VxError>) {
    match value {
        Value::Enum(variant) => {
            if !enum_def.variants.iter().any(|v| v.name.as_str() == variant) {
                errors.push(VxError::UnknownVariant {
                    type_name: enum_def.name.to_string(),
                    variant: variant.clone(),
                });
            }
        }
        _ => errors.push(VxError::TypeMismatch {
            expected: format!("enum `{}`", enum_def.name),
            actual: value_type_name(value).to_string(),
        }),
    }
}

fn validate_flags(value: &Value, flags_def: &FlagsDef, errors: &mut Vec<VxError>) {
    match value {
        Value::Flags(names) => {
            for name in names {
                if !flags_def.bits.iter().any(|b| b.name.as_str() == name) {
                    errors.push(VxError::UnknownVariant {
                        type_name: flags_def.name.to_string(),
                        variant: name.clone(),
                    });
                }
            }
        }
        _ => errors.push(VxError::TypeMismatch {
            expected: format!("flags `{}`", flags_def.name),
            actual: value_type_name(value).to_string(),
        }),
    }
}

fn validate_union(
    value: &Value,
    union_def: &UnionDef,
    registry: &TypeRegistry,
    errors: &mut Vec<VxError>,
) {
    match value {
        Value::Union { variant, fields } => {
            let variant_def = union_def
                .variants
                .iter()
                .find(|v| v.name.as_str() == variant.as_str());
            match variant_def {
                None => errors.push(VxError::UnknownVariant {
                    type_name: union_def.name.to_string(),
                    variant: variant.clone(),
                }),
                Some(vd) => {
                    // Check extra fields
                    for field_name in fields.keys() {
                        if !vd.fields.iter().any(|f| f.name.as_str() == field_name) {
                            errors.push(VxError::UnknownField {
                                type_name: format!("{}::{}", union_def.name, variant),
                                field: field_name.clone(),
                            });
                        }
                    }
                    // Validate present fields
                    for field_def in &vd.fields {
                        if let Some(v) = fields.get(field_def.name.as_str()) {
                            validate_resolved(v, &field_def.resolved_type, registry, errors);
                        }
                    }
                }
            }
        }
        _ => errors.push(VxError::TypeMismatch {
            expected: format!("union `{}`", union_def.name),
            actual: value_type_name(value).to_string(),
        }),
    }
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
        Value::Fixed32(_) => "fixed32",
        Value::Fixed64(_) => "fixed64",
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
        Value::Set(_) => "set",
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

    fn compile_schema(source: &str) -> vexil_lang::CompiledSchema {
        let result = vexil_lang::compile(source);
        let has_errors = result
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error);
        assert!(!has_errors, "schema errors: {:?}", result.diagnostics);
        result.compiled.expect("schema should compile")
    }

    #[test]
    fn valid_message() {
        let schema = compile_schema(
            r#"
            namespace test.validate
            message Point { x @0 : u32  y @1 : u32 }
        "#,
        );
        let mut fields = BTreeMap::new();
        fields.insert("x".to_string(), Value::U32(10));
        fields.insert("y".to_string(), Value::U32(20));
        let result = validate(&Value::Message(fields), "Point", &schema);
        assert!(result.is_ok());
    }

    #[test]
    fn extra_field_rejected() {
        let schema = compile_schema(
            r#"
            namespace test.validate.extra
            message Point { x @0 : u32 }
        "#,
        );
        let mut fields = BTreeMap::new();
        fields.insert("x".to_string(), Value::U32(1));
        fields.insert("unknown".to_string(), Value::U32(2));
        let result = validate(&Value::Message(fields), "Point", &schema);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| matches!(e, VxError::UnknownField { field, .. } if field == "unknown")));
    }

    #[test]
    fn type_mismatch_detected() {
        let schema = compile_schema(
            r#"
            namespace test.validate.mismatch
            message Item { count @0 : u32 }
        "#,
        );
        let mut fields = BTreeMap::new();
        fields.insert("count".to_string(), Value::String("oops".to_string()));
        let result = validate(&Value::Message(fields), "Item", &schema);
        assert!(result.is_err());
    }

    #[test]
    fn missing_fields_ok() {
        let schema = compile_schema(
            r#"
            namespace test.validate.missing
            message Point { x @0 : u32  y @1 : u32 }
        "#,
        );
        // Only x present, y missing — should be OK
        let mut fields = BTreeMap::new();
        fields.insert("x".to_string(), Value::U32(5));
        let result = validate(&Value::Message(fields), "Point", &schema);
        assert!(result.is_ok());
    }

    #[test]
    fn unknown_enum_variant() {
        let schema = compile_schema(
            r#"
            namespace test.validate.enum
            enum Color { Red @0  Green @1  Blue @2 }
            message Pixel { color @0 : Color }
        "#,
        );
        let mut fields = BTreeMap::new();
        fields.insert("color".to_string(), Value::Enum("Purple".to_string()));
        let result = validate(&Value::Message(fields), "Pixel", &schema);
        assert!(result.is_err());
    }
}
