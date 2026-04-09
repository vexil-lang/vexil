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

### v1.0.0 (2026-04-09)

Language complete. New types, codegen for all backends, optimized runtime, comprehensive docs.

**New types:**
- `fixed32` (Q16.16), `fixed64` (Q32.32) — deterministic fixed-point arithmetic
- `vec2<T>`, `vec3<T>`, `vec4<T>`, `quat<T>`, `mat3<T>`, `mat4<T>` — geometric primitives
- `array<T, N>` — fixed-size arrays (no count prefix on wire)
- `set<T>` — sorted unique collections
- `bits { ... }` — inline bitfields

**New declarations:**
- `type Name = Target` — transparent type aliases
- `const Name : Type = Value` — compile-time constants with cross-reference arithmetic
- `where value > 0` — field-level validation constraints (auto-checked on encode/decode)

**Runtime improvements:**
- `BitWriter::with_capacity(n)` — pre-allocate to avoid reallocations
- `BitWriter::reset()` — reuse writer for batch encoding
- `BitReader::read_bytes_ref()` — zero-copy byte slice reads
- `BitReader::read_string_ref()` — zero-copy string reads
- `write_bits` / `read_bits` fast path for byte-aligned writes
- Checked arithmetic in const evaluation (no overflow panics)
- `CompiledSchema` compile-time `Send + Sync` assertion

**Codegen:**
- All three backends (Rust, TypeScript, Go) generate pack/unpack for new types
- Golden test files for all new types
- 14 new compliance vector tests

**Docs:**
- 105-file conformance corpus (41 valid, 64 invalid)
- 32 golden byte tests
- 11 book chapters
- Doc comments on all public items across all crates
- Human-voice README, FAQ, contributing guide

**Spec:** 1.0.0-draft (frozen wire format)

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
