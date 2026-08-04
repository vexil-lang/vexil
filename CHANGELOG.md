# Changelog

This file summarizes repository-wide release milestones. Detailed Rust crate
changes remain in each crate's changelog.

## Unreleased

### Language and compatibility

- Classify every message field addition as breaking under the current unframed
  message wire. Nested and aggregate values do not carry an old-schema boundary,
  so decoders no longer promise to infer appended fields from EOF or trailing
  bytes. Go and Python generated decoders now propagate EOF at an optional
  presence bit, matching Rust and TypeScript. Existing encoded bytes are
  unchanged.

### Tooling

- Add checked, portable project-output construction for codegen backends and
  validate the complete generated path set before `vexilc build` writes files.
  Existing `CodegenBackend` implementations remain source compatible.
- Add a diagnostics-first `vexilc lsp` stdio server for unsaved single-file
  documents, with full-document synchronization and UTF-16-safe ranges. This
  is available from current source builds and was not included in `vexilc`
  0.6.0.

## 0.6.0 release wave — 2026-08-03

This focused 0.x stabilization wave is not a 1.0 declaration. It closes known
contract gaps, makes generated output safer, and refreshes the path from
installation to a working schema.

### Language and compiler

- Enforce SemVer requirements on imports, including direct, aliased,
  transitive, and diamond project graphs.
- Support transparent aliases for concrete container and named types, including
  imported aliases, without changing canonical hashes or wire layouts.
- Reject message invariants with a dedicated diagnostic until portable
  encode/decode semantics are specified; they are no longer silently accepted.
- Preserve typed tombstones as wire-inert history metadata.
- Generate portable trait methods from the supported straight-line expression
  subset and reject unsupported bodies before emission.

### Wire behavior and generated code

- Keep the released `result<T, E>` discriminants: `0` encodes `Err`, `1`
  encodes `Ok`.
- Preserve unknown `@non_exhaustive` union discriminants and payload bytes in
  Rust, TypeScript, Go, and Python, with bounded length-prefix decoding and
  collision-safe generated names.
- Correct nested optional encoding, constraint validation, deterministic map
  and set ordering, inline bitfields, geometric values, newtype keys, and
  target-specific generated-code defects found by native compilation and
  execution.
- Correct Rust delta encoders for named fields by bringing the generated
  `Pack` trait call into scope.
- Expand generated Go and Python execution against the shared compliance
  vectors while retaining an honest, representative-coverage claim.

### Tooling and release preparation

- Make generated Rust, TypeScript, Go, and Python code part of enforced native
  validation rather than relying on textual goldens alone.
- Publish Python `vexil-runtime` 0.1.0 through Trusted Publishing with a tested
  public API, dual-license metadata, and built-distribution checks.
- Refresh the README and book around a tested first-run path, target support
  boundaries, release notes, and version-neutral future work.
- Replace the accumulated example collection with a guided path: quickstart,
  project evolution, cross-language wire agreement, and live telemetry. CI now
  regenerates and runs that path to catch stale generated output.
- Remove the repository's mutating pre-commit hook. Formatting remains an
  explicit local command and a CI check instead of silently widening a commit.

### Compatibility notes

- This release does not introduce a new wire format. The Result correction
  restores the already published byte contract; decoders do not accept both
  discriminant orders.
- Unknown non-exhaustive union values now round-trip instead of losing their
  payload. Applications matching union values should retain their generated
  unknown case.
- Cross-field constraints, regex constraints, invariants, RPC, a standard
  library, and other roadmap ideas have no promised release number.

## Published component versions

There has been no Vexil 1.0 release. Components version independently; the
versions published in this release wave are:

| Component | Published version |
| --- | --- |
| `vexil-lang` and Rust/TypeScript/Go/Python generators | `0.5.0` |
| Rust `vexil-runtime` and `vexilc` | `0.6.0` |
| `vexil-store` | `0.5.0` |
| TypeScript runtime | `0.5.2` |
| Go runtime module | `0.1.1` |
| Python `vexil-runtime` | `0.1.0` |

Earlier root tags `v0.1.0` and `v0.2.0` established the compiler, schema hash,
project compilation, store formats, and initial release pipeline. Component
changelogs and Git tags are authoritative for package-specific history.

## Component changelogs

- [vexil-lang](crates/vexil-lang/CHANGELOG.md)
- [vexil-runtime](crates/vexil-runtime/CHANGELOG.md)
- [vexil-codegen-rust](crates/vexil-codegen-rust/CHANGELOG.md)
- [vexil-codegen-ts](crates/vexil-codegen-ts/CHANGELOG.md)
- [vexil-codegen-go](crates/vexil-codegen-go/CHANGELOG.md)
- [vexil-codegen-py](crates/vexil-codegen-py/CHANGELOG.md)
- [vexil-store](crates/vexil-store/CHANGELOG.md)
- [vexilc](crates/vexilc/CHANGELOG.md)
