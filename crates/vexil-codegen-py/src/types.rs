use vexil_lang::ast::{PrimitiveType, SemanticType};
use vexil_lang::ir::{ResolvedType, TypeDef, TypeRegistry};

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
            format!("{inner_str} | None")
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
            format!("tuple[bool, {ok_str} | {err_str}]")
        }
        ResolvedType::BitsInline(names) => {
            let _bits = names.len() as u8;
            "int".to_string()
        }
        ResolvedType::FixedArray(inner, size) => {
            let inner_str = py_type(inner, registry);
            format!("tuple[{inner_str}, ...]  # fixed[{size}]")
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
