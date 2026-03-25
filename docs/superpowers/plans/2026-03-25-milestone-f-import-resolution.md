# Milestone F: Multi-File Import Resolution — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve imports across multiple `.vexil` files, compile them incrementally in topological order, and generate one `.rs` file per `.vexil` file with cross-file `use` statements.

**Architecture:** A `SchemaLoader` trait abstracts file resolution. An import graph builder (DFS) discovers all dependencies and detects cycles. Files are compiled in topological order, with dependency types cloned into each file's `TypeRegistry` via ID remapping. Codegen produces namespace-mirrored output with `mod.rs` scaffolding.

**Tech Stack:** Rust, vexil-lang (existing compiler), vexil-codegen (existing codegen)

**Spec:** `docs/superpowers/specs/2026-03-25-milestone-f-import-resolution-design.md`

---

## Codebase Reference

Key types and signatures the implementer must know:

**AST (`crates/vexil-lang/src/ast/mod.rs`):**
- `Schema { namespace: Option<Spanned<NamespaceDecl>>, imports: Vec<Spanned<ImportDecl>>, declarations: Vec<Spanned<Decl>>, ... }`
- `NamespaceDecl { path: Vec<Spanned<SmolStr>> }`
- `ImportDecl { kind: ImportKind, path: Vec<Spanned<SmolStr>>, version: Option<Spanned<String>> }`
- `ImportKind::Wildcard | Named { names } | Aliased { alias }`

**IR (`crates/vexil-lang/src/ir/mod.rs`):**
- `CompiledSchema { namespace: Vec<SmolStr>, annotations, registry: TypeRegistry, declarations: Vec<TypeId> }`
- `FieldDef { name: SmolStr, span: Span, ordinal: u32, resolved_type, encoding, annotations }`
- `MessageDef { name, span, fields: Vec<FieldDef>, tombstones, annotations, wire_size: Option<WireSize> }`
- `UnionVariantDef { name, span, ordinal, fields: Vec<FieldDef>, tombstones, annotations }`
- `NewtypeDef { name, span, inner_type, terminal_type, annotations }`

**IR types (`crates/vexil-lang/src/ir/types.rs`):**
- `TypeRegistry` has: `register()`, `register_stub()`, `lookup()`, `get()`, `get_mut()`, `is_stub()`, `iter()`
- `Encoding::Default | Varint | ZigZag | Delta(Box<Encoding>)` — no `Fixed` variant

**Diagnostic (`crates/vexil-lang/src/diagnostic.rs`):**
- `Diagnostic { severity: Severity, span: Span, class: ErrorClass, message: String }` — no file path yet

**Lowering (`crates/vexil-lang/src/lower.rs`):**
- `pub fn lower(schema: &Schema) -> (Option<CompiledSchema>, Vec<Diagnostic>)`
- `LowerCtx { registry: TypeRegistry, diagnostics: Vec<Diagnostic>, wildcard_imports: HashSet<SmolStr> }`

**Type checking (`crates/vexil-lang/src/typeck.rs`):**
- `pub fn check(compiled: &mut CompiledSchema) -> Vec<Diagnostic>` — note: takes `&mut`

**Codegen (`crates/vexil-codegen/src/lib.rs`):**
- `pub fn generate(compiled: &CompiledSchema) -> Result<String, CodegenError>`

**Hard invariants from CLAUDE.md:** No `unwrap()`/`expect()` in non-test code. Use `?`, match, or error types.

---

## File Structure

### New Files

| File | Responsibility |
|------|---------------|
| `crates/vexil-lang/src/resolve.rs` | `SchemaLoader` trait, `FilesystemLoader`, `InMemoryLoader`, `LoadError` |
| `crates/vexil-lang/src/project.rs` | `ImportGraph`, `compile_project()`, DFS graph builder, topological sort |
| `crates/vexil-lang/src/remap.rs` | TypeId remapping: clone types from one registry to another with ID translation |
| `corpus/projects/simple/` | Multi-file test: A imports B |
| `corpus/projects/diamond/` | Multi-file test: A→B,C→D |
| `corpus/projects/mixed/` | Multi-file test: wildcard + named + aliased |

### Modified Files

| File | Change |
|------|--------|
| `crates/vexil-lang/src/lib.rs` | Add `pub mod resolve; pub mod project; pub mod remap;` and `compile_project()` re-export |
| `crates/vexil-lang/src/lower.rs` | Add `lower_with_deps()` that accepts `DependencyContext`, injects real types |
| `crates/vexil-lang/src/ir/types.rs` | Add `TypeRegistry::fill_stub()` (note: `iter()` already exists) |
| `crates/vexil-lang/src/diagnostic.rs` | Add `source_file: Option<PathBuf>` to `Diagnostic` |
| `crates/vexil-codegen/src/lib.rs` | `generate_with_imports()` accepts import map, emits `use` statements |
| `crates/vexilc/src/main.rs` | Add `cmd_build` subcommand |

