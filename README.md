<h1 align="center">Vexil</h1>
<p align="center"><em>A typed schema definition language with first-class encoding semantics.</em></p>

<p align="center">
  <a href="https://github.com/vexil-lang/vexil/actions/workflows/ci.yml">
    <img src="https://github.com/vexil-lang/vexil/actions/workflows/ci.yml/badge.svg" alt="CI">
  </a>
  <a href="https://crates.io/crates/vexilc">
    <img src="https://img.shields.io/crates/v/vexilc" alt="vexilc on crates.io">
  </a>
  <a href="https://crates.io/crates/vexil-lang">
    <img src="https://img.shields.io/crates/v/vexil-lang?label=vexil-lang" alt="vexil-lang on crates.io">
  </a>
  <img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue" alt="License: MIT OR Apache-2.0">
  <img src="https://img.shields.io/badge/rust-1.94%2B-orange" alt="Rust 1.94+">
</p>

---

## What is Vexil?

Vexil (Validated Exchange Language) is a schema definition language (SDL) in the tradition of Protocol Buffers and Cap'n Proto, distinguished by two properties:

**Encoding semantics are part of the type system.** The type `u4` means exactly 4 bits on the wire — not "an integer that fits in 4 bits." The annotation `@varint` on a `u64` field changes the wire encoding to unsigned LEB128. The schema is the wire contract, not just the shape contract.

**The schema is the single source of truth.** Each schema has a deterministic BLAKE3 hash. That hash is embedded in generated code as a compile-time constant. A mismatch between the schema a sender compiled against and the schema a receiver compiled against is detectable at runtime, before any data corruption occurs.

## Quick Look

A sensor telemetry schema:

```vexil
namespace sensor.packet

enum SensorKind : u8 {
    Temperature @0
    Humidity    @1
    Pressure    @2
    Light       @3
}

message SensorReading {
    channel  @0 : u4              # 4 bits — values 0..15
    kind     @1 : SensorKind
    value    @2 : u16
    sequence @3 : u32 @varint     # variable-length encoding
    delta_ts @4 : i32 @zigzag    # signed, ZigZag-encoded
}
```

Generated Rust code encodes and decodes in a few lines:

```rust
use vexil_runtime::{BitWriter, BitReader, Pack, Unpack};

let reading = SensorReading {
    channel: 0, kind: SensorKind::Temperature,
    value: 2350, sequence: 1, delta_ts: -50,
};

let mut w = BitWriter::new();
reading.pack(&mut w).unwrap();
let bytes = w.finish();   // compact, bit-packed

let mut r = BitReader::new(&bytes);
let decoded = SensorReading::unpack(&mut r).unwrap();
assert_eq!(decoded.value, 2350);
```

The same schema generates TypeScript with identical wire output:

```typescript
import { BitWriter, BitReader } from '@vexil/runtime';

const w = new BitWriter();
encodeSensorReading({
  channel: 0, kind: 'Temperature',
  value: 2350, sequence: 1, delta_ts: -50,
}, w);
const bytes = w.finish();  // identical bytes as Rust

const r = new BitReader(bytes);
const decoded = decodeSensorReading(r);
// decoded.value === 2350
```

## Features

- **Sub-byte integer types** — `u1`..`u7` and `i1`..`i7`; each occupies exactly N bits on the wire with LSB-first bit packing
- **Encoding annotations** — `@varint` (unsigned LEB128), `@zigzag` (ZigZag + LEB128), `@delta` (delta from previous value) directly in the schema
- **Rich type vocabulary** — `message`, `enum`, `flags`, `union`, `newtype`, and `config` declarations
- **Schema versioning** — BLAKE3 hash of the canonical schema form; mismatch is detectable at the protocol boundary
- **Multi-language code generation** — Rust and TypeScript backends from the same schema, with byte-identical wire output verified by compliance vectors
- **Deterministic encoding** — same data always produces identical bytes, enabling content addressing and replay detection
- **Structured error model** — every invalid input produces a distinct error class with file, line, column, and a human-readable description
- **82-file conformance corpus** — 26 valid schemas and 56 invalid schemas; a conformant implementation must accept all valid and reject all invalid

## Comparison

