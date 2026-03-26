# SDK Architecture Design

> **Scope:** Compiler-as-library SDK for Vexil schema language. Covers public API tiers, codegen backend trait, crate restructuring, and error model. Does NOT cover TypeScript backend, query-based compiler, or LSP — those are separate sub-projects that build on this foundation.

**Goal:** Formalize `vexil-lang` as a stable SDK that codegen backends, linters, and future LSP tooling consume, without introducing new crates or premature abstractions.

**Architecture:** Single-crate layered API with three stability tiers. A `CodegenBackend` trait enables pluggable code generation. The existing `vexil-codegen` crate is renamed to `vexil-codegen-rust` as the first backend implementation.

**Tech Stack:** Rust, thiserror, smol_str, existing vexil-lang IR types.

---

## 1. Crate Structure

### Current
```
vexil-lang       — compiler (lexer → parser → validate → lower → typeck)
vexil-codegen    — Rust code generation
vexilc           — CLI binary
```

### Target
```
vexil-lang          — compiler + SDK API + CodegenBackend trait
vexil-codegen-rust  — Rust backend (renamed from vexil-codegen)
vexilc              — CLI binary (gains --target flag)
```

No new crates are introduced. The `CodegenBackend` trait lives in `vexil-lang` because backend crates already depend on it for IR types.

---

## 2. Public API Tiers

### Tier 1 — Stable (Backend authors, CLI tools)

These types and functions form the stable contract. They will not break across milestones.

```rust
// Entry points
pub fn compile(source: &str) -> CompileResult;
pub fn compile_project(
    root: &str,
    path: &Path,
    loader: &dyn SchemaLoader,
) -> Result<ProjectResult, ProjectError>;

// IR types (read-only for consumers)
pub struct CompiledSchema {
    pub namespace: Vec<SmolStr>,
    pub annotations: ResolvedAnnotations,
    pub registry: TypeRegistry,
    pub declarations: Vec<TypeId>,
}

pub struct ProjectResult {
    pub schemas: Vec<(String, CompiledSchema)>,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct TypeRegistry { /* opaque, accessed via methods */ }
pub enum TypeDef { Message, Enum, Flags, Union, Newtype, Config }
pub enum ResolvedType { Primitive, SubByte, Semantic, Named, Optional, Array, Map, Result }

// Backend trait
pub trait CodegenBackend { ... }
pub enum CodegenError { ... }

// Extension point
pub trait SchemaLoader { ... }
```

### Tier 2 — Semi-stable (Advanced consumers, future LSP)

Public and documented, but may evolve. Consumers should expect changes across major milestones (especially when query-based compiler lands).

```rust
pub fn parse(source: &str) -> ParseResult;
pub mod ast;        // Schema, Decl, TypeExpr, Annotation
pub mod lower;      // lower_with_deps(), DependencyContext
pub mod typeck;     // check()
pub mod validate;   // validate()
pub mod diagnostic; // Diagnostic, Severity, Span
pub mod remap;      // clone_types_into(), remap_resolved_type()
```

### Tier 3 — Internal (`pub(crate)`, free to churn)

Lexer internals, parser combinators, lowering context (`LowerCtx`), canonical hash implementation details.

### Stability communication

Tier boundaries are documented via module-level doc comments (`/// # Stability: Tier 1`). No `#[doc(hidden)]` — clarity through documentation, not hiding.

---

## 3. CodegenBackend Trait

```rust
pub trait CodegenBackend {
    /// Backend identifier, e.g. "rust", "typescript"
    fn name(&self) -> &str;

    /// File extension for generated files, e.g. "rs", "ts"
    fn file_extension(&self) -> &str;

    /// Generate code for a single compiled schema.
    /// For simple use cases, REPL, or single-file compilation.
    fn generate(&self, compiled: &CompiledSchema) -> Result<String, CodegenError>;

    /// Generate all files for a multi-file project.
    /// Returns path -> content map. Backend owns import strategy and file layout.
    fn generate_project(
        &self,
        result: &ProjectResult,
    ) -> Result<BTreeMap<PathBuf, String>, CodegenError>;
}
```

### Design properties

- **Backend owns imports:** Cross-file import generation is inherently language-specific (Rust `use` vs TypeScript `import`). The backend receives the full `ProjectResult` and decides its own strategy. No shared import infrastructure.
- **Backend owns file layout:** Namespace-to-path mapping may differ per language. The `BTreeMap<PathBuf, String>` return gives full control.
- **Standalone + trait:** Each backend crate exports a struct (e.g. `RustBackend`) for direct use AND implements `CodegenBackend` for CLI dispatch.
- **No registration system:** The CLI uses a hardcoded match on `--target`. Plugin discovery is a future concern.

---

## 4. Error Model

```rust
#[derive(Debug, thiserror::Error)]
pub enum CodegenError {
    #[error("unsupported type `{type_name}` in {backend} backend")]
    UnsupportedType { type_name: String, backend: String },

    #[error("missing required annotation `{annotation}` ({context})")]
    MissingAnnotation { annotation: String, context: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("backend error: {0}")]
    BackendSpecific(Box<dyn std::error::Error + Send + Sync>),
}
```

Common variants cover cases the CLI needs to handle uniformly. `BackendSpecific` allows backends to surface language-specific errors without polluting the shared type.

---

## 5. Migration: `vexil-codegen` -> `vexil-codegen-rust`

### What changes
- Crate renamed: `vexil-codegen` -> `vexil-codegen-rust`
- New struct: `pub struct RustBackend;` implementing `CodegenBackend`
- `generate()` becomes a method on `RustBackend` (free function kept as convenience wrapper during transition)
- `generate_with_imports()` becomes internal — called by `RustBackend::generate_project()` which builds the import path map from `ProjectResult`
- `generate_mod_file()` called internally by `generate_project()` for `mod.rs` scaffolding

