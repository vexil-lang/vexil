# Frequently Asked Questions

## How is Vexil different from Protocol Buffers / Cap'n Proto / FlatBuffers?

Most schema languages describe the *shape* of data — "this field is a 32-bit integer." Vexil describes the *encoding* too — "this field is 4 bits, LSB-first, packed with its neighbors." The type `u4` means exactly 4 bits on the wire. The annotation `@varint` on a `u64` changes the encoding to unsigned LEB128. This makes Vexil particularly suited for bandwidth-constrained or bit-packed protocols where you'd otherwise hand-roll the encoding.

Other differences:
- **Sub-byte types** — `u1`..`u63` and `i2`..`i63` occupy exactly N bits
- **Schema hashing** — BLAKE3 hash of the canonical form detects sender/receiver mismatch before data corruption
- **No self-description on the wire** — the schema is the contract, keeping messages compact
- **Delta encoding** — `@delta` annotation generates stateful encoder/decoder pairs that transmit field-level deltas, reducing wire size for streaming use cases

Trade-offs: Vexil does not yet support formal schema evolution rules, and does not offer zero-copy decoding. Language targets are currently Rust and TypeScript; more backends are planned.

## Is Vexil production-ready?

Vexil is pre-1.0. The language spec is draft, and the wire format may change before v1.0. It has an 83-file conformance corpus (27 valid, 56 invalid), 436+ tests across the workspace, and cross-implementation compliance vectors verified between Rust and TypeScript. The BLAKE3 schema hash provides a safety net against incompatible changes.

If your system cannot tolerate wire format changes between versions, wait for v1.0.

## What languages are supported?

Rust and TypeScript code generation both ship today. Both backends produce byte-identical wire output, verified by the compliance vector suite. The [`CodegenBackend`](https://docs.rs/vexil-lang/latest/vexil_lang/codegen/trait.CodegenBackend.html) trait makes adding new language backends straightforward — contributions are welcome.

## Why not just use `#[repr(packed)]` or C bitfields?

Hand-rolled bit packing works for a single language on a single platform. Vexil gives you:
- A language-agnostic schema that can generate code for multiple targets
- Deterministic, cross-platform encoding (LSB-first, defined endianness)
- Schema hashing for version mismatch detection
- Structured validation and error reporting
- A conformance corpus that any implementation can test against

## Does Vexil support schema evolution?

Partially. The wire format tolerates trailing bytes, so decoders compiled against an older schema can read messages with appended fields. The BLAKE3 schema hash detects when a sender and receiver are using different schema versions. However, there are no formal evolution rules (field deprecation, variant removal) that guarantee wire compatibility across versions. This is planned for a future release.

The `@removed` annotation documents which ordinals were previously in use, preventing accidental reuse.

## Can I use Vexil for network protocols? File formats? IPC?

Yes to all. The wire encoding is deterministic, compact, and does not include metadata — it's suitable for any context where you control both ends and want minimal overhead. The [`vexil-store`](https://crates.io/crates/vexil-store) crate adds a binary file format (`.vxb`) for persisting schema-typed data.

## What is `@delta` encoding?

The `@delta` annotation on a message causes the code generator to emit stateful encoder/decoder pairs. Numeric fields are transmitted as deltas from the previous frame rather than absolute values. Non-numeric fields (strings, arrays, enums) are sent in full each frame. This reduces wire size significantly for streaming scenarios where values change incrementally — for example, a system monitoring dashboard sending CPU and memory readings every second.
