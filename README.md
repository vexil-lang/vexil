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

- `u1`..`u64` and `i2`..`i64` occupy exactly N bits on the wire, LSB-first
- `@varint` (unsigned LEB128), `@zigzag` (ZigZag + LEB128), and `@delta` (per-field delta from previous value) are declared in the schema
- **Ten declaration kinds**: `message`, `enum`, `flags`, `union`, `newtype`, `config`, `type` (alias), `const`, `trait`, `impl`
- **Fixed-point types**: `fixed32` (Q16.16), `fixed64` (Q32.32) — deterministic fractional arithmetic, no IEEE 754 surprises
- **Geometric types**: `vec2<T>`, `vec3<T>`, `vec4<T>`, `quat<T>`, `mat3<T>`, `mat4<T>` — T can be fixed32, fixed64, f32, or f64
- **Fixed-size arrays**: `array<T, N>` — no length prefix on wire, size is part of the schema
- **Set type**: `set<T>` — sorted on encode, duplicates silently dropped
- **Inline bitfields**: `bits { a, b, c }` — anonymous flags, exactly N bits
- **Type aliases**: `type UserId = u64` — same wire encoding, better names
- **Compile-time constants**: `const MaxSize : u32 = 1024` — usable in array sizes and where clauses
- **Where clauses**: `field @0 : u32 where value > 0` — validated on encode and decode
- **Traits and impl**: `trait SensorData { sensor_id @0 : u32 }` — structural contracts, zero wire impact
- **Invariants**: `invariant { value >= 0 }` — cross-field conditions in messages
- **Type param bounds**: `type Sorted<T: Ord> = array<T>` — constrain generic types
- BLAKE3 hash of the canonical schema form, embedded as a compile-time constant in generated code
- Rust, TypeScript, Go, and Python backends from the same schema, byte-identical output verified by compliance vectors
- Same data always produces the same bytes — no maps with random iteration order, no padding variance
- Every invalid input yields a distinct error with file, line, column, and a description
- 108-file conformance corpus (43 valid, 65 invalid) that any implementation must pass

## Fixed-Point Types

`fixed32` is Q16.16 (32 bits, ~0.000015 precision). `fixed64` is Q32.32 (64 bits, ~9 decimal digits). Unlike IEEE 754 floats, the same operation produces the same result on every platform. We use this in the Orix ecosystem for deterministic simulation — every tick computes identically regardless of CPU or compiler.

```vexil
message Position {
    latitude  @0 : fixed32
    longitude @1 : fixed32
    altitude  @2 : fixed64
}
```

## Geometric Types

These are built-in parameterized types for graphics and simulation code. Wire encoding: components in order (x, y, z, w), no padding, no count prefix. You can mix deterministic (fixed-point) and standard (float) in the same message:

```vexil
message Transform {
    position    @0 : vec3<fixed64>   # deterministic simulation
    gl_pos      @1 : vec3<f32>       # GPU-ready
    rotation    @2 : quat<fixed64>
    model       @3 : mat4<f32>       # column-major 4x4
}
```

## Fixed-Size Arrays and Sets

`array<T, N>` has no count prefix — just N elements on the wire. `set<T>` is sorted on encode so the wire is deterministic regardless of insertion order:

```vexil
const Vertices = 256

message Mesh {
    positions @0 : array<vec3<f32>, Vertices>
    indices   @1 : array<u16, 512>
    tags      @2 : set<string>      # deduplicated, sorted
}
```

## Inline Bitfields

```vexil
message FileHeader {
    version @0 : u8
    perms   @1 : bits { r, w, x, hidden, system }
}
```

Five flags, five bits on the wire. That's it.

## Type Aliases and Constants

Aliases are transparent — `type UserId = u64` means `UserId` and `u64` produce identical bytes. Constants can reference each other with simple arithmetic:

```vexil
type UserId = u64
const TicksPerSec : u32 = 64
const TickMs : u32 = 1000 / TicksPerSec   # evaluates to 15

message Frame {
    sender @0 : UserId
    ts     @1 : u32
}
```

