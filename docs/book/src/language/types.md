# Types

## Primitive types

| Type | Size | Description |
|------|------|-------------|
| `bool` | 1 bit | True or false |
| `u8` -- `u64` | 8--64 bits | Unsigned integers |
| `i8` -- `i64` | 8--64 bits | Signed integers (two's complement) |
| `f32` | 32 bits | IEEE 754 single-precision float |
| `f64` | 64 bits | IEEE 754 double-precision float |

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
| `map<K, V>` | LEB128 count + key-value pairs |
| `result<T, E>` | Boolean tag + ok or error value |

## Wire encoding

All types encode to a deterministic byte sequence. Fixed-size types pack at their natural bit width. Variable-length types (string, bytes, array, map) use LEB128 length prefixes.

The `@varint` annotation changes a fixed-width integer to unsigned LEB128 encoding. The `@zigzag` annotation uses ZigZag encoding for signed integers (small magnitudes use fewer bytes).

See the [language specification](https://github.com/vexil-lang/vexil/blob/main/spec/vexil-spec.md) for complete encoding rules.
