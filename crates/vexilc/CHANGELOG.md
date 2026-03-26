# Changelog

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
