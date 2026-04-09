# Types

## Primitive types

| Type | Size | Description |
|------|------|-------------|
| `bool` | 1 bit | True or false |
| `u8` -- `u64` | 8--64 bits | Unsigned integers |
| `i8` -- `i64` | 8--64 bits | Signed integers (two's complement) |
| `f32` | 32 bits | IEEE 754 single-precision float |
| `f64` | 64 bits | IEEE 754 double-precision float |
| `fixed32` | 32 bits | Q16.16 fixed-point (two's complement) |
| `fixed64` | 64 bits | Q32.32 fixed-point (two's complement) |

Fixed-point types (`fixed32`, `fixed64`) provide deterministic fractional arithmetic. Unlike IEEE 754 floats, the same operation always produces the same result across platforms — essential for simulation, networking, and content-addressed data.

The `@varint` annotation is valid on `fixed32` and `fixed64`, encoding the raw `i32`/`i64` as unsigned LEB128 for variable-length wire representation.

## Sub-byte types

| Type | Size | Description |
|------|------|-------------|
| `u1` -- `u7` | 1--7 bits | Unsigned sub-byte integers |
| `i2` -- `i7` | 2--7 bits | Signed sub-byte integers |

Sub-byte fields are packed LSB-first within each byte. This is unique to Vexil -- Protocol Buffers and other formats cannot represent fields smaller than one byte.

## Semantic types

| Type | Wire encoding | Description |
|------|--------------|-------------|
| `string` | LEB128 length + UTF-8 | Text |
| `bytes` | LEB128 length + raw | Binary data |
| `uuid` | 16 bytes | UUID as raw bytes |
| `timestamp` | 64-bit signed | Unix epoch (interpretation is application-defined) |
| `rgb` | 3 bytes | Red, green, blue |
| `hash` | 32 bytes | BLAKE3 hash |

## Parameterized types

| Type | Description |
|------|-------------|
| `optional<T>` | Presence bit + value |
| `array<T>` | LEB128 count + elements |
| `array<T, N>` | Fixed-size array (no count prefix, N elements) |
| `map<K, V>` | LEB128 count + sorted key-value pairs |
| `result<T, E>` | Boolean tag + ok or error value |
| `set<T>` | LEB128 count + sorted unique elements |

Fixed-size arrays (`array<T, N>`) have no length prefix on the wire -- the size is part of the schema. `N` must be a compile-time constant.

Sets (`set<T>`) are unordered unique collections. Elements are sorted on encode for deterministic wire representation. Duplicates are silently deduplicated.

## Geometric types

Graphics and simulation primitives with deterministic wire encoding:

| Type | Components | Description |
|------|-----------|-------------|
| `vec2<T>` | x, y | 2D vector |
| `vec3<T>` | x, y, z | 3D vector |
| `vec4<T>` | x, y, z, w | 4D vector or homogeneous coordinate |
| `quat<T>` | x, y, z, w | Quaternion rotation |
| `mat3<T>` | 9 components | 3x3 matrix (column-major) |
| `mat4<T>` | 16 components | 4x4 matrix (column-major) |

Valid element types: `fixed32`, `fixed64`, `f32`, `f64`.

Examples:

```vexil
message Transform {
    position @0 : vec3<fixed64>   # deterministic simulation position
    rotation @1 : quat<fixed64>   # deterministic quaternion
    gl_pos   @2 : vec3<f32>       # GPU-ready render position
    model    @3 : mat4<f32>       # 4x4 transform matrix
}
```

Wire encoding: components written in order (x, y, z, w), no padding, no count prefix. Total size = N components x element size.

## Inline bitfields

Anonymous flags for compact permission or storage bits:

```vexil
message FileHeader {
    perms @0 : bits { r, w, x, hidden, system }
}
```

Wire encoding: exactly N bits (one per named flag), LSB-first. The example above uses 5 bits.

## Wire encoding

All types encode to a deterministic byte sequence. Fixed-size types pack at their natural bit width. Variable-length types (string, bytes, array, map, set) use LEB128 length prefixes.

The `@varint` annotation changes a fixed-width integer to unsigned LEB128 encoding. The `@zigzag` annotation uses ZigZag encoding for signed integers (small magnitudes use fewer bytes). The `@delta` annotation generates stateful encoder/decoder pairs that transmit field-level deltas.

See the [language specification](https://github.com/vexil-lang/vexil/blob/main/spec/vexil-spec.md) for complete encoding rules.
