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

### v0.4.1 (2026-03-28)

Delta streaming, schema evolution, Go backend, three-way cross-language interop.

- **Delta encoding:** `@delta` on messages with automatic varint/zigzag selection
- **Schema evolution:** `vexilc compat` CLI, `_unknown` field preservation, typed tombstones, `SchemaHandshake`
- **Go backend:** `vexil-codegen-go` crate + `packages/runtime-go` module
- **Cross-language:** Rust ↔ TypeScript ↔ Go interop verified by compliance vectors
- **Examples:** System monitor dashboard (Rust → browser via WebSocket), cross-language sensor telemetry

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
