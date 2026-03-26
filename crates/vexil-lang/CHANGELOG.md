# Changelog

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
