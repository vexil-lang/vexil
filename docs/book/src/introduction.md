# Vexil

**Vexil** (Validated Exchange Language) is a typed schema definition language with first-class encoding semantics. It describes the shape, constraints, and wire encoding of data crossing system boundaries.

## What makes Vexil different?

**Encoding is part of the type system.** The type `u4` means exactly 4 bits on the wire. The annotation `@varint` on a `u64` changes the wire encoding to unsigned LEB128. The schema IS the wire contract, not just the shape contract.

**Deterministic encoding.** Same data always produces identical bytes. This enables BLAKE3 content addressing, deduplication, and replay detection -- things that Protocol Buffers, Cap'n Proto, and FlatBuffers cannot guarantee.

**Multi-language.** Generate code for Rust, TypeScript, and Go from the same `.vexil` schema. All three produce byte-identical wire output, verified by compliance vectors.

## Quick example

```vexil
namespace sensor.packet

enum SensorKind : u8 {
    Temperature @0
    Humidity    @1
    Pressure    @2
}

message SensorReading {
    channel  @0 : u4              # 4 bits on the wire
    kind     @1 : SensorKind
    value    @2 : u16
    sequence @3 : u32 @varint     # variable-length encoding
}
```

Generate code:

```sh
vexilc codegen sensor.vexil --target rust
vexilc codegen sensor.vexil --target typescript
vexilc codegen sensor.vexil --target go
vexilc codegen sensor.vexil --target python
```

## Installation

```sh
cargo install vexilc
```

Pre-built binaries for Linux, macOS, and Windows are available on the [Releases page](https://github.com/vexil-lang/vexil/releases).