### What doesn't change
- All existing codegen logic (emitters, type mapping, wire format handling)
- Test corpus and integration tests
- Output format of generated Rust code

### CLI changes
- `vexilc build` gains `--target <rust|typescript>` flag (default: `rust`)
- Dispatches to appropriate `CodegenBackend` impl
- File writing logic extracted from `cmd_build` into shared helper consuming `BTreeMap<PathBuf, String>`

---

## 6. Codegen Backend Independence

Each backend crate is fully self-contained. No shared codegen utilities, helper traits, or base classes. Each backend reads `CompiledSchema`/`ProjectResult` and produces strings independently.

**Rationale:** Traversal patterns are trivial (iterate `declarations`, look up in `registry`). Premature abstraction constrains backends. If a pattern emerges after 2-3 backends exist, extract then.

---

## 7. Future Milestones (Context, Not Scope)

These milestones build on this SDK design. They are documented here for architectural validation — none are in scope for this spec.

### TypeScript Backend (next milestone)
- New crate `vexil-codegen-ts` implements `CodegenBackend`
- `generate_project()` produces `index.ts` barrel files
- Import strategy: `import { Foo } from './base'`
- Validates that the trait surface is sufficient for non-Rust targets
- Ships `vexilc build --target typescript`

### Query-Based Compiler
- Refactor `vexil-lang` internals behind the existing Tier 1 API
- Pipeline stages become cached queries: `parse(file)`, `resolve(file)`, `typecheck(file)`
- `compile_project()` and `compile()` remain identical externally
- Tier 2 API evolves: stages gain per-file incremental semantics
- Enables LSP without breaking any backend

### LSP
- Consumes Tier 2 API (query-based pipeline stages)
- Per-file re-checking on edit
- Go-to-definition via name resolution queries
- Separate crate `vexil-lsp`

**Key invariant:** Tier 1 API never breaks across these milestones. Backends written today keep working.

---

## 8. Decision Log

### Crate structure: single crate vs split SDK crate

**Chosen:** Single `vexil-lang` crate with layered public API.

**Rejected:** Separate `vexil-sdk` wrapper crate.

**Rationale:** The compiler's modules are already cleanly layered. A wrapper crate adds indirection without practical benefit — we control the stability boundary through `pub` vs `pub(crate)` visibility and documentation. Early extraction is wasted effort; if API surface needs diverge later, split then.

### Trait location: `vexil-lang` vs `vexil-codegen-api` leaf crate

**Chosen:** Trait lives in `vexil-lang`.

**Rejected:** Tiny `vexil-codegen-api` crate holding only the trait + error types.

**Rationale:** Backend crates need `vexil-lang` anyway to read IR types (`CompiledSchema`, `TypeRegistry`, `ResolvedType`). A separate crate for the trait would add a dependency with no practical decoupling — backends cannot avoid depending on `vexil-lang`.

### Backend plugin model: trait vs functions vs hybrid

**Chosen:** Hybrid — `CodegenBackend` trait for CLI dispatch, standalone functions for direct use.

**Rejected alternatives:**
- **Trait-only:** Forces all consumers through dynamic dispatch even when the backend is known at compile time.
- **Function-only:** No polymorphism. CLI would need backend-specific imports and manual dispatch without a common interface.

**Rationale:** The trait gives the CLI a uniform dispatch table and enables future plugin loading. But each backend is also a normal Rust crate usable without the trait indirection.

### Cross-file imports: shared infrastructure vs backend-owned

**Chosen:** Backend-owned. `generate_project()` receives full `ProjectResult`, backend decides import strategy.

**Rejected:** Shared import path computation in `vexil-lang` (the existing `generate_with_imports()` approach with `HashMap<TypeId, String>`).

**Rationale:** Import syntax is inherently language-specific — Rust `use crate::foo::Bar`, TypeScript `import { Bar } from './foo'`, Python `from foo import Bar`. A shared model would either be too generic to be useful or leak language assumptions. The backend is the right place for this logic.

### Error model: shared, opaque, or extensible

**Chosen:** Shared base enum with `BackendSpecific(Box<dyn Error>)` variant.

**Rejected alternatives:**
- **Fully shared `CodegenError`:** Cannot cover backend-specific failure modes without growing unboundedly.
- **`Box<dyn Error>` return:** Callers lose typed matching on common cases the CLI needs to handle uniformly.

**Rationale:** Common variants (`UnsupportedType`, `MissingAnnotation`, `Io`) let the CLI handle frequent errors uniformly. `BackendSpecific` gives backends escape hatch for language-specific issues without polluting the shared type.

### Shared codegen utilities: yes vs no

**Chosen:** No shared utilities. Each backend is fully independent.

**Rejected:** `codegen_util` module in `vexil-lang` with type traversal helpers.

**Rationale:** The traversal patterns are trivial — iterate `declarations`, look up in `registry`. Premature abstraction constrains backends with assumptions about how they walk the IR. If a clear pattern emerges after 2-3 backends, extract then.

### Overall approach: minimal extraction vs layered API vs query-based redesign

**Chosen:** Layered API (Approach B) now, query-based redesign (Approach C) as a dedicated future milestone.

**Rejected alternatives:**
- **Approach A (Minimal trait extraction):** No API stability boundary. Doesn't prepare for LSP at all.
- **Approach C now (Full query-based redesign):** Major rewrite before a second consumer validates the API. Would delay TypeScript backend significantly. Better to build it after two backends exist and LSP requirements are concrete.

**Rationale:** Approach B formalizes the SDK with clear tiers and the backend trait, preparing the ground for query-based internals without building them prematurely. The TypeScript backend (next milestone) validates the Tier 1 API surface before we invest in query infrastructure.
