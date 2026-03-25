use crate::ast::Schema;
use crate::diagnostic::Diagnostic;

/// Validate a parsed Schema, returning any semantic diagnostics.
/// Stub implementation — returns empty vec.
pub fn validate(_schema: &Schema) -> Vec<Diagnostic> {
    Vec::new()
}
