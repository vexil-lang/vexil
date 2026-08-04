//! # Stability: Tier 1
//!
//! Codegen backend trait and shared error type.
//!
//! Implement [`CodegenBackend`] to generate a target programmatically. The
//! `vexilc` binary selects its built-in targets with a closed match and does
//! not discover third-party backends at runtime.

/// Typed, target-independent projection of portable trait-function bodies.
pub mod portable;

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
///
/// A backend should treat [`CompiledSchema::declarations`](crate::CompiledSchema::declarations)
/// as the local declarations to emit. Other registry entries may come from
/// imports. Project output paths must be relative to the caller's output
/// directory, stable for the same [`ProjectResult`], and free of parent
/// traversal. Returning a [`BTreeMap`] makes write order deterministic, but the
/// backend still owns naming, collision handling, imports, and scaffolding.
///
/// `vexil-lang` does not write the generated output. The caller decides whether
/// and where returned files are written.
///
/// # Minimal backend
///
/// This backend emits one sorted declaration list per namespace. The example
/// exercises both the single-schema and project contracts.
///
/// ```
/// use std::collections::BTreeMap;
/// use std::path::{Path, PathBuf};
///
/// use vexil_lang::resolve::InMemoryLoader;
/// use vexil_lang::{
///     compile, compile_project, CodegenBackend, CodegenError, CompiledSchema,
///     ProjectResult,
/// };
///
/// struct NamesBackend;
///
/// impl CodegenBackend for NamesBackend {
///     fn name(&self) -> &str {
///         "names"
///     }
///
///     fn file_extension(&self) -> &str {
///         "names"
///     }
///
///     fn generate(&self, schema: &CompiledSchema) -> Result<String, CodegenError> {
///         let mut names = schema.type_names();
///         names.sort_unstable();
///         Ok(format!("{}\n", names.join("\n")))
///     }
///
///     fn generate_project(
///         &self,
///         project: &ProjectResult,
///     ) -> Result<BTreeMap<PathBuf, String>, CodegenError> {
///         let mut files = BTreeMap::new();
///         for (namespace, schema) in &project.schemas {
///             let mut path = namespace.split('.').collect::<PathBuf>();
///             path.set_extension(self.file_extension());
///             files.insert(path, self.generate(schema)?);
///         }
///         Ok(files)
///     }
/// }
///
/// let source = "namespace demo.single\nmessage Ping { id @0 : u32 }";
/// let result = compile(source);
/// assert!(!result.has_errors(), "{:?}", result.diagnostics);
/// let compiled = result.compiled.expect("successful compilation has IR");
/// assert_eq!(NamesBackend.generate(&compiled)?, "Ping\n");
///
/// let project = compile_project(
///     source,
///     Path::new("demo/single.vexil"),
///     &InMemoryLoader::new(),
/// )
/// .expect("valid project");
/// assert!(project.diagnostics.is_empty(), "{:?}", project.diagnostics);
/// let files = NamesBackend.generate_project(&project)?;
/// assert!(files.contains_key(Path::new("demo/single.names")));
/// # Ok::<(), CodegenError>(())
/// ```
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
    /// module-scaffolding files (e.g. `mod.rs`, `index.ts`). It must not write
    /// the returned output files itself or return absolute paths or paths
    /// containing `..`.
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