---

## Task 1: Diagnostic File Attribution

**Files:**
- Modify: `crates/vexil-lang/src/diagnostic.rs`

Do this first — adding the field to `Diagnostic` affects all later code. Doing it early avoids retroactive fixups.

- [ ] **Step 1: Add source_file field and update constructors**

In `crates/vexil-lang/src/diagnostic.rs`, add `source_file: Option<std::path::PathBuf>` to the `Diagnostic` struct, and update both constructors to default it to `None`:

```rust
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
```

- [ ] **Step 2: Fix all compilation errors from the new field**

Search the codebase for any place that constructs `Diagnostic { ... }` directly (not via `Diagnostic::error()` / `Diagnostic::warning()`). Add `source_file: None` to each. Also check tests that pattern-match or construct Diagnostic values.

Run: `cargo build -p vexil-lang 2>&1 | head -50` to find all sites.

- [ ] **Step 3: Run full test suite**

Run: `cargo test -p vexil-lang`
Expected: All existing tests pass

- [ ] **Step 4: Clippy + format + commit**

```bash
cargo fmt --all && cargo clippy -p vexil-lang -- -D warnings
git add crates/vexil-lang/src/diagnostic.rs
git commit -m "feat(vexil-lang): add source_file to Diagnostic for multi-file errors"
```

---

## Task 2: SchemaLoader Trait & Implementations

**Files:**
- Create: `crates/vexil-lang/src/resolve.rs`
- Modify: `crates/vexil-lang/src/lib.rs`

- [ ] **Step 1: Write failing tests for InMemoryLoader**

In `crates/vexil-lang/src/resolve.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_loader_found() {
        let mut loader = InMemoryLoader::new();
        loader.add("foo.bar.types", "namespace foo.bar.types\nmessage Foo { x @0 : u32 }");
        let (source, path) = loader.load(&["foo", "bar", "types"]).unwrap();
        assert!(source.contains("message Foo"));
        assert_eq!(path, PathBuf::from("<memory>/foo.bar.types"));
    }

    #[test]
    fn in_memory_loader_not_found() {
        let loader = InMemoryLoader::new();
        let result = loader.load(&["no", "such", "ns"]);
        assert!(matches!(result, Err(LoadError::NotFound { .. })));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p vexil-lang -- resolve::tests --nocapture`
Expected: FAIL — module `resolve` does not exist.

- [ ] **Step 3: Implement SchemaLoader trait, LoadError, and InMemoryLoader**

```rust
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum LoadError {
    NotFound { namespace: String },
    Ambiguous { namespace: String, paths: Vec<PathBuf> },
    Io { namespace: String, message: String },
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::NotFound { namespace } => {
                write!(f, "schema not found for namespace `{namespace}`")
            }
            LoadError::Ambiguous { namespace, paths } => {
                write!(f, "namespace `{namespace}` found in multiple include roots: ")?;
                for (i, p) in paths.iter().enumerate() {
                    if i > 0 { write!(f, " and ")?; }
                    write!(f, "`{}`", p.display())?;
                }
                Ok(())
            }
            LoadError::Io { namespace, message } => {
                write!(f, "IO error loading `{namespace}`: {message}")
            }
        }
    }
}

pub trait SchemaLoader {
    fn load(&self, namespace: &[&str]) -> Result<(String, PathBuf), LoadError>;
}

/// In-memory loader for testing.
pub struct InMemoryLoader {
    pub schemas: HashMap<String, String>,
}

impl InMemoryLoader {
    pub fn new() -> Self {
        Self { schemas: HashMap::new() }
    }

    pub fn add(&mut self, dotted_ns: &str, source: &str) {
        self.schemas.insert(dotted_ns.to_string(), source.to_string());
    }
}

impl SchemaLoader for InMemoryLoader {
    fn load(&self, namespace: &[&str]) -> Result<(String, PathBuf), LoadError> {
        let key = namespace.join(".");
        match self.schemas.get(&key) {
            Some(source) => Ok((source.clone(), PathBuf::from(format!("<memory>/{key}")))),
            None => Err(LoadError::NotFound { namespace: key }),
        }
    }
}
```

Note: `schemas` is `pub` so tests in other modules (project.rs) can access source text directly.

- [ ] **Step 4: Register module in lib.rs**

Add `pub mod resolve;` to `crates/vexil-lang/src/lib.rs`.

- [ ] **Step 5: Run tests — should pass**

Run: `cargo test -p vexil-lang -- resolve::tests --nocapture`
Expected: PASS (2 tests)

