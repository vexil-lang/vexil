use std::collections::BTreeMap;

/// Dynamic representation of any Vexil-typed value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    // Primitives
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    /// Sub-byte integer (1-7 bits). `width` is the declared bit count.
    Bits {
        value: u64,
        width: u8,
    },

    // Semantic types
    String(String),
    Bytes(Vec<u8>),
    Rgb([u8; 3]),
    Uuid([u8; 16]),
    /// Microseconds since Unix epoch.
    Timestamp(i64),
    /// BLAKE3 hash (32 bytes).
    Hash([u8; 32]),

    // Parameterized types
    None,
    Some(Box<Value>),
    Array(Vec<Value>),
    /// Map preserves insertion order. Keys are (key, value) pairs.
    Map(Vec<(Value, Value)>),
    Ok(Box<Value>),
    Err(Box<Value>),

    // Composite types
    /// Message with named fields.
    Message(BTreeMap<String, Value>),
    /// Enum variant (bare name).
    Enum(String),
    /// Flags (set of flag names).
    Flags(Vec<String>),
    /// Union variant with named fields.
    Union {
        variant: String,
        fields: BTreeMap<String, Value>,
    },
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
