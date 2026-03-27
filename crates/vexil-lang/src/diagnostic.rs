//! # Stability: Tier 2
//!
//! Diagnostic types for reporting errors and warnings from all compiler stages.

use crate::span::Span;

/// The severity level of a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    /// A fatal error that prevents compilation from proceeding.
    Error,
    /// A non-fatal warning that does not block compilation.
    Warning,
}

/// Classification of a diagnostic by its root cause.
///
/// Used for programmatic matching on error types (e.g. in test assertions
/// or IDE quick-fix suggestions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorClass {
    // Lexer
    InvalidCharacter,
    InvalidEscape,
    UnterminatedString,

    // Structure
    MissingNamespace,
    DuplicateNamespace,
    ImportAfterDecl,
    ImportNamedAliasedCombined,

    // Namespace
    NamespaceInvalidComponent,
    NamespaceReserved,
    NamespaceEmpty,

    // Declaration
    DeclNameInvalid,
    DeclNameDuplicate,

    // Field
    FieldNameInvalid,
    FieldNameDuplicate,
    OrdinalDuplicate,
    OrdinalTooLarge,
    OrdinalReusedAfterRemoved,

    // Type
    UnknownType,
    ConfigTypeAsField,
    NewtypeOverNewtype,
    NewtypeOverConfig,
    InvalidMapKey,

    // Config
    ConfigMissingDefault,
    ConfigHasOrdinal,
    ConfigInvalidType,
    ConfigEncodingAnnotation,

    // Enum/Flags/Union
    EnumOrdinalDuplicate,
    EnumOrdinalTooLarge,
    EnumBackingTooNarrow,
    EnumBackingInvalid,
    EnumVariantNameInvalid,
    FlagsBitTooHigh,
    UnionOrdinalDuplicate,
    UnionOrdinalTooLarge,
    UnionVariantNameInvalid,

    // Annotation
    DuplicateAnnotation,
    NonExhaustiveInvalidTarget,
    DeprecatedMissingReason,
    RemovedMissingReason,
    LimitInvalidTarget,
    LimitExceedsGlobal,
    LimitZero,
    VarintInvalidTarget,
    ZigzagInvalidTarget,
    VarintZigzagCombined,
    DeltaInvalidTarget,
    TypeValueOverflow,
    VersionAfterNamespace,
    VersionDuplicate,
    VersionInvalidSemver,

    // IR / Type checker
    RecursiveTypeInfinite,
    EncodingTypeMismatch,
    UnresolvedType,

    // Generic
    UnexpectedToken,
    UnexpectedEof,
}

/// A compiler diagnostic (error or warning) with location and classification.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    /// Whether this is an error or a warning.
    pub severity: Severity,
    /// Source location where the diagnostic was raised.
    pub span: Span,
    /// Machine-readable error classification.
    pub class: ErrorClass,
    /// Human-readable description of the problem.
    pub message: String,
    /// Source file path, if known (set during multi-file compilation).
    pub source_file: Option<std::path::PathBuf>,
}

impl Diagnostic {
    /// Create an error-severity diagnostic.
    pub fn error(span: Span, class: ErrorClass, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            span,
            class,
            message: message.into(),
            source_file: None,
        }
    }

    /// Create a warning-severity diagnostic.
    pub fn warning(span: Span, class: ErrorClass, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            span,
            class,
            message: message.into(),
            source_file: None,
        }
    }

    /// Attach a source file path to this diagnostic (builder pattern).
    pub fn with_file(mut self, path: std::path::PathBuf) -> Self {
        self.source_file = Some(path);
        self
    }
}