- [ ] **Step 6: Write failing tests for FilesystemLoader**

```rust
    #[test]
    fn filesystem_loader_finds_file() {
        let tmp = std::env::temp_dir().join("vexil-fs-test-find");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("foo/bar")).unwrap();
        std::fs::write(
            tmp.join("foo/bar/types.vexil"),
            "namespace foo.bar.types\nmessage T { x @0 : u32 }",
        ).unwrap();

        let loader = FilesystemLoader::new(vec![tmp.clone()]);
        let (source, path) = loader.load(&["foo", "bar", "types"]).unwrap();
        assert!(source.contains("message T"));
        assert_eq!(path, tmp.join("foo/bar/types.vexil"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn filesystem_loader_not_found() {
        let tmp = std::env::temp_dir().join("vexil-fs-test-notfound");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let loader = FilesystemLoader::new(vec![tmp.clone()]);
        let result = loader.load(&["no", "such", "ns"]);
        assert!(matches!(result, Err(LoadError::NotFound { .. })));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn filesystem_loader_ambiguous() {
        let tmp = std::env::temp_dir().join("vexil-fs-test-ambig");
        let _ = std::fs::remove_dir_all(&tmp);
        let root_a = tmp.join("a");
        let root_b = tmp.join("b");
        std::fs::create_dir_all(root_a.join("ns")).unwrap();
        std::fs::create_dir_all(root_b.join("ns")).unwrap();
        std::fs::write(root_a.join("ns/file.vexil"), "namespace ns.file").unwrap();
        std::fs::write(root_b.join("ns/file.vexil"), "namespace ns.file").unwrap();

        let loader = FilesystemLoader::new(vec![root_a, root_b]);
        let result = loader.load(&["ns", "file"]);
        assert!(matches!(result, Err(LoadError::Ambiguous { .. })));

        let _ = std::fs::remove_dir_all(&tmp);
    }
```

- [ ] **Step 7: Implement FilesystemLoader**

```rust
pub struct FilesystemLoader {
    include_paths: Vec<PathBuf>,
}

impl FilesystemLoader {
    pub fn new(include_paths: Vec<PathBuf>) -> Self {
        Self { include_paths }
    }
}

impl SchemaLoader for FilesystemLoader {
    fn load(&self, namespace: &[&str]) -> Result<(String, PathBuf), LoadError> {
        let ns_key = namespace.join(".");
        let mut rel_path = PathBuf::new();
        for segment in namespace {
            rel_path.push(segment);
        }
        rel_path.set_extension("vexil");

        let mut found: Vec<PathBuf> = Vec::new();
        for root in &self.include_paths {
            let candidate = root.join(&rel_path);
            if candidate.is_file() {
                found.push(candidate);
            }
        }

        match found.len() {
            0 => Err(LoadError::NotFound { namespace: ns_key }),
            1 => {
                // Safe: we just checked len() == 1
                let path = found.remove(0);
                let source = std::fs::read_to_string(&path).map_err(|e| LoadError::Io {
                    namespace: ns_key,
                    message: e.to_string(),
                })?;
                Ok((source, path))
            }
            _ => Err(LoadError::Ambiguous {
                namespace: ns_key,
                paths: found,
            }),
        }
    }
}
```

Note: uses `found.remove(0)` instead of `found.into_iter().next().unwrap()` to avoid `unwrap()` in non-test code.

- [ ] **Step 8: Run all resolve tests + clippy + commit**

```bash
cargo test -p vexil-lang -- resolve::tests --nocapture
cargo fmt --all && cargo clippy -p vexil-lang -- -D warnings
git add crates/vexil-lang/src/resolve.rs crates/vexil-lang/src/lib.rs
git commit -m "feat(vexil-lang): SchemaLoader trait + FilesystemLoader + InMemoryLoader"
```

---

## Task 3: Import Graph Builder with Cycle Detection

**Files:**
- Create: `crates/vexil-lang/src/project.rs`
- Modify: `crates/vexil-lang/src/lib.rs`

- [ ] **Step 1: Write failing tests**

