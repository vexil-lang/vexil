# Frequently Asked Questions

## How is Vexil different from Protocol Buffers / Cap'n Proto / FlatBuffers?

Most schema languages describe the *shape* of data ("this field is a 32-bit integer"). Vexil describes the *encoding* too ("this field is 4 bits, LSB-first, packed with its neighbors"). `u4` means exactly 4 bits on the wire. `@varint` on a `u64` switches it to unsigned LEB128. If you've ever hand-rolled a bit-packed protocol because Protobuf couldn't express sub-byte fields, that's the problem Vexil solves.

Other differences:
- **Sub-byte types** -- `u1`..`u63` and `i2`..`i63` occupy exactly N bits
- **Schema hashing** -- BLAKE3 hash of the canonical form catches sender/receiver mismatch before data corruption
- **No self-description on the wire** -- the schema is the contract, messages are compact
- **Delta encoding** -- `@delta` generates stateful encoder/decoder pairs for streaming use cases

The trade-off: Vexil doesn't have formal schema evolution rules yet. Language targets are Rust, TypeScript, Go, and Python, not the "15 languages" that Protobuf supports. If you need Java or C# today, Vexil isn't ready for you.

## Is Vexil production-ready?

We're at v1.0-draft. The wire format is frozen and the language spec won't change without a major version bump. We have a 108-file conformance corpus (43 valid, 65 invalid), 560+ tests, and cross-implementation compliance vectors between Rust and TypeScript. The BLAKE3 schema hash means incompatible schema versions are caught at handshake time, not at runtime corruption.

That said: Go and Python backends don't have compliance vectors yet, and we haven't done a security audit. If your system can't tolerate any wire format risk, wait for v1.0 final. If you're building something where "it works and we have tests" is enough, it's ready.

## What languages are supported?

Rust and TypeScript have full code generation with compliance-verified byte output. Go and Python work but haven't been added to the compliance vector suite. I'd trust them for internal use but would verify byte output manually before shipping a cross-language protocol.

The [`CodegenBackend`](https://docs.rs/vexil-lang/latest/vexil_lang/codegen/trait.CodegenBackend.html) trait is public. If you want to add a backend, it's a weekend project. Implement `generate()` and `generate_project()`, and the compiler handles the rest.

## Why not just use `#[repr(packed)]` or C bitfields?

Hand-rolled bit packing works when you control one language on one platform. It falls apart the moment you need:
- A TypeScript client reading the same bytes as a Rust server
- A wire format that's identical on ARM and x86 (bitfield layout differs)
- A schema hash to detect version mismatch before data corruption
- Structured error reporting when a 4-bit field gets a value > 15

Vexil gives you all of that from a single schema file.

## Does Vexil support schema evolution?

Partially. The wire format tolerates trailing bytes, so an older decoder can read messages with fields appended by a newer schema. The BLAKE3 schema hash detects when sender and receiver are on different versions.

What we don't have yet: formal field deprecation rules, variant removal guarantees, or wire-compatible schema diffing. The `@removed` annotation documents which ordinals were previously in use to prevent accidental reuse. Full evolution support is planned for 1.1.

## Can I use Vexil for network protocols? File formats? IPC?

Yes to all three. The wire encoding is deterministic and compact with no metadata. It works anywhere you control both ends and want minimal overhead. The `vexil-store` crate adds `.vxb` binary files for persisting schema-typed data. We use it for WebSocket streaming in the system-monitor example and for content-addressed storage in the Orix project.

## What is `@delta` encoding?

`@delta` on a message generates stateful encoder/decoder pairs. Numeric fields transmit as deltas from the previous frame. Non-numeric fields (strings, arrays, enums) go full-size each frame.

In the system-monitor example, a full `SystemSnapshot` is ~42 bytes. Steady-state delta frames drop to ~25-30 bytes because most deltas are small or zero. It's not compression. It's just not re-transmitting things that didn't change.

## What's the deal with fixed-point types?

`fixed32` is Q16.16. That's 16 bits integer, 16 bits fraction, 32 bits total. `fixed64` is Q32.32. The point is deterministic arithmetic: the same operation gives the same result on every CPU and compiler. IEEE 754 floats don't do that. Rounding modes, denormal handling, and FPU quirks can produce different results on ARM vs x86.

If you're building a simulation where every node needs to compute identical results, or a content-addressed system where the same input must produce the same hash, fixed-point is what you want. If you're rendering graphics and don't care about determinism, use `f32` instead.
