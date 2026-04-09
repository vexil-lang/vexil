use std::collections::HashSet;
use vexil_lang::ast::{PrimitiveType, SemanticType, SubByteType};
use vexil_lang::ir::{ResolvedType, TypeDef, TypeId, TypeRegistry};

/// Convert a ResolvedType to its Rust type string.
/// `needs_box` contains `(type_id, field_index)` pairs that need Box wrapping.
/// `current_context` is `Some((type_id, field_index))` when we're in a boxed context.
pub fn rust_type(
    ty: &ResolvedType,
    registry: &TypeRegistry,
    needs_box: &HashSet<(TypeId, usize)>,
    context: Option<(TypeId, usize)>,
) -> String {
    match ty {
        ResolvedType::Primitive(p) => primitive_type(p).to_string(),
        ResolvedType::SubByte(s) => sub_byte_type(s).to_string(),
        ResolvedType::Semantic(s) => semantic_type(s).to_string(),
        ResolvedType::Named(id) => {
            let name = match registry.get(*id) {
                Some(def) => type_def_name(def),
                None => "UnresolvedType".to_string(),
            };
            if context.is_some_and(|ctx| needs_box.contains(&ctx)) {
                format!("Box<{name}>")
            } else {
                name
            }
        }
        ResolvedType::Optional(inner) => {
            let inner_str = rust_type(inner, registry, needs_box, context);
            format!("Option<{inner_str}>")
        }
        ResolvedType::Array(inner) => {
            let inner_str = rust_type(inner, registry, needs_box, None);
            format!("Vec<{inner_str}>")
        }
        ResolvedType::FixedArray(inner, size) => {
            let inner_str = rust_type(inner, registry, needs_box, None);
            format!("[{inner_str}; {size}]")
        }
        ResolvedType::Set(inner) => {
            let inner_str = rust_type(inner, registry, needs_box, None);
            format!("std::collections::BTreeSet<{inner_str}>")
        }
        ResolvedType::Map(k, v) => {
            let k_str = rust_type(k, registry, needs_box, None);
            let v_str = rust_type(v, registry, needs_box, None);
            format!("BTreeMap<{k_str}, {v_str}>")
        }
        ResolvedType::Result(ok, err) => {
            let ok_str = rust_type(ok, registry, needs_box, context);
            let err_str = rust_type(err, registry, needs_box, context);
            format!("Result<{ok_str}, {err_str}>")
        }
        ResolvedType::BitsInline(names) => {
            let bits = names.len() as u8;
            containing_int_type(bits).to_string()
        }
        ResolvedType::Vec2(inner) => {
            let inner_str = rust_type(inner, registry, needs_box, None);
            format!("[{inner_str}; 2]")
        }
        ResolvedType::Vec3(inner) => {
            let inner_str = rust_type(inner, registry, needs_box, None);
            format!("[{inner_str}; 3]")
        }
        ResolvedType::Vec4(inner) => {
            let inner_str = rust_type(inner, registry, needs_box, None);
            format!("[{inner_str}; 4]")
        }
        ResolvedType::Quat(inner) => {
            let inner_str = rust_type(inner, registry, needs_box, None);
            format!("[{inner_str}; 4]")
        }
        ResolvedType::Mat3(inner) => {
            let inner_str = rust_type(inner, registry, needs_box, None);
            format!("[{inner_str}; 9]")
        }
        ResolvedType::Mat4(inner) => {
            let inner_str = rust_type(inner, registry, needs_box, None);
            format!("[{inner_str}; 16]")
        }
        _ => "UnknownType".to_string(),
    }
}

/// Map a Vexil primitive type to its Rust type name.
fn primitive_type(p: &PrimitiveType) -> &'static str {
    match p {
        PrimitiveType::Bool => "bool",
        PrimitiveType::U8 => "u8",
        PrimitiveType::U16 => "u16",
        PrimitiveType::U32 => "u32",
        PrimitiveType::U64 => "u64",
        PrimitiveType::I8 => "i8",
        PrimitiveType::I16 => "i16",
        PrimitiveType::I32 => "i32",
        PrimitiveType::I64 => "i64",
        PrimitiveType::F32 => "f32",
        PrimitiveType::F64 => "f64",
        PrimitiveType::Fixed32 => "i32",
        PrimitiveType::Fixed64 => "i64",
        PrimitiveType::Void => "()",
    }
}

/// Map a sub-byte type to its containing Rust integer type (signed or unsigned).
fn sub_byte_type(s: &SubByteType) -> &'static str {
    let containing = containing_int_type(s.bits);
    if s.signed {
        match containing {
            "u8" => "i8",
            "u16" => "i16",
            "u32" => "i32",
            _ => "i64",
        }
    } else {
        containing
    }
}

/// Returns the smallest unsigned Rust integer type that can hold `bits` bits.
pub(crate) fn containing_int_type(bits: u8) -> &'static str {
    match bits {
        0..=8 => "u8",
        9..=16 => "u16",
        17..=32 => "u32",
        _ => "u64",
    }
}

/// Map a Vexil semantic type to its Rust type representation.
fn semantic_type(s: &SemanticType) -> &'static str {
    match s {
        SemanticType::String => "String",
        SemanticType::Bytes => "Vec<u8>",
        SemanticType::Rgb => "(u8, u8, u8)",
        SemanticType::Uuid => "[u8; 16]",
        SemanticType::Timestamp => "i64",
        SemanticType::Hash => "[u8; 32]",
    }
}

/// Extract the name from any TypeDef variant.
fn type_def_name(def: &TypeDef) -> String {
    match def {
        TypeDef::Message(m) => m.name.to_string(),
        TypeDef::Enum(e) => e.name.to_string(),
        TypeDef::Flags(f) => f.name.to_string(),
        TypeDef::Union(u) => u.name.to_string(),
        TypeDef::Newtype(n) => n.name.to_string(),
        TypeDef::Config(c) => c.name.to_string(),
        _ => "UnknownTypeDef".to_string(),
    }
}
