# Frequently Asked Questions

## How is Vexil different from Protocol Buffers / Cap'n Proto / FlatBuffers?

Most schema languages describe the *shape* of data ("this field is a 32-bit integer"). Vexil describes the *encoding* too ("this field is 4 bits, LSB-first, packed with its neighbors"). `u4` means exactly 4 bits on the wire. `@varint` on a `u64` switches it to unsigned LEB128. If you've ever hand-rolled a bit-packed protocol because Protobuf couldn't express sub-byte fields, that's the problem Vexil solves.

Other differences:
- **Sub-byte types** -- `u1`..`u63` and `i2`..`i63` occupy exactly N bits
- **Schema hashing** -- BLAKE3 hash of the canonical form catches sender/receiver mismatch before data corruption
- **No self-description on the wire** -- the schema is the contract, messages are compact
- **Delta encoding** -- `@delta` generates stateful encoder/decoder pairs for streaming use cases

The trade-off: language targets are Rust, TypeScript, Go, and Python, not the "15 languages" that Protobuf supports. If you need Java or C# today, Vexil isn't ready for you.

## Is Vexil production-ready?

The binary wire format hasn't changed since April 2026 and breaking it would require a major version bump, while the language specification remains a draft. The repository includes a 126-file conformance corpus (50 valid and 76 invalid), broad Rust and TypeScript byte-vector coverage, and a representative shared generated-wire matrix for Go and Python. BLAKE3 schema hashes help peers detect incompatible schemas before exchanging application data. Neither the wire format nor the corpus have been exercised by external implementations or independently audited yet.

This is not a claim of a final stable release or a substitute for application-level compatibility testing. The Go and Python matrix is not exhaustive for every schema or environment, and the project has not published a security audit. Review the [limitations](docs/limitations-and-gaps.md) and verify your target combination before shipping a cross-language protocol.

## What languages are supported?

Rust and TypeScript have broad cross-language byte-vector coverage. Generated Go and Python are verified against a representative shared wire matrix, not every schema or environment; verify your application-specific schema and target combination before shipping a protocol.

The [`CodegenBackend`](https://docs.rs/vexil-lang/latest/vexil_lang/codegen/trait.CodegenBackend.html) trait is public. If you want to add a backend, it's a weekend project. Implement `generate()` and `generate_project()`, and the compiler handles the rest.

## Why not just use `#[repr(packed)]` or C bitfields?

Hand-rolled bit packing works when you control one language on one platform. It falls apart the moment you need:
- A TypeScript client reading the same bytes as a Rust server
- A wire format that's identical on ARM and x86 (bitfield layout differs)
- A schema hash to detect version mismatch before data corruption
- Structured error reporting when a 4-bit field gets a value > 15

Vexil gives you all of that from a single schema file.

## Does Vexil support schema evolution?

Yes, formally. [Spec §9](spec/language.md) defines schema versioning (`@version`, SemVer 2.0.0) and [§10](spec/language.md) is a normative table classifying every kind of schema change as compatible (patch/minor) or breaking (major) — adding a field, removing one, changing a type or ordinal, adding a variant to a `@non_exhaustive` enum, and so on. §11.10 walks through the actual encode/decode mechanics for each case.

`vexilc compat old.vexil new.vexil` implements that table: it diffs two compiled schemas and reports every change, its classification, and the suggested version bump (exit code 0 if compatible, 1 if breaking). Reusing an ordinal after `@removed` is a compile-time error, not just a convention. This repository's own CI runs `vexilc compat` against every PR that touches a versioned `.vexil` file and fails the build if a breaking change isn't paired with at least the required `@version` bump.

The wire format itself also tolerates trailing bytes, so an older decoder can read messages with fields appended by a newer schema without any of the above — that's forward compatibility for free, on top of the formal rules.

## Can I use Vexil for network protocols? File formats? IPC?

Yes to all three. The wire encoding is deterministic and compact with no metadata. It works anywhere you control both ends and want minimal overhead. The `vexil-store` crate adds `.vxb` binary files for persisting schema-typed data. We use it for WebSocket streaming in the system-monitor example and for content-addressed storage in the Orix project.

## What is `@delta` encoding?

`@delta` on a message generates stateful encoder/decoder pairs. Numeric fields transmit as deltas from the previous frame. Non-numeric fields (strings, arrays, enums) go full-size each frame.

In the system-monitor example, a full `SystemSnapshot` is ~42 bytes. Steady-state delta frames drop to ~25-30 bytes because most deltas are small or zero. It's not compression. It's just not re-transmitting things that didn't change.

## What's the deal with fixed-point types?

`fixed32` is Q16.16. That's 16 bits integer, 16 bits fraction, 32 bits total. `fixed64` is Q32.32. The point is deterministic arithmetic: the same operation gives the same result on every CPU and compiler. IEEE 754 floats don't do that. Rounding modes, denormal handling, and FPU quirks can produce different results on ARM vs x86.

If you're building a simulation where every node needs to compute identical results, or a content-addressed system where the same input must produce the same hash, fixed-point is what you want. If you're rendering graphics and don't care about determinism, use `f32` instead.
