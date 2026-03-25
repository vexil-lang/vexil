use crate::span::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

/// Category of error, for structured tooling.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorClass {
    Syntax,
    Semantic,
    Type,
    Import,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub class: ErrorClass,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn error(class: ErrorClass, message: impl Into<String>, span: Span) -> Self {
        Self {
            severity: Severity::Error,
            class,
            message: message.into(),
            span,
        }
    }

    pub fn warning(class: ErrorClass, message: impl Into<String>, span: Span) -> Self {
        Self {
            severity: Severity::Warning,
            class,
            message: message.into(),
            span,
        }
    }
}
