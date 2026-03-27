# Changelog

All notable changes to this project will be documented in this file.



## 0.2.0 (2026-03-27)

### New Features

- `RustBackend` — implements the `CodegenBackend` trait from `vexil-lang`; supports single-file (`generate()`) and multi-file project (`generate_project()`) code generation
- Cross-file import `use` statements emitted automatically in `generate_project()`

### Bug Fixes

- `generate_with_imports` visibility tightened to `pub(crate)`
- Tier 1 re-exports aligned with `vexil-lang` public API

## 0.1.0 (2026-03-26)

Initial release. Generates Rust structs, enums, `Pack`/`Unpack` impls, and schema hash constants from compiled Vexil schemas.
