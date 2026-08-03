# Changelog

## [0.5.0] - 2026-08-03

### Added

- **codegen:** Preserve unknown union payloads ([d18c3b0](https://github.com/vexil-lang/vexil/commit/d18c3b0756ecb0b57971332d1308433ceab0df57))
- **lang:** Support concrete type aliases ([0a3c42a](https://github.com/vexil-lang/vexil/commit/0a3c42ac883603e856ac0c97f6ed83a3424fbb42))
- Complete trait function codegen readiness ([#119](https://github.com/vexil-lang/vexil/issues/119)) ([42a2434](https://github.com/vexil-lang/vexil/commit/42a2434374cd9d6ea5b7b7b3b448d61e1fd34ef9))

### Fixed

- **codegen:** Repair generic trait projection ([8a9df39](https://github.com/vexil-lang/vexil/commit/8a9df391a669c86495691cff731159aa88c1bb12))
- **codegen:** Close revival contract gaps ([407f490](https://github.com/vexil-lang/vexil/commit/407f490f233508a58025ec23ceefdfcde0c46ffa))
- **codegen-rust:** Emit lint-clean Rust ([#106](https://github.com/vexil-lang/vexil/issues/106)) ([1b44cbc](https://github.com/vexil-lang/vexil/commit/1b44cbc32d1bf9db7e194a37c0add32ff679ddd5))
- **lang:** Reject unsupported invariants ([6a208f6](https://github.com/vexil-lang/vexil/commit/6a208f6273e54828b33c8f36b3837f8b40ca5e76))
- **lang:** Enforce import version constraints ([3bfee23](https://github.com/vexil-lang/vexil/commit/3bfee23a93e3913b3b3de0fdc1bc289473fa06f1))
### Documentation

- Relaunch Vexil for revival release ([f3529ff](https://github.com/vexil-lang/vexil/commit/f3529ff42869199156ef2f114ad220d1d462d8e8))

- Give Vexil a fresh public face ([e833356](https://github.com/vexil-lang/vexil/commit/e833356fb67b42cf3dce7e19092a8a332749eb82))

### Testing

- **corpus:** Rename trait function signature fixture ([#122](https://github.com/vexil-lang/vexil/issues/122)) ([3b70f5c](https://github.com/vexil-lang/vexil/commit/3b70f5cab9b7b2c59d5e50537d6de0e01e3a2162))
- **lang:** Cover diamond alias remapping ([7e8184f](https://github.com/vexil-lang/vexil/commit/7e8184f3e70ff0926660404e4acbafae9c3ba455))
### Other

- **deps:** Refresh Rust and example toolchain dependencies ([#93](https://github.com/vexil-lang/vexil/issues/93)) ([bb93fb2](https://github.com/vexil-lang/vexil/commit/bb93fb2a27104e5600adb735c54415682d1d5938))
- Align published crate metadata ([fa36e68](https://github.com/vexil-lang/vexil/commit/fa36e6890df0e5d14c45a322eeacd4269d77b529))


## Unreleased

### Added

- Enforce schema `@version` and import SemVer requirements across project graphs.
- Resolve transparent aliases for concrete container and imported named types.

### Fixed

- Reject unsupported message invariants instead of silently compiling them.
- Preserve declaration identity through aliases, imports, tombstones, traits,
  and diamond dependency remapping.

## [0.4.3] - 2026-03-29

### Fixed

- fix(vexilc,resolve): check --include, prefix-stripped loader, no panic on unresolved TypeId (#48)
- fix(parser): don't treat annotation after bare import as version constraint

### Documentation

- docs: update all READMEs and changelogs for v0.5.0 — watch, init, hash, Go backend

## [0.4.1] - 2026-03-28

### Fixed

- fix: code quality polish — remove unwrap, fix rustdoc, add crate docs, add derives

### Other

- chore(release): v0.4.1

## [0.3.0] - 2026-03-28

### Added

- feat(vexil-lang): typed tombstones — retain the original type as history metadata
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
