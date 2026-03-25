# Milestone F: Multi-File Import Resolution

## Goal

Given a root `.vexil` file and a set of include paths, resolve all imports transitively, compile each file with full type information from its dependencies, and produce one generated `.rs` file per `.vexil` file in a namespace-mirrored directory tree.

## Scope

- **In scope:** Filesystem-based resolution, cycle detection, cross-file type linking, per-file codegen with Rust `use` statements, `mod.rs` scaffolding.
- **Out of scope:** Version constraint resolution (parsed but deferred to Milestone G), remote registry fetching, package lockfiles.

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Resolution scope | Filesystem only | Version/registry is Milestone G |
| Path mapping | Dots → directories | `foo.bar` → `foo/bar.vexil`; standard convention (Protobuf, Go) |
| Entry point | `SchemaLoader` trait | Decouples from filesystem; enables in-memory testing |
| Duplicate detection | Error on duplicates | Prevents silent shadowing across include roots |
| Type linking | Incremental (topological) | Dependencies compiled first, injected as real types |
| Output | One `.rs` per `.vexil` | Each file owns its output; mirrors Protobuf/Cap'n Proto |
| Output structure | Namespace-mirrored dirs | `foo.bar.types` → `foo/bar/types.rs` with `mod.rs` chain |
| Cycle detection | Eager DFS | Detected at the offending import with full chain in error |
| Wildcard collision | Error on use, not import | Matches Rust/Python/Java behavior; friendlier |
| Export visibility | All top-level types | No pub/private distinction yet (YAGNI) |

## Architecture

### Pipeline

```
Root file path + include paths
        |
        v
   SchemaLoader (trait)
        |
        v
   Import Graph Builder (DFS, eager cycle detection)
        |
        v
   Topological Sort (compilation order)
        |
        v
   Incremental Compilation (per file, dependencies injected)
        |
        v
   Per-file CompiledSchema with cross-file type references
        |
        v
   Codegen (one .rs per .vexil + mod.rs scaffolding)
```

### Key Types

#### SchemaLoader trait

```rust
pub trait SchemaLoader {
    fn load(&self, namespace: &[&str]) -> Result<(String, PathBuf), LoadError>;
}
```

Returns `(source_text, canonical_path)`. `LoadError` covers: not found, IO error, ambiguous (found in multiple roots).

#### FilesystemLoader

- Constructed with an ordered list of include paths.
- For `load(&["foo", "bar", "types"])`, searches each root for `foo/bar/types.vexil`.
- If found in multiple roots: error listing both paths (e.g. "namespace `foo.bar.types` found in multiple include roots: `/path/a/foo/bar/types.vexil` and `/path/b/foo/bar/types.vexil`").

#### InMemoryLoader

```rust
pub struct InMemoryLoader {
    schemas: HashMap<String, String>,  // "foo.bar.types" -> source text
}
```

For testing. Maps dotted namespace strings to source text. The `load()` implementation joins the `&[&str]` segments with `"."` to look up the key.

#### ImportGraph

```rust
pub struct ImportGraph {
    /// Namespace -> parsed Schema + source path
    pub schemas: HashMap<String, (Schema, PathBuf)>,
    /// Namespace -> list of direct dependency namespaces
    pub edges: HashMap<String, Vec<String>>,
    /// Topological order (dependencies before dependents)
    pub topo_order: Vec<String>,
}
```

Built by DFS with eager cycle detection. Topological sort computed after DFS completes (guaranteed to succeed since cycles already rejected).

#### ProjectResult

```rust
pub struct ProjectResult {
    /// Per-namespace compilation results, in topological order
    pub schemas: Vec<(String, CompiledSchema)>,
    /// All diagnostics across all files
    pub diagnostics: Vec<Diagnostic>,
}
```

### Import Graph Construction

1. Parse the root file, extract its `ImportDecl` list.
2. For each import, map namespace path to loader key, call `loader.load()`.
3. Parse the loaded file, extract its imports, recurse.
4. Track a resolution stack (`Vec<String>`) during DFS. If a namespace appears on the stack, report cycle with full chain.

Cycle error example:

```
error: circular import detected
  -- foo/bar.vexil:3
  |
  | import baz.qux
  |
  = cycle: foo.bar -> baz.qux -> foo.bar
```

### Incremental Compilation

Walk `topo_order` front-to-back. For each file:

1. Gather its dependency `CompiledSchema`s (already compiled by topo order).
2. Build a dependency context filtered by import kind:
   - **Named** `import { Foo, Bar } from ns`: only `Foo` and `Bar` from that namespace. If `Foo` does not exist in the dependency's exported types, emit error: "imported name `Foo` not found in namespace `ns`".
   - **Wildcard** `import ns`: all exported types from that namespace.
   - **Aliased** `import ns as A`: all exported types, registered under qualified keys (e.g. `"A.Foo"`, `"A.Bar"`) so that `TypeExpr::Qualified("A", "Foo")` resolves correctly.
