# CLAUDE.md

## What is Vexil?

Vexil (Validated Exchange Language) is a typed schema definition language with first-class encoding semantics. It describes the shape, constraints, and wire encoding of data crossing system boundaries. LSB-first bit packing, no self-description on the wire — the schema is the contract.

This repo contains the language spec, formal PEG grammar, conformance corpus, and the reference implementation (4-crate Rust workspace, v0.1.0).

## Repo Structure

```
spec/
  vexil-spec.md              # Language specification (normative, v0.1.0-draft-r2)
  vexil-grammar.peg           # Formal PEG grammar derived from spec
corpus/
  MANIFEST.md                 # Index of all test files with spec references
  valid/                      # Conformant impl MUST accept all
  invalid/                    # Conformant impl MUST reject all
  projects/                   # Multi-file project tests (simple, diamond, mixed)
crates/
  vexil-lang/                 # Core: lexer, parser, AST, IR, type checker, canonical, project compiler
  vexil-codegen-rust/         # Rust backend: CodegenBackend impl, struct/enum/encode/decode generation
  vexil-runtime/              # Runtime support: Encode/Decode traits, BitpackEncoder/Decoder
  vexilc/                     # CLI: check, codegen, build subcommands
docs/superpowers/specs/       # Design specs (SDK, TS backend, LSP, release model)
docs/superpowers/plans/       # Implementation plans (milestones C–F, v0.1.0 release)
```

## Build Commands

```bash
cargo build --workspace              # build everything
cargo test --workspace               # all tests (~258)
cargo test -p vexil-lang             # core crate only
cargo test -p vexil-codegen-rust     # codegen + project integration tests
cargo clippy --workspace -- -D warnings  # must be clean
cargo fmt --all                      # format
cargo fmt --all -- --check           # CI format check
```

## Crate Architecture

```
vexil-lang          Core compiler library. Zero internal deps.
                    Pipeline: Lexer → Parser → AST → Lower → IR → TypeCheck → Validate
                    Also: project compiler (multi-file), canonical form, BLAKE3 hash
                    Public API tiered: Tier 1 (stable), Tier 2 (semi-stable), Tier 3 (internal)

vexil-codegen-rust  Rust code generation. Depends on vexil-lang.
                    Implements CodegenBackend trait. Generates structs, enums, encode/decode.
                    generate() for single-file, generate_project() for multi-file with imports.

vexil-runtime       Encode/Decode traits + BitpackEncoder/BitpackDecoder.
                    No workspace deps. Used by generated code.

vexilc              CLI binary. Depends on vexil-lang + vexil-codegen-rust.
                    Subcommands: check, codegen (--target), build (--target, --include, --output)
```

## SDK Architecture (v0.1.0)

- `CodegenBackend` trait in `vexil-lang::codegen` — pluggable backends implement `generate()` + `generate_project()`
- `CodegenError` — shared error enum with `BackendSpecific(Box<dyn Error>)` for extensibility
- API stability tiers: Tier 1 (compile, IR types, trait), Tier 2 (AST, pipeline stages), Tier 3 (lexer, parser, canonical)
- `generate_with_imports` is `pub(crate)` in vexil-codegen-rust — backends own their import strategy
- Cross-file imports: `generate_project()` builds global type→path map, identifies imported types per schema

## Key Types

- `CompiledSchema` — single-file compilation result (registry, declarations, namespace, wire_size)
- `ProjectResult` — multi-file result: `Vec<(String, CompiledSchema)>` in topological order
- `TypeRegistry` — opaque type store indexed by `TypeId`
- `declarations` = TypeIds declared in THIS file; registry minus declarations = imported types
- `TypeDef` — Message, Enum, Flags, Union, Newtype, Config

## Milestone Status

- **B** Frontend (lexer, parser, AST, corpus tests) — DONE
- **C** Lowering → IR → Type Checker — DONE
- **D** Rust codegen backend — DONE
- **E** Canonical form + BLAKE3 schema hash — DONE
- **F** Multi-file import resolution (transitive remap, diamond dedup) — DONE
- **v0.1.0** SDK architecture, release CI, CodegenBackend trait — RELEASED
- **G** Package manager (registry, lockfile, fetch/publish) — PLANNED
- **TS backend** — SPECCED (docs/superpowers/specs/2026-03-26-typescript-backend-design.md)
- **LSP** — SPECCED (docs/superpowers/specs/2026-03-26-lsp-editor-tooling-design.md)

## Design Decisions

- **Separate AST and IR** — AST is source-faithful (errors, LSP), IR is resolved (type checking, codegen)
- **Wire encoding:** LSB-first bit packing, LEB128 varints, ZigZag signed, BLAKE3 schema hash
- **Build sequence:** spec → grammar → corpus → reference implementation (spec-driven + TDD)
- **Lockstep versioning** — all workspace crates share version (v0.MILESTONE.PATCH)
- **Trunk-based development** — all work on main, releases are tags (tag-triggered CI)

## Code Standards

- Rust edition 2021, MSRV 1.94
- `thiserror` for error types
- `#[derive(Debug, Clone, PartialEq)]` on data types, `serde::Serialize, Deserialize` on wire types
- No `unwrap()` or `expect()` in non-test code
- All `unsafe` blocks require `// SAFETY:` comments
- Explicit re-exports only — no `pub use foo::*`

## Git Workflow

- Pre-commit hook runs `cargo fmt --check` and rejects unformatted code
- `VEXIL_NO_FMT=1` to bypass format check; `VEXIL_COMMIT_TASK=1` for task commits
- Always `cargo fmt --all` before committing
- Always `git pull origin main` before starting — multi-agent sessions are common
- CI/release workflow changes MUST go on a branch, not main
