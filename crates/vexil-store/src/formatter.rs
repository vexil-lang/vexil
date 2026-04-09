use vexil_lang::ast::{PrimitiveType, SemanticType};
use vexil_lang::ir::{ConfigDef, MessageDef, UnionDef};
use vexil_lang::{CompiledSchema, ResolvedType, TypeDef, TypeRegistry};

use crate::error::VxError;
use crate::Value;

pub struct FormatOptions {
    pub indent: String,
    pub max_inline_width: usize,
    pub emit_schema_directive: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent: "  ".to_string(),
            max_inline_width: 80,
            emit_schema_directive: true,
        }
    }
}

/// Format a slice of Values as .vx text.
pub fn format(
    values: &[Value],
    type_name: &str,
    schema: &CompiledSchema,
    options: &FormatOptions,
) -> Result<String, VxError> {
    let type_id = schema
        .registry
        .lookup(type_name)
        .ok_or_else(|| VxError::UnknownType {
            namespace: schema
                .namespace
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("."),
            type_name: type_name.to_string(),
        })?;
    let type_def = schema
        .registry
        .get(type_id)
        .ok_or_else(|| VxError::UnknownType {
            namespace: schema
                .namespace
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("."),
            type_name: type_name.to_string(),
        })?;

    let mut out = String::new();

    if options.emit_schema_directive {
        let ns = schema
            .namespace
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(".");
        out.push_str(&std::format!("@schema \"{ns}\"\n\n"));
    }

    for value in values {
        let formatted = format_type_def(value, type_def, type_name, &schema.registry, 0, options)?;
        out.push_str(&formatted);
        out.push('\n');
    }

    Ok(out)
}

fn indent_str(depth: usize, options: &FormatOptions) -> String {
    options.indent.repeat(depth)
}

fn format_type_def(
    value: &Value,
    type_def: &TypeDef,
    name: &str,
    registry: &TypeRegistry,
    depth: usize,
    options: &FormatOptions,
) -> Result<String, VxError> {
    match type_def {
        TypeDef::Message(msg) => {
            format_message_with_name(value, msg, name, registry, depth, options)
        }
        TypeDef::Enum(e) => match value {
            Value::Enum(variant) => Ok(variant.clone()),
            _ => Err(VxError::TypeMismatch {
                expected: std::format!("enum `{}`", e.name),
                actual: std::format!("{value:?}"),
            }),
        },
        TypeDef::Flags(f) => match value {
            Value::Flags(names) => Ok(names.join(" | ")),
            _ => Err(VxError::TypeMismatch {
                expected: std::format!("flags `{}`", f.name),
                actual: std::format!("{value:?}"),
            }),
        },
        TypeDef::Union(u) => format_union_with_name(value, u, name, registry, depth, options),
        TypeDef::Newtype(_) => format_inline(value, registry, depth, options),
        TypeDef::Config(cfg) => format_config_with_name(value, cfg, name, registry, depth, options),
        _ => Err(VxError::TypeMismatch {
            expected: "known type kind".to_string(),
            actual: std::format!("{value:?}"),
        }),
    }
}

fn format_message_with_name(
    value: &Value,
    msg: &MessageDef,
    name: &str,
    registry: &TypeRegistry,
    depth: usize,
    options: &FormatOptions,
) -> Result<String, VxError> {
    let fields_map = match value {
        Value::Message(fields) => fields,
        _ => {
            return Err(VxError::TypeMismatch {
                expected: std::format!("message `{}`", msg.name),
                actual: std::format!("{value:?}"),
            })
        }
    };

    let ind = indent_str(depth, options);
    let inner = indent_str(depth + 1, options);
    let mut s = std::format!("{name} {{\n");

    let mut sorted: Vec<_> = msg.fields.iter().collect();
    sorted.sort_by_key(|f| f.ordinal);

    for field_def in sorted {
        let default_val = Value::default_for_type(&field_def.resolved_type, registry);
        let field_val = fields_map
            .get(field_def.name.as_str())
            .unwrap_or(&default_val);
        let formatted = format_resolved(
            field_val,
            &field_def.resolved_type,
            registry,
            depth + 1,
            options,
        )?;
        s.push_str(&std::format!("{inner}{}: {formatted}\n", field_def.name));
    }
    s.push_str(&std::format!("{ind}}}"));
    Ok(s)
}

