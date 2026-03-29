# Changelog

## [0.5.1] - 2026-03-29

### Documentation

- docs: update all READMEs and changelogs for v0.5.0 — watch, init, hash, Go backend

## [0.5.0] - 2026-03-28

### Added

- feat(vexil-runtime): add SchemaHandshake for connection-time identity checking
- feat(@vexil/runtime): add SchemaHandshake for connection-time identity checking
- feat(runtime): add read_remaining()/readRemaining() to BitReader
- feat: TypeScript backend, compliance infrastructure, and benchmarks (#10)
- feat(vexil-codegen): annotation emission — doc, deprecated, since, non_exhaustive, tombstones
- feat(vexil-runtime): BitReader — LSB-first reader with LEB128, ZigZag, recursion depth tracking
- feat(vexil-runtime): BitWriter — LSB-first bit accumulator with LE integers, NaN canonicalization, LEB128
- feat(vexil-runtime): LEB128 encode/decode + ZigZag mapping
- feat(vexil-runtime): scaffold crate — error types, Pack/Unpack traits, global limits

### Fixed

- fix: code quality polish — remove unwrap, fix rustdoc, add crate docs, add derives
- fix(vexil-codegen): resolve 4 pre-existing codegen bugs found by compile_check

### Documentation

- docs: publication readiness — fix versions, add Go README, fix package names, update changelog
- docs: rewrite READMEs, fix stale facts, remove AI patterns
- docs: add API documentation to vexil-runtime public items
- docs: add per-crate README files for crates.io

### Other

- chore(release): v0.4.1
- chore(release): bump all crates to v0.4.0
- chore(vexil-runtime): update changelog for 0.3.0
- chore(vexil-runtime): bump version to 0.3.0
- chore(release): bump crate versions
- chore(release): v0.2.3 (#14)
- chore(release): v0.2.3 (#11)
- chore(release): v0.2.2
- chore(release): v0.2.1
- chore: release v0.2.0
- test(vexil-runtime): wire format round-trip tests — sub-byte, optional, result, delta, union, LEB128

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
