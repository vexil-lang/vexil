# Frequently Asked Questions

## How is Vexil different from Protocol Buffers / Cap'n Proto / FlatBuffers?

Most schema languages describe the *shape* of data — "this field is a 32-bit integer." Vexil describes the *encoding* too — "this field is 4 bits, LSB-first, packed with its neighbors." The type `u4` means exactly 4 bits on the wire. The annotation `@varint` on a `u64` changes the encoding to unsigned LEB128. This makes Vexil particularly suited for bandwidth-constrained or bit-packed protocols where you'd otherwise hand-roll the encoding.

Other differences:
- **Sub-byte types** — `u1`..`u63` and `i2`..`i63` occupy exactly N bits
- **Schema hashing** — BLAKE3 hash of the canonical form detects sender/receiver mismatch before data corruption
- **No self-description on the wire** — the schema is the contract, keeping messages compact

Trade-offs: Vexil currently supports Rust only (TypeScript is planned), does not yet support schema evolution, and does not offer zero-copy decoding.

## Is Vexil production-ready?

Vexil is at v0.2. The language spec is draft, and the wire format may change before v1.0. It has a 74-file conformance corpus and 258+ tests, and the BLAKE3 schema hash provides a safety net against incompatible changes. That said, if your system cannot tolerate wire format changes between versions, wait for v1.0.

## What languages are supported?

Rust code generation ships today. A TypeScript backend is [designed and specced](docs/superpowers/specs/2026-03-26-typescript-backend-design.md). The [`CodegenBackend`](https://docs.rs/vexil-lang/latest/vexil_lang/codegen/trait.CodegenBackend.html) trait makes adding new language backends straightforward — contributions are welcome.

## Why not just use `#[repr(packed)]` or C bitfields?

Hand-rolled bit packing works for a single language on a single platform. Vexil gives you:
- A language-agnostic schema that can generate code for multiple targets
- Deterministic, cross-platform encoding (LSB-first, defined endianness)
- Schema hashing for version mismatch detection
- Structured validation and error reporting
- A conformance corpus that any implementation can test against

## Does Vexil support schema evolution?

Not yet. The BLAKE3 schema hash detects when a sender and receiver are using different schema versions, but there are no evolution rules (adding fields, deprecating variants) that preserve wire compatibility. This is planned for a future version.

For now, the `@removed` annotation documents which ordinals were previously in use, so they won't be accidentally reused.

## Can I use Vexil for network protocols? File formats? IPC?

Yes to all. The wire encoding is deterministic, compact, and does not include metadata — it's suitable for any context where you control both ends and want minimal overhead. The [`vexil-store`](https://crates.io/crates/vexil-store) crate adds a binary file format (`.vxb`) for persisting schema-typed data.
