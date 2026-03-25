//! Import graph discovery and topological ordering for multi-file compilation.
//!
//! [`build_import_graph`] performs a depth-first traversal starting from a root
//! schema, recursively loading all transitive imports via a [`SchemaLoader`].
//! Cycles are detected eagerly; diamond dependencies are deduplicated.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

use crate::ast::Schema;
use crate::diagnostic::Diagnostic;
use crate::ir::CompiledSchema;
use crate::lower::DependencyContext;
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
    let root_ns = namespace_string(&root_schema)?;

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

// ── ProjectResult ────────────────────────────────────────────────────────────

/// The result of compiling a multi-file Vexil project.
#[derive(Debug, Clone)]
pub struct ProjectResult {
    /// Per-namespace compilation results, in topological order.
    pub schemas: Vec<(String, CompiledSchema)>,
    /// All diagnostics across all files.
    pub diagnostics: Vec<Diagnostic>,
}

/// Compile a multi-file Vexil project starting from a root schema.
///
/// Builds the import graph, then lowers and type-checks each namespace in
/// topological order so that dependencies are always compiled before their
/// dependents.
pub fn compile_project(
    root_source: &str,
    root_path: &Path,
    loader: &dyn SchemaLoader,
) -> Result<ProjectResult, ProjectError> {
    let mut graph = build_import_graph(root_source, root_path, loader)?;

    let mut compiled_schemas: HashMap<String, CompiledSchema> = HashMap::new();
    let mut all_diagnostics: Vec<Diagnostic> = Vec::new();
    let mut result_schemas: Vec<(String, CompiledSchema)> = Vec::new();

    let topo_order = graph.topo_order.clone();

    for ns in &topo_order {
        // Remove from graph to take ownership and avoid cloning the Schema.
        let (schema, _source_text, path) = match graph.schemas.remove(ns) {
            Some(entry) => entry,
            None => continue,
        };

        // Build DependencyContext from direct imports only.
        let dep_edges = graph.edges.get(ns).cloned().unwrap_or_default();
        let dep_ctx = DependencyContext {
            schemas: dep_edges
                .iter()
                .filter_map(|dep_ns| {
                    compiled_schemas
                        .get(dep_ns)
                        .map(|cs| (dep_ns.clone(), cs.clone()))
                })
                .collect(),
        };

        let (compiled, lower_diags) = crate::lower::lower_with_deps(&schema, Some(&dep_ctx));

        // Tag diagnostics with source file.
        all_diagnostics.extend(lower_diags.into_iter().map(|d| d.with_file(path.clone())));

        if let Some(mut compiled) = compiled {
            let check_diags = crate::typeck::check(&mut compiled);
            all_diagnostics.extend(check_diags.into_iter().map(|d| d.with_file(path.clone())));

            compiled_schemas.insert(ns.clone(), compiled.clone());
            result_schemas.push((ns.clone(), compiled));
        }
        // If compiled is None, error is already in lower_diags — continue.
    }

    Ok(ProjectResult {
        schemas: result_schemas,
        diagnostics: all_diagnostics,
    })
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

fn namespace_string(schema: &Schema) -> Result<String, ProjectError> {
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
        .ok_or_else(|| ProjectError::ParseError {
            namespace: "<unknown>".to_owned(),
            message: "schema must declare a namespace".to_owned(),
        })
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
    use crate::diagnostic::Severity;
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

    // ── compile_project tests ────────────────────────────────────────────

    #[test]
    fn compile_project_simple_a_imports_b() {
        let mut loader = InMemoryLoader::new();
        loader.schemas.insert(
            "b".to_string(),
            "namespace b\nmessage Dep { y @0 : u32 }".to_string(),
        );

        let root = "namespace a\nimport { Dep } from b\nmessage Root { d @0 : Dep }";
        let result = compile_project(root, &PathBuf::from("<test>"), &loader).unwrap();

        assert_eq!(result.schemas.len(), 2);
        let errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }

    #[test]
    fn compile_project_diamond() {
        let mut loader = InMemoryLoader::new();
        loader.schemas.insert(
            "d".to_string(),
            "namespace d\nenum Base : u8 { X @0 }".to_string(),
        );
        loader.schemas.insert(
            "b".to_string(),
            "namespace b\nimport { Base } from d\nmessage Left { b @0 : Base }".to_string(),
        );
        loader.schemas.insert(
            "c".to_string(),
            "namespace c\nimport { Base } from d\nmessage Right { b @0 : Base }".to_string(),
        );

        let root = "namespace a\nimport { Left } from b\nimport { Right } from c\nmessage Root { l @0 : Left\n  r @1 : Right }";
        let result = compile_project(root, &PathBuf::from("<test>"), &loader).unwrap();

        // All 4 namespaces should be compiled.
        assert_eq!(result.schemas.len(), 4);
        // Verify topological order: d before b and c, b and c before a.
        let positions: HashMap<&str, usize> = result
            .schemas
            .iter()
            .enumerate()
            .map(|(i, (ns, _))| (ns.as_str(), i))
            .collect();
        assert!(positions["d"] < positions["b"]);
        assert!(positions["d"] < positions["c"]);
        assert!(positions["b"] < positions["a"]);
        assert!(positions["c"] < positions["a"]);
    }

    #[test]
    fn compile_project_single_file_no_imports() {
        let loader = InMemoryLoader::new();
        let root = "namespace solo\nmessage Msg { x @0 : u32 }";
        let result = compile_project(root, &PathBuf::from("<test>"), &loader).unwrap();

        assert_eq!(result.schemas.len(), 1);
        assert_eq!(result.schemas[0].0, "solo");
        let errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }

    #[test]
    fn compile_project_diagnostics_have_source_file() {
        let mut loader = InMemoryLoader::new();
        loader.schemas.insert(
            "b".to_string(),
            "namespace b\nmessage Dep { y @0 : u32 }".to_string(),
        );

        let root = "namespace a\nimport { Dep } from b\nmessage Root { d @0 : Dep }";
        let result = compile_project(root, &PathBuf::from("root.vxl"), &loader).unwrap();

        // All diagnostics (if any) should have source_file set.
        for diag in &result.diagnostics {
            assert!(
                diag.source_file.is_some(),
                "diagnostic missing source_file: {diag:?}"
            );
        }
    }
}
