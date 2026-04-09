use vexil_lang::ast::{PrimitiveType, SemanticType, SubByteType};
use vexil_lang::ir::{ResolvedType, TypeDef, TypeRegistry};

/// Convert a ResolvedType to its TypeScript type string.
pub fn ts_type(ty: &ResolvedType, registry: &TypeRegistry) -> String {
    match ty {
        ResolvedType::Primitive(p) => primitive_type(p).to_string(),
        ResolvedType::SubByte(_) => "number".to_string(),
        ResolvedType::Semantic(s) => semantic_type(s).to_string(),
        ResolvedType::Named(id) => match registry.get(*id) {
            Some(def) => type_def_name(def),
            None => "unknown".to_string(),
        },
        ResolvedType::Optional(inner) => {
            let inner_str = ts_type(inner, registry);
            format!("{inner_str} | null")
        }
        ResolvedType::Array(inner) => {
            let inner_str = ts_type(inner, registry);
            format!("{inner_str}[]")
        }
        ResolvedType::FixedArray(inner, _size) => {
            // TypeScript doesn't have native fixed array types, use regular array
            let inner_str = ts_type(inner, registry);
            format!("{inner_str}[]")
        }
        ResolvedType::Set(inner) => {
            let inner_str = ts_type(inner, registry);
            format!("Set<{inner_str}>")
        }
        ResolvedType::Map(k, v) => {
            let k_str = ts_type(k, registry);
            let v_str = ts_type(v, registry);
            format!("Map<{k_str}, {v_str}>")
        }
        ResolvedType::Result(ok, err) => {
            let ok_str = ts_type(ok, registry);
            let err_str = ts_type(err, registry);
            format!("{{ ok: {ok_str} }} | {{ err: {err_str} }}")
        }
        ResolvedType::BitsInline(_) => "number".to_string(),
        _ => "unknown".to_string(),
    }
}

fn primitive_type(p: &PrimitiveType) -> &'static str {
    match p {
        PrimitiveType::Bool => "boolean",
        PrimitiveType::U8 | PrimitiveType::U16 | PrimitiveType::U32 => "number",
        PrimitiveType::I8 | PrimitiveType::I16 | PrimitiveType::I32 => "number",
        PrimitiveType::U64 | PrimitiveType::I64 => "bigint",
        PrimitiveType::F32 | PrimitiveType::F64 => "number",
        PrimitiveType::Fixed32 | PrimitiveType::Fixed64 => "number",
        PrimitiveType::Void => "void",
    }
}

fn semantic_type(s: &SemanticType) -> &'static str {
    match s {
        SemanticType::String => "string",
        SemanticType::Bytes => "Uint8Array",
        SemanticType::Rgb => "[number, number, number]",
        SemanticType::Uuid => "Uint8Array",
        SemanticType::Timestamp => "bigint",
        SemanticType::Hash => "Uint8Array",
    }
}

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

/// Returns true if a SubByteType is signed.
pub fn sub_byte_signed(s: &SubByteType) -> bool {
    s.signed
}