In `crates/vexil-lang/src/project.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::InMemoryLoader;

    #[test]
    fn simple_import_graph() {
        let mut loader = InMemoryLoader::new();
        loader.add("a.dep", "namespace a.dep\nmessage Dep { y @0 : u32 }");
        loader.add("a.root", "namespace a.root\nimport a.dep\nmessage Root { x @0 : u32 }");
        let source = loader.schemas["a.root"].clone();
        let graph = build_import_graph(&source, &PathBuf::from("<test>"), &loader).unwrap();
        assert_eq!(graph.topo_order.len(), 2);
        let dep_pos = graph.topo_order.iter().position(|n| n == "a.dep").unwrap();
        let root_pos = graph.topo_order.iter().position(|n| n == "a.root").unwrap();
        assert!(dep_pos < root_pos);
    }

    #[test]
    fn direct_cycle_detected() {
        let mut loader = InMemoryLoader::new();
        loader.add("cyc.a", "namespace cyc.a\nimport cyc.b\nmessage A { x @0 : u32 }");
        loader.add("cyc.b", "namespace cyc.b\nimport cyc.a\nmessage B { y @0 : u32 }");
        let source = loader.schemas["cyc.a"].clone();
        let result = build_import_graph(&source, &PathBuf::from("<test>"), &loader);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("circular import"));
    }

    #[test]
    fn transitive_cycle_detected() {
        let mut loader = InMemoryLoader::new();
        loader.add("cyc.a", "namespace cyc.a\nimport cyc.b\nmessage A { x @0 : u32 }");
        loader.add("cyc.b", "namespace cyc.b\nimport cyc.c\nmessage B { y @0 : u32 }");
        loader.add("cyc.c", "namespace cyc.c\nimport cyc.a\nmessage C { z @0 : u32 }");
        let source = loader.schemas["cyc.a"].clone();
        let result = build_import_graph(&source, &PathBuf::from("<test>"), &loader);
        assert!(result.is_err());
    }

    #[test]
    fn diamond_dependency() {
        let mut loader = InMemoryLoader::new();
        loader.add("d.base", "namespace d.base\nmessage Base { z @0 : u32 }");
        loader.add("d.left", "namespace d.left\nimport d.base\nmessage Left { x @0 : u32 }");
        loader.add("d.right", "namespace d.right\nimport d.base\nmessage Right { y @0 : u32 }");
        loader.add("d.root", "namespace d.root\nimport d.left\nimport d.right\nmessage Root { x @0 : u32 }");
        let source = loader.schemas["d.root"].clone();
        let graph = build_import_graph(&source, &PathBuf::from("<test>"), &loader).unwrap();
        assert_eq!(graph.topo_order.len(), 4);
        let base_pos = graph.topo_order.iter().position(|n| n == "d.base").unwrap();
        let root_pos = graph.topo_order.iter().position(|n| n == "d.root").unwrap();
        assert!(base_pos < root_pos);
    }
}
```

- [ ] **Step 2: Implement ImportGraph, ProjectError, and build_import_graph**

Key helper for extracting namespace from parsed `Schema`:

```rust
fn namespace_string(schema: &crate::ast::Schema) -> String {
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
```

Note: `schema.namespace` is `Option<Spanned<NamespaceDecl>>` — must use `.as_ref()` then access `.node.path`.

Import namespace extraction:

```rust
fn import_namespaces(schema: &crate::ast::Schema) -> Vec<String> {
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
```

DFS with cycle detection — the `dfs()` function uses a `stack: Vec<String>` for cycle detection and `visited: HashSet<String>` for diamond dedup. Post-order push to `topo_order` gives correct topological ordering. See Task 2 Step 3 in the spec for the full implementation.

- [ ] **Step 3: Register module + run tests + commit**

```bash
cargo test -p vexil-lang -- project::tests --nocapture
cargo fmt --all && cargo clippy -p vexil-lang -- -D warnings
git add crates/vexil-lang/src/project.rs crates/vexil-lang/src/lib.rs
git commit -m "feat(vexil-lang): import graph builder with DFS cycle detection"
```

---

## Task 4: TypeId Remapping

**Files:**
- Create: `crates/vexil-lang/src/remap.rs`
- Modify: `crates/vexil-lang/src/ir/types.rs` (add `fill_stub()` only — `iter()` already exists)
- Modify: `crates/vexil-lang/src/lib.rs`

- [ ] **Step 1: Add `fill_stub()` to TypeRegistry**

In `crates/vexil-lang/src/ir/types.rs`:

```rust
/// Fill a stub slot with a real type definition.
pub fn fill_stub(&mut self, id: TypeId, def: TypeDef) {
    let idx = id.0 as usize;
    if idx < self.types.len() {
        self.types[idx] = Some(def);
    }
}
```

- [ ] **Step 2: Write failing test for remap**

In `crates/vexil-lang/src/remap.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remap_clones_types_with_new_ids() {
        // Compile a real schema to get a valid registry
        let source = "namespace test.remap\nmessage Foo { x @0 : u32 }";
        let result = crate::compile(source);
        let compiled = result.compiled.unwrap();
        let foo_id = compiled.declarations[0];

        let mut target = crate::ir::TypeRegistry::new();
        let id_map = clone_types_into(&compiled.registry, &compiled.declarations, &mut target);

        assert_eq!(id_map.len(), 1);
        let new_id = id_map[&foo_id];
        assert!(!target.is_stub(new_id));
        if let Some(crate::ir::TypeDef::Message(m)) = target.get(new_id) {
            assert_eq!(m.name.as_str(), "Foo");
        } else {
            panic!("expected Message");
        }
    }
}
```

