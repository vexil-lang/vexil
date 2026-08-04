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
use std::path::{Path, PathBuf};

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
/// Use [`ProjectOutputBuilder`] to enforce the shared portable path and
/// collision contract while assembling that map.
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
///     ProjectOutputBuilder, ProjectResult,
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
///         let mut files = ProjectOutputBuilder::new();
///         for (namespace, schema) in &project.schemas {
///             let mut path = namespace.split('.').collect::<PathBuf>();
///             path.set_extension(self.file_extension());
///             files.add(path, self.generate(schema)?)?;
///         }
///         Ok(files.finish())
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

/// A checked, deterministic collection of generated project files.
///
/// The builder rejects non-portable paths and case-insensitive collisions
/// before returning a [`BTreeMap`]. Existing third-party backends may continue
/// to construct their maps directly, but maintained backends use this type so
/// an invalid insertion cannot silently replace an earlier file.
#[derive(Debug, Default)]
pub struct ProjectOutputBuilder {
    files: BTreeMap<PathBuf, String>,
    identities: BTreeMap<String, PathBuf>,
}

impl ProjectOutputBuilder {
    /// Create an empty project output.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add one generated file after validating its portable relative path.
    ///
    /// Accepted components contain only ASCII letters, digits, `_`, `-`, and
    /// `.`. Rooted paths, drive prefixes, `.` and `..`, empty components,
    /// Windows device names, mixed separators, and case-insensitive collisions
    /// are rejected.
    pub fn add(
        &mut self,
        path: impl Into<PathBuf>,
        content: impl Into<String>,
    ) -> Result<(), OutputPathError> {
        let requested_path = path.into();
        let checked = checked_output_path(&requested_path)?;
        if let Some(existing) = self.identities.get(&checked.identity) {
            return Err(OutputPathError::Duplicate {
                path: requested_path,
                existing: existing.clone(),
            });
        }

        self.identities
            .insert(checked.identity, checked.path.clone());
        self.files.insert(checked.path, content.into());
        Ok(())
    }

    /// Finish the output after all fallible insertions have succeeded.
    #[must_use]
    pub fn finish(self) -> BTreeMap<PathBuf, String> {
        self.files
    }
}

/// Validate and reconstruct every path in a generated project map.
///
/// This is defense in depth for maps returned by backends that do not use
/// [`ProjectOutputBuilder`]. It consumes the input and returns a map whose keys
/// use host-native separators reconstructed from the portable components. A
/// caller must write this returned map rather than the original keys. The
/// function detects unsafe paths and portable case-insensitive collisions, but
/// cannot recover an exact duplicate that was already overwritten while
/// constructing the input map.
pub fn validate_project_output(
    files: BTreeMap<PathBuf, String>,
) -> Result<BTreeMap<PathBuf, String>, OutputPathError> {
    let mut output = ProjectOutputBuilder::new();
    for (path, content) in files {
        output.add(path, content)?;
    }
    Ok(output.finish())
}

/// A portable generated-output path violation.
///
/// The [`diagnostic_id`](OutputPathError::diagnostic_id) is stable for callers
/// that need to classify failures without matching human-readable wording.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum OutputPathError {
    /// The output path contains no components.
    #[error("codegen-output-empty-path: generated output path is empty")]
    Empty,

    /// The path is rooted or has a platform prefix such as a Windows drive.
    #[error("codegen-output-rooted-path: generated output path {path:?} is rooted or prefixed")]
    RootedOrPrefixed {
        /// Rejected path.
        path: PathBuf,
    },

    /// The path contains `.` or `..` traversal.
    #[error("codegen-output-traversal: generated output path {path:?} contains `{component}`")]
    Traversal {
        /// Rejected path.
        path: PathBuf,
        /// Traversal component.
        component: String,
    },

    /// A component is outside the portable generated-output grammar.
    #[error(
        "codegen-output-non-portable-component: generated output path {path:?} contains non-portable component `{component}`"
    )]
    NonPortableComponent {
        /// Rejected path.
        path: PathBuf,
        /// Rejected component or a description when it is not UTF-8.
        component: String,
    },

    /// Two paths have the same portable case-insensitive identity.
    #[error(
        "codegen-output-duplicate-path: generated output path {path:?} collides with {existing:?}"
    )]
    Duplicate {
        /// Later path that collided.
        path: PathBuf,
        /// Earlier path with the same identity.
        existing: PathBuf,
    },
}

impl OutputPathError {
    /// Stable diagnostic identifier for CLI and programmatic classification.
    #[must_use]
    pub const fn diagnostic_id(&self) -> &'static str {
        match self {
            Self::Empty => "codegen-output-empty-path",
            Self::RootedOrPrefixed { .. } => "codegen-output-rooted-path",
            Self::Traversal { .. } => "codegen-output-traversal",
            Self::NonPortableComponent { .. } => "codegen-output-non-portable-component",
            Self::Duplicate { .. } => "codegen-output-duplicate-path",
        }
    }
}

impl From<OutputPathError> for CodegenError {
    fn from(error: OutputPathError) -> Self {
        Self::BackendSpecific(Box::new(error))
    }
}

struct CheckedOutputPath {
    path: PathBuf,
    identity: String,
}

