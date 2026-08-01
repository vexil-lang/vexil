# Vexil Wire Format — Limitations, Gaps, and What We Haven't Done Yet

Last reviewed: 2026-08-01

## What We've Verified

- **Deterministic encoding:** Golden byte vectors match between Rust and TypeScript for all primitive types, sub-byte packing, messages, enums, unions, optionals, arrays, maps, sets, and evolution scenarios.
- **Schema evolution:** Formally specified (spec §9/§10) and enforced, not just informal convention — `vexilc compat` classifies every change as compatible or breaking against that table, ordinal reuse after `@removed` is a compile error, and this repo's CI runs `vexilc compat` on PRs that touch a versioned `.vexil` file. Field append and variant addition work (forward and backward); trailing bytes are tolerated by older decoders.
- **Recursion safety:** Depth limit of 64 enforced at encode and decode in both Rust and TypeScript. Stack overflow is prevented, not just unlikely.
- **NaN canonicalization:** All NaN inputs produce the same quiet NaN bytes (f32: `0x7FC00000`, f64: `0x7FF8000000000000`). We don't allow NaN payloads to vary.
- **Delta encoding:** `@delta` works in both Rust and TypeScript. The system-monitor example uses it over WebSocket with live data.
- **Zero-copy reads:** `BitReader` can return `&[u8]` and `&str` slices backed by the input buffer. No copies for large payloads.
- **Map key ordering:** Canonical sort order is defined for all valid key types. Encoders sort before writing.
- **Result and open-union contracts:** Result uses `0 = Err` and `1 = Ok`.
  Unknown non-exhaustive union values preserve their discriminant and bounded
  payload bytes across all four generated targets.

## Known Limitations

- **No streaming decode:** You need the entire message in memory before you can start decoding. If you're working with unbounded streams, you need a framing layer on top. The transport header in Appendix A of the spec is one option.
- **No built-in compression:** Wire format is uncompressed. Layer zstd or LZ4 on top if you need it. We decided not to bake compression into the format because different use cases want different compression.
- **No self-description:** The wire bytes contain no type info. Both sides need the schema. This is a design choice, not a gap — it keeps messages small. But it means you can't debug a packet without the schema file.
- **Generated Go coverage is representative:** Generated Go is verified against a shared representative wire matrix. This is not exhaustive for every schema or deployment environment.
- **Trait function bodies use a bounded portable subset:** Rust, TypeScript, Go, and Python generate instance methods for straight-line bodies with immutable locals, receiver-field mutation, returns, and unary/binary expressions. Free calls, method calls, external/native bodies, and non-receiver assignment remain explicit diagnostics before output; there is no runtime trait dispatch.

## What's Missing

Things that would be useful but aren't implemented:

- **Reflection / runtime type info:** Generated code doesn't emit metadata for schema introspection. If you need it, you'll have to build it on top.
- **Runtime type guards (TypeScript):** The generated TypeScript has interfaces but no runtime validation. You can't check "is this object a valid `SensorReading`?" without the schema.
- **Schema registry:** No built-in distribution mechanism. The BLAKE3 hash gives you identity but not discovery.
- **Generated Python coverage is representative:** Generated Python is verified against a shared representative wire matrix. This is not exhaustive for every schema or deployment environment.
- **Reserved invariants are rejected:** The parser retains precise syntax and
  spans, but compilation fails until portable encode/decode enforcement is
  specified. Cross-field and regex constraints likewise have no release-number
  promise.

## Performance

Wire size is competitive for sub-byte fields (bit-packing beats byte-aligned formats like Protobuf). For byte-aligned fields, it's roughly equivalent — no tags, no descriptors, just the data.

Encode/decode benchmarks are in `vexil-bench`. Run `cargo bench -p vexil-bench` to get numbers on your machine. We haven't published formal throughput comparisons with Protobuf because the numbers depend heavily on field mix and we'd rather you measure your own workload.