- [ ] **Step 3: Implement clone_types_into and remap helpers**

The `clone_types_into` function:
1. Phase 1: Register stubs in target registry for each source type (gets new IDs)
2. Phase 2: Clone each type definition, remapping all internal `TypeId` references via `remap_resolved_type()`

The `remap_resolved_type()` function recursively walks `ResolvedType` and replaces `Named(old_id)` with `Named(new_id)` using the mapping. Primitives, SubByte, Semantic pass through unchanged.

The `remap_type_def()` function handles each `TypeDef` variant. Must include all struct fields:
- `MessageDef`: `name, span, fields, tombstones, annotations, wire_size` — remap each field's `resolved_type`
- `FieldDef`: `name, span, ordinal, resolved_type, encoding, annotations` — remap `resolved_type`
- `UnionDef` + `UnionVariantDef`: `name, span, ordinal, fields, tombstones, annotations` + `wire_size` on UnionDef
- `NewtypeDef`: `name, span, inner_type, terminal_type, annotations` — remap both `inner_type` and `terminal_type`
- `EnumDef`, `FlagsDef`, `ConfigDef`: clone directly (no TypeId references in their fields)

- [ ] **Step 4: Register module + run tests + commit**

```bash
cargo test -p vexil-lang -- remap::tests --nocapture
cargo fmt --all && cargo clippy -p vexil-lang -- -D warnings
git add crates/vexil-lang/src/remap.rs crates/vexil-lang/src/ir/types.rs crates/vexil-lang/src/lib.rs
git commit -m "feat(vexil-lang): TypeId remapping for cross-file type injection"
```

---

## Task 5: Dependency Context in Lowering

**Files:**
- Modify: `crates/vexil-lang/src/lower.rs`

This is the most complex task — it wires dependency types into the lowering phase.

- [ ] **Step 1: Define DependencyContext**

Add to `lower.rs`:

```rust
use crate::ir::CompiledSchema;

/// Pre-compiled dependency information for multi-file compilation.
pub struct DependencyContext {
    /// Maps import namespace string → compiled schema
    pub schemas: HashMap<String, CompiledSchema>,
}
```

- [ ] **Step 2: Add `lower_with_deps()` function**

Refactor: extract the body of `lower()` into `lower_internal(schema, deps)`, then have `lower()` call `lower_internal(schema, None)`.

```rust
pub fn lower(schema: &Schema) -> (Option<CompiledSchema>, Vec<Diagnostic>) {
    lower_with_deps(schema, None)
}

pub fn lower_with_deps(
    schema: &Schema,
    deps: Option<&DependencyContext>,
) -> (Option<CompiledSchema>, Vec<Diagnostic>) {
    let mut ctx = LowerCtx::new();

    // Phase 1: Register import types (real or stubs)
    register_import_types(schema, &mut ctx, deps);

    // Phase 2-N: existing lowering logic (register_declarations, lower_declarations, etc.)
    // ... identical to current lower() body after register_import_stubs() ...
}
```

- [ ] **Step 3: Implement register_import_types**

This replaces `register_import_stubs()` when `deps` is `Some`:

