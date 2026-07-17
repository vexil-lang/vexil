# Vexil

Vexil (Validated Exchange Language) is a typed schema language where the wire encoding is part of the type system. It describes the shape, constraints, and wire encoding of data crossing system boundaries.

## What makes Vexil different?

**Encoding is part of the type system.** `u4` means exactly 4 bits on the wire. `@varint` on a `u64` switches to unsigned LEB128. The schema IS the wire contract, not a hint about the wire format.

**Deterministic encoding.** Same data always produces identical bytes. This enables BLAKE3 content addressing, deduplication, and replay detection. These are things Protobuf, Cap'n Proto, and FlatBuffers don't guarantee.

**Multi-language.** Generate Rust, TypeScript, Go, and Python from the same `.vexil` schema. Rust and TypeScript have byte-identical output verified by compliance vectors; Go and Python lack that verification.

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