fn checked_output_path(path: &Path) -> Result<CheckedOutputPath, OutputPathError> {
    let raw = path
        .to_str()
        .ok_or_else(|| OutputPathError::NonPortableComponent {
            path: path.to_path_buf(),
            component: "<non-UTF-8>".to_string(),
        })?;
    if raw.is_empty() {
        return Err(OutputPathError::Empty);
    }

    let bytes = raw.as_bytes();
    let has_drive_prefix = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    if raw.starts_with('/') || raw.starts_with('\\') || has_drive_prefix {
        return Err(OutputPathError::RootedOrPrefixed {
            path: path.to_path_buf(),
        });
    }

    if raw.contains('/') && raw.contains('\\') {
        return Err(OutputPathError::NonPortableComponent {
            path: path.to_path_buf(),
            component: "<mixed-separators>".to_string(),
        });
    }

    let mut portable = PathBuf::new();
    let mut identity = Vec::new();
    for component in raw.split(['/', '\\']) {
        if component == "." || component == ".." {
            return Err(OutputPathError::Traversal {
                path: path.to_path_buf(),
                component: component.to_string(),
            });
        }
        if !portable_component(component) {
            return Err(OutputPathError::NonPortableComponent {
                path: path.to_path_buf(),
                component: if component.is_empty() {
                    "<empty>".to_string()
                } else {
                    component.to_string()
                },
            });
        }

        portable.push(component);
        identity.push(component.to_ascii_lowercase());
    }

    Ok(CheckedOutputPath {
        path: portable,
        identity: identity.join("/"),
    })
}

fn portable_component(component: &str) -> bool {
    if component.is_empty()
        || component.ends_with('.')
        || !component.is_ascii()
        || !component
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return false;
    }

    let basename = component
        .split_once('.')
        .map_or(component, |(basename, _)| basename)
        .to_ascii_uppercase();
    !matches!(basename.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        && !(basename.len() == 4
            && (basename.starts_with("COM") || basename.starts_with("LPT"))
            && matches!(basename.as_bytes()[3], b'1'..=b'9'))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_output_builder_accepts_nested_portable_paths() {
        let mut output = ProjectOutputBuilder::new();
        output
            .add("demo/generated.rs", "source")
            .expect("portable path");
        output
            .add("other\\generated.rs", "other source")
            .expect("portable alternate separator");
        let files = output.finish();
        assert_eq!(
            files.get(Path::new("demo/generated.rs")),
            Some(&"source".to_string())
        );
        assert_eq!(
            files.get(Path::new("other/generated.rs")),
            Some(&"other source".to_string())
        );
    }

    #[test]
    fn project_output_builder_rejects_empty_path() {
        let mut output = ProjectOutputBuilder::new();
        let error = output.add("", "source").expect_err("empty path");
        assert_eq!(error.diagnostic_id(), "codegen-output-empty-path");
    }

    #[test]
    fn project_output_builder_rejects_rooted_and_prefixed_paths() {
        for path in [
            "/escape.rs",
            "\\escape.rs",
            "C:\\escape.rs",
            "c:relative.rs",
        ] {
            let mut output = ProjectOutputBuilder::new();
            let error = output.add(path, "source").expect_err("rooted path");
            assert_eq!(error.diagnostic_id(), "codegen-output-rooted-path");
        }
    }

    #[test]
    fn project_output_builder_rejects_traversal() {
        for path in ["./generated.rs", "demo/../generated.rs"] {
            let mut output = ProjectOutputBuilder::new();
            let error = output.add(path, "source").expect_err("traversal path");
            assert_eq!(error.diagnostic_id(), "codegen-output-traversal");
        }
    }

    #[test]
    fn project_output_builder_rejects_non_portable_components() {
        for path in [
            "demo//generated.rs",
            "demo/generated.rs/",
            "demo/generated .rs",
            "demo/généré.rs",
            "demo/CON.txt",
            "demo/lpt9.log",
            "demo/mixed\\generated.rs",
        ] {
            let mut output = ProjectOutputBuilder::new();
            let error = output.add(path, "source").expect_err("non-portable path");
            assert_eq!(
                error.diagnostic_id(),
                "codegen-output-non-portable-component",
                "{path}"
            );
        }
    }

    #[test]
    fn project_output_builder_rejects_exact_and_case_folded_duplicates() {
        let mut output = ProjectOutputBuilder::new();
        output.add("demo/File.rs", "first").expect("first path");
        let error = output
            .add("demo/file.rs", "second")
            .expect_err("case-folded duplicate");
        assert_eq!(error.diagnostic_id(), "codegen-output-duplicate-path");

        let mut exact_output = ProjectOutputBuilder::new();
        exact_output
            .add("demo/file.rs", "first")
            .expect("first exact path");
        let exact_error = exact_output
            .add("demo/file.rs", "second")
            .expect_err("exact duplicate");
        assert_eq!(exact_error.diagnostic_id(), "codegen-output-duplicate-path");
    }

    #[test]
    fn project_output_validation_detects_case_folded_map_collisions() {
        let files = BTreeMap::from([
            (PathBuf::from("demo/File.rs"), String::new()),
            (PathBuf::from("demo/file.rs"), String::new()),
        ]);
        let error = validate_project_output(files).expect_err("case-folded duplicate");
        assert_eq!(error.diagnostic_id(), "codegen-output-duplicate-path");
    }

    #[test]
    fn output_path_error_uses_existing_backend_specific_boundary() {
        let error = CodegenError::from(OutputPathError::Empty);
        let CodegenError::BackendSpecific(source) = error else {
            panic!("output path error must use BackendSpecific");
        };
        assert_eq!(
            source
                .downcast_ref::<OutputPathError>()
                .map(OutputPathError::diagnostic_id),
            Some("codegen-output-empty-path")
        );
    }
}
