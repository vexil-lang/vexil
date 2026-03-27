# Vexil Wire Format — Limitations, Gaps, and Room for Improvement

A living document tracking what has been validated, what is known to be
limited, and where improvements would have the most impact.

Last updated: 2026-03-28

## What Was Validated

- **Deterministic encoding:** Golden byte vectors produce identical bytes in
  both Rust and TypeScript implementations for all primitive types, sub-byte
  packing, messages, enums, unions, optionals, arrays, maps, and evolution
  scenarios.
- **Schema evolution:** Forward and backward compatibility verified for field
  append and variant addition. Trailing bytes tolerated by decoders.
- **Recursion safety:** Depth limit of 64 enforced at both encode and decode
  time in both Rust and TypeScript runtimes. Stack overflow prevented.
- **NaN canonicalization:** All NaN inputs produce canonical quiet NaN bytes
  (f32: 0x7FC00000, f64: 0x7FF8000000000000).
- **Cross-implementation compliance:** Rust and TypeScript implementations
  pass the same golden byte vector suite.
- **TypeScript backend:** Full code generation for all 6 declaration kinds
  (message, enum, flags, union, newtype, config) with cross-file imports.
- **Delta encoding:** `@delta` annotation generates stateful encoder/decoder
  pairs in both Rust and TypeScript. Numeric fields transmit deltas from the
  previous frame; non-numeric fields are sent in full. Verified in the
  system-monitor example with live WebSocket streaming.

## Known Limitations

- **No zero-copy decode:** BitReader copies data for strings and byte arrays.
  Applications needing zero-copy access to large payloads should consider
  a bytes-reference mode (future work).
- **No streaming / incremental decode:** The entire message must be available
  in memory before decoding starts. Not suitable for unbounded streams
  without framing.
- **No built-in compression:** Wire format is uncompressed. Applications can
  layer compression (zstd, etc.) on top.
- **No self-description:** The wire format contains no type information.
  Both sides must agree on the schema. This is by design (schema = contract)
  but means debug tooling needs the schema to interpret wire bytes.
- **Map key ordering:** Map entries are encoded in iteration order. For
  deterministic encoding, implementations must sort map keys before encoding.
  The spec does not mandate a sort order — this is left to the application.

## Gaps

- **Reflection metadata:** No runtime type information emitted by codegen.
  Consumers needing schema introspection at runtime would need a separate
  metadata format.
- **Runtime validation / type guards:** Generated TypeScript code does not
  emit type guards or runtime validators. Can be layered on top of existing
  interfaces without codegen changes.
- **Schema registry integration:** No built-in registry or discovery. Schema
  hash provides identity but not distribution.
- **Additional backend targets:** Only Rust and TypeScript backends exist.
  Python, Go, and C backends are potential future work.

## Performance Characteristics

- **Wire size:** Competitive for sub-byte fields (bit-packing is more compact
  than byte-aligned formats). Equivalent for byte-aligned fields. No overhead
  for field tags or type descriptors.
- **Encode/decode throughput:** Benchmarks available via `cargo bench -p vexil-bench`.
- **Comparison with protobuf:** Vexil's deterministic encoding enables content
  addressing (BLAKE3 hash) which protobuf cannot guarantee. Throughput
  comparison pending formal benchmarks.

## Room for Improvement

Prioritized by likely consumer demand:

1. **Zero-copy byte slices** — Return `&[u8]` / `Uint8Array` views instead
   of copies for large payloads. Requires lifetime tracking in BitReader.
2. **Streaming decode** — Allow progressive decode with a framing layer.
   The standard transport header (Appendix A of the spec) provides a
   starting point.
3. **Wire size optimization** — Consider optional field presence bitsets
   for messages with many optional fields (reduces per-field overhead from
   1 bit per optional to amortized cost).
4. **Additional backend targets** — Python, Go, C.
5. **Compression integration** — First-class zstd or LZ4 support at the
   transport layer.
6. **Map key ordering** — Define a canonical sort order for map keys to
   ensure deterministic encoding without application-level coordination.
