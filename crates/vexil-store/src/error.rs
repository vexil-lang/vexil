/// Errors from parsing or formatting `.vx` text.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum VxError {
    /// Parse error at a specific file location.
    #[error("{file}:{line}:{col}: {message}")]
    Parse {
        /// Source file name.
        file: String,
        /// 1-based line number.
        line: usize,
        /// 1-based column number.
        col: usize,
        /// Human-readable error message.
        message: String,
    },
    /// Referenced schema namespace was not found.
    #[error("schema not found: {namespace}")]
    SchemaNotFound {
        /// The namespace that was looked up.
        namespace: String,
    },
    /// A type name is not defined in the schema.
    #[error("unknown type `{type_name}` in schema `{namespace}`")]
    UnknownType {
        /// Schema namespace.
        namespace: String,
        /// Name of the unknown type.
        type_name: String,
    },
    /// A value has the wrong type for its expected context.
    #[error("type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        /// Description of the expected type.
        expected: String,
        /// Description of the actual value received.
        actual: String,
    },
    /// A value exceeds the range of its target type.
    #[error("value overflow: {value} does not fit in {ty}")]
    Overflow {
        /// The overflowing value (as a string).
        value: String,
        /// The target type description.
        ty: String,
    },
    /// A field name is not defined on the target type.
    #[error("unknown field `{field}` on type `{type_name}`")]
    UnknownField {
        /// Name of the type.
        type_name: String,
        /// Name of the unknown field.
        field: String,
    },
    /// A variant name is not defined on the target enum/flags/union.
    #[error("unknown variant `{variant}` on type `{type_name}`")]
    UnknownVariant {
        /// Name of the type.
        type_name: String,
        /// Name of the unknown variant.
        variant: String,
    },
    /// Expected an `@schema` directive but none was found.
    #[error("missing @schema directive")]
    MissingSchemaDirective,
    /// A validation error with a descriptive message.
    #[error("validation: {message}")]
    Validation {
        /// Validation error message.
        message: String,
    },
}

/// Errors from reading/writing binary `.vxb` files.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum VxbError {
    /// The file does not start with a valid Vexil magic signature.
    #[error("not a valid Vexil binary file (bad magic bytes)")]
    BadMagic,
    /// The file format version is newer than what this implementation supports.
    #[error("unsupported format version {version} (max supported: {max_supported})")]
    UnsupportedVersion {
        /// Version found in the file.
        version: u8,
        /// Maximum version this implementation can read.
        max_supported: u8,
    },
    /// The schema hash in the file does not match the loaded schema.
    #[error("schema hash mismatch: file={file_hash}, loaded={loaded_hash}")]
    SchemaHashMismatch {
        /// Hash from the binary file (hex-encoded).
        file_hash: String,
        /// Hash of the loaded schema (hex-encoded).
        loaded_hash: String,
    },
    /// Decompression of a compressed payload failed.
    #[error("decompression failed: {message}")]
    DecompressionFailed {
        /// Error description.
        message: String,
    },
    /// An I/O error occurred during reading or writing.
    #[error("I/O error: {message}")]
    Io {
        /// Error description.
        message: String,
    },
    /// Wraps a [`vexil_runtime::EncodeError`] from the bit-level encoder.
    #[error("encode error: {0}")]
    Encode(#[from] vexil_runtime::EncodeError),
    /// Wraps a [`vexil_runtime::DecodeError`] from the bit-level decoder.
    #[error("decode error: {0}")]
    Decode(#[from] vexil_runtime::DecodeError),
}

/// Errors from schema-driven encoding (`Value` -> bitpack bytes).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum StoreEncodeError {
    /// A value has the wrong type for its schema-defined context.
    #[error("type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        /// Expected type description.
        expected: String,
        /// Actual value type encountered.
        actual: String,
    },
    /// A type ID exists in the value but not in the registry.
    #[error("unknown type")]
    UnknownTypeId,
    /// A variant name is not defined on the target type.
    #[error("unknown variant `{variant}` on type `{type_name}`")]
    UnknownVariant {
        /// Name of the type.
        type_name: String,
        /// Name of the unknown variant.
        variant: String,
    },
    /// A field name is not defined on the target type.
    #[error("unknown field `{field}` on type `{type_name}`")]
    UnknownField {
        /// Name of the type.
        type_name: String,
        /// Name of the unknown field.
        field: String,
    },
    /// A value exceeds the range of its target bit width.
    #[error("value {value} does not fit in {bits} bits")]
    Overflow {
        /// The overflowing value (as a string).
        value: String,
        /// Target bit width.
        bits: u8,
    },
    /// Wraps a [`vexil_runtime::EncodeError`] from the bit-level writer.
    #[error("bitpack error: {0}")]
    Bitpack(#[from] vexil_runtime::EncodeError),
    /// The requested type name was not found in the schema.
    #[error("type `{type_name}` not found in schema")]
    TypeNotFound {
        /// The type name that was looked up.
        type_name: String,
    },
    /// Nesting depth exceeded the recursion limit.
    #[error("recursion depth exceeded")]
    RecursionLimit,
}

/// Errors from schema-driven decoding (bitpack bytes -> `Value`).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum StoreDecodeError {
    /// A value has the wrong type at the current decode context.
    #[error("type mismatch at {context}: expected {expected}")]
    TypeMismatch {
        /// Context where the mismatch occurred (e.g. `"resolved"`, `"flags"`).
        context: String,
        /// Expected type description.
        expected: String,
    },
    /// A type ID exists in the value but not in the registry.
    #[error("unknown type")]
    UnknownTypeId,
    /// An ordinal in the wire data does not match any enum variant.
    #[error("unknown enum variant ordinal {ordinal} on type `{type_name}`")]
    UnknownVariant {
        /// Name of the enum type.
        type_name: String,
        /// The unknown ordinal value.
        ordinal: u64,
    },
    /// A union discriminant in the wire data does not match any variant.
    #[error("unknown union discriminant {discriminant} on type `{type_name}`")]
    UnknownUnionDiscriminant {
        /// Name of the union type.
        type_name: String,
        /// The unknown discriminant value.
        discriminant: u64,
    },
    /// Wraps a [`vexil_runtime::DecodeError`] from the bit-level reader.
    #[error("bitpack error: {0}")]
    Bitpack(#[from] vexil_runtime::DecodeError),
    /// The requested type name was not found in the schema.
    #[error("type `{type_name}` not found in schema")]
    TypeNotFound {
        /// The type name that was looked up.
        type_name: String,
    },
    /// Nesting depth exceeded the recursion limit.
    #[error("recursion depth exceeded")]
    RecursionLimit,
}
