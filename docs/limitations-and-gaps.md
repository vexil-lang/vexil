# Compatibility and Current Limits

Last reviewed: 2026-08-04

This page separates verified behavior from adoption boundaries and future work.
It is intentionally conservative: repository tests are evidence for the tested
surfaces, not a guarantee for every application or environment.

## Verified in this repository

- The compiler accepts and rejects the schemas listed in the conformance corpus.
- Rust and TypeScript exercise broad golden and byte-vector coverage.
- Generated Go and Python compile and execute a representative shared wire
  matrix, including core scalars, collections, optionals, results, and open
  unions.
- Fields are packed LSB-first, multi-byte scalars are little-endian, unsigned
  varints use LEB128, and signed varints use ZigZag followed by LEB128.
- Canonical schema hashes use BLAKE3 and ignore comments and formatting.
- Maps and sets use defined canonical ordering before encoding.
- Recursion and collection limits are enforced by the maintained runtimes.
- `vexilc compat` applies the schema-evolution classifications defined by the
  language specification.
- Unknown non-exhaustive union variants preserve their discriminant and bounded
  payload in all four generated targets covered by the shared matrix.

The [support matrix](book/src/getting-started/support-matrix.md) describes the
evidence for each target in more detail.

## Adoption boundaries

### Verification depth differs by target

Rust and TypeScript have the broadest generated-code and byte-vector coverage.
Go and Python coverage is representative rather than exhaustive. Test the exact
schemas, target versions, and deployment platforms your protocol uses.

### The wire is not self-describing

Peers need the schema out of band. The generated BLAKE3 hash can identify a
contract, but it does not discover, authenticate, or distribute schemas.

### Message field additions are breaking

Message values do not carry an internal length boundary. In nested messages and
inline aggregates, a decoder cannot distinguish an appended field from the
parent's next field or the next aggregate element. `vexilc compat` therefore
classifies every message field addition as major. A top-level reader that can
stop early is not evidence of general schema compatibility.

### Transport remains application-owned

Vexil does not define framing for unbounded streams, retries, authentication,
encryption, discovery, congestion behavior, or service semantics. Scope each
decoder to a bounded payload and define those operational rules separately.

### Compression is layered

The wire format does not include built-in compression. Compress complete,
bounded Vexil payloads or frames when your threat model and workload justify it.
Do not confuse stateful `@delta` encoding with general compression.

### External evidence is limited

All maintained implementations currently live in this repository. There is no
independent implementation, external interoperability report, or completed
third-party security audit.

## Deliberately unavailable today

- incremental decode from a partial byte stream;
- generated reflection or runtime schema metadata;
- generated TypeScript runtime validators for arbitrary application objects;
- built-in schema discovery or registry services;
- portable enforcement for reserved message invariants;
- cross-field, regex, or user-defined constraint functions;
- language-defined RPC, transport, encryption, or standard-library profiles;
- additional maintained target languages beyond Rust, TypeScript, Go, and
  Python.

These are backlog concerns, not promises for a named future release.

## Performance claims

Vexil can be compact when schemas use sub-byte fields or omit metadata that
other formats carry. Actual size and throughput depend on the schema, value
distribution, runtime, and workload.

The repository contains Criterion benchmarks in `crates/vexil-bench`. Run them
against your workload. The project does not publish a universal performance
ranking against other serialization systems.
