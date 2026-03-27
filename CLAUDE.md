# CLAUDE.md

## What is Vexil?

Vexil (Validated Exchange Language) is a typed schema definition language with first-class encoding semantics. It describes the shape, constraints, and wire encoding of data crossing system boundaries. LSB-first bit packing, no self-description on the wire — the schema is the contract.

This repo contains the language spec, formal PEG grammar, conformance corpus, compliance vectors, and the reference implementation (7-crate Rust workspace + TypeScript runtime).

## Repo Structure

```
spec/
  vexil-spec.md              # Language specification (normative, §1–§14 + appendices)
  vexil-grammar.peg           # Formal PEG grammar derived from spec
corpus/
  MANIFEST.md                 # Index of all test files with spec references
  valid/                      # Conformant impl MUST accept all (26 files)
  invalid/                    # Conformant impl MUST reject all (56 files)
  projects/                   # Multi-file project tests (simple, diamond, mixed)
compliance/
  vectors/                    # Golden byte vectors (JSON) — cross-implementation contract
crates/
  vexil-lang/                 # Core: lexer, parser, AST, IR, type checker, canonical, project compiler
  vexil-codegen-rust/         # Rust backend: CodegenBackend impl, struct/enum/encode/decode generation
  vexil-codegen-ts/           # TypeScript backend: CodegenBackend impl, interfaces/encode/decode generation
  vexil-runtime/              # Runtime support: Pack/Unpack traits, BitWriter/BitReader
  vexil-store/                # Schema-driven .vx text and .vxb binary file formats
  vexil-bench/                # Benchmark suite (Criterion, publish = false)
  vexilc/                     # CLI: check, codegen, build subcommands
packages/
  runtime-ts/                 # @vexil/runtime npm package: TypeScript BitWriter/BitReader
docs/
  limitations-and-gaps.md     # Wire format limitations, gaps, room for improvement
  superpowers/specs/          # Design specs (SDK, TS backend, LSP, release model)
  superpowers/plans/          # Implementation plans
examples/
  cross-language/             # Rust ↔ Node interop via binary files
  system-monitor/             # Real-time dashboard: Rust → browser via Vexil WebSocket
```

## Build Commands

```bash
cargo build --workspace              # build everything
cargo test --workspace               # all tests (~500)
cargo test -p vexil-lang             # core crate only
cargo test -p vexil-codegen-rust     # Rust codegen + golden + compliance tests
cargo test -p vexil-codegen-ts       # TypeScript codegen + golden tests
cargo clippy --workspace -- -D warnings  # must be clean
cargo fmt --all                      # format
cargo fmt --all -- --check           # CI format check
cargo bench -p vexil-bench           # encode/decode benchmarks
cd packages/runtime-ts && npx vitest run  # TypeScript runtime tests (120)
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

vexil-codegen-ts    TypeScript code generation. Depends on vexil-lang.
                    Implements CodegenBackend trait. Generates interfaces, encode/decode functions.
                    Barrel index.ts files for namespace directories. Cross-file relative imports.

vexil-runtime       Pack/Unpack traits + BitWriter/BitReader.
                    No workspace deps. Used by generated Rust code.

@vexil/runtime      TypeScript npm package at packages/runtime-ts/.
                    BitWriter/BitReader matching Rust wire format byte-for-byte.
                    Zero dependencies. Used by generated TypeScript code.

vexil-store         Schema-driven encoder/decoder with .vx text and .vxb binary formats.
                    Depends on vexil-lang + vexil-runtime.

vexil-bench         Benchmark suite (Criterion). publish = false.
                    Envelope, DrawText, OutputChunk, batch benchmarks.

vexilc              CLI binary. Depends on vexil-lang + vexil-codegen-rust + vexil-codegen-ts.
                    Subcommands: check, codegen (--target), build (--target, --include, --output)
                    Targets: rust (default), typescript
```

