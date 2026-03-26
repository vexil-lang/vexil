# LSP / Editor Tooling Design

> **Scope:** Language server for the Vexil schema language. Covers LSP architecture, feature tiers, compiler integration, project manifest, and `vexilc lsp` subcommand. Does NOT cover query-based compiler internals, package management, or editor-specific extensions (VS Code, JetBrains).

**Goal:** Ship a lightweight LSP that provides diagnostics, go-to-definition, and hover using the existing `compile_project()` pipeline. No incremental compilation — full recompilation on every change, which is fast enough for schema-sized files.

**Architecture:** `vexil-lsp` library crate using `tower-lsp`, launched via `vexilc lsp`. Re-compiles the full project on each file change. Project structure discovered via `vexil.toml` manifest.

**Tech Stack:** Rust, tower-lsp, vexil-lang (Tier 1 + Tier 2 API).

**Depends on:** SDK Architecture Design (2026-03-26-sdk-architecture-design.md) — specifically the Tier 1/Tier 2 API surface and `ProjectResult` type.

---

## 1. Architecture

```
vexil-lsp           — Library crate, LSP protocol implementation
vexilc lsp          — Binary entry point, launches the LSP server
```

`vexil-lsp` depends on `vexil-lang` (Tier 1 + Tier 2 API). Uses `tower-lsp` for the LSP protocol layer.

### State model

- In-memory map of `FileId → source text` (overlay for unsaved editor buffers)
- Custom `SchemaLoader` that reads from the overlay first, filesystem second
- Project root detected by walking up for `vexil.toml` or directory of `.vexil` files
- Full recompilation via `compile_project()` on every file change
- No caching, no incremental compilation

### Why no incrementality

Schema files are small (hundreds of lines, not thousands). A 10-file project recompiles in milliseconds. Full recompilation is simple, correct, and fast enough. If profiling shows otherwise, the query-based compiler can be added later behind the same Tier 1 API without changing the LSP.

---

## 2. Feature Tiers

### Tier A — Essential (this milestone)

**Diagnostics:** Errors and warnings pushed to editor on save/change. Maps `vexil-lang` `Diagnostic` (with `Span`) to LSP `Diagnostic` (with `Range`). Covers:
- Parse errors (syntax)
- Validation errors (duplicate names, reserved words, invalid annotations)
- Type errors (unresolved types, recursive cycles, wire size issues)
- Import resolution failures (missing files, cycles, ambiguous names)

**Go-to-definition:** Click on a type name → jump to its declaration. For imported types, jump to the source file and span. Uses name resolution from the lowering phase.

**Hover:** Hover on a type name → show type summary as markdown (kind, fields/variants, wire size). Hover on a field → show resolved type and encoding info.

### Tier B — Planned (future milestone)

- **Autocomplete:** Type names, field types, annotation names
- **Find references:** All uses of a type across files
- **Rename symbol:** Type names with cross-file updates

### Tier C — Future

- Code actions (quick fixes for common errors)
- Semantic highlighting
- Document/workspace symbols

---

## 3. Compiler Integration

### What the LSP needs from `vexil-lang`

**Diagnostics** — Already exposed. `compile_project()` returns `Vec<Diagnostic>` with `Span` and `source_file`. Map `Span` → LSP `Range` using line/column lookup on source text.

**Go-to-definition** — Requires:
1. Position → token lookup (reparse the file or maintain a simple token-to-span index)
2. Token → type name → `TypeRegistry::lookup(name)` → `TypeId`
3. `TypeId` → declaring file + `Span`

**Hover** — `TypeRegistry::get(TypeId)` → `TypeDef`. Format as markdown showing kind, fields, wire size.

### Required `vexil-lang` changes

**`ProjectResult` gains type location tracking:**
```rust
pub struct ProjectResult {
    pub schemas: Vec<(String, CompiledSchema)>,
    pub diagnostics: Vec<Diagnostic>,
    pub type_locations: HashMap<TypeId, (PathBuf, Span)>,  // NEW
}
```

`type_locations` maps each `TypeId` to the file and span where it was declared. Built during `compile_project()` as each file is compiled. This is a Tier 1 addition (stable).

No other API changes needed. All LSP features are built from existing `TypeRegistry`, `TypeDef`, `ResolvedType`, and `Diagnostic` types.

---

## 4. `vexil.toml` Project Manifest

Minimal format for project structure discovery:

```toml
[project]
name = "my-schemas"
root = "schemas/root.vexil"      # entry point for compilation
include = ["schemas/"]           # directories to search for imports
```

### Usage