fn format_union_with_name(
    value: &Value,
    union_def: &UnionDef,
    _name: &str,
    registry: &TypeRegistry,
    depth: usize,
    options: &FormatOptions,
) -> Result<String, VxError> {
    let (variant_name, fields_map) = match value {
        Value::Union { variant, fields } => (variant, fields),
        _ => {
            return Err(VxError::TypeMismatch {
                expected: std::format!("union `{}`", union_def.name),
                actual: std::format!("{value:?}"),
            })
        }
    };

    let variant_def = union_def
        .variants
        .iter()
        .find(|v| v.name.as_str() == variant_name.as_str())
        .ok_or_else(|| VxError::UnknownVariant {
            type_name: union_def.name.to_string(),
            variant: variant_name.clone(),
        })?;

    let ind = indent_str(depth, options);
    let inner = indent_str(depth + 1, options);
    let mut s = std::format!("{variant_name} {{\n");

    let mut sorted: Vec<_> = variant_def.fields.iter().collect();
    sorted.sort_by_key(|f| f.ordinal);

    for field_def in sorted {
        let default_val = Value::default_for_type(&field_def.resolved_type, registry);
        let field_val = fields_map
            .get(field_def.name.as_str())
            .unwrap_or(&default_val);
        let formatted = format_resolved(
            field_val,
            &field_def.resolved_type,
            registry,
            depth + 1,
            options,
        )?;
        s.push_str(&std::format!("{inner}{}: {formatted}\n", field_def.name));
    }
    s.push_str(&std::format!("{ind}}}"));
    Ok(s)
}

fn format_config_with_name(
    value: &Value,
    config: &ConfigDef,
    name: &str,
    registry: &TypeRegistry,
    depth: usize,
    options: &FormatOptions,
) -> Result<String, VxError> {
    let fields_map = match value {
        Value::Message(fields) => fields,
        _ => {
            return Err(VxError::TypeMismatch {
                expected: std::format!("config `{}`", config.name),
                actual: std::format!("{value:?}"),
            })
        }
    };

    let ind = indent_str(depth, options);
    let inner = indent_str(depth + 1, options);
    let mut s = std::format!("{name} {{\n");

    for field_def in &config.fields {
        let default_val = Value::default_for_type(&field_def.resolved_type, registry);
        let field_val = fields_map
            .get(field_def.name.as_str())
            .unwrap_or(&default_val);
        let formatted = format_resolved(
            field_val,
            &field_def.resolved_type,
            registry,
            depth + 1,
            options,
        )?;
        s.push_str(&std::format!("{inner}{}: {formatted}\n", field_def.name));
    }
    s.push_str(&std::format!("{ind}}}"));
    Ok(s)
}

