//! Conversion between `CompiledSchema` IR types and `Value` trees.
//!
//! These functions produce `Value` instances that conform to the `vexil.schema`
//! meta-schema, suitable for encoding to `.vxc` / `.vxcp` binary files via the
//! schema-driven encoder.

use std::collections::BTreeMap;

use vexil_lang::ast::{PrimitiveType, SemanticType};
use vexil_lang::ir::{
    EnumDef, FlagsDef, MessageDef, NewtypeDef, ResolvedAnnotations, TombstoneDef, UnionDef,
};
use vexil_lang::{CompiledSchema, ResolvedType, TypeDef};

use crate::Value;

/// Convert a `CompiledSchema` to a `Value` conforming to `vexil.schema.CompiledSchema`.
pub fn compiled_schema_to_value(schema: &CompiledSchema) -> Value {
    let schema_hash = vexil_lang::canonical::schema_hash(schema);

    let types: Vec<Value> = schema
        .registry
        .iter()
        .map(|(_, td)| type_def_to_value(td))
        .collect();

    let declarations: Vec<Value> = schema
        .declarations
        .iter()
        .map(|id| Value::U32(id.index()))
        .collect();

    let namespace: Vec<Value> = schema
        .namespace
        .iter()
        .map(|s| Value::String(s.to_string()))
        .collect();

    Value::Message(BTreeMap::from([
        ("namespace".to_string(), Value::Array(namespace)),
        ("types".to_string(), Value::Array(types)),
        ("declarations".to_string(), Value::Array(declarations)),
        ("schema_hash".to_string(), Value::Hash(schema_hash)),
        (
            "annotations".to_string(),
            annotations_to_value(&schema.annotations),
        ),
    ]))
}

/// Convert multiple `CompiledSchema`s to a `Value` conforming to `vexil.schema.SchemaStore`.
pub fn schema_store_to_value(schemas: &[&CompiledSchema]) -> Value {
    let entries: Vec<Value> = schemas
        .iter()
        .map(|s| compiled_schema_to_value(s))
        .collect();
    Value::Message(BTreeMap::from([(
        "schemas".to_string(),
        Value::Array(entries),
    )]))
}

fn type_def_to_value(td: &TypeDef) -> Value {
    match td {
        TypeDef::Message(m) => Value::Union {
            variant: "Message".to_string(),
            fields: BTreeMap::from([("def".to_string(), message_def_to_value(m))]),
        },
        TypeDef::Enum(e) => Value::Union {
            variant: "Enum".to_string(),
            fields: BTreeMap::from([("def".to_string(), enum_def_to_value(e))]),
        },
        TypeDef::Flags(f) => Value::Union {
            variant: "Flags".to_string(),
            fields: BTreeMap::from([("def".to_string(), flags_def_to_value(f))]),
        },
        TypeDef::Union(u) => Value::Union {
            variant: "Union".to_string(),
            fields: BTreeMap::from([("def".to_string(), union_def_to_value(u))]),
        },
        TypeDef::Newtype(n) => Value::Union {
            variant: "Newtype".to_string(),
            fields: BTreeMap::from([("def".to_string(), newtype_def_to_value(n))]),
        },
        // Config types are compile-time only and not part of the meta-schema.
        // Skip them by encoding as a zero-field Message placeholder.
        _ => Value::Union {
            variant: "Message".to_string(),
            fields: BTreeMap::from([(
                "def".to_string(),
                Value::Message(BTreeMap::from([
                    ("name".to_string(), Value::String(String::new())),
                    ("fields".to_string(), Value::Array(Vec::new())),
                    ("tombstones".to_string(), Value::Array(Vec::new())),
                    (
                        "annotations".to_string(),
                        annotations_to_value(&ResolvedAnnotations::default()),
                    ),
                ])),
            )]),
        },
    }
}

fn message_def_to_value(m: &MessageDef) -> Value {
    Value::Message(BTreeMap::from([
        ("name".to_string(), Value::String(m.name.to_string())),
        (
            "fields".to_string(),
            Value::Array(m.fields.iter().map(field_def_to_value).collect()),
        ),
        (
            "tombstones".to_string(),
            Value::Array(m.tombstones.iter().map(tombstone_to_value).collect()),
        ),
        (
            "annotations".to_string(),
            annotations_to_value(&m.annotations),
        ),
    ]))
}

fn field_def_to_value(f: &vexil_lang::ir::FieldDef) -> Value {
    Value::Message(BTreeMap::from([
        ("name".to_string(), Value::String(f.name.to_string())),
        ("ordinal".to_string(), Value::U32(f.ordinal)),
        (
            "resolved_type".to_string(),
            resolved_type_to_value(&f.resolved_type),
        ),
        ("encoding".to_string(), field_encoding_to_value(&f.encoding)),
        (
            "annotations".to_string(),
            annotations_to_value(&f.annotations),
        ),
    ]))
}

