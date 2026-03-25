use crate::ast::{PrimitiveType, SemanticType, SubByteType};
use crate::ir::{CompiledSchema, ResolvedType, TypeDef, TypeRegistry};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute the canonical form of a single-file schema per spec §7.
/// Returns a deterministic UTF-8 string — single-space-delimited, no newlines.
pub fn canonical_form(compiled: &CompiledSchema) -> String {
    let mut out = String::new();
    // namespace
    out.push_str("namespace ");
    out.push_str(&compiled.namespace.join("."));
    // TODO: schema-level annotations, declarations
    out
}

/// Compute the BLAKE3 hash of the canonical form.
pub fn schema_hash(compiled: &CompiledSchema) -> [u8; 32] {
    let form = canonical_form(compiled);
    *blake3::hash(form.as_bytes()).as_bytes()
}

// ---------------------------------------------------------------------------
// Type string helpers (module-private)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn type_str(ty: &ResolvedType, registry: &TypeRegistry) -> String {
    #[allow(unreachable_patterns)]
    match ty {
        ResolvedType::Primitive(p) => primitive_str(p).to_owned(),
        ResolvedType::SubByte(s) => sub_byte_str(s),
        ResolvedType::Semantic(s) => semantic_str(s).to_owned(),
        ResolvedType::Named(id) => {
            if let Some(def) = registry.get(*id) {
                type_def_name(def).to_owned()
            } else {
                debug_assert!(false, "unresolved TypeId {:?} in canonical form", id);
                "<unresolved>".to_owned()
            }
        }
        ResolvedType::Optional(inner) => format!("optional<{}>", type_str(inner, registry)),
        ResolvedType::Array(inner) => format!("array<{}>", type_str(inner, registry)),
        ResolvedType::Map(k, v) => {
            format!("map<{}, {}>", type_str(k, registry), type_str(v, registry))
        }
        ResolvedType::Result(ok, err) => {
            format!(
                "result<{}, {}>",
                type_str(ok, registry),
                type_str(err, registry)
            )
        }
        _ => {
            debug_assert!(false, "unknown ResolvedType variant in canonical form");
            "<unknown>".to_owned()
        }
    }
}

#[allow(dead_code)]
fn primitive_str(p: &PrimitiveType) -> &'static str {
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
        PrimitiveType::Void => "void",
    }
}

#[allow(dead_code)]
fn sub_byte_str(s: &SubByteType) -> String {
    if s.signed {
        format!("i{}", s.bits)
    } else {
        format!("u{}", s.bits)
    }
}

#[allow(dead_code)]
fn semantic_str(s: &SemanticType) -> &'static str {
    match s {
        SemanticType::String => "string",
        SemanticType::Bytes => "bytes",
        SemanticType::Rgb => "rgb",
        SemanticType::Uuid => "uuid",
        SemanticType::Timestamp => "timestamp",
        SemanticType::Hash => "hash",
    }
}

#[allow(dead_code)]
fn type_def_name(def: &TypeDef) -> &str {
    #[allow(unreachable_patterns)]
    match def {
        TypeDef::Message(d) => d.name.as_str(),
        TypeDef::Enum(d) => d.name.as_str(),
        TypeDef::Flags(d) => d.name.as_str(),
        TypeDef::Union(d) => d.name.as_str(),
        TypeDef::Newtype(d) => d.name.as_str(),
        TypeDef::Config(d) => d.name.as_str(),
        _ => {
            debug_assert!(false, "unknown TypeDef variant in canonical form");
            "<unknown>"
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{PrimitiveType, SemanticType, SubByteType};

    fn dummy_registry() -> crate::ir::TypeRegistry {
        crate::ir::TypeRegistry::new()
    }

    #[test]
    fn minimal_namespace_only() {
        let result = crate::compile("namespace test.minimal\nmessage Empty {}");
        let compiled = result.compiled.unwrap();
        let form = canonical_form(&compiled);
        assert!(form.starts_with("namespace test.minimal"));
        let hash = schema_hash(&compiled);
        assert_eq!(hash.len(), 32);
    }

    // -----------------------------------------------------------------------
    // Task 2: type string tests
    // -----------------------------------------------------------------------

    #[test]
    fn type_string_primitives() {
        assert_eq!(
            type_str(
                &ResolvedType::Primitive(PrimitiveType::Bool),
                &dummy_registry()
            ),
            "bool"
        );
        assert_eq!(
            type_str(
                &ResolvedType::Primitive(PrimitiveType::U32),
                &dummy_registry()
            ),
            "u32"
        );
        assert_eq!(
            type_str(
                &ResolvedType::Primitive(PrimitiveType::I64),
                &dummy_registry()
            ),
            "i64"
        );
        assert_eq!(
            type_str(
                &ResolvedType::Primitive(PrimitiveType::F64),
                &dummy_registry()
            ),
            "f64"
        );
        assert_eq!(
            type_str(
                &ResolvedType::Primitive(PrimitiveType::Void),
                &dummy_registry()
            ),
            "void"
        );
    }

    #[test]
    fn type_string_sub_byte() {
        assert_eq!(
            type_str(
                &ResolvedType::SubByte(SubByteType {
                    bits: 3,
                    signed: false
                }),
                &dummy_registry()
            ),
            "u3"
        );
        assert_eq!(
            type_str(
                &ResolvedType::SubByte(SubByteType {
                    bits: 5,
                    signed: true
                }),
                &dummy_registry()
            ),
            "i5"
        );
    }

    #[test]
    fn type_string_semantic() {
        assert_eq!(
            type_str(
                &ResolvedType::Semantic(SemanticType::String),
                &dummy_registry()
            ),
            "string"
        );
        assert_eq!(
            type_str(
                &ResolvedType::Semantic(SemanticType::Uuid),
                &dummy_registry()
            ),
            "uuid"
        );
        assert_eq!(
            type_str(
                &ResolvedType::Semantic(SemanticType::Timestamp),
                &dummy_registry()
            ),
            "timestamp"
        );
    }

    #[test]
    fn type_string_parameterized() {
        let inner = ResolvedType::Primitive(PrimitiveType::U32);
        assert_eq!(
            type_str(
                &ResolvedType::Optional(Box::new(inner.clone())),
                &dummy_registry()
            ),
            "optional<u32>"
        );
        assert_eq!(
            type_str(
                &ResolvedType::Array(Box::new(inner.clone())),
                &dummy_registry()
            ),
            "array<u32>"
        );
        let key = ResolvedType::Semantic(SemanticType::String);
        assert_eq!(
            type_str(
                &ResolvedType::Map(Box::new(key), Box::new(inner.clone())),
                &dummy_registry()
            ),
            "map<string, u32>"
        );
        let ok = ResolvedType::Primitive(PrimitiveType::U32);
        let err = ResolvedType::Semantic(SemanticType::String);
        assert_eq!(
            type_str(
                &ResolvedType::Result(Box::new(ok), Box::new(err)),
                &dummy_registry()
            ),
            "result<u32, string>"
        );
    }
}
