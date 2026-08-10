use std::collections::HashMap;
use vexil_lang::ast::{PrimitiveType, SemanticType, SubByteType};

use vexil_lang::ir::{ResolvedType, TypeDef, TypeId, TypeRegistry};

/// Target-local ownership information for names referenced by generated code.
///
/// Single-schema generation leaves this empty and therefore preserves bare
/// declaration names. Project generation records imported TypeIds against
/// deterministic namespace bindings.
#[derive(Debug, Default)]
pub(crate) struct TypeScriptSymbols {
    imported: HashMap<TypeId, String>,
}

impl TypeScriptSymbols {
    pub(crate) fn insert_import(&mut self, id: TypeId, binding: String) {
        self.imported.insert(id, binding);
    }

    pub(crate) fn type_ref(&self, id: TypeId, registry: &TypeRegistry) -> String {
        let name = registry
            .get(id)
            .map(type_def_name)
            .unwrap_or_else(|| "unknown".to_string());
        match self.imported.get(&id) {
            Some(binding) => format!("{binding}.{name}"),
            None => name,
        }
    }

    pub(crate) fn encode_ref(&self, id: TypeId, registry: &TypeRegistry) -> String {
        let name = registry
            .get(id)
            .map(type_def_name)
            .unwrap_or_else(|| "Unknown".to_string());
        match self.imported.get(&id) {
            Some(binding) => format!("{binding}.encode{name}"),
            None => format!("encode{name}"),
        }
    }

    pub(crate) fn decode_ref(&self, id: TypeId, registry: &TypeRegistry) -> String {
        let name = registry
            .get(id)
            .map(type_def_name)
            .unwrap_or_else(|| "Unknown".to_string());
        match self.imported.get(&id) {
            Some(binding) => format!("{binding}.decode{name}"),
            None => format!("decode{name}"),
        }
    }
}

/// Convert a ResolvedType to its TypeScript type string.
pub fn ts_type(ty: &ResolvedType, registry: &TypeRegistry) -> String {
    ts_type_with_symbols(ty, registry, &TypeScriptSymbols::default())
}

pub(crate) fn ts_type_with_symbols(
    ty: &ResolvedType,
    registry: &TypeRegistry,
    symbols: &TypeScriptSymbols,
) -> String {
    match ty {
        ResolvedType::Primitive(p) => primitive_type(p).to_string(),
        ResolvedType::SubByte(_) => "number".to_string(),
        ResolvedType::Semantic(s) => semantic_type(s).to_string(),
        ResolvedType::Named(id) => symbols.type_ref(*id, registry),
        ResolvedType::Optional(inner) => {
            let inner_str = ts_type_with_symbols(inner, registry, symbols);
            if optional_payload_needs_wrapper(inner) {
                format!("[{inner_str}] | null")
            } else {
                format!("{inner_str} | null")
            }
        }
        ResolvedType::Array(inner) => {
            let inner_str = ts_type_with_symbols(inner, registry, symbols);
            if array_element_needs_wrapper(inner) {
                format!("({inner_str})[]")
            } else {
                format!("{inner_str}[]")
            }
        }
        ResolvedType::FixedArray(inner, _size) => {
            // TypeScript doesn't have native fixed array types, use regular array
            let inner_str = ts_type_with_symbols(inner, registry, symbols);
            if array_element_needs_wrapper(inner) {
                format!("({inner_str})[]")
            } else {
                format!("{inner_str}[]")
            }
        }
        ResolvedType::Set(inner) => {
            let inner_str = ts_type_with_symbols(inner, registry, symbols);
            format!("Set<{inner_str}>")
        }
        ResolvedType::Map(k, v) => {
            let k_str = ts_type_with_symbols(k, registry, symbols);
            let v_str = ts_type_with_symbols(v, registry, symbols);
            format!("Map<{k_str}, {v_str}>")
        }
        ResolvedType::Result(ok, err) => {
            let ok_str = ts_type_with_symbols(ok, registry, symbols);
            let err_str = ts_type_with_symbols(err, registry, symbols);
            format!("{{ ok: {ok_str} }} | {{ err: {err_str} }}")
        }
        ResolvedType::BitsInline(_) => "number".to_string(),
        ResolvedType::Vec2(inner) => {
            let inner_str = ts_type_with_symbols(inner, registry, symbols);
            format!("[{inner_str}, {inner_str}]")
        }
        ResolvedType::Vec3(inner) => {
            let inner_str = ts_type_with_symbols(inner, registry, symbols);
            format!("[{inner_str}, {inner_str}, {inner_str}]")
        }
        ResolvedType::Vec4(inner) | ResolvedType::Quat(inner) => {
            let inner_str = ts_type_with_symbols(inner, registry, symbols);
            format!("[{inner_str}, {inner_str}, {inner_str}, {inner_str}]")
        }
        ResolvedType::Mat3(inner) => {
            let inner_str = ts_type_with_symbols(inner, registry, symbols);
            format!("{inner_str}[]") // 9 elements
        }
        ResolvedType::Mat4(inner) => {
            let inner_str = ts_type_with_symbols(inner, registry, symbols);
            format!("{inner_str}[]") // 16 elements
        }
        _ => "unknown".to_string(),
    }
}

/// TypeScript needs a one-tuple to distinguish an absent outer optional from
/// a present payload whose value is itself `null`.
pub fn optional_payload_needs_wrapper(ty: &ResolvedType) -> bool {
    matches!(ty, ResolvedType::Optional(_))
}

fn array_element_needs_wrapper(ty: &ResolvedType) -> bool {
    matches!(ty, ResolvedType::Optional(_) | ResolvedType::Result(_, _))
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
        TypeDef::Trait(t) => t.name.to_string(),
        _ => "UnknownTypeDef".to_string(),
    }
}

/// Returns true if a SubByteType is signed.
pub fn sub_byte_signed(s: &SubByteType) -> bool {
    s.signed
}
