//! # Stability: Tier 2
//!
use crate::span::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

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

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub span: Span,
    pub class: ErrorClass,
    pub message: String,
    pub source_file: Option<std::path::PathBuf>,
}

impl Diagnostic {
    pub fn error(span: Span, class: ErrorClass, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            span,
            class,
            message: message.into(),
            source_file: None,
        }
    }

    pub fn warning(span: Span, class: ErrorClass, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            span,
            class,
            message: message.into(),
            source_file: None,
        }
    }

    pub fn with_file(mut self, path: std::path::PathBuf) -> Self {
        self.source_file = Some(path);
        self
    }
}