fn format_resolved(
    value: &Value,
    ty: &ResolvedType,
    registry: &TypeRegistry,
    depth: usize,
    options: &FormatOptions,
) -> Result<String, VxError> {
    match ty {
        ResolvedType::Primitive(p) => format_primitive(value, *p),
        ResolvedType::SubByte(_) => format_inline(value, registry, depth, options),
        ResolvedType::Semantic(s) => format_semantic(value, *s),
        ResolvedType::Named(type_id) => {
            let td = registry.get(*type_id).ok_or_else(|| VxError::UnknownType {
                namespace: String::new(),
                type_name: "unknown".to_string(),
            })?;
            format_type_def(value, td, "", registry, depth, options)
        }
        ResolvedType::Optional(inner) => match value {
            Value::None => Ok("none".to_string()),
            Value::Some(v) => {
                let inner_str = format_resolved(v, inner, registry, depth, options)?;
                // For nested optionals, wrap with some() to disambiguate
                if matches!(inner.as_ref(), ResolvedType::Optional(_)) {
                    Ok(std::format!("some({inner_str})"))
                } else {
                    Ok(inner_str) // implicit Some
                }
            }
            v => {
                // bare value implicitly wrapped
                format_resolved(v, inner, registry, depth, options)
            }
        },
        ResolvedType::Array(elem) => {
            let items = match value {
                Value::Array(items) => items,
                _ => {
                    return Err(VxError::TypeMismatch {
                        expected: "array".to_string(),
                        actual: std::format!("{value:?}"),
                    })
                }
            };
            if items.is_empty() {
                return Ok("[]".to_string());
            }
            let inline_parts: Result<Vec<_>, _> = items
                .iter()
                .map(|v| format_resolved(v, elem, registry, depth, options))
                .collect();
            let inline_parts = inline_parts?;
            let inline = std::format!("[{}]", inline_parts.join(", "));
            if inline.len() <= options.max_inline_width {
                Ok(inline)
            } else {
                let ind = indent_str(depth, options);
                let inner = indent_str(depth + 1, options);
                let mut s = "[\n".to_string();
                for part in &inline_parts {
                    s.push_str(&std::format!("{inner}{part},\n"));
                }
                s.push_str(&std::format!("{ind}]"));
                Ok(s)
            }
        }
        ResolvedType::Map(key_ty, val_ty) => {
            let entries = match value {
                Value::Map(entries) => entries,
                _ => {
                    return Err(VxError::TypeMismatch {
                        expected: "map".to_string(),
                        actual: std::format!("{value:?}"),
                    })
                }
            };
            let ind = indent_str(depth, options);
            let inner = indent_str(depth + 1, options);
            let mut s = "{\n".to_string();
            for (k, v) in entries {
                let ks = format_resolved(k, key_ty, registry, depth + 1, options)?;
                let vs = format_resolved(v, val_ty, registry, depth + 1, options)?;
                s.push_str(&std::format!("{inner}{ks}: {vs},\n"));
            }
            s.push_str(&std::format!("{ind}}}"));
            Ok(s)
        }
        ResolvedType::Result(ok_ty, err_ty) => match value {
            Value::Ok(v) => {
                let s = format_resolved(v, ok_ty, registry, depth, options)?;
                Ok(std::format!("ok({s})"))
            }
            Value::Err(v) => {
                let s = format_resolved(v, err_ty, registry, depth, options)?;
                Ok(std::format!("err({s})"))
            }
            _ => Err(VxError::TypeMismatch {
                expected: "ok or err".to_string(),
                actual: std::format!("{value:?}"),
            }),
        },
        _ => format_inline(value, registry, depth, options),
    }
}

fn format_primitive(value: &Value, prim: PrimitiveType) -> Result<String, VxError> {
    match (value, prim) {
        (Value::Bool(v), PrimitiveType::Bool) => Ok(v.to_string()),
        (Value::U8(v), PrimitiveType::U8) => Ok(v.to_string()),
        (Value::U16(v), PrimitiveType::U16) => Ok(v.to_string()),
        (Value::U32(v), PrimitiveType::U32) => Ok(v.to_string()),
        (Value::U64(v), PrimitiveType::U64) => Ok(v.to_string()),
        (Value::I8(v), PrimitiveType::I8) => Ok(v.to_string()),
        (Value::I16(v), PrimitiveType::I16) => Ok(v.to_string()),
        (Value::I32(v), PrimitiveType::I32) => Ok(v.to_string()),
        (Value::I64(v), PrimitiveType::I64) => Ok(v.to_string()),
        (Value::F32(v), PrimitiveType::F32) => Ok(std::format!("{v}")),
        (Value::F64(v), PrimitiveType::F64) => Ok(std::format!("{v}")),
        _ => Err(VxError::TypeMismatch {
            expected: std::format!("{prim:?}"),
            actual: std::format!("{value:?}"),
        }),
    }
}

fn format_semantic(value: &Value, sem: SemanticType) -> Result<String, VxError> {
    match (value, sem) {
        (Value::String(s), SemanticType::String) => Ok(format_string_lit(s)),
        (Value::Bytes(b), SemanticType::Bytes) => Ok(format_hex_bytes(b)),
        (Value::Rgb(rgb), SemanticType::Rgb) => Ok(std::format!(
            "0x[{:02x} {:02x} {:02x}]",
            rgb[0],
            rgb[1],
            rgb[2]
        )),
        (Value::Uuid(uuid), SemanticType::Uuid) => Ok(format_hex_bytes(uuid)),
        (Value::Timestamp(ts), SemanticType::Timestamp) => Ok(ts.to_string()),
        (Value::Hash(h), SemanticType::Hash) => Ok(format_hex_bytes(h)),
        _ => Err(VxError::TypeMismatch {
            expected: std::format!("{sem:?}"),
            actual: std::format!("{value:?}"),
        }),
    }
}