```rust
fn register_import_types(
    schema: &Schema,
    ctx: &mut LowerCtx,
    deps: Option<&DependencyContext>,
) {
    for imp in &schema.imports {
        let ns_key = imp.node.path.iter()
            .map(|s| s.node.as_str())
            .collect::<Vec<_>>()
            .join(".");

        // Emit warning for version constraints (deferred to Milestone G)
        if let Some(ref ver) = imp.node.version {
            ctx.diagnostics.push(Diagnostic::warning(
                ver.span,
                ErrorClass::UnexpectedToken, // or add new ErrorClass
                format!("version constraints are not yet enforced; ignoring `@ {}`", ver.node),
            ));
        }

        match deps.and_then(|d| d.schemas.get(&ns_key)) {
            Some(dep_compiled) => {
                // Real dependency available — inject types by import kind
                match &imp.node.kind {
                    ImportKind::Named { names } => {
                        // Clone only the named types
                        for name_spanned in names {
                            let name = &name_spanned.node;
                            // Find the type in dep's declarations
                            let found = dep_compiled.declarations.iter().find(|&&id| {
                                dep_compiled.registry.get(id)
                                    .map(|d| crate::remap::type_def_name(d) == *name)
                                    .unwrap_or(false)
                            });
                            match found {
                                Some(&id) => {
                                    let id_map = crate::remap::clone_types_into(
                                        &dep_compiled.registry, &[id], &mut ctx.registry,
                                    );
                                    // The cloned type is now in ctx.registry under its original name
                                }
                                None => {
                                    ctx.emit(
                                        name_spanned.span,
                                        ErrorClass::UnresolvedType,
                                        format!("imported name `{name}` not found in namespace `{ns_key}`"),
                                    );
                                }
                            }
                        }
                    }
                    ImportKind::Wildcard => {
                        // Clone all exported types; track origins for collision detection
                        crate::remap::clone_types_into(
                            &dep_compiled.registry,
                            &dep_compiled.declarations,
                            &mut ctx.registry,
                        );
                        ctx.wildcard_imports.insert(SmolStr::new(&ns_key));
                    }
                    ImportKind::Aliased { alias } => {
                        // Clone all types under qualified names: "Alias.TypeName"
                        for &id in &dep_compiled.declarations {
                            if let Some(def) = dep_compiled.registry.get(id) {
                                let original_name = crate::remap::type_def_name(def);
                                let qualified = format!("{}.{}", alias.node, original_name);
                                let mut cloned = def.clone();
                                // Rename the cloned type to the qualified name
                                set_type_name(&mut cloned, SmolStr::new(&qualified));
                                let new_id = ctx.registry.register(SmolStr::new(&qualified), cloned);
                                // Also register the alias itself so qualified lookups work
                            }
                        }
                    }
                }
            }
            None => {
                // No dependency context (single-file mode) — fall back to stubs
                // This is the existing register_import_stubs() behavior
                match &imp.node.kind {
                    ImportKind::Named { names } => {
                        for name_spanned in names {
                            ctx.registry.register_stub(name_spanned.node.clone());
                        }
                    }
                    ImportKind::Wildcard => {
                        ctx.wildcard_imports.insert(SmolStr::new(&ns_key));
                    }
                    ImportKind::Aliased { alias } => {
                        ctx.registry.register_stub(alias.node.clone());
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 4: Write tests**

```rust
#[cfg(test)]
mod dep_tests {
    use super::*;

    #[test]
    fn lower_with_dependency_resolves_named_import() {
        let dep_result = crate::compile("namespace dep.types\nmessage Foo { x @0 : u32 }");
        let dep_compiled = dep_result.compiled.unwrap();

        let root_source = "namespace root\nimport { Foo } from dep.types\nmessage Bar { f @0 : Foo }";
        let root_schema = crate::parse(root_source).schema.unwrap();

        let mut dep_ctx = DependencyContext { schemas: HashMap::new() };
        dep_ctx.schemas.insert("dep.types".to_string(), dep_compiled);

        let (compiled, diags) = lower_with_deps(&root_schema, Some(&dep_ctx));
        assert!(compiled.is_some(), "should compile: {:?}", diags);
        let compiled = compiled.unwrap();
        for &id in &compiled.declarations {
            if let Some(TypeDef::Message(m)) = compiled.registry.get(id) {
                if m.name == "Bar" {
                    if let ResolvedType::Named(ref_id) = &m.fields[0].resolved_type {
                        assert!(!compiled.registry.is_stub(*ref_id), "Foo should not be a stub");
                    }
                }
            }
        }
    }

    #[test]
    fn local_declaration_shadows_wildcard() {
        let dep_result = crate::compile("namespace dep\nmessage Foo { x @0 : u32 }");
        let dep_compiled = dep_result.compiled.unwrap();

        let root_source = "namespace root\nimport dep\nmessage Foo { y @0 : string }\nmessage Bar { f @0 : Foo }";
        let root_schema = crate::parse(root_source).schema.unwrap();

        let mut dep_ctx = DependencyContext { schemas: HashMap::new() };
        dep_ctx.schemas.insert("dep".to_string(), dep_compiled);

        let (compiled, diags) = lower_with_deps(&root_schema, Some(&dep_ctx));
        assert!(compiled.is_some());
        let errors: Vec<_> = diags.iter().filter(|d| d.severity == crate::diagnostic::Severity::Error).collect();
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    }

