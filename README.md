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

Vexil describes both the shape *and* the wire encoding of data crossing system boundaries. The type `u4` occupies exactly 4 bits. The annotation `@varint` switches a field to LEB128. The schema is the wire contract, not just the shape contract.

Each schema produces a deterministic BLAKE3 hash, embedded in generated code at compile time. If a sender and receiver compile against different schemas, the mismatch is detectable before any data is read.

## Quick look

```vexil
namespace sensor.packet

enum SensorKind : u8 {
    Temperature @0
    Humidity    @1
    Pressure    @2
    Light       @3
}

message SensorReading {
    channel  @0 : u4              # 4 bits, values 0..15
    kind     @1 : SensorKind
    value    @2 : u16
    sequence @3 : u32 @varint     # variable-length encoding
    delta_ts @4 : i32 @zigzag    # signed, ZigZag-encoded
}
```

Generated Rust:

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

The same schema generates TypeScript that produces identical bytes:

```typescript
import { BitWriter, BitReader } from '@vexil-lang/runtime';

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

## What Vexil does

- `u1`..`u63` and `i2`..`i63` occupy exactly N bits on the wire, LSB-first
- `@varint` (unsigned LEB128), `@zigzag` (ZigZag + LEB128), and `@delta` (per-field delta from previous value) are declared in the schema
- Six declaration kinds: `message`, `enum`, `flags`, `union`, `newtype`, `config`
- BLAKE3 hash of the canonical schema form, embedded as a compile-time constant in generated code
- Rust, TypeScript, and Go backends from the same schema, byte-identical output verified by compliance vectors
- Same data always produces the same bytes, enabling content addressing and replay detection
- Every invalid input yields a distinct error with file, line, column, and a human-readable description
- 83-file conformance corpus (27 valid, 56 invalid) that any implementation must pass

## Comparison

| | Vexil | Protobuf | Cap'n Proto | FlatBuffers |
|---|:---:|:---:|:---:|:---:|
| Sub-byte types (`u1`..`u63`) | **Yes** | -- | -- | -- |
| Encoding annotations in schema | **Yes** | -- | -- | -- |
| Schema hash (mismatch detection) | **BLAKE3** | -- | -- | -- |
| LSB-first bit packing | **Yes** | -- | -- | -- |
| Self-describing wire format | No | Optional | No | Optional |
| Zero-copy decode | No | No | **Yes** | **Yes** |
| Deterministic encoding | **Yes** | No (maps) | No (padding) | No (vtables) |
| Schema evolution | **Yes** | **Yes** | **Yes** | **Yes** |
| Language targets | Rust, TS, Go | **Many** | **Many** | **Many** |

## Install

```sh
cargo install vexilc
```

Pre-built binaries for Linux, Windows, and macOS are on the [Releases page](https://github.com/vexil-lang/vexil/releases).

To build from source (requires Rust 1.94+):

```sh
git clone https://github.com/vexil-lang/vexil
cd vexil
cargo build --release --bin vexilc
```

## Usage

### CLI

```sh
# Check a schema for errors (prints BLAKE3 hash on success)
vexilc check schema.vexil

# Generate code
vexilc codegen schema.vexil --output out.rs                    # Rust (default)
vexilc codegen schema.vexil --output out.ts --target typescript # TypeScript
vexilc codegen schema.vexil --output out.go --target go         # Go

# Compile a multi-file project
vexilc build root.vexil --include ./schemas --output ./generated
vexilc build root.vexil --include ./schemas --output ./generated --target typescript

# Auto-rebuild on schema changes
vexilc watch root.vexil --include ./schemas --output ./generated

# Scaffold a new schema file
vexilc init my_schema.vexil --namespace my.namespace

# Print BLAKE3 schema hash
vexilc hash schema.vexil

# Check schema compatibility (breaking change detection)
vexilc compat old.vexil new.vexil

# Schema-driven data tools
vexilc pack  data.vx  --schema s.vexil --type T -o data.vxb  # text -> binary
vexilc unpack data.vxb --schema s.vexil --type T              # binary -> text

# Version and help
vexilc --version
vexilc --help
```

Errors render with source spans:

```
Error: duplicate field name
   --> schema.vexil:8:5
    |
  8 |     value: u32,
    |     ^^^^^ field "value" was already declared on line 5
```

### Library

```toml
[dependencies]
vexil-lang = "0.4"
```

```rust
use vexil_lang::{compile, Severity};

let result = compile(source);
if result.diagnostics.iter().any(|d| d.severity == Severity::Error) {
    // handle errors
}
if let Some(compiled) = result.compiled {
    let hash = vexil_lang::canonical::schema_hash(&compiled);
    // pass compiled to a CodegenBackend
}
```

## Repository layout

```
spec/
  vexil-spec.md          # Language specification (normative, S1-S14)
  vexil-grammar.peg      # Formal PEG grammar
corpus/
  valid/                 # 27 schemas -- conformant impl must accept all
  invalid/               # 56 schemas -- conformant impl must reject all
  projects/              # Multi-file integration tests
compliance/
  vectors/               # Golden byte vectors (JSON), cross-implementation contract
crates/
  vexil-lang/            # Compiler: lexer, parser, IR, type checker, canonical hash
  vexil-codegen-rust/    # Rust code generation
  vexil-codegen-ts/      # TypeScript code generation
  vexil-codegen-go/      # Go code generation
  vexil-runtime/         # Rust runtime: BitWriter/BitReader, Pack/Unpack, LEB128, ZigZag
  vexilc/                # CLI with ariadne error rendering
  vexil-store/           # .vx text and .vxb binary file formats
  vexil-bench/           # Encode/decode benchmarks (Criterion)
packages/
  runtime-ts/            # @vexil-lang/runtime -- TypeScript BitWriter/BitReader (npm)
  runtime-go/            # github.com/vexil-lang/vexil/packages/runtime-go -- Go runtime
examples/
  sensor-packet/         # Sub-byte types, encoding annotations, compact enums
  command-protocol/      # Unions, flags, limits -- RPC-style protocol
  multi-file-project/    # Cross-file imports and project compilation
  cross-language/        # Rust <-> Node.js interop via binary files
  system-monitor/        # Live dashboard: Rust -> browser via @delta WebSocket
```

## Documentation

- [Language Specification](spec/vexil-spec.md)
- [FAQ](FAQ.md)
- [Examples](examples/)
- [Limitations and Gaps](docs/limitations-and-gaps.md)
- API reference: [vexil-lang](https://docs.rs/vexil-lang) | [vexil-runtime](https://docs.rs/vexil-runtime) | [vexil-codegen-rust](https://docs.rs/vexil-codegen-rust) | [vexil-codegen-ts](https://docs.rs/vexil-codegen-ts) | [vexil-codegen-go](https://docs.rs/vexil-codegen-go) | [vexil-store](https://docs.rs/vexil-store)

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md). Language changes and protocol modifications go through the RFC process in [GOVERNANCE.md](./GOVERNANCE.md).

## License

Licensed under either of [MIT](./LICENSE-MIT) or [Apache-2.0](./LICENSE-APACHE) at your option.
