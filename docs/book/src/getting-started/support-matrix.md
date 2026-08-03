# Choose a Target

All four generators consume the same compiled schema, but their native evidence
is not identical. Choose a target based on what is verified today, then test the
specific schemas and environments your application will ship.

| Surface | Distribution | Evidence in this repository |
| --- | --- | --- |
| Compiler and CLI | Rust crates and release binaries | Workspace tests, corpus, project graphs, diagnostics, and compatibility checks |
| Editor diagnostics | `vexilc lsp` over stdio | Full-document single-file synchronization, compiler diagnostics, and UTF-16 range tests |
| Rust generated code | `vexil-runtime` | Broad golden, native compile, Clippy, and byte-vector coverage |
| TypeScript generated code | `@vexil-lang/runtime` | Native type-check/build/tests and broad byte-vector coverage |
| Go generated code | versioned Go module | Native compile and execution over a representative shared wire matrix |
| Python generated code | source install; first PyPI release candidate | Static checking and native execution over a representative shared wire matrix |

The curated [cross-language example](../examples/cross-language.md) compares one
readable fixture across all four targets. The generated-wire test suite covers a
larger representative matrix.

## Shared contract points

Every maintained target is expected to agree on:

- LSB-first bit packing and little-endian multi-byte scalars;
- LEB128 and ZigZag integer encodings;
- field and variant ordinals;
- canonical collection ordering;
- Result discriminants (`0 = Err`, `1 = Ok`);
- bounded preservation of unknown non-exhaustive union variants;
- canonical BLAKE3 schema hashes.

Differences in generated language API shape are target-specific. Differences in
wire bytes for the same schema and value are defects.

## Current boundaries

- Go and Python coverage is representative, not exhaustive.
- The language server is diagnostics-only and single-file. It does not load
  imports or projects and does not advertise completion, navigation, hover,
  formatting, incremental synchronization, or a bundled editor extension.
- The Python runtime is prepared for its first PyPI release but should be
  installed from source until the tagged Trusted Publishing workflow succeeds.
- No independent implementation or external security audit has been completed.
- The compiler does not promise unimplemented constraints, RPC, transport,
  encryption profiles, reflection, or a standard library for a named version.

Continue with [Compatibility and Limits](./compatibility.md) before deploying a
new protocol.
