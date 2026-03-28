# Changelog

## [0.4.1] - 2026-03-28

### Fixed

- fix: code quality polish — remove unwrap, fix rustdoc, add crate docs, add derives

### Other

- chore(release): v0.4.1

## [0.3.0] - 2026-03-28

### Added

- feat(vexil-lang): typed tombstones — @removed with original type for decode-and-discard
- feat(vexil-lang): add compat module with report types
- feat(vexil-lang): desugar @delta on message to per-field @delta

### Fixed

- fix: message-level @delta implies varint/zigzag for wire size reduction

### Documentation

- docs: fix merge conflict markers and restore rewritten READMEs
- docs: update README, FAQ, CLAUDE.md, and crate READMEs for delta streaming
- docs: rewrite READMEs, fix stale facts, remove AI patterns

### Other

- chore(release): bump crate versions
- corpus: add 027_delta_on_message for @delta on message declarations

All notable changes to this project will be documented in this file.



## 0.2.0 (2026-03-27)

### New Features

- `meta_schema()` and `pack_schema()` — pre-compiled `vexil.schema` and `vexil.pack` schemas exposed as static references for use by `vexil-store`
- `CodegenBackend` trait — pluggable code generation; implement `generate()` + `generate_project()` to add a new target language
- `CodegenError` — shared error type with `BackendSpecific(Box<dyn Error>)` for backend extensibility
- Multi-file project compiler (`compile_project()`) — resolves transitive imports, detects cycles, deduplicates diamonds
- `SchemaLoader` trait + `FilesystemLoader` + `InMemoryLoader` — abstraction layer for multi-root schema resolution
- `source_file` field on `Diagnostic` — pinpoints errors to the originating file in multi-file compilations

### Bug Fixes

- Transitive type remapping and diamond deduplication in `clone_types_into`
- Aliased import TypeId remapping for cross-file type references
- Reject schemas without a namespace in the import graph (prevents HashMap key collisions)

### API Stability

Stability tiers documented on all public modules. `compile()`, IR types, and `CodegenBackend` are Tier 1 (stable for the v0.x series).

## 0.1.0 (2026-03-26)

Initial release. Lexer, parser, AST, IR lowering, type checker, canonical form, and BLAKE3 schema hash. All 74 conformance corpus files (18 valid, 56 invalid) pass.