## Where Clauses

Constraints are checked on encode and decode. Invalid data never hits the wire:

```vexil
message UserProfile {
    age      @0 : u8  where value in 0..150
    score    @1 : i32 where value >= 0 && value <= 100
    username @2 : string where len(value) in 3..32
}
```

Cross-field constraints (`where amount <= balance`) and regex matching are deferred to 1.1.

## Comparison

| | Vexil | Protobuf | Cap'n Proto | FlatBuffers |
|---|:---:|:---:|:---:|:---:|
| Sub-byte types (`u1`..`u63`) | **Yes** | -- | -- | -- |
| Encoding annotations in schema | **Yes** | -- | -- | -- |
| Schema hash (mismatch detection) | **BLAKE3** | -- | -- | -- |
| LSB-first bit packing | **Yes** | -- | -- | -- |
| Self-describing wire format | No | Optional | No | Optional |
| Zero-copy decode | **Yes** | No | **Yes** | **Yes** |
| Deterministic encoding | **Yes** | No (maps) | No (padding) | No (vtables) |
| Schema evolution | **Yes** | **Yes** | **Yes** | **Yes** |
| Language targets | Rust, TS, Go, Python | **Many** | **Many** | **Many** |

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
vexilc codegen schema.vexil --output out.py --target python      # Python

# Compile a multi-file project
vexilc build root.vexil --include ./schemas --output ./generated

# Auto-rebuild on schema changes
vexilc watch root.vexil --include ./schemas --output ./generated

# Print BLAKE3 schema hash
vexilc hash schema.vexil

# Check schema compatibility (breaking change detection)
vexilc compat old.vexil new.vexil

# Schema-driven data tools
vexilc pack  data.vx  --schema s.vexil --type T -o data.vxb  # text -> binary
vexilc unpack data.vxb --schema s.vexil --type T              # binary -> text
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
vexil-lang = "1.0"
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
  valid/                 # 43 schemas -- conformant impl must accept all
  invalid/               # 65 schemas -- conformant impl must reject all
  projects/              # Multi-file integration tests
compliance/
  vectors/               # Golden byte vectors, cross-implementation contract
crates/
  vexil-lang/            # Compiler: lexer, parser, IR, type checker, canonical hash
  vexil-codegen-rust/    # Rust code generation
  vexil-codegen-ts/      # TypeScript code generation
  vexil-codegen-go/      # Go code generation
  vexil-codegen-py/      # Python code generation
  vexil-runtime/         # Rust runtime: BitWriter/BitReader, Pack/Unpack, LEB128, ZigZag
  vexilc/                # CLI with ariadne error rendering
  vexil-store/           # .vx text and .vxb binary file formats
  vexil-bench/           # Encode/decode benchmarks (Criterion)
packages/
  runtime-ts/            # @vexil-lang/runtime -- TypeScript BitWriter/BitReader (npm)
  runtime-go/            # github.com/vexil-lang/vexil/packages/runtime-go -- Go runtime
  runtime-py/            # vexil_runtime -- Python BitWriter/BitReader (PyPI)
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
- [**vexmon**](https://github.com/vexil-lang/vexmon) — real-time system monitor using Vexil over WebSocket (~300 B/s for full telemetry)
- API reference: [vexil-lang](https://docs.rs/vexil-lang) | [vexil-runtime](https://docs.rs/vexil-runtime) | [vexil-codegen-rust](https://docs.rs/vexil-codegen-rust) | [vexil-codegen-ts](https://docs.rs/vexil-codegen-ts) | [vexil-codegen-go](https://docs.rs/vexil-codegen-go) | [vexil-codegen-py](https://docs.rs/vexil-codegen-py) | [vexil-store](https://docs.rs/vexil-store)

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md). Language changes and protocol modifications go through the RFC process in [GOVERNANCE.md](./GOVERNANCE.md).

## License

Licensed under either of [MIT](./LICENSE-MIT) or [Apache-2.0](./LICENSE-APACHE) at your option.