fn resolved_type_to_value(rt: &ResolvedType) -> Value {
    match rt {
        ResolvedType::Primitive(p) => Value::Union {
            variant: "Primitive".to_string(),
            fields: BTreeMap::from([("kind".to_string(), primitive_type_to_value(p))]),
        },
        ResolvedType::SubByte(sbt) => Value::Union {
            variant: "SubByte".to_string(),
            fields: BTreeMap::from([
                ("bits".to_string(), Value::U8(sbt.bits)),
                ("signed".to_string(), Value::Bool(sbt.signed)),
            ]),
        },
        ResolvedType::Semantic(s) => Value::Union {
            variant: "Semantic".to_string(),
            fields: BTreeMap::from([("kind".to_string(), semantic_type_to_value(s))]),
        },
        ResolvedType::Named(id) => Value::Union {
            variant: "Named".to_string(),
            fields: BTreeMap::from([("type_id".to_string(), Value::U32(id.index()))]),
        },
        ResolvedType::Optional(inner) => Value::Union {
            variant: "Optional".to_string(),
            fields: BTreeMap::from([("inner".to_string(), resolved_type_to_value(inner))]),
        },
        ResolvedType::Array(inner) => Value::Union {
            variant: "Array".to_string(),
            fields: BTreeMap::from([("inner".to_string(), resolved_type_to_value(inner))]),
        },
        ResolvedType::Map(k, v) => Value::Union {
            variant: "Map".to_string(),
            fields: BTreeMap::from([
                ("key".to_string(), resolved_type_to_value(k)),
                ("value".to_string(), resolved_type_to_value(v)),
            ]),
        },
        ResolvedType::Result(ok, err) => Value::Union {
            variant: "Result".to_string(),
            fields: BTreeMap::from([
                ("ok".to_string(), resolved_type_to_value(ok)),
                ("err".to_string(), resolved_type_to_value(err)),
            ]),
        },
        _ => Value::Union {
            variant: "Primitive".to_string(),
            fields: BTreeMap::from([("kind".to_string(), Value::Enum("Void".to_string()))]),
        },
    }
}

fn primitive_type_to_value(p: &PrimitiveType) -> Value {
    let name = match p {
        PrimitiveType::Bool => "Bool",
        PrimitiveType::U8 => "U8",
        PrimitiveType::U16 => "U16",
        PrimitiveType::U32 => "U32",
        PrimitiveType::U64 => "U64",
        PrimitiveType::I8 => "I8",
        PrimitiveType::I16 => "I16",
        PrimitiveType::I32 => "I32",
        PrimitiveType::I64 => "I64",
        PrimitiveType::F32 => "F32",
        PrimitiveType::F64 => "F64",
        PrimitiveType::Fixed32 => "Fixed32",
        PrimitiveType::Fixed64 => "Fixed64",
        PrimitiveType::Void => "Void",
    };
    Value::Enum(name.to_string())
}

fn semantic_type_to_value(s: &SemanticType) -> Value {
    let name = match s {
        SemanticType::String => "String",
        SemanticType::Bytes => "Bytes",
        SemanticType::Rgb => "Rgb",
        SemanticType::Uuid => "Uuid",
        SemanticType::Timestamp => "Timestamp",
        SemanticType::Hash => "Hash",
    };
    Value::Enum(name.to_string())
}

fn field_encoding_to_value(fe: &vexil_lang::ir::FieldEncoding) -> Value {
    let encoding_name = match &fe.encoding {
        vexil_lang::ir::Encoding::Default => "Default",
        vexil_lang::ir::Encoding::Varint => "Varint",
        vexil_lang::ir::Encoding::ZigZag => "ZigZag",
        // Delta wraps an inner encoding; the meta-schema only has Default/Varint/ZigZag.
        // Encode Delta as Varint (its most common inner encoding).
        _ => "Varint",
    };
    let limit_value = match fe.limit {
        Some(n) => Value::Some(Box::new(Value::U64(n))),
        None => Value::None,
    };
    Value::Message(BTreeMap::from([
        (
            "encoding".to_string(),
            Value::Enum(encoding_name.to_string()),
        ),
        ("limit".to_string(), limit_value),
    ]))
}

