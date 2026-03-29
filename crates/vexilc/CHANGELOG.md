# Changelog

## [0.5.1] - 2026-03-29

### Fixed

- fix(vexilc,resolve): check --include, prefix-stripped loader, no panic on unresolved TypeId (#48)

### Documentation

- docs: update all READMEs and changelogs for v0.5.0 — watch, init, hash, Go backend

## [0.5.0] - 2026-03-29

### Added

- `vexilc watch` — auto-rebuild on schema file changes with 200ms debounce
- `vexilc init [name]` — scaffold a new `.vexil` schema file
- `vexilc hash <file>` — print BLAKE3 schema hash
- `--version` / `-V` and `--help` / `-h` flags
- `--target go` — Go code generation backend

## [0.3.0] - 2026-03-28

### Added

- feat(vexilc): add compat subcommand for breaking change detection

### Documentation

- docs: fix merge conflict markers and restore rewritten READMEs
- docs: update README, FAQ, CLAUDE.md, and crate READMEs for delta streaming
- docs: rewrite READMEs, fix stale facts, remove AI patterns

### Other

- chore(release): bump crate versions
- test(vexilc): CLI integration tests for compat subcommand

## 0.2.0 (2026-03-27)

### New Features

- `vexilc store pack` — encode a `.vx` text file to a `.vxb` binary data file using a compiled schema
- `vexilc store unpack` — decode a `.vxb` binary file back to `.vx` text
- `vexilc store format` — render a `.vxb` file as human-readable output
- `--target` flag on `codegen` and `build` — selects the code generation backend (`rust` is the only target currently)

### Bug Fixes

- `codegen` subcommand now dispatches correctly through the `CodegenBackend` trait

## 0.1.0 (2026-03-26)

Initial release. `check` (validate schema), `codegen` (generate Rust code), and `build` (multi-file project compilation) subcommands with ariadne-rendered diagnostics and source spans.