- **`vexilc lsp`** — Discovers project structure on startup. Walks up from open file to find `vexil.toml`.
- **`vexilc build`** — Reads root and include paths. Replaces CLI flags when present.
- **CLI flags override** `vexil.toml` values when both are provided.

### Discovery

LSP walks up from the open file looking for `vexil.toml`. If not found, falls back to:
1. Treating the workspace root as an include directory
2. Requiring an open `.vexil` file to start

### Scope

Intentionally minimal. No dependency versioning, no package management, no build targets. Those are separate concerns for future milestones.

---

## 5. LSP Server Implementation

### Initialization

```
Client opens .vexil file
  → LSP discovers project root (walk up for vexil.toml)
  → LSP builds initial SchemaLoader from include paths
  → LSP runs compile_project() for initial diagnostics
  → LSP pushes diagnostics to client
```

### On file change

```
Client edits file (didChange / didSave)
  → LSP updates in-memory overlay for that file
  → LSP re-runs compile_project() with overlay-aware SchemaLoader
  → LSP maps new Diagnostic set to LSP diagnostics
  → LSP pushes updated diagnostics to client
```

### On go-to-definition request

```
Client requests definition at (file, line, col)
  → LSP finds token at position (reparse or token index)
  → LSP looks up type name in TypeRegistry
  → LSP finds TypeId in type_locations map
  → LSP returns (file, range) to client
```

### On hover request

```
Client requests hover at (file, line, col)
  → LSP finds token at position
  → LSP looks up TypeDef from TypeRegistry
  → LSP formats TypeDef as markdown summary
  → LSP returns hover content to client
```

---

## 6. Binary Entry Point

`vexilc lsp` subcommand:

```
vexilc lsp [--stdio]
```

Default communication is stdio (standard for LSP). The `--stdio` flag is explicit but optional (matches convention of other LSP servers).

No TCP or socket mode initially. Editors universally support stdio.

---

## 7. Decision Log

### Standalone binary vs `vexilc` subcommand

**Chosen:** `vexilc lsp` subcommand.

**Rejected:** Standalone `vexil-lsp` binary.

**Rationale:** Users install one tool and get everything. The LSP crate still exists as a library (`vexil-lsp`) for clean separation, but the binary entry point is `vexilc lsp`. This is the pattern used by `deno lsp`, `biome lsp`, and others.

### Incremental compilation: query-based vs full recompilation

**Chosen:** Full recompilation on every change. No caching.

**Rejected alternatives:**
- **Query-based compiler (salsa-style):** Major refactor of compiler internals. Over-engineering for schema-sized files that recompile in milliseconds.
- **Per-stage caching:** Added complexity with marginal benefit for small file counts.

**Rationale:** Schema files are small. Full recompilation is fast, simple, and correct. If profiling shows a need, query-based internals can be added behind the existing Tier 1 API without changing the LSP.

### Query API: salsa vs hand-rolled vs deferred

**Chosen:** Design the query API interfaces, defer framework choice.

**Rejected alternatives:**
- **Commit to salsa now:** Adds a significant dependency and learning curve before we know if incrementality is needed.
- **Commit to hand-rolled now:** Premature implementation decision.

**Rationale:** The spec defines what queries exist and their invalidation semantics. Whether they're backed by salsa or a simple HashMap is an implementation detail. Per-file invalidation with small file counts means even a naive approach works.

### Feature scope: full LSP vs essential features

**Chosen:** Tier A (diagnostics, go-to-definition, hover) as requirements. Tier B (autocomplete, find references, rename) planned. Tier C (code actions, semantic highlighting) future.

**Rejected:** Full LSP feature set in one milestone.

**Rationale:** Tier A covers the features that provide the most value with the least implementation effort. Each tier builds on the previous. Shipping Tier A first gets real editor support to users quickly.

### Project discovery: auto-detect vs explicit config vs both

**Chosen:** Auto-detect with `vexil.toml` as optional explicit config.

**Rejected:** Explicit-only configuration.

**Rationale:** Most LSPs auto-detect (rust-analyzer finds `Cargo.toml`, typescript finds `tsconfig.json`). `vexil.toml` provides explicit control when auto-detection isn't sufficient. CLI flags override both for scripting use cases.

### Multi-file state: overlay loader vs file watcher

**Chosen:** In-memory overlay with custom `SchemaLoader`. LSP updates overlay on `didChange`/`didSave` events.

**Rejected:** Filesystem watcher that detects external changes.

**Rationale:** The LSP protocol already notifies the server of file changes in the workspace. An additional file watcher adds complexity and race conditions. The overlay-first, filesystem-fallback pattern is standard for LSP implementations.
