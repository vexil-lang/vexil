#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EncodeError {
    #[error("field `{field}`: value does not fit in {bits} bits")]
    ValueOutOfRange { field: &'static str, bits: u8 },
    #[error("field `{field}`: length {actual} exceeds limit {limit}")]
    LimitExceeded { field: &'static str, limit: u64, actual: u64 },
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum DecodeError {
    #[error("unexpected end of input")]
    UnexpectedEof,
    #[error("invalid UTF-8 in string field")]
    InvalidUtf8,
    #[error("invalid or overlong varint encoding")]
    InvalidVarint,
    #[error("field `{field}`: length {actual} exceeds limit {limit}")]
    LimitExceeded { field: &'static str, limit: u64, actual: u64 },
    #[error("unknown enum variant {value} for type `{type_name}`")]
    UnknownEnumVariant { type_name: &'static str, value: u64 },
    #[error("unknown union variant {discriminant} for type `{type_name}`")]
    UnknownUnionVariant { type_name: &'static str, discriminant: u64 },
    #[error("decoded removed field ordinal {ordinal} (removed in {removed_in}): {reason}")]
    RemovedField { ordinal: u16, removed_in: &'static str, reason: &'static str },
    #[error("field `{field}`: {message}")]
    InvalidValue { field: &'static str, message: String },
    #[error("recursive type nesting exceeded 64 levels")]
    RecursionLimitExceeded,
    #[error("schema hash mismatch")]
    SchemaMismatch { local: [u8; 32], remote: [u8; 32] },
}
