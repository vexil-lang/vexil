use vexil_lang::ast::{PrimitiveType, SemanticType};
use vexil_lang::ir::{ResolvedType, TypeDef, TypeRegistry};

/// Convert a ResolvedType to its Go type string.
pub fn go_type(ty: &ResolvedType, registry: &TypeRegistry) -> String {
    match ty {
        ResolvedType::Primitive(p) => primitive_type(p).to_string(),
        ResolvedType::SubByte(_) => "uint8".to_string(),
        ResolvedType::Semantic(s) => semantic_type(s).to_string(),
        ResolvedType::Named(id) => match registry.get(*id) {
            Some(def) => type_def_name(def),
            None => "interface{}".to_string(),
        },
        ResolvedType::Optional(inner) => {
            let inner_str = go_type(inner, registry);
            // Pointer type for optional
            format!("*{inner_str}")
        }
        ResolvedType::Array(inner) => {
            let inner_str = go_type(inner, registry);
            format!("[]{inner_str}")
        }
        ResolvedType::Map(k, v) => {
            let k_str = go_type(k, registry);
            let v_str = go_type(v, registry);
            format!("map[{k_str}]{v_str}")
        }
        ResolvedType::Result(ok, err) => {
            // Go doesn't have a native Result type; use a struct
            let ok_str = go_type(ok, registry);
            let err_str = go_type(err, registry);
            format!("Result[{ok_str}, {err_str}]")
        }
        ResolvedType::BitsInline(names) => {
            let bits = names.len() as u8;
            containing_int_type(bits).to_string()
        }
        ResolvedType::FixedArray(inner, size) => {
            let inner_str = go_type(inner, registry);
            format!("[{size}]{inner_str}")
        }
        ResolvedType::Set(inner) => {
            let inner_str = go_type(inner, registry);
            format!("map[{inner_str}]struct{{}}")
        }
        ResolvedType::Vec2(inner) => {
            let inner_str = go_type(inner, registry);
            format!("[2]{inner_str}")
        }
        ResolvedType::Vec3(inner) => {
            let inner_str = go_type(inner, registry);
            format!("[3]{inner_str}")
        }
        ResolvedType::Vec4(inner) | ResolvedType::Quat(inner) => {
            let inner_str = go_type(inner, registry);
            format!("[4]{inner_str}")
        }
        ResolvedType::Mat3(inner) => {
            let inner_str = go_type(inner, registry);
            format!("[9]{inner_str}")
        }
        ResolvedType::Mat4(inner) => {
            let inner_str = go_type(inner, registry);
            format!("[16]{inner_str}")
        }
        _ => "interface{}".to_string(),
    }
}

fn primitive_type(p: &PrimitiveType) -> &'static str {
    match p {
        PrimitiveType::Bool => "bool",
        PrimitiveType::U8 => "uint8",
        PrimitiveType::U16 => "uint16",
        PrimitiveType::U32 => "uint32",
        PrimitiveType::U64 => "uint64",
        PrimitiveType::I8 => "int8",
        PrimitiveType::I16 => "int16",
        PrimitiveType::I32 => "int32",
        PrimitiveType::I64 => "int64",
        PrimitiveType::F32 => "float32",
        PrimitiveType::F64 => "float64",
        PrimitiveType::Fixed32 => "int32",
        PrimitiveType::Fixed64 => "int64",
        PrimitiveType::Void => "struct{}",
    }
}

fn semantic_type(s: &SemanticType) -> &'static str {
    match s {
        SemanticType::String => "string",
        SemanticType::Bytes => "[]byte",
        SemanticType::Rgb => "[3]uint8",
        SemanticType::Uuid => "[16]byte",
        SemanticType::Timestamp => "int64",
        SemanticType::Hash => "[32]byte",
    }
}

/// Returns the smallest unsigned Go integer type that can hold `bits` bits.
pub(crate) fn containing_int_type(bits: u8) -> &'static str {
    match bits {
        0..=8 => "uint8",
        9..=16 => "uint16",
        17..=32 => "uint32",
        _ => "uint64",
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
pub fn sub_byte_signed(s: &vexil_lang::ast::SubByteType) -> bool {
    s.signed
}

/// Convert a snake_case string to Go PascalCase.
///
/// Handles common Go acronyms: ID, URL, HTTP, API, etc.
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
    // Handle common Go acronyms
    let acronyms = [
        ("Id", "ID"),
        ("Url", "URL"),
        ("Http", "HTTP"),
        ("Api", "API"),
        ("Ip", "IP"),
        ("Tcp", "TCP"),
        ("Udp", "UDP"),
        ("Tls", "TLS"),
        ("Ssl", "SSL"),
        ("Dns", "DNS"),
        ("Cpu", "CPU"),
        ("Gpu", "GPU"),
        ("Ram", "RAM"),
        ("Io", "IO"),
        ("Os", "OS"),
    ];
    for (lower, upper) in &acronyms {
        // Replace only at word boundary (end of string or followed by uppercase)
        if result.ends_with(lower) {
            let prefix_len = result.len() - lower.len();
            result.truncate(prefix_len);
            result.push_str(upper);
        }
    }
    result
}

/// Convert a snake_case string to Go camelCase (unexported).
pub fn to_camel_case(s: &str) -> String {
    let pascal = to_pascal_case(s);
    let mut chars = pascal.chars();
    match chars.next() {
        Some(c) => {
            let mut result: String = c.to_lowercase().collect();
            result.extend(chars);
            result
        }
        None => String::new(),
    }
}
