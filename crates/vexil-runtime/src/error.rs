/// Errors that can occur while encoding (packing) a value to wire format.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EncodeError {
    /// A field value does not fit in the declared bit width.
    #[error("field `{field}`: value does not fit in {bits} bits")]
    ValueOutOfRange { field: &'static str, bits: u8 },
    /// A collection or byte sequence exceeds its maximum allowed length.
    #[error("field `{field}`: length {actual} exceeds limit {limit}")]
    LimitExceeded {
        field: &'static str,
        limit: u64,
        actual: u64,
    },
    /// Recursive type nesting exceeded [`MAX_RECURSION_DEPTH`](crate::MAX_RECURSION_DEPTH).
    #[error("recursive type nesting exceeded 64 levels")]
    RecursionLimitExceeded,
}

/// Errors that can occur while decoding (unpacking) a value from wire format.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum DecodeError {
    /// The input ended before all expected fields were read.
    #[error("unexpected end of input")]
    UnexpectedEof,
    /// A string field contained bytes that are not valid UTF-8.
    #[error("invalid UTF-8 in string field")]
    InvalidUtf8,
    /// A LEB128 varint was too long or used an overlong encoding.
    #[error("invalid or overlong varint encoding")]
    InvalidVarint,
    /// A decoded length or count exceeded its safety limit.
    #[error("field `{field}`: length {actual} exceeds limit {limit}")]
    LimitExceeded {
        field: &'static str,
        limit: u64,
        actual: u64,
    },
    /// The discriminant for an enum did not match any known variant.
    #[error("unknown enum variant {value} for type `{type_name}`")]
    UnknownEnumVariant { type_name: &'static str, value: u64 },
    /// The discriminant for a union did not match any known variant.
    #[error("unknown union variant {discriminant} for type `{type_name}`")]
    UnknownUnionVariant {
        type_name: &'static str,
        discriminant: u64,
    },
    /// A field that was marked as removed in the schema was encountered.
    #[error("decoded removed field ordinal {ordinal} (removed in {removed_in}): {reason}")]
    RemovedField {
        ordinal: u16,
        removed_in: &'static str,
        reason: &'static str,
    },
    /// A field value was syntactically valid but semantically invalid.
    #[error("field `{field}`: {message}")]
    InvalidValue {
        field: &'static str,
        message: String,
    },
    /// Recursive type nesting exceeded [`MAX_RECURSION_DEPTH`](crate::MAX_RECURSION_DEPTH).
    #[error("recursive type nesting exceeded 64 levels")]
    RecursionLimitExceeded,
    /// The BLAKE3 schema hash in the data did not match the local schema.
    #[error("schema hash mismatch")]
    SchemaMismatch { local: [u8; 32], remote: [u8; 32] },
}
