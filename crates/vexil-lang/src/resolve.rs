//! Schema resolution: load schema source text by namespace.
//!
//! A *namespace* is an ordered slice of identifier segments, e.g.
//! `["foo", "bar", "types"]`.  A [`SchemaLoader`] knows how to
//! translate that into source text and a canonical [`PathBuf`] that
//! the rest of the compiler can use for diagnostics and cycle detection.

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

// ── LoadError ────────────────────────────────────────────────────────────────

/// Errors that can occur while loading a schema by namespace.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum LoadError {
    /// No loader could locate the requested namespace.
    NotFound { namespace: String },
    /// The namespace matched files in more than one search root.
    Ambiguous {
        namespace: String,
        paths: Vec<PathBuf>,
    },
    /// An I/O error occurred while reading the file.
    Io { namespace: String, message: String },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::NotFound { namespace } => {
                write!(f, "schema not found: `{namespace}`")
            }
            LoadError::Ambiguous { namespace, paths } => {
                let list: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
                write!(
                    f,
                    "schema `{namespace}` is ambiguous — found in: {}",
                    list.join(", ")
                )
            }
            LoadError::Io { namespace, message } => {
                write!(f, "I/O error loading `{namespace}`: {message}")
            }
        }
    }
}

// ── SchemaLoader trait ───────────────────────────────────────────────────────

/// Abstraction over schema source retrieval.
///
/// Implementations are responsible for translating a namespace (e.g.
/// `["foo", "bar", "types"]`) into `(source_text, canonical_path)`.
/// The `canonical_path` is used for diagnostics and import cycle detection
/// only — it need not correspond to a real filesystem entry.
pub trait SchemaLoader {
    /// Load the schema identified by `namespace`.
    ///
    /// Returns `(source_text, canonical_path)` on success, or a
    /// [`LoadError`] describing why the load failed.
    fn load(&self, namespace: &[&str]) -> Result<(String, PathBuf), LoadError>;
}

// ── InMemoryLoader ───────────────────────────────────────────────────────────

/// A loader backed by an in-memory map — primarily for unit tests.
///
/// Keys are dotted namespace strings (`"foo.bar.types"`).  The
/// `schemas` field is `pub` so that test modules in other crates can
/// populate it directly without going through a builder API.
#[derive(Debug, Default, Clone)]
pub struct InMemoryLoader {
    /// Dotted namespace string → source text.
    pub schemas: HashMap<String, String>,
}

impl InMemoryLoader {
    /// Create an empty loader.
    pub fn new() -> Self {
        Self::default()
    }
}

impl SchemaLoader for InMemoryLoader {
    fn load(&self, namespace: &[&str]) -> Result<(String, PathBuf), LoadError> {
        let key = namespace.join(".");
        match self.schemas.get(&key) {
            Some(src) => {
                let path = PathBuf::from(format!("<memory>/{key}"));
                Ok((src.clone(), path))
            }
            None => Err(LoadError::NotFound { namespace: key }),
        }
    }
}

// ── FilesystemLoader ─────────────────────────────────────────────────────────

/// A loader that searches one or more root directories for `.vexil` files.
///
/// Namespace segments are joined with the platform directory separator:
/// `["foo", "bar", "types"]` → `{root}/foo/bar/types.vexil`.
///
/// If the file exists under **multiple** roots, [`LoadError::Ambiguous`] is
/// returned.  If it exists under none, [`LoadError::NotFound`] is returned.
#[derive(Debug, Clone)]
pub struct FilesystemLoader {
    roots: Vec<PathBuf>,
}

impl FilesystemLoader {
    /// Create a loader that searches `roots` in order.
    pub fn new(roots: impl IntoIterator<Item = impl Into<PathBuf>>) -> Self {
        Self {
            roots: roots.into_iter().map(Into::into).collect(),
        }
    }
}