    #[test]
    fn named_import_nonexistent_type_errors() {
        let dep_result = crate::compile("namespace dep\nmessage Foo { x @0 : u32 }");
        let dep_compiled = dep_result.compiled.unwrap();

        let root_source = "namespace root\nimport { Bar } from dep\nmessage Baz { x @0 : u32 }";
        let root_schema = crate::parse(root_source).schema.unwrap();

        let mut dep_ctx = DependencyContext { schemas: HashMap::new() };
        dep_ctx.schemas.insert("dep".to_string(), dep_compiled);

        let (_compiled, diags) = lower_with_deps(&root_schema, Some(&dep_ctx));
        assert!(diags.iter().any(|d| d.message.contains("not found")));
    }
}
```

- [ ] **Step 5: Run tests + full regression**

```bash
cargo test -p vexil-lang -- dep_tests --nocapture
cargo test -p vexil-lang  # full suite, no regressions
```

- [ ] **Step 6: Clippy + format + commit**

```bash
cargo fmt --all && cargo clippy -p vexil-lang -- -D warnings
git add crates/vexil-lang/src/lower.rs
git commit -m "feat(vexil-lang): dependency context injection in lowering phase"
```

---

## Task 6: Name Resolution Precedence & Wildcard Collision

**Files:**
- Modify: `crates/vexil-lang/src/lower.rs`

- [ ] **Step 1: Add wildcard_origins tracking to LowerCtx**

```rust
struct LowerCtx {
    registry: TypeRegistry,
    diagnostics: Vec<Diagnostic>,
    wildcard_imports: HashSet<SmolStr>,
    /// Maps type name → source namespace for wildcard imports.
    /// None means ambiguous (multiple wildcards provide this name).
    wildcard_origins: HashMap<SmolStr, Option<String>>,
    /// Names from local declarations (registered during register_declarations).
    local_names: HashSet<SmolStr>,
}
```

- [ ] **Step 2: Populate wildcard_origins during register_import_types**

When injecting wildcard types, for each type name:
- If not in `local_names` (local shadows wildcard — skip collision tracking):
  - If not in `wildcard_origins`, insert `Some(ns_key)`
  - If already there with a different namespace, set to `None` (ambiguous)

- [ ] **Step 3: Check ambiguity in resolve_type_expr**

When resolving `TypeExpr::Named(name)`:
1. If `local_names.contains(name)` → resolve locally (highest precedence)
2. If `registry.lookup(name)` finds it and it's from a named import → use it
3. If `wildcard_origins.get(name)` is `Some(None)` → ambiguity error
4. If `wildcard_origins.get(name)` is `Some(Some(_))` → use the wildcard type

- [ ] **Step 4: Write test for wildcard collision**

```rust
#[test]
fn wildcard_collision_on_use_errors() {
    let dep_a = crate::compile("namespace dep.a\nmessage Foo { x @0 : u32 }").compiled.unwrap();
    let dep_b = crate::compile("namespace dep.b\nmessage Foo { y @0 : string }").compiled.unwrap();

    let root_source = "namespace root\nimport dep.a\nimport dep.b\nmessage Bar { f @0 : Foo }";
    let root_schema = crate::parse(root_source).schema.unwrap();

    let mut dep_ctx = DependencyContext { schemas: HashMap::new() };
    dep_ctx.schemas.insert("dep.a".to_string(), dep_a);
    dep_ctx.schemas.insert("dep.b".to_string(), dep_b);

    let (_compiled, diags) = lower_with_deps(&root_schema, Some(&dep_ctx));
    assert!(diags.iter().any(|d| d.message.contains("ambiguous")));
}
```

- [ ] **Step 5: Run tests + commit**

```bash
cargo test -p vexil-lang --lib -- --nocapture
cargo fmt --all && cargo clippy -p vexil-lang -- -D warnings
git add crates/vexil-lang/src/lower.rs
git commit -m "feat(vexil-lang): name resolution precedence + wildcard collision detection"
```

---

## Task 7: compile_project() Orchestrator

**Files:**
- Modify: `crates/vexil-lang/src/project.rs`
- Modify: `crates/vexil-lang/src/lib.rs`

- [ ] **Step 1: Define ProjectResult and implement compile_project()**

```rust
pub struct ProjectResult {
    pub schemas: Vec<(String, CompiledSchema)>,
    pub diagnostics: Vec<Diagnostic>,
}
```

`compile_project()` walks `topo_order`, builds `DependencyContext` from already-compiled schemas, calls `lower_with_deps()`, then `typeck::check(&mut compiled)` (note: `&mut`).

- [ ] **Step 2: Write integration tests**

Test: simple A→B, diamond A→B,C→D, aliased imports, version constraint warnings.

- [ ] **Step 3: Add public re-export**

In `lib.rs`: `pub use project::compile_project;`

- [ ] **Step 4: Run tests + commit**

```bash
cargo test -p vexil-lang -- project::tests --nocapture
cargo fmt --all && cargo clippy -p vexil-lang -- -D warnings
git add crates/vexil-lang/src/project.rs crates/vexil-lang/src/lib.rs
git commit -m "feat(vexil-lang): compile_project() multi-file orchestrator"
```

---

## Task 8: Codegen Cross-File References

**Files:**
- Modify: `crates/vexil-codegen/src/lib.rs`

- [ ] **Step 1: Add generate_with_imports() and generate_mod_file()**

`generate_with_imports()` accepts an optional `HashMap<TypeId, String>` mapping imported TypeIds to Rust module paths. Emits `use` statements after the header. Existing `generate()` calls it with `None`.

`generate_mod_file(module_names)` produces a `mod.rs` with `// Code generated by vexilc. DO NOT EDIT.` header and `pub mod X;` lines.

