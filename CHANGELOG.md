# Changelog

Individual crate changelogs track detailed changes:

- [vexil-lang](crates/vexil-lang/CHANGELOG.md)
- [vexil-runtime](crates/vexil-runtime/CHANGELOG.md)
- [vexil-codegen-rust](crates/vexil-codegen-rust/CHANGELOG.md)
- [vexil-codegen-ts](crates/vexil-codegen-ts/CHANGELOG.md)
- [vexil-codegen-go](crates/vexil-codegen-go/CHANGELOG.md)
- [vexil-store](crates/vexil-store/CHANGELOG.md)
- [vexilc](crates/vexilc/CHANGELOG.md)

## Releases

### v0.5.0 / v0.4.2 (2026-03-29)

CLI polish, code quality, Go backend, delta streaming, schema evolution.

- **CLI:** `vexilc watch` (auto-rebuild on save), `vexilc init`, `vexilc hash`, `--version`, `--help`
- **Go backend:** `vexil-codegen-go` crate + `packages/runtime-go` module
- **Delta encoding:** `@delta` on messages with automatic varint/zigzag selection
- **Schema evolution:** `vexilc compat` CLI, `_unknown` field preservation, typed tombstones, `SchemaHandshake`
- **Cross-language:** Rust ↔ TypeScript ↔ Go interop verified by compliance vectors
- **Examples:** System monitor dashboard (Rust → browser via WebSocket), cross-language sensor telemetry
- **Code quality:** No `unwrap()` in production code, all public APIs documented, rustdoc clean

### v0.2.4 (2026-03-27)

TypeScript backend, compliance infrastructure, benchmarks.

- **TypeScript backend:** `vexil-codegen-ts` crate + `@vexil-lang/runtime` npm package
- **Compliance vectors:** 8 JSON golden byte vector files
- **Benchmark suite:** `vexil-bench` crate with Criterion
- **Spec §11:** Encoding edge cases (normative)

### v0.2.0 (2026-03-26)

SDK architecture, vexil-store, release pipeline.

### v0.1.0 (2026-03-26)

Initial release. Lexer, parser, AST, IR, type checker, canonical form, BLAKE3 schema hash.