impl SchemaLoader for FilesystemLoader {
    fn load(&self, namespace: &[&str]) -> Result<(String, PathBuf), LoadError> {
        let dotted = namespace.join(".");
        // Build the relative path: segments joined by OS sep, ".vexil" extension.
        let mut rel = PathBuf::new();
        for seg in namespace {
            rel.push(seg);
        }
        rel.set_extension("vexil");

        // Collect every root that contains the file.
        let mut found: Vec<PathBuf> = self
            .roots
            .iter()
            .map(|root| root.join(&rel))
            .filter(|p| p.is_file())
            .collect();

        match found.len() {
            0 => Err(LoadError::NotFound { namespace: dotted }),
            1 => {
                let path = found.remove(0);
                let src = std::fs::read_to_string(&path).map_err(|e| LoadError::Io {
                    namespace: dotted,
                    message: e.to_string(),
                })?;
                Ok((src, path))
            }
            _ => Err(LoadError::Ambiguous {
                namespace: dotted,
                paths: found,
            }),
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // ── InMemoryLoader ────────────────────────────────────────────────────

    #[test]
    fn in_memory_loader_found() {
        let mut loader = InMemoryLoader::new();
        loader
            .schemas
            .insert("foo.bar.types".into(), "schema foo.bar.types {}".into());

        let (src, path) = loader.load(&["foo", "bar", "types"]).unwrap();
        assert!(
            src.contains("foo.bar.types"),
            "source should contain namespace"
        );
        assert_eq!(path, PathBuf::from("<memory>/foo.bar.types"));
    }

    #[test]
    fn in_memory_loader_not_found() {
        let loader = InMemoryLoader::new();
        let err = loader.load(&["does", "not", "exist"]).unwrap_err();
        assert!(
            matches!(err, LoadError::NotFound { ref namespace } if namespace == "does.not.exist"),
            "expected NotFound, got {err:?}"
        );
    }

    // ── FilesystemLoader ──────────────────────────────────────────────────

    #[test]
    fn filesystem_loader_finds_file() {
        let dir = tempdir();
        let nested = dir.path().join("foo").join("bar");
        fs::create_dir_all(&nested).unwrap();
        let file = nested.join("types.vexil");
        fs::write(&file, "schema foo.bar.types {}").unwrap();

        let loader = FilesystemLoader::new([dir.path()]);
        let (src, path) = loader.load(&["foo", "bar", "types"]).unwrap();
        assert!(src.contains("foo.bar.types"));
        assert_eq!(path, file);
    }

    #[test]
    fn filesystem_loader_not_found() {
        let dir = tempdir();
        let loader = FilesystemLoader::new([dir.path()]);
        let err = loader.load(&["missing", "schema"]).unwrap_err();
        assert!(
            matches!(err, LoadError::NotFound { .. }),
            "expected NotFound, got {err:?}"
        );
    }

    #[test]
    fn filesystem_loader_ambiguous() {
        let dir1 = tempdir();
        let dir2 = tempdir();

        // Create the same file in both roots.
        for dir in [dir1.path(), dir2.path()] {
            let nested = dir.join("net").join("types");
            fs::create_dir_all(&nested).unwrap();
            fs::write(nested.join("core.vexil"), "schema net.types.core {}").unwrap();
        }

        let loader = FilesystemLoader::new([dir1.path(), dir2.path()]);
        let err = loader.load(&["net", "types", "core"]).unwrap_err();
        assert!(
            matches!(&err, LoadError::Ambiguous { namespace, paths }
                if namespace == "net.types.core" && paths.len() == 2),
            "expected Ambiguous with 2 paths, got {err:?}"
        );
    }

    // ── helpers ───────────────────────────────────────────────────────────

    /// Minimal temp-dir wrapper that cleans up on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn path(&self) -> &PathBuf {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn tempdir() -> TempDir {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("vexil_resolve_test_{n}_{}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        TempDir(path)
    }
}
