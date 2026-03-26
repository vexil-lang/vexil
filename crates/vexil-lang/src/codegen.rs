//! # Stability: Tier 1
//!
//! Codegen backend trait and shared error type. Implement [`CodegenBackend`]
//! to add a new code-generation target to `vexilc`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::ir::CompiledSchema;
use crate::project::ProjectResult;

/// A pluggable code-generation backend.
///
/// Each backend translates compiled Vexil schemas into source code for a
/// specific target language.  Implement this trait to add support for a new
/// language.
///
/// Backends are used in two modes:
/// - **Single-file** via [`generate`](CodegenBackend::generate) — for REPL,
///   quick checks, or single-schema compilation.
/// - **Project-level** via [`generate_project`](CodegenBackend::generate_project)
///   — for multi-file projects.  The backend owns cross-file import strategy
///   and output file layout.
pub trait CodegenBackend {
    /// Backend identifier, e.g. `"rust"`, `"typescript"`.
    fn name(&self) -> &str;

    /// File extension for generated files, e.g. `"rs"`, `"ts"`.
    fn file_extension(&self) -> &str;

    /// Generate code for a single compiled schema.
    fn generate(&self, compiled: &CompiledSchema) -> Result<String, CodegenError>;

    /// Generate all files for a multi-file project.
    ///
    /// Returns a map from relative output path to file content.
    /// The backend is responsible for cross-file import statements and
    /// module-scaffolding files (e.g. `mod.rs`, `index.ts`).
    fn generate_project(
        &self,
        result: &ProjectResult,
    ) -> Result<BTreeMap<PathBuf, String>, CodegenError>;
}

/// Errors that can occur during code generation.
#[derive(Debug, thiserror::Error)]
pub enum CodegenError {
    /// The backend does not support a type used in the schema.
    #[error("unsupported type `{type_name}` in {backend} backend")]
    UnsupportedType {
        /// Name of the unsupported type.
        type_name: String,
        /// Backend that encountered the error.
        backend: String,
    },

    /// A required annotation is missing from the schema.
    #[error("missing required annotation `{annotation}` ({context})")]
    MissingAnnotation {
        /// The annotation that was expected.
        annotation: String,
        /// Where it was expected.
        context: String,
    },

    /// An I/O error occurred during code generation.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A backend-specific error not covered by the common variants.
    #[error("backend error: {0}")]
    BackendSpecific(Box<dyn std::error::Error + Send + Sync>),
}