fn annotations_to_value(a: &ResolvedAnnotations) -> Value {
    let deprecated = match &a.deprecated {
        Some(info) => Value::Some(Box::new(Value::String(info.reason.to_string()))),
        None => Value::None,
    };
    let since = match &a.since {
        Some(s) => Value::Some(Box::new(Value::String(s.to_string()))),
        None => Value::None,
    };
    let doc: Vec<Value> = a.doc.iter().map(|s| Value::String(s.to_string())).collect();
    let revision = match a.revision {
        Some(r) => Value::Some(Box::new(Value::U64(r))),
        None => Value::None,
    };
    let version = match &a.version {
        Some(v) => Value::Some(Box::new(Value::String(v.to_string()))),
        None => Value::None,
    };
    Value::Message(BTreeMap::from([
        ("deprecated".to_string(), deprecated),
        ("since".to_string(), since),
        ("doc".to_string(), Value::Array(doc)),
        ("revision".to_string(), revision),
        ("non_exhaustive".to_string(), Value::Bool(a.non_exhaustive)),
        ("version".to_string(), version),
    ]))
}

fn tombstone_to_value(t: &TombstoneDef) -> Value {
    let since = match &t.since {
        Some(s) => Value::Some(Box::new(Value::String(s.to_string()))),
        None => Value::None,
    };
    Value::Message(BTreeMap::from([
        ("ordinal".to_string(), Value::U32(t.ordinal)),
        ("reason".to_string(), Value::String(t.reason.to_string())),
        ("since".to_string(), since),
    ]))
}

fn enum_def_to_value(e: &EnumDef) -> Value {
    let variants: Vec<Value> = e
        .variants
        .iter()
        .map(|v| {
            Value::Message(BTreeMap::from([
                ("name".to_string(), Value::String(v.name.to_string())),
                ("ordinal".to_string(), Value::U32(v.ordinal)),
                (
                    "annotations".to_string(),
                    annotations_to_value(&v.annotations),
                ),
            ]))
        })
        .collect();
    Value::Message(BTreeMap::from([
        ("name".to_string(), Value::String(e.name.to_string())),
        ("wire_bits".to_string(), Value::U8(e.wire_bits)),
        ("variants".to_string(), Value::Array(variants)),
        (
            "tombstones".to_string(),
            Value::Array(e.tombstones.iter().map(tombstone_to_value).collect()),
        ),
        (
            "annotations".to_string(),
            annotations_to_value(&e.annotations),
        ),
    ]))
}

fn flags_def_to_value(f: &FlagsDef) -> Value {
    let bits: Vec<Value> = f
        .bits
        .iter()
        .map(|b| {
            Value::Message(BTreeMap::from([
                ("name".to_string(), Value::String(b.name.to_string())),
                ("ordinal".to_string(), Value::U32(b.bit)),
                (
                    "annotations".to_string(),
                    annotations_to_value(&b.annotations),
                ),
            ]))
        })
        .collect();
    Value::Message(BTreeMap::from([
        ("name".to_string(), Value::String(f.name.to_string())),
        ("wire_bytes".to_string(), Value::U8(f.wire_bytes)),
        ("bits".to_string(), Value::Array(bits)),
        (
            "tombstones".to_string(),
            Value::Array(f.tombstones.iter().map(tombstone_to_value).collect()),
        ),
        (
            "annotations".to_string(),
            annotations_to_value(&f.annotations),
        ),
    ]))
}

fn union_def_to_value(u: &UnionDef) -> Value {
    let variants: Vec<Value> = u
        .variants
        .iter()
        .map(|v| {
            Value::Message(BTreeMap::from([
                ("name".to_string(), Value::String(v.name.to_string())),
                ("ordinal".to_string(), Value::U32(v.ordinal)),
                (
                    "fields".to_string(),
                    Value::Array(v.fields.iter().map(field_def_to_value).collect()),
                ),
                (
                    "tombstones".to_string(),
                    Value::Array(v.tombstones.iter().map(tombstone_to_value).collect()),
                ),
                (
                    "annotations".to_string(),
                    annotations_to_value(&v.annotations),
                ),
            ]))
        })
        .collect();
    Value::Message(BTreeMap::from([
        ("name".to_string(), Value::String(u.name.to_string())),
        ("variants".to_string(), Value::Array(variants)),
        (
            "tombstones".to_string(),
            Value::Array(u.tombstones.iter().map(tombstone_to_value).collect()),
        ),
        (
            "annotations".to_string(),
            annotations_to_value(&u.annotations),
        ),
    ]))
}

fn newtype_def_to_value(n: &NewtypeDef) -> Value {
    Value::Message(BTreeMap::from([
        ("name".to_string(), Value::String(n.name.to_string())),
        (
            "inner_type".to_string(),
            resolved_type_to_value(&n.inner_type),
        ),
        (
            "terminal_type".to_string(),
            resolved_type_to_value(&n.terminal_type),
        ),
        (
            "annotations".to_string(),
            annotations_to_value(&n.annotations),
        ),
    ]))
}