## SDK Architecture (v0.2.0)

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
- **v0.2.0** vexil-store, release-plz + cargo-dist pipeline — RELEASED
- **v0.3.0** TypeScript backend, compliance vectors, benchmarks — RELEASED
- **G** Package manager (registry, lockfile, fetch/publish) — PLANNED
- **LSP** — SPECCED (docs/superpowers/specs/2026-03-26-lsp-editor-tooling-design.md)

## Design Decisions

- **Separate AST and IR** — AST is source-faithful (errors, LSP), IR is resolved (type checking, codegen)
- **Wire encoding:** LSB-first bit packing, LEB128 varints, ZigZag signed, BLAKE3 schema hash
- **Build sequence:** spec → grammar → corpus → reference implementation (spec-driven + TDD)
- **Per-crate versioning** — each crate versions independently; only crates with actual changes get bumped
- **Trunk-based development** — small fixes and patches go directly on main; milestone-sized features use a `feature/<name>` branch and merge via PR

## Code Standards

- Rust edition 2021, MSRV 1.94
- `thiserror` for error types
- `#[derive(Debug, Clone, PartialEq)]` on data types
- No `unwrap()` or `expect()` in non-test code
- All `unsafe` blocks require `// SAFETY:` comments
- Explicit re-exports only — no `pub use foo::*`

## Golden Files

Codegen golden tests live in `crates/vexil-codegen-rust/tests/golden/` (Rust) and `crates/vexil-codegen-ts/tests/golden/` (TypeScript).
To regenerate after intentional codegen changes:

```bash
UPDATE_GOLDEN=1 cargo test -p vexil-codegen-rust
UPDATE_GOLDEN=1 cargo test -p vexil-codegen-ts
```

## Compliance Vectors

Cross-implementation golden byte vectors live in `compliance/vectors/*.json`.
Both Rust and TypeScript implementations must produce identical bytes for each vector.
Rust validator: `crates/vexil-codegen-rust/tests/golden_bytes.rs`.
TypeScript validator: `packages/runtime-ts/tests/compliance.test.ts`.

## Corpus Contribution

Adding a corpus file requires two things:
1. The `.vexil` file in `corpus/valid/` or `corpus/invalid/`
2. A corresponding entry in `corpus/MANIFEST.md` with spec reference

Corpus files are named `NNN_description.vexil`. Check the highest existing number before adding.

## Release Lifecycle

- **Patch (v0.x.y)** — bug fix only; must not change wire format; no audit needed
- **Minor (v0.x.0)** — milestone complete; full pre-release audit; spec revision tagged if language changed
- **Major (v1.0.0)** — Tier 1 API frozen, spec at v1.0, corpus contract stable

Wire format changes require RFC (14-day comment period per GOVERNANCE.md).
Corpus file additions are non-breaking; modifications to existing files are breaking.

**Tooling:** Releases are fully automated via release-plz + cargo-dist + npm publish.

- On every push to `main`, release-plz opens/updates a Release PR with version bumps and changelogs
- Merging the Release PR triggers: crates.io publish (only changed crates) → git tags → cargo-dist binary builds → GitHub Release with artifacts + checksums
- The `release` job ONLY runs when the commit message starts with `chore(release):` — docs/CI changes won't trigger accidental releases
- `vexil-runtime-v*` tags also trigger npm publish of `@vexil/runtime` (same version)
- `release-plz.toml` configures per-crate independent versioning — only crates with changes get bumped
- `cliff.toml` configures changelog generation from conventional commits
- `dist-workspace.toml` configures cargo-dist targets and installers
- Never edit `Cargo.toml` versions by hand — release-plz manages them via the Release PR

## Git Workflow

- Pre-commit hook runs `cargo fmt --all` and re-stages with `git add -u` — commits are always formatted
- `VEXIL_NO_FMT=1` to bypass format check; `VEXIL_COMMIT_TASK=1` for task commits
- Always `cargo fmt --all` before committing
- Always `git pull origin main` before starting — multi-agent sessions are common
- CI/release workflow changes MUST go on a branch, not main
- Milestone-sized features use `feature/<name>` branches; worktrees live in `.worktrees/`