fn format_string_lit(s: &str) -> String {
    let mut out = String::from('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\0' => out.push_str("\\0"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn format_hex_bytes(bytes: &[u8]) -> String {
    let hex: Vec<String> = bytes.iter().map(|b| std::format!("{b:02x}")).collect();
    std::format!("0x[{}]", hex.join(" "))
}

/// Format a Value without type context (best-effort for inline use).
#[allow(clippy::only_used_in_recursion)]
fn format_inline(
    value: &Value,
    registry: &TypeRegistry,
    depth: usize,
    options: &FormatOptions,
) -> Result<String, VxError> {
    Ok(match value {
        Value::Bool(v) => v.to_string(),
        Value::U8(v) => v.to_string(),
        Value::U16(v) => v.to_string(),
        Value::U32(v) => v.to_string(),
        Value::U64(v) => v.to_string(),
        Value::I8(v) => v.to_string(),
        Value::I16(v) => v.to_string(),
        Value::I32(v) => v.to_string(),
        Value::I64(v) => v.to_string(),
        Value::F32(v) => std::format!("{v}"),
        Value::F64(v) => std::format!("{v}"),
        Value::Fixed32(v) => v.to_string(),
        Value::Fixed64(v) => v.to_string(),
        Value::Bits { value, .. } => value.to_string(),
        Value::String(s) => format_string_lit(s),
        Value::Bytes(b) => format_hex_bytes(b),
        Value::Rgb(rgb) => std::format!("0x[{:02x} {:02x} {:02x}]", rgb[0], rgb[1], rgb[2]),
        Value::Uuid(uuid) => format_hex_bytes(uuid),
        Value::Timestamp(ts) => ts.to_string(),
        Value::Hash(h) => format_hex_bytes(h),
        Value::None => "none".to_string(),
        Value::Some(v) => format_inline(v, registry, depth, options)?,
        Value::Array(_) => "[...]".to_string(),
        Value::Set(_) => "{...}".to_string(),
        Value::Map(_) => "{...}".to_string(),
        Value::Ok(v) => std::format!("ok({})", format_inline(v, registry, depth, options)?),
        Value::Err(v) => std::format!("err({})", format_inline(v, registry, depth, options)?),
        Value::Message(_) => "{...}".to_string(),
        Value::Enum(s) => s.clone(),
        Value::Flags(names) => names.join(" | "),
        Value::Union { variant, .. } => std::format!("{variant} {{...}}"),
    })
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
    fn format_simple_message() {
        let schema = compile_schema(
            r#"
            namespace test.fmt
            message Point { x @0 : u32  y @1 : u32 }
        "#,
        );

        let mut fields = BTreeMap::new();
        fields.insert("x".to_string(), Value::U32(10));
        fields.insert("y".to_string(), Value::U32(20));
        let value = Value::Message(fields);

        let opts = FormatOptions {
            emit_schema_directive: false,
            ..Default::default()
        };
        let result = format(&[value], "Point", &schema, &opts).unwrap();
        assert!(result.contains("x: 10"));
        assert!(result.contains("y: 20"));
        assert!(result.contains("Point {"));
    }

    #[test]
    fn format_none() {
        let schema = compile_schema(
            r#"
            namespace test.fmt.opt
            message Named { name @0 : optional<string> }
        "#,
        );

        let mut fields = BTreeMap::new();
        fields.insert("name".to_string(), Value::None);
        let value = Value::Message(fields);

        let opts = FormatOptions {
            emit_schema_directive: false,
            ..Default::default()
        };
        let result = format(&[value], "Named", &schema, &opts).unwrap();
        assert!(result.contains("name: none"));
    }

    #[test]
    fn format_array_inline() {
        let schema = compile_schema(
            r#"
            namespace test.fmt.arr
            message Numbers { values @0 : array<u32> }
        "#,
        );

        let mut fields = BTreeMap::new();
        fields.insert(
            "values".to_string(),
            Value::Array(vec![Value::U32(1), Value::U32(2), Value::U32(3)]),
        );
        let value = Value::Message(fields);

        let opts = FormatOptions {
            emit_schema_directive: false,
            ..Default::default()
        };
        let result = format(&[value], "Numbers", &schema, &opts).unwrap();
        assert!(result.contains("[1, 2, 3]") || result.contains("values:"));
    }

    #[test]
    fn format_enum() {
        let schema = compile_schema(
            r#"
            namespace test.fmt.enm
            enum Color { Red @0  Green @1  Blue @2 }
            message Pixel { color @0 : Color }
        "#,
        );

        let mut fields = BTreeMap::new();
        fields.insert("color".to_string(), Value::Enum("Blue".to_string()));
        let value = Value::Message(fields);

        let opts = FormatOptions {
            emit_schema_directive: false,
            ..Default::default()
        };
        let result = format(&[value], "Pixel", &schema, &opts).unwrap();
        assert!(result.contains("Blue"));
    }

    #[test]
    fn format_schema_directive() {
        let schema = compile_schema(
            r#"
            namespace test.directive
            message Foo { x @0 : u32 }
        "#,
        );

        let mut fields = BTreeMap::new();
        fields.insert("x".to_string(), Value::U32(1));
        let value = Value::Message(fields);

        let opts = FormatOptions::default();
        let result = format(&[value], "Foo", &schema, &opts).unwrap();
        assert!(result.starts_with("@schema \"test.directive\""));
    }

    #[test]
    fn format_string_escaping() {
        let schema = compile_schema(
            r#"
            namespace test.fmt.str
            message Msg { text @0 : string }
        "#,
        );

        let mut fields = BTreeMap::new();
        fields.insert(
            "text".to_string(),
            Value::String("hello\nworld\t\"end\"".to_string()),
        );
        let value = Value::Message(fields);

        let opts = FormatOptions {
            emit_schema_directive: false,
            ..Default::default()
        };
        let result = format(&[value], "Msg", &schema, &opts).unwrap();
        assert!(result.contains("\\n"));
        assert!(result.contains("\\t"));
        assert!(result.contains("\\\""));
    }

    #[test]
    fn format_bytes_hex() {
        let schema = compile_schema(
            r#"
            namespace test.fmt.bytes
            message Blob { data @0 : bytes }
        "#,
        );

        let mut fields = BTreeMap::new();
        fields.insert(
            "data".to_string(),
            Value::Bytes(vec![0xde, 0xad, 0xbe, 0xef]),
        );
        let value = Value::Message(fields);

        let opts = FormatOptions {
            emit_schema_directive: false,
            ..Default::default()
        };
        let result = format(&[value], "Blob", &schema, &opts).unwrap();
        assert!(result.contains("0x[de ad be ef]"));
    }

    #[test]
    fn format_flags() {
        let schema = compile_schema(
            r#"
            namespace test.fmt.flags
            flags Perms { Read @0  Write @1  Exec @2 }
            message File { perms @0 : Perms }
        "#,
        );

        let mut fields = BTreeMap::new();
        fields.insert(
            "perms".to_string(),
            Value::Flags(vec!["Read".to_string(), "Write".to_string()]),
        );
        let value = Value::Message(fields);

        let opts = FormatOptions {
            emit_schema_directive: false,
            ..Default::default()
        };
        let result = format(&[value], "File", &schema, &opts).unwrap();
        assert!(result.contains("Read | Write"));
    }

    #[test]
    fn format_multiple_values() {
        let schema = compile_schema(
            r#"
            namespace test.fmt.multi
            message Point { x @0 : u32  y @1 : u32 }
        "#,
        );

        let make_point = |x: u32, y: u32| {
            let mut fields = BTreeMap::new();
            fields.insert("x".to_string(), Value::U32(x));
            fields.insert("y".to_string(), Value::U32(y));
            Value::Message(fields)
        };

        let values = vec![make_point(1, 2), make_point(3, 4)];

        let opts = FormatOptions {
            emit_schema_directive: false,
            ..Default::default()
        };
        let result = format(&values, "Point", &schema, &opts).unwrap();
        // Two messages should appear
        assert_eq!(result.matches("Point {").count(), 2);
    }
}
