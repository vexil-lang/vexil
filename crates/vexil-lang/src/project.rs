//! Import graph discovery and topological ordering for multi-file compilation.
//!
//! [`build_import_graph`] performs a depth-first traversal starting from a root
//! schema, recursively loading all transitive imports via a [`SchemaLoader`].
//! Cycles are detected eagerly; diamond dependencies are deduplicated.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

use crate::ast::Schema;
use crate::resolve::{LoadError, SchemaLoader};

// ── ProjectError ─────────────────────────────────────────────────────────────

/// Errors that can occur while building the import graph.
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectError {
    /// A circular import chain was detected.
    CircularImport { chain: Vec<String> },
    /// A schema could not be loaded.
    Load(LoadError),
    /// A schema loaded successfully but failed to parse.
    ParseError { namespace: String, message: String },
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectError::CircularImport { chain } => {
                write!(f, "circular import detected: {}", chain.join(" → "))
            }
            ProjectError::Load(err) => write!(f, "load error: {err}"),
            ProjectError::ParseError { namespace, message } => {
                write!(f, "parse error in `{namespace}`: {message}")
            }
        }
    }
}

impl From<LoadError> for ProjectError {
    fn from(err: LoadError) -> Self {
        ProjectError::Load(err)
    }
}

// ── ImportGraph ───────────────────────────────────────────────────────────────

