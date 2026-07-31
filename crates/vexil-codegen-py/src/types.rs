use vexil_lang::ast::{PrimitiveType, SemanticType};
use vexil_lang::ir::{ResolvedType, TypeDef, TypeRegistry};

/// Python reserved words, which cannot appear as identifiers.
///
/// Soft keywords (`match`, `case`, `type`, `_`) are valid identifiers and are
/// deliberately absent.
const PY_KEYWORDS: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
    "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while",
    "with", "yield",
];

/// Render a Vexil field name as an injective, valid Python identifier.
///
/// Vexil permits names that are Python reserved words (see corpus
/// `014_keywords_as_fields`). It also permits `self` and `unknown`, which
/// conflict with generated method parameters and message unknown-byte storage.
/// Conflicting names use an underscore-prefixed namespace that authored Vexil
/// field names cannot occupy, so names such as `from` and `from_` stay distinct.
pub fn py_ident(name: &str) -> String {
    if PY_KEYWORDS.contains(&name) || matches!(name, "self" | "unknown") {
        format!("_vexil_{name}")
    } else {
        name.to_string()
    }
}

/// Render a Vexil generic parameter in a generator-owned Python namespace.
///
/// Type parameters are module-level `TypeVar` bindings. Prefixing every one
/// prevents collisions with imported typing helpers, generated declarations,
/// and other authored upper-case names.
pub fn py_type_param_ident(name: &str) -> String {
    format!("_VexilTypeParam_{name}")
}

/// Convert a ResolvedType to its Python type annotation string.
pub fn py_type(ty: &ResolvedType, registry: &TypeRegistry) -> String {
    match ty {
        ResolvedType::Primitive(p) => primitive_type(p).to_string(),
        ResolvedType::SubByte(_) => "int".to_string(),
        ResolvedType::Semantic(s) => semantic_type(s).to_string(),
        ResolvedType::Named(id) => match registry.get(*id) {
            Some(def) => type_def_name(def),
            None => "object".to_string(),
        },
        ResolvedType::Optional(inner) => {
            let inner_str = py_type(inner, registry);
            if optional_payload_needs_wrapper(inner) {
                format!("tuple[{inner_str}] | None")
            } else {
                format!("{inner_str} | None")
            }
        }
        ResolvedType::Array(inner) => {
            let inner_str = py_type(inner, registry);
            format!("list[{inner_str}]")
        }
        ResolvedType::Map(k, v) => {
            let k_str = py_type(k, registry);
            let v_str = py_type(v, registry);
            format!("dict[{k_str}, {v_str}]")
        }
        ResolvedType::Result(ok, err) => {
            let ok_str = py_type(ok, registry);
            let err_str = py_type(err, registry);
            format!("tuple[_VexilLiteral[True], {ok_str}] | tuple[_VexilLiteral[False], {err_str}]")
        }
        ResolvedType::BitsInline(names) => {
            let _bits = names.len() as u8;
            "int".to_string()
        }
        ResolvedType::FixedArray(inner, _size) => {
            // No trailing `# fixed[N]` comment: a type string is substituted
            // into nested annotations, where a comment swallows the rest of
            // the line. Matches the fixed-size geometric types below.
            let inner_str = py_type(inner, registry);
            format!("tuple[{inner_str}, ...]")
        }
        ResolvedType::Set(inner) => {
            let inner_str = py_type(inner, registry);
            format!("set[{inner_str}]")
        }
        ResolvedType::Vec2(inner)
        | ResolvedType::Vec3(inner)
        | ResolvedType::Vec4(inner)
        | ResolvedType::Quat(inner)
        | ResolvedType::Mat3(inner)
        | ResolvedType::Mat4(inner) => {
            let inner_str = py_type(inner, registry);
            format!("tuple[{inner_str}, ...]")
        }
        _ => "object".to_string(),
    }
}

/// Python needs a one-tuple to distinguish an absent outer optional from a
/// present payload whose value is itself `None`.
pub fn optional_payload_needs_wrapper(ty: &ResolvedType) -> bool {
    matches!(
        ty,
        ResolvedType::Primitive(PrimitiveType::Void) | ResolvedType::Optional(_)
    )
}

fn primitive_type(p: &PrimitiveType) -> &'static str {
    match p {
        PrimitiveType::Bool => "bool",
        PrimitiveType::U8 | PrimitiveType::U16 | PrimitiveType::U32 | PrimitiveType::U64 => "int",
        PrimitiveType::I8 | PrimitiveType::I16 | PrimitiveType::I32 | PrimitiveType::I64 => "int",
        PrimitiveType::F32 | PrimitiveType::F64 => "float",
        PrimitiveType::Fixed32 | PrimitiveType::Fixed64 => "int",
        PrimitiveType::Void => "None",
    }
}

fn semantic_type(s: &SemanticType) -> &'static str {
    match s {
        SemanticType::String => "str",
        SemanticType::Bytes => "bytes",
        SemanticType::Rgb => "tuple[int, int, int]",
        SemanticType::Uuid => "bytes",
        SemanticType::Timestamp => "int",
        SemanticType::Hash => "bytes",
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
pub fn sub_byte_signed(s: &vexil_lang::ast::SubByteType) -> bool {
    s.signed
}

/// Convert snake_case to PascalCase (Python class/field convention).
pub fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;
    for ch in s.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.extend(ch.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }
    result
}

/// Get the struct format character for a primitive type.
pub fn struct_format_char(p: &PrimitiveType) -> &'static str {
    match p {
        PrimitiveType::Bool => "?",
        PrimitiveType::U8 => "B",
        PrimitiveType::U16 => "<H",
        PrimitiveType::U32 => "<I",
        PrimitiveType::U64 => "<Q",
        PrimitiveType::I8 => "b",
        PrimitiveType::I16 => "<h",
        PrimitiveType::I32 => "<i",
        PrimitiveType::I64 => "<q",
        PrimitiveType::F32 => "<f",
        PrimitiveType::F64 => "<d",
        PrimitiveType::Fixed32 => "<i",
        PrimitiveType::Fixed64 => "<q",
        PrimitiveType::Void => "",
    }
}
