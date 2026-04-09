use std::collections::BTreeMap;

/// Dynamic representation of any Vexil-typed value.
///
/// `Value` is the universal in-memory data model for the vexil-store crate.
/// It can represent any Vexil schema type, from primitive scalars to deeply
/// nested messages and unions. Values are produced by the parser and decoder,
/// and consumed by the encoder, formatter, and validator.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    // Primitives
    /// Boolean value (`true` or `false`).
    Bool(bool),
    /// Unsigned 8-bit integer.
    U8(u8),
    /// Unsigned 16-bit integer.
    U16(u16),
    /// Unsigned 32-bit integer.
    U32(u32),
    /// Unsigned 64-bit integer.
    U64(u64),
    /// Signed 8-bit integer.
    I8(i8),
    /// Signed 16-bit integer.
    I16(i16),
    /// Signed 32-bit integer.
    I32(i32),
    /// Signed 64-bit integer.
    I64(i64),
    /// 32-bit IEEE 754 floating-point.
    F32(f32),
    /// 64-bit IEEE 754 floating-point.
    F64(f64),
    /// Q16.16 fixed-point (raw i32 on wire).
    Fixed32(i32),
    /// Q32.32 fixed-point (raw i64 on wire).
    Fixed64(i64),
    /// Sub-byte integer (1-7 bits). `width` is the declared bit count.
    Bits {
        /// The unsigned integer value stored in the bits.
        value: u64,
        /// Number of bits (1-64).
        width: u8,
    },

    // Semantic types
    /// UTF-8 string.
    String(String),
    /// Arbitrary byte sequence.
    Bytes(Vec<u8>),
    /// RGB color, stored as `[r, g, b]`.
    Rgb([u8; 3]),
    /// 128-bit UUID.
    Uuid([u8; 16]),
    /// Microseconds since Unix epoch.
    Timestamp(i64),
    /// BLAKE3 hash (32 bytes).
    Hash([u8; 32]),

    // Parameterized types
    /// Absent optional value.
    None,
    /// Present optional value wrapping an inner value.
    Some(Box<Value>),
    /// Ordered sequence of values of a uniform type.
    Array(Vec<Value>),
    /// Set of unique values. Elements must be valid map key types.
    Set(Vec<Value>),
    /// Map of key-value pairs. Preserves insertion order.
    Map(Vec<(Value, Value)>),
    /// Successful result value.
    Ok(Box<Value>),
    /// Error result value.
    Err(Box<Value>),

    // Composite types
    /// Message with named fields stored in a `BTreeMap`.
    Message(BTreeMap<String, Value>),
    /// Enum variant identified by its name.
    Enum(String),
    /// Flags (set of flag names).
    Flags(Vec<String>),
    /// Union variant with named fields.
    Union {
        /// The name of the active variant.
        variant: String,
        /// Fields of the active variant.
        fields: BTreeMap<String, Value>,
    },
}

impl Value {
    /// Returns the default `Value` for a given resolved type.
    ///
    /// This produces the zero-value / canonical default for each type:
    /// `false` for booleans, `0` for numeric types, empty collections
    /// for containers, `None` for optionals, and the first variant for
    /// enums and unions.
    pub fn default_for_type(
        ty: &vexil_lang::ResolvedType,
        registry: &vexil_lang::TypeRegistry,
    ) -> Self {
        use vexil_lang::ast::{PrimitiveType, SemanticType};
        use vexil_lang::ResolvedType;
        match ty {
            ResolvedType::Primitive(p) => match p {
                PrimitiveType::Bool => Value::Bool(false),
                PrimitiveType::U8 => Value::U8(0),
                PrimitiveType::U16 => Value::U16(0),
                PrimitiveType::U32 => Value::U32(0),
                PrimitiveType::U64 => Value::U64(0),
                PrimitiveType::I8 => Value::I8(0),
                PrimitiveType::I16 => Value::I16(0),
                PrimitiveType::I32 => Value::I32(0),
                PrimitiveType::I64 => Value::I64(0),
                PrimitiveType::F32 => Value::F32(0.0),
                PrimitiveType::F64 => Value::F64(0.0),
                PrimitiveType::Fixed32 => Value::Fixed32(0),
                PrimitiveType::Fixed64 => Value::Fixed64(0),
                PrimitiveType::Void => Value::Bool(false),
            },
            ResolvedType::SubByte(sbt) => Value::Bits {
                value: 0,
                width: sbt.bits,
            },
            ResolvedType::Semantic(s) => match s {
                SemanticType::String => Value::String(String::new()),
                SemanticType::Bytes => Value::Bytes(Vec::new()),
                SemanticType::Rgb => Value::Rgb([0, 0, 0]),
                SemanticType::Uuid => Value::Uuid([0; 16]),
                SemanticType::Timestamp => Value::Timestamp(0),
                SemanticType::Hash => Value::Hash([0; 32]),
            },
            ResolvedType::Named(type_id) => {
                if let Some(type_def) = registry.get(*type_id) {
                    Self::default_for_typedef(type_def, registry)
                } else {
                    Value::None
                }
            }
            ResolvedType::Optional(_) => Value::None,
            ResolvedType::Array(_) => Value::Array(Vec::new()),
            ResolvedType::Set(_) => Value::Set(Vec::new()),
            ResolvedType::Map(_, _) => Value::Map(Vec::new()),
            ResolvedType::Result(ok_ty, _) => {
                Value::Ok(Box::new(Self::default_for_type(ok_ty, registry)))
            }
            _ => Value::None,
        }
    }

    fn default_for_typedef(
        type_def: &vexil_lang::TypeDef,
        registry: &vexil_lang::TypeRegistry,
    ) -> Self {
        use vexil_lang::TypeDef;
        match type_def {
            TypeDef::Message(_) => Value::Message(std::collections::BTreeMap::new()),
            TypeDef::Enum(e) => {
                if let Some(first) = e.variants.first() {
                    Value::Enum(first.name.to_string())
                } else {
                    Value::Enum(String::new())
                }
            }
            TypeDef::Flags(_) => Value::Flags(Vec::new()),
            TypeDef::Union(u) => {
                if let Some(first) = u.variants.first() {
                    Value::Union {
                        variant: first.name.to_string(),
                        fields: std::collections::BTreeMap::new(),
                    }
                } else {
                    Value::Union {
                        variant: String::new(),
                        fields: std::collections::BTreeMap::new(),
                    }
                }
            }
            TypeDef::Newtype(nt) => Self::default_for_type(&nt.terminal_type, registry),
            TypeDef::Config(_) => Value::Message(std::collections::BTreeMap::new()),
            _ => Value::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_clone_eq() {
        let v1 = Value::U32(42);
        let v2 = v1.clone();
        assert_eq!(v1, v2);
    }

    #[test]
    fn value_message() {
        let mut fields = BTreeMap::new();
        fields.insert("id".to_string(), Value::U32(1));
        fields.insert("name".to_string(), Value::String("test".to_string()));
        let msg = Value::Message(fields);
        assert!(matches!(msg, Value::Message(_)));
    }

    #[test]
    fn value_nested_optional() {
        let v = Value::Some(Box::new(Value::None));
        assert_eq!(v, Value::Some(Box::new(Value::None)));
    }

    #[test]
    fn value_union() {
        let mut fields = BTreeMap::new();
        fields.insert("radius".to_string(), Value::F64(3.14));
        let v = Value::Union {
            variant: "Circle".to_string(),
            fields,
        };
        assert!(matches!(v, Value::Union { .. }));
    }
}