/// The fully resolved import graph for a Vexil project.
#[derive(Debug, Clone)]
pub struct ImportGraph {
    /// `namespace → (parsed schema, source text, canonical path)`
    pub schemas: HashMap<String, (Schema, String, PathBuf)>,
    /// `namespace → list of dependency namespaces`
    pub edges: HashMap<String, Vec<String>>,
    /// Namespaces in topological order: dependencies appear before dependents.
    pub topo_order: Vec<String>,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Build the import graph starting from `root_source`.
///
/// The root schema is pre-parsed from `root_source` / `root_path`; all
/// transitive imports are fetched via `loader`.
///
/// Returns an [`ImportGraph`] whose [`topo_order`] lists namespaces in
/// dependency-first order, suitable for driving compilation.
pub fn build_import_graph(
    root_source: &str,
    root_path: &Path,
    loader: &dyn SchemaLoader,
) -> Result<ImportGraph, ProjectError> {
    let mut graph = ImportGraph {
        schemas: HashMap::new(),
        edges: HashMap::new(),
        topo_order: Vec::new(),
    };

    // Parse the root schema.
    let root_schema = parse_source(root_source, "<root>")?;
    let root_ns = namespace_string(&root_schema);

    // Insert root before DFS so we have it available.
    graph.schemas.insert(
        root_ns.clone(),
        (
            root_schema.clone(),
            root_source.to_owned(),
            root_path.to_path_buf(),
        ),
    );

    let mut visited: HashSet<String> = HashSet::new();
    let mut stack: Vec<String> = Vec::new();

    dfs(
        &root_ns,
        &root_schema,
        loader,
        &mut graph,
        &mut visited,
        &mut stack,
    )?;

    Ok(graph)
}

// ── DFS ───────────────────────────────────────────────────────────────────────

fn dfs(
    ns: &str,
    schema: &Schema,
    loader: &dyn SchemaLoader,
    graph: &mut ImportGraph,
    visited: &mut HashSet<String>,
    stack: &mut Vec<String>,
) -> Result<(), ProjectError> {
    // Mark that we are currently processing this namespace.
    stack.push(ns.to_owned());

    let deps = import_namespaces(schema);
    graph.edges.insert(ns.to_owned(), deps.clone());

    for dep_ns in &deps {
        // Cycle check: if dep is on the current DFS stack → circular import.
        if let Some(cycle_start) = stack.iter().position(|s| s == dep_ns) {
            let mut chain: Vec<String> = stack[cycle_start..].to_vec();
            chain.push(dep_ns.clone());
            return Err(ProjectError::CircularImport { chain });
        }

        // Diamond dedup: already fully processed.
        if visited.contains(dep_ns.as_str()) {
            continue;
        }

        // Load and parse the dependency.
        let seg: Vec<&str> = dep_ns.split('.').collect();
        let (src, path) = loader.load(&seg)?;
        let dep_schema = parse_source(&src, dep_ns)?;

        // Store before recursing.
        graph
            .schemas
            .insert(dep_ns.clone(), (dep_schema.clone(), src, path));

        dfs(dep_ns, &dep_schema, loader, graph, visited, stack)?;
    }

    // Pop from stack and mark as fully visited (post-order).
    stack.pop();
    visited.insert(ns.to_owned());
    graph.topo_order.push(ns.to_owned());

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_source(source: &str, namespace_hint: &str) -> Result<Schema, ProjectError> {
    let result = crate::parse(source);

    // Any error-severity diagnostic means the parse failed.
    let first_error = result
        .diagnostics
        .iter()
        .find(|d| d.severity == crate::diagnostic::Severity::Error)
        .map(|d| d.message.clone());

    if let Some(msg) = first_error {
        return Err(ProjectError::ParseError {
            namespace: namespace_hint.to_owned(),
            message: msg,
        });
    }

    // The parser always returns Some(schema), but handle None defensively.
    result.schema.ok_or_else(|| ProjectError::ParseError {
        namespace: namespace_hint.to_owned(),
        message: "no schema produced".to_owned(),
    })
}

fn namespace_string(schema: &Schema) -> String {
    schema
        .namespace
        .as_ref()
        .map(|ns| {
            ns.node
                .path
                .iter()
                .map(|s| s.node.as_str())
                .collect::<Vec<_>>()
                .join(".")
        })
        .unwrap_or_default()
}

fn import_namespaces(schema: &Schema) -> Vec<String> {
    schema
        .imports
        .iter()
        .map(|imp| {
            imp.node
                .path
                .iter()
                .map(|s| s.node.as_str())
                .collect::<Vec<_>>()
                .join(".")
        })
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::InMemoryLoader;
    use std::path::PathBuf;

    fn make_loader(entries: &[(&str, &str)]) -> InMemoryLoader {
        let mut loader = InMemoryLoader::new();
        for (ns, src) in entries {
            loader.schemas.insert(ns.to_string(), src.to_string());
        }
        loader
    }

    /// A imports B → topo order has B before A.
    #[test]
    fn simple_import_graph() {
        let loader = make_loader(&[("b", "namespace b")]);

        let root_src = "namespace a\nimport b";
        let root_path = PathBuf::from("<memory>/a");

        let graph = build_import_graph(root_src, &root_path, &loader)
            .expect("should build graph without error");

        assert_eq!(graph.schemas.len(), 2, "should have a and b");
        assert_eq!(graph.topo_order.len(), 2);

        let pos_a = graph.topo_order.iter().position(|s| s == "a").unwrap();
        let pos_b = graph.topo_order.iter().position(|s| s == "b").unwrap();
        assert!(pos_b < pos_a, "b must appear before a in topo_order");
    }

    /// A imports B, B imports A → CircularImport error.
    #[test]
    fn direct_cycle_detected() {
        let loader = make_loader(&[
            ("b", "namespace b\nimport a"),
            ("a", "namespace a\nimport b"),
        ]);

        let root_src = loader.schemas["a"].clone();
        let root_path = PathBuf::from("<memory>/a");

        let err = build_import_graph(&root_src, &root_path, &loader)
            .expect_err("should detect circular import");

        assert!(
            matches!(err, ProjectError::CircularImport { .. }),
            "expected CircularImport, got {err:?}"
        );
    }

    /// A → B → C → A → CircularImport error.
    #[test]
    fn transitive_cycle_detected() {
        let loader = make_loader(&[
            ("b", "namespace b\nimport c"),
            ("c", "namespace c\nimport a"),
            ("a", "namespace a\nimport b"),
        ]);

        let root_src = loader.schemas["a"].clone();
        let root_path = PathBuf::from("<memory>/a");

        let err = build_import_graph(&root_src, &root_path, &loader)
            .expect_err("should detect transitive cycle");

        assert!(
            matches!(err, ProjectError::CircularImport { .. }),
            "expected CircularImport, got {err:?}"
        );
    }

    /// A imports B and C, both import D → 4 entries, D before A.
    #[test]
    fn diamond_dependency() {
        let loader = make_loader(&[
            ("b", "namespace b\nimport d"),
            ("c", "namespace c\nimport d"),
            ("d", "namespace d"),
        ]);

        // A imports both B and C.
        let root_src = "namespace a\nimport b\nimport c";
        let root_path = PathBuf::from("<memory>/a");

        let graph = build_import_graph(root_src, &root_path, &loader)
            .expect("should build graph without error");

        assert_eq!(graph.schemas.len(), 4, "should have a, b, c, d");
        assert_eq!(
            graph.topo_order.len(),
            4,
            "topo_order should have 4 entries"
        );

        let pos_d = graph.topo_order.iter().position(|s| s == "d").unwrap();
        let pos_a = graph.topo_order.iter().position(|s| s == "a").unwrap();
        assert!(pos_d < pos_a, "d must appear before a in topo_order");
    }
}