- [ ] **Step 2: Write tests + commit**

```bash
cargo test -p vexil-codegen
cargo fmt --all && cargo clippy -p vexil-codegen -- -D warnings
git add crates/vexil-codegen/src/lib.rs
git commit -m "feat(vexil-codegen): cross-file use statements + mod.rs generation"
```

---

## Task 9: vexilc build Command

**Files:**
- Modify: `crates/vexilc/src/main.rs`

- [ ] **Step 1: Implement cmd_build**

Takes root file path, `--include` paths, `--output` dir, `--rust-path-prefix` (default `crate`). Runs `compile_project()` + codegen for each schema. Generates namespace-mirrored directory tree with `mod.rs` scaffolding.

- [ ] **Step 2: Test manually with a multi-file corpus project**

- [ ] **Step 3: Commit**

```bash
cargo fmt --all && cargo clippy -p vexilc -- -D warnings
git add crates/vexilc/src/main.rs
git commit -m "feat(vexilc): add build command for multi-file compilation"
```

---

## Task 10: Multi-File Integration Tests

**Files:**
- Create: `corpus/projects/simple/simple/` (types.vexil, main.vexil)
- Create: `corpus/projects/diamond/diamond/` (base.vexil, left.vexil, right.vexil, root.vexil)
- Create: `corpus/projects/mixed/mix/` (types.vexil, shapes.vexil, app.vexil)
- Create: `crates/vexil-codegen/tests/project_compile_check.rs`

Note: corpus directory structure mirrors namespaces. `corpus/projects/simple/` is the include root; `simple/types.vexil` contains `namespace simple.types`, matching the path `simple/types.vexil` relative to the include root.

- [ ] **Step 1: Create corpus files**

**simple/simple/types.vexil:**
```
namespace simple.types
message Coord { x @0 : f32  y @1 : f32 }
```

**simple/simple/main.vexil:**
```
namespace simple.main
import { Coord } from simple.types
message Player { pos @0 : Coord  name @1 : string }
```

**diamond/diamond/base.vexil:** `namespace diamond.base` + `message Id { value @0 : u64 }`
**diamond/diamond/left.vexil:** imports diamond.base, has `message LeftNode { id @0 : Id  label @1 : string }`
**diamond/diamond/right.vexil:** imports diamond.base, has `message RightNode { id @0 : Id  count @1 : u32 }`
**diamond/diamond/root.vexil:** imports left + right, has `message Graph { left @0 : LeftNode  right @1 : RightNode }`

**mixed/mix/types.vexil:** `message Point { x @0 : i32  y @1 : i32 }` + `enum Color { Red @0  Green @1  Blue @2 }`
**mixed/mix/shapes.vexil:** `import mix.types` (wildcard), `message Circle { center @0 : Point  radius @1 : f32 }`
**mixed/mix/app.vexil:** `import mix.types as T` + `import { Circle } from mix.shapes` + `message Canvas { shape @0 : Circle  origin @1 : T.Point }`

- [ ] **Step 2: Write project_compile_check.rs**

Integration test that:
1. Loads root file from corpus
2. Creates `FilesystemLoader` with corpus project dir as include root
3. Runs `compile_project()`
4. Asserts no errors
5. Generates code for each schema into a temp crate with namespace-mirrored directories
6. Generates `mod.rs` files
7. Runs `cargo check` on the temp crate

- [ ] **Step 3: Run integration tests**

```bash
cargo test -p vexil-codegen --test project_compile_check
```

- [ ] **Step 4: Run full workspace test suite**

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

- [ ] **Step 5: Commit**

```bash
git add corpus/projects/ crates/vexil-codegen/tests/project_compile_check.rs
git commit -m "test(vexil-lang): multi-file project integration tests"
```

---

## Summary

| Task | Description | Dependencies |
|------|-------------|-------------|
| 1 | Diagnostic file attribution | None (do first) |
| 2 | SchemaLoader trait + implementations | None |
| 3 | Import graph builder + cycle detection | Task 2 |
| 4 | TypeId remapping | None (parallel with 2-3) |
| 5 | Dependency context in lowering | Tasks 4 |
| 6 | Name resolution precedence | Task 5 |
| 7 | compile_project() orchestrator | Tasks 3, 5, 6 |
| 8 | Codegen cross-file references | Task 7 |
| 9 | vexilc build command | Tasks 7, 8 |
| 10 | Multi-file integration tests | Tasks 7, 8 |

**Total:** ~1,100 new/modified lines, 10 commits
