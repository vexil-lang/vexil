/// Errors from parsing or formatting .vx text.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum VxError {
    #[error("{file}:{line}:{col}: {message}")]
    Parse {
        file: String,
        line: usize,
        col: usize,
        message: String,
    },
    #[error("schema not found: {namespace}")]
    SchemaNotFound { namespace: String },
    #[error("unknown type `{type_name}` in schema `{namespace}`")]
    UnknownType {
        namespace: String,
        type_name: String,
    },
    #[error("type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },
    #[error("value overflow: {value} does not fit in {ty}")]
    Overflow { value: String, ty: String },
    #[error("unknown field `{field}` on type `{type_name}`")]
    UnknownField { type_name: String, field: String },
    #[error("unknown variant `{variant}` on type `{type_name}`")]
    UnknownVariant { type_name: String, variant: String },
    #[error("missing @schema directive")]
    MissingSchemaDirective,
    #[error("validation: {message}")]
    Validation { message: String },
}

/// Errors from reading/writing binary .vxb files.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum VxbError {
    #[error("not a valid Vexil binary file (bad magic bytes)")]
    BadMagic,
    #[error("unsupported format version {version} (max supported: {max_supported})")]
    UnsupportedVersion { version: u8, max_supported: u8 },
    #[error("schema hash mismatch: file={file_hash}, loaded={loaded_hash}")]
    SchemaHashMismatch {
        file_hash: String,
        loaded_hash: String,
    },
    #[error("decompression failed: {message}")]
    DecompressionFailed { message: String },
    #[error("I/O error: {message}")]
    Io { message: String },
    #[error("encode error: {0}")]
    Encode(#[from] vexil_runtime::EncodeError),
    #[error("decode error: {0}")]
    Decode(#[from] vexil_runtime::DecodeError),
}

/// Errors from schema-driven encoding (Value -> bitpack bytes).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum StoreEncodeError {
    #[error("type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },
    #[error("unknown type")]
    UnknownTypeId,
    #[error("unknown variant `{variant}` on type `{type_name}`")]
    UnknownVariant { type_name: String, variant: String },
    #[error("unknown field `{field}` on type `{type_name}`")]
    UnknownField { type_name: String, field: String },
    #[error("value {value} does not fit in {bits} bits")]
    Overflow { value: String, bits: u8 },
    #[error("bitpack error: {0}")]
    Bitpack(#[from] vexil_runtime::EncodeError),
    #[error("type `{type_name}` not found in schema")]
    TypeNotFound { type_name: String },
    #[error("recursion depth exceeded")]
    RecursionLimit,
}

/// Errors from schema-driven decoding (bitpack bytes -> Value).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum StoreDecodeError {
    #[error("type mismatch at {context}: expected {expected}")]
    TypeMismatch { context: String, expected: String },
    #[error("unknown type")]
    UnknownTypeId,
    #[error("unknown enum variant ordinal {ordinal} on type `{type_name}`")]
    UnknownVariant { type_name: String, ordinal: u64 },
    #[error("unknown union discriminant {discriminant} on type `{type_name}`")]
    UnknownUnionDiscriminant {
        type_name: String,
        discriminant: u64,
    },
    #[error("bitpack error: {0}")]
    Bitpack(#[from] vexil_runtime::DecodeError),
    #[error("type `{type_name}` not found in schema")]
    TypeNotFound { type_name: String },
    #[error("recursion depth exceeded")]
    RecursionLimit,
}