| | Vexil | Protobuf | Cap'n Proto | FlatBuffers |
|---|:---:|:---:|:---:|:---:|
| Sub-byte types (`u1`..`u63`) | **Yes** | — | — | — |
| Encoding annotations in schema | **Yes** | — | — | — |
| Schema hash (mismatch detection) | **BLAKE3** | — | — | — |
| LSB-first bit packing | **Yes** | — | — | — |
| Self-describing wire format | No | Optional | No | Optional |
| Zero-copy decode | No | No | **Yes** | **Yes** |
| Deterministic encoding | **Yes** | No (maps) | No (padding) | No (vtables) |
| Schema evolution | **Yes** | **Yes** | **Yes** | **Yes** |
| Language targets | Rust, TypeScript | **Many** | **Many** | **Many** |

## Installation

### cargo install

```sh
cargo install vexilc
```

### Pre-built binaries

Pre-built binaries for Linux, Windows, and macOS are available on the
[Releases page](https://github.com/vexil-lang/vexil/releases).

### From source

Requires Rust 1.94 or later ([install via rustup](https://rustup.rs)).

```sh
git clone https://github.com/vexil-lang/vexil
cd vexil
cargo build --release --bin vexilc
# Binary is at target/release/vexilc
```

## Usage

### CLI

Check a schema for errors:

```sh
vexilc check schema.vexil
```

Generate code from a schema:

```sh
vexilc codegen schema.vexil --output out.rs                    # Rust (default)
vexilc codegen schema.vexil --output out.ts --target typescript # TypeScript
```

Compile a multi-file project:

```sh
vexilc build root.vexil --include ./schemas --output ./generated
vexilc build root.vexil --include ./schemas --output ./generated --target typescript
```

Errors are rendered with source spans and structured diagnostics:

```
Error: duplicate field name
   --> schema.vexil:8:5
    |
  8 |     value: u32,
    |     ^^^^^ field "value" was already declared on line 5
```

### Library

Add `vexil-lang` to your `Cargo.toml`:

```toml
[dependencies]
vexil-lang = "0.2"
```

Parse and compile a schema programmatically:

```rust
let result = vexil_lang::compile(source);
if result.diagnostics.iter().any(|d| d.severity == Severity::Error) {
    // handle errors
}
if let Some(compiled) = result.compiled {
    // use compiled schema
}
```

## Repository Structure

```
spec/
  vexil-spec.md          # Language specification (normative, §1-§14)
  vexil-grammar.peg      # Formal PEG grammar derived from spec
corpus/
  valid/                 # 26 conformant schemas — all must be accepted
  invalid/               # 56 invalid schemas — all must be rejected
  projects/              # Multi-file project fixtures for integration tests
compliance/
  vectors/               # Golden byte vectors (JSON) — cross-implementation contract
crates/
  vexil-lang/            # Compiler library: lexer, parser, IR, type checker, canonical hash
  vexil-codegen-rust/    # Rust code generation backend
  vexil-codegen-ts/      # TypeScript code generation backend
  vexil-runtime/         # Rust runtime: bit I/O, Pack/Unpack traits, LEB128, ZigZag
  vexilc/                # CLI frontend with ariadne error rendering
  vexil-store/           # .vx text format and .vxb binary format for schemas and data
  vexil-bench/           # Encode/decode benchmarks (Criterion)
packages/
  runtime-ts/            # @vexil/runtime npm package: TypeScript BitWriter/BitReader
examples/
  cross-language/        # Rust <-> Node.js interop demo
  system-monitor/        # Real-time dashboard: Rust -> browser via Vexil WebSocket
```

## Documentation

- [Language Specification](spec/vexil-spec.md)
- [FAQ](FAQ.md)
- [Examples](examples/)
- [Limitations and Gaps](docs/limitations-and-gaps.md)
- API reference: [vexil-lang](https://docs.rs/vexil-lang) · [vexil-runtime](https://docs.rs/vexil-runtime) · [vexil-codegen-rust](https://docs.rs/vexil-codegen-rust) · [vexil-codegen-ts](https://docs.rs/vexil-codegen-ts) · [vexil-store](https://docs.rs/vexil-store)

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](./CONTRIBUTING.md) before opening a pull request.
For architectural decisions, language changes, and protocol modifications, see [GOVERNANCE.md](./GOVERNANCE.md).

## License

Licensed under either of

- [MIT License](./LICENSE-MIT)
- [Apache License, Version 2.0](./LICENSE-APACHE)

at your option.
