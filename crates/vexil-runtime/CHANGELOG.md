# Changelog

## [0.3.0] - 2026-03-28

### Added

- feat(vexil-runtime): add SchemaHandshake for connection-time identity checking
- feat(@vexil-lang/runtime): add SchemaHandshake for connection-time identity checking
- feat(runtime): add read_remaining()/readRemaining() to BitReader

### Documentation

- docs: rewrite READMEs, fix stale facts, remove AI patterns

### Other

- chore(release): bump crate versions

All notable changes to this project will be documented in this file.



## 0.2.0 (2026-03-27)

### Bug Fixes

- `Box<T>` blanket impls for `Pack`/`Unpack` — enables boxing of recursive types in generated code
- Delta-encoded fields now correctly round-trip through `Pack`/`Unpack`
- Union variant payload writer/reader fixed for `Named` type fields
- Config optional defaults now correctly wrapped in `Some()` when non-`None`

## 0.1.0 (2026-03-26)

Initial release. `Pack`/`Unpack` traits, `BitWriter`/`BitReader` (LSB-first bit I/O), LEB128 encode/decode, ZigZag mapping, NaN canonicalization for `f32`/`f64`, and global recursion depth limit.
