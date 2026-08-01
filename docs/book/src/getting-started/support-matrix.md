# Support Matrix

Vexil is a multi-target 0.x project. The language and wire specifications are
the contract; each target has a different depth of native verification today.

| Surface | Availability | Verification level |
| --- | --- | --- |
| Compiler and CLI | Rust crates and release binaries | Full workspace tests, corpus, project graphs, compatibility checks |
| Rust generated code | `vexil-runtime` | Broad corpus, golden, compile, Clippy, and byte-vector coverage |
| TypeScript generated code | `@vexil-lang/runtime` | Native TypeScript build and broad byte-vector coverage |
| Go generated code | Versioned Go module | Native compile/execute tests over a representative shared wire matrix |
| Python generated code | Source install; 0.1.0 PyPI candidate | Pyright plus native execution over a representative shared wire matrix |

“Representative” is deliberate: Go and Python exercise important primitives,
collections, evolution, optionals, Result values, and unions, but not every
schema shape or deployment environment. Validate your own schemas in the
target runtimes you ship.

## Stable contract points

- Fields are packed LSB-first; multi-byte scalars are little-endian.
- Unsigned varints use LEB128 and signed varints use ZigZag plus LEB128.
- `result<T, E>` uses discriminant `0` for `Err` and `1` for `Ok`.
- Unknown `@non_exhaustive` union values retain both their discriminant and
  bounded payload bytes in every generated target.
- Canonical schema hashes use BLAKE3 and are independent of comments and
  formatting.

## Deliberate boundaries

- The wire is not self-describing. Both sides need the schema.
- Streaming decode, compression, schema discovery, and transport behavior are
  application or profile concerns.
- Message `invariant` syntax is reserved but rejected until portable semantics
  are specified and implemented.
- Cross-field constraints, regex constraints, RPC, encryption profiles, and a
  standard library are future decisions, not promises attached to a version.
- No independent implementation or external security audit has been completed.

For details and mitigations, read [Compatibility and Limitations](./compatibility.md)
and the repository's [limitations document](https://github.com/vexil-lang/vexil/blob/main/docs/limitations-and-gaps.md).