3. Clone imported type definitions into the file's `TypeRegistry`, assigning new local `TypeId`s. Maintain a remapping table (`HashMap<TypeId, TypeId>`) from source registry IDs to local IDs. All field type references within cloned definitions must be remapped to local IDs. For diamond dependencies (A imports B and C, both import D), types from D are cloned once per importing file — structural equality (`PartialEq` on type definitions) ensures they are compatible.
4. Run existing lowering -> type checking pipeline with real cross-file types.
5. Store resulting `CompiledSchema` keyed by namespace.

**Name resolution precedence** (checked during type checking when resolving `TypeExpr::Named`):
1. **Local declarations** — always win, shadow everything.
2. **Named imports** — explicit names take next precedence.
3. **Wildcard imports** — checked last. If a name matches multiple wildcard imports, emit ambiguity error requiring disambiguation (use named or aliased import instead).

Aliased imports are never ambiguous — they require explicit `A.TypeName` qualification.

**Named import validation:** If `import { Foo } from ns` and `Foo` is not exported by `ns`, emit error at the import declaration span.

**Version constraints:** Parsed and stored in the AST but not checked in Milestone F. If a version constraint is present, emit a warning: "version constraints are not yet enforced; ignoring `@ ^1.0.0`". This makes the non-conformance visible without breaking builds. Full version checking is Milestone G.

**Exports:** All top-level type declarations in a file are exported. No visibility modifiers.

**Diagnostics:** In multi-file compilation, each `Diagnostic` carries a `source_file: PathBuf` in addition to the existing `Span`. This allows error messages to reference the correct file. The existing single-file `compile()` path sets `source_file` to a synthetic `"<input>"` path for backward compatibility.

### Codegen Output

Given output root `out/`, namespace `foo.bar.types` generates:

```
out/
  foo/
    bar/
      types.rs        <- generated code
      mod.rs           <- pub mod types;
    mod.rs             <- pub mod bar;
  mod.rs               <- pub mod foo;
```

Cross-file references emit Rust `use` statements:

```rust
use crate::foo::bar::types::Baz;
```

The `crate::` prefix is configurable via `--rust-path-prefix` (default: `crate`).

**SCHEMA_HASH:** Each file gets its own hash based on its own canonical form. Imported types are referenced by qualified name in the canonical form, which is already deterministic.

**mod.rs generation:** vexilc generates all intermediate `mod.rs` files with a `// Code generated by vexilc. DO NOT EDIT.` header (same as `.rs` output files). Regeneration overwrites them.

## Changes to Existing Code

### Unchanged

- `compile()` single-file entry point
- Parser, lexer, AST types
- Existing lowering pipeline (reused per-file)
- Per-file codegen logic

### Modified

| File | Change |
|------|--------|
| `lower.rs` | `register_import_stubs()` gets new path: when dependency context provided, inject real types instead of stubs. Stub path remains for single-file `compile()`. |
| `typeck.rs` | Wildcard collision detection: when resolving a `Named` type matching multiple wildcard imports, emit ambiguity error. |
| `codegen/lib.rs` | `generate()` accepts optional map of imported namespace -> Rust module path, emits `use` statements. |

### New Modules

| Module | Location | Responsibility |
|--------|----------|---------------|
| `resolve.rs` | `vexil-lang` | `SchemaLoader` trait, `FilesystemLoader`, `InMemoryLoader`, `LoadError` |
| `project.rs` | `vexil-lang` | `compile_project()`, import graph DFS, topo sort, incremental compilation |
| `cmd_build` | `vexilc` | Subcommand: root file + `--include` + `--output`, runs full pipeline |

Estimated scope: ~400-500 lines new code in resolve.rs + project.rs, ~100 lines changes to existing files, ~50 lines for vexilc cmd_build.

## Testing Strategy

### Unit Tests (InMemoryLoader, no filesystem)

**resolve.rs:**
- FilesystemLoader finds files in include paths
- FilesystemLoader errors on duplicate across roots
- FilesystemLoader errors on not found
- Namespace-to-path mapping correctness

**project.rs:**
- Cycle detection (direct A->B->A, transitive A->B->C->A)
- Topological order correctness
- Diamond dependencies (A imports B and C, both import D; D compiled once)
- Wildcard collision detected on use, not on import
- Local declarations shadow wildcard imports (no error)
- Named import precedence over wildcard
- Named import of nonexistent type emits error
- Aliased import qualified access (`A.Foo` resolves correctly)
- TypeId remapping correctness (imported types get new local IDs)
- Diamond dependency type compatibility

### Integration Tests

- New corpus directory `corpus/projects/` with multi-file test cases (each subdirectory is a project)
- `compile_check.rs`-style test: `compile_project()` + codegen + `cargo check`
- At least 3 fixtures: simple (A imports B), diamond (A->B,C->D), mixed import kinds

### Existing Tests

No changes. Single-file corpus tests continue to work via `compile()`.
