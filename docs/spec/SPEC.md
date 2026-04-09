---
status: FROZEN
version: 1.0.0
frozen_date: 2026-04-10
---

> **SPEC FROZEN**: This specification is locked for Vexil 1.0.
> Changes require major version bump.

# Vexil 1.0 Binary Format Specification

**Version:** 1.0.0  
**Status:** FROZEN  
**Date:** 2026-04-10

---

## 1. Overview

This document specifies the Vexil 1.0 binary wire format. It is the authoritative reference for encoder and decoder implementations. All conformant implementations MUST produce identical bytes for the same input and MUST reconstruct identical values from the same bytes.

### 1.1 Design Principles

- **Schema is the contract:** No type tags or self-description on the wire
- **LSB-first bit packing:** Sub-byte fields pack into bytes least-significant-bit first
- **Little-endian multi-byte:** All multi-byte integers use little-endian byte order
- **Canonical form:** The BLAKE3 hash of canonical schema form identifies the contract

### 1.2 Compliance

A conformant implementation MUST:

1. Produce byte-identical output to the reference implementation for all valid inputs
2. Decode all valid byte sequences to their defined values
3. Reject invalid byte sequences with structured errors
4. Pass all compliance vectors in `/compliance/vectors/`

---

## 2. Bit Stream Fundamentals

### 2.1 LSB-First Bit Packing

Bits are packed into bytes starting from the least significant bit (bit 0). The first field occupies the lowest bits of the first byte. When a byte fills (8 bits), packing continues to the next byte.

**Example:** Packing `u3` value 5 (binary `101`) followed by `u5` value 18 (binary `10010`):

```
LSB-first in byte: [10010_101] = 0x95
                         ^^^ 5 (u3)
                   ^^^^^     18 (u5)
```

### 2.2 Byte Alignment

Multi-byte types (u16, u32, u64, f32, f64, strings, bytes) require byte alignment. Before writing such a type, the bit stream is flushed to the next byte boundary. After writing, the stream resumes at the next available bit position.

### 2.3 Padding Rules

- Messages MUST be padded to a byte boundary
- Padding bits MUST be zero on encode
- Decoders MUST ignore padding bits
- An empty message (zero fields) encodes as a single `0x00` byte

---

## 3. Scalar Encodings

### 3.1 Integer Types

| Type  | Width | Encoding                                 |
|-------|-------|------------------------------------------|
| `u8`  | 8 bit | Single byte, unsigned                      |
| `u16` | 16 bit | 2 bytes, little-endian                    |
| `u32` | 32 bit | 4 bytes, little-endian                    |
| `u64` | 64 bit | 8 bytes, little-endian                    |
| `i8`  | 8 bit | Single byte, two's complement              |
| `i16` | 16 bit | 2 bytes, two's complement, little-endian  |
| `i32` | 32 bit | 4 bytes, two's complement, little-endian  |
| `i64` | 64 bit | 8 bytes, two's complement, little-endian  |

**Examples:**
- `u16` value 0x0102 encodes as `[0x02, 0x01]`
- `i32` value -1 encodes as `[0xFF, 0xFF, 0xFF, 0xFF]`

### 3.2 Sub-Byte Integer Types

| Type | Width | Range                      |
|------|-------|----------------------------|
| `uN` | N bit | 0 to 2^N − 1               |
| `iN` | N bit | −2^(N−1) to 2^(N−1) − 1    |

Valid N: 1 ≤ N ≤ 64, N ∉ {8,16,32,64}

Sub-byte types pack LSB-first. No padding is inserted between fields.

**Example:** `u3` value 7 (`111`) + `u5` value 31 (`11111`) + `u6` value 63 (`111111`):

```
Byte 0: 11111_111 = 0xFF  (u5[4:0] + u3[2:0])
Byte 1: 00_111111 = 0x3F  (padding + u6[5:0])
```

### 3.3 Boolean Encoding

- `bool` occupies exactly 1 bit
- `false` = 0, `true` = 1
- Participates in sub-byte packing identically to `u1`

### 3.4 Float Encoding

| Type  | Width | Encoding                                      |
|-------|-------|-----------------------------------------------|
| `f32` | 32 bit | IEEE 754 binary32, little-endian             |
| `f64` | 64 bit | IEEE 754 binary64, little-endian             |

**NaN Canonicalization:** All NaN values MUST be normalized to the canonical quiet NaN:
- `f32`: `0x7FC00000`
- `f64`: `0x7FF8000000000000`

**Negative Zero:** MUST be preserved (distinct from positive zero).

**Examples:**
- `f32` value 1.5 encodes as `[0x00, 0x00, 0xC0, 0x3F]`
- `f32` NaN encodes as `[0x00, 0x00, 0xC0, 0x7F]` (canonicalized)

### 3.5 Fixed-Point Encoding

| Type      | Width | Format      | Wire Encoding                    |
|-----------|-------|-------------|----------------------------------|
| `fixed32` | 32 bit | Q16.16     | Same as `i32` (two's complement) |
| `fixed64` | 64 bit | Q32.32     | Same as `i64` (two's complement) |

The scale factor is schema-defined, not on the wire. `fixed32` 1.0 = 65536 in the raw integer representation.

`@varint` is valid on fixed-point types; the value is LEB128-encoded as signed. `@zigzag` MUST NOT be applied.

---

## 4. Varint and ZigZag Encodings

### 4.1 LEB128 (Unsigned)

LEB128 encodes unsigned integers using 7 bits per byte with a continuation flag:

- Bits 0-6: payload (7 bits)
- Bit 7: continuation flag (1 = more bytes follow, 0 = last byte)

**Examples:**
- 0 encodes as `[0x00]`
- 127 encodes as `[0x7F]`
- 128 encodes as `[0x80, 0x01]`
- 300 encodes as `[0xAC, 0x02]`

Maximum length prefix: 4 bytes (values up to 2^28 − 1).
Maximum full u64: 10 bytes.

### 4.2 ZigZag + LEB128 (Signed)

ZigZag maps signed integers to unsigned for efficient LEB128 encoding:

| Signed | ZigZag Encoded |
|--------|----------------|
| 0      | 0              |
| -1     | 1              |
| 1      | 2              |
| -2     | 3              |
| 2      | 4              |

**Formula:**
- Encode: `(n << 1) ^ (n >> (bits - 1))`
- Decode: `(n >> 1) ^ -(n & 1)`

**Examples:**
- -1 encodes as ZigZag(1) + LEB128 = `[0x01]`
- -42 encodes as ZigZag(83) + LEB128 = `[0xD3, 0x01]`

---

## 5. Semantic Type Encodings

### 5.1 String

Wire format: `[LEB128 length][UTF-8 bytes]`

- Empty string: `[0x00]`
- "hello": `[0x05, 0x68, 0x65, 0x6C, 0x6C, 0x6F]`

Maximum length: 67,108,864 bytes (2^26, 64 MiB).
Invalid UTF-8 MUST be rejected.

### 5.2 Bytes

Wire format: `[LEB128 length][raw bytes]`

Same encoding as string, but opaque bytes (no UTF-8 validation).

### 5.3 UUID

16 bytes, big-endian byte order.

### 5.4 Timestamp

64-bit signed integer (`i64`), microseconds since Unix epoch (1970-01-01T00:00:00Z).

### 5.5 RGB

24 bits total: 8 bits R, 8 bits G, 8 bits B.

### 5.6 Hash

256 bits (32 bytes), opaque raw bytes.

---

## 6. Parameterized Type Encodings

### 6.1 Optional<T>

Wire format: `[1-bit presence flag][T if present]`

- Presence bit = 0: absent, no T data follows
- Presence bit = 1: present, T follows

The presence bit participates in sub-byte packing. If present and T is byte-aligned, the stream flushes before T.

**Examples:**
- `none`: `[0x00]` (presence bit 0, flush)
- `some(42)` as `optional<u32>`: `[0x01, 0x2A, 0x00, 0x00, 0x00]` (bit + flush + u32)

### 6.2 Array<T>

Wire format: `[LEB128 count][element 0][element 1]...`

Maximum count: 16,777,216 (2^24).

**Example:** `[1, 2, 3]` as `array<u32>`:
`[0x03, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00]`

### 6.3 Fixed-Size Array<T, N>

Wire format: `[element 0][element 1]...[element N-1]`

No length prefix. Total bits = N × bit_size(T).

### 6.4 Map<K, V>

Wire format: `[LEB128 pair count][K0][V0][K1][V1]...`

Keys MUST be sorted in ascending canonical order before encoding. Maximum pairs: 16,777,216.

**Sort order by key type:**
- Integers: ascending numeric order
- Strings/bytes: ascending lexicographic order
- UUID: lexicographic order of 16-byte big-endian representation
- Enum: ascending by ordinal value
- Flags: ascending by bit value (1 << ordinal)

**Example:** `{"key": 42}` as `map<string, u32>`:
`[0x01, 0x03, 0x6B, 0x65, 0x79, 0x2A, 0x00, 0x00, 0x00]`
(count=1, key_len=3, "key", value=42)

### 6.5 Set<T>

Wire format identical to `array<T>`, but elements MUST be sorted and deduplicated.

### 6.6 Result<T, E>

Wire format: `[1-bit discriminant][T if 0, E if 1]`

- Discriminant 0 = Ok(T)
- Discriminant 1 = Err(E)

Either T or E MAY be `void` (no additional bits for that variant).

---

## 7. Compound Type Encodings

### 7.1 Message Encoding

Wire format: `[fields in ascending ordinal order][padding to byte boundary]`

- Fields are written in `@N` ordinal order, not source order
- Sub-byte fields pack LSB-first
- Byte-aligned fields trigger alignment flush
- Final padding to byte boundary

**Example:**
```vexil
message M {
    flag @0 : bool
    count @1 : u16
    name @2 : string
}
```

Value `{flag: true, count: 42, name: "test"}` encodes as:
`[0x01, 0x2A, 0x00, 0x04, 0x74, 0x65, 0x73, 0x74]`
- `0x01`: bool true (1 bit) + flush = 0x01
- `0x2A, 0x00`: u16 42 little-endian
- `0x04`: LEB128 length 4
- `0x74, 0x65, 0x73, 0x74`: "test"

### 7.2 Empty Message

A message with no fields encodes as a single `0x00` byte.

### 7.3 Enum Encoding

Enums encode as unsigned integers. The bit width is the minimum needed to represent the maximum ordinal:

| Max Ordinal | Bits | Type |
|-------------|------|------|
| 0-1         | 1    | u1 equivalent |
| 2-3         | 2    | u2 equivalent |
| 4-7         | 3    | u3 equivalent |
| ...         | ...  | ... |
| 256+        | 16   | u16 |

`@non_exhaustive` enums use minimum 8 bits.

Explicit backing type (`: u8`, `: u16`, etc.) overrides automatic width.

**Example:**
```vexil
enum Status { Active @0, Inactive @1 }
```
- `Active` encodes as `[0x00]` (1 bit, padded)
- `Inactive` encodes as `[0x01]` (1 bit, padded)

### 7.4 Flags Encoding

Flags encode as little-endian integers. The width is the minimum power-of-2 bytes accommodating all bit positions:

| Max Bit Position | Width |
|------------------|-------|
| 0-7              | u8    |
| 8-15             | u16   |
| 16-31            | u32   |
| 32-63            | u64   |

Bit position N has wire value `1 << N`.

**Example:**
```vexil
flags Perms { Read @0, Write @1, Execute @2 }
```
- `Read`: value 1 (bit 0)
- `Write`: value 2 (bit 1)
- `Read | Write`: value 3

Multi-byte flags trigger byte alignment.

### 7.5 Inline Bitfields

`bits { a, b, c }` encodes identically to a `flags` type of the same width.

---

## 8. Union Encoding

### 8.1 Union Wire Format

Unions encode as: `[discriminant LEB128][payload length LEB128][variant fields]`

1. Flush to byte boundary if mid-byte
2. Write variant ordinal as unsigned LEB128
3. Write payload byte length as unsigned LEB128
4. Write variant fields in ascending ordinal order

The length prefix enables skipping unknown variants for `@non_exhaustive` unions.

**Example:**
```vexil
union Shape {
    Circle @0 { radius @0 : f32 }
    Rect @1 { w @0 : f32, h @1 : f32 }
}
```

`Circle { radius: 1.5 }` encodes as:
`[0x00, 0x04, 0x00, 0x00, 0xC0, 0x3F]`
- `0x00`: discriminant 0 (LEB128)
- `0x04`: payload length 4 bytes
- `0x00, 0x00, 0xC0, 0x3F`: f32 1.5 little-endian

### 8.2 Empty Variants

Variants with no fields write discriminant + zero length:
`[discriminant, 0x00]`

### 8.3 Non-Exhaustive Unions

For `@non_exhaustive` unions, decoders:
1. Read discriminant
2. Read payload length
3. Skip exactly that many bytes if discriminant is unknown

---

## 9. Newtype Encoding

`newtype` wraps exactly one type with no wire overhead. The inner type's encoding is used directly.

**Example:**
```vexil
newtype UserId : u64
```
Encodes identically to `u64`.

---

## 10. Delta Encoding

### 10.1 Delta Field Semantics

Fields marked with `@delta` encode as the difference from the previous value (per-encoder state). The first value encodes as the absolute value.

### 10.2 Delta Wire Format

Delta-encoded values appear on the wire as their absolute difference (current - previous), encoded using the field's normal encoding.

**Example:**
```vexil
message M {
    @delta
    counter @0 : u32
}
```

Frame sequence:
1. `{counter: 100}` → `[0x64, 0x00, 0x00, 0x00]` (absolute 100)
2. `{counter: 110}` → `[0x0A, 0x00, 0x00, 0x00]` (delta 10)
3. Reset → state returns to zero
4. `{counter: 100}` → `[0x64, 0x00, 0x00, 0x00]` (absolute 100)

### 10.3 Delta Reset

Encoders maintain delta state. After a reset, the next value encodes as absolute.

---

## 11. Transport Framing

### 11.1 Length-Prefixed Frames

For streaming (TCP, WebSocket, etc.), frames use length-prefixing:

`[LEB128 payload length][payload bytes]`

**Example:**
- Frame 1: u32(42) → `[0x04, 0x2A, 0x00, 0x00, 0x00]`
- Frame 2: string("hello") → `[0x06, 0x05, 0x68, 0x65, 0x6C, 0x6C, 0x6F]`

### 11.2 Schema Handshake

Connection-time schema identity check:

```
SchemaHandshake {
    hash: [u8; 32],      // BLAKE3 hash of canonical schema
    version: string,      // Human-readable version
}
```

Wire format: `[32 bytes hash][LEB128 version_len][version_bytes]`

Mismatching hashes indicate incompatible schemas.

---

## 12. Limits and Constraints

| Limit | Value | Description |
|-------|-------|-------------|
| MAX_BYTES_LENGTH | 2^26 (67,108,864) | Max string/bytes length |
| MAX_COLLECTION_COUNT | 2^24 (16,777,216) | Max array/map/set elements |
| MAX_LENGTH_PREFIX_BYTES | 4 | Max LEB128 length prefix bytes |
| MAX_RECURSION_DEPTH | 64 | Max nesting depth for recursive types |
| MAX_ORDINAL | 65535 | Max field/variant ordinal |
| MAX_FIXED_ARRAY_SIZE | 65536 | Max `array<T, N>` size |

---

## 13. Encoding Annotations Summary

| Annotation | Valid On | Effect |
|------------|----------|--------|
| `@varint` | Integer types, fixed-point | Encode as LEB128 unsigned |
| `@zigzag` | Signed integer types | ZigZag encode then LEB128 |
| `@delta` | Numeric types | Delta encode from previous |

`@varint` and `@zigzag` MUST NOT be applied to sub-byte types (`uN`/`iN` where N < 8).
`@zigzag` MUST NOT be applied to fixed-point types.

---

## 14. Compliance Vectors

Implementations MUST pass all compliance vectors in `/compliance/vectors/`:

- `primitives.json` - Basic scalar types
- `sub_byte.json` - Sub-byte integer packing
- `enums.json` - Enum discriminants
- `arrays_maps.json` - Collections
- `optionals.json` - Optional types
- `messages.json` - Message encoding
- `unions.json` - Union variants
- `delta.json` - Delta encoding
- `evolution.json` - Schema evolution
- `v1_types.json` - V1.0 types (fixed, geometric, set, bits)

Each vector specifies schema, input value, and expected bytes. Implementations MUST produce identical bytes and reconstruct identical values.

---

## 15. Reference Implementation

The Rust `vexil-runtime` crate is the normative reference implementation. All other implementations MUST match its byte output for all valid inputs.

Key reference modules:
- `bit_writer.rs` - LSB-first encoding
- `bit_reader.rs` - LSB-first decoding
- `leb128.rs` - LEB128 encode/decode
- `zigzag.rs` - ZigZag encode/decode
- `framing.rs` - Length-prefixed frames
- `handshake.rs` - Schema identity

---

## Appendix A: Wire Format Examples

### A.1 Complex Message

```vexil
message Packet {
    header @0 : u16
    flags @1 : bits { encrypted @0, compressed @1 }
    payload @2 : bytes
    checksum @3 : optional<u32>
}
```

Value:
```json
{
    "header": 0xABCD,
    "flags": {"encrypted": true, "compressed": false},
    "payload": [0x01, 0x02, 0x03],
    "checksum": null
}
```

Wire bytes:
1. `0xCD, 0xAB` - u16 0xABCD (after flush from flags bit)
2. `0x01` - flags (encrypted=1, compressed=0), then flush
3. `0x03, 0x01, 0x02, 0x03` - bytes length 3, then payload
4. `0x00` - optional none (1 bit + flush)

Final: `[0xCD, 0xAB, 0x01, 0x03, 0x01, 0x02, 0x03, 0x00]`

### A.2 Union with Multiple Variants

```vexil
union Value {
    Null @0 {}
    Number @1 { val @0 : f64 }
    Text @2 { val @0 : string }
}
```

`Number { val: 3.14 }`:
1. `0x01` - discriminant 1 (LEB128)
2. `0x08` - payload length 8
3. `0x1F, 0x85, 0xEB, 0x51, 0xB8, 0x1E, 0x09, 0x40` - f64 3.14 LE

Final: `[0x01, 0x08, 0x1F, 0x85, 0xEB, 0x51, 0xB8, 0x1E, 0x09, 0x40]`

---

## Appendix B: Changes from Pre-1.0

This frozen spec includes these features added for 1.0:

- Fixed-point types (`fixed32`, `fixed64`)
- Geometric types (`vec2`, `vec3`, `vec4`, `quat`, `mat3`, `mat4`)
- Fixed-size arrays (`array<T, N>`)
- Set type (`set<T>`)
- Inline bitfields (`bits { ... }`)
- Const declarations
- Type aliases
- Where clause constraints

---

**END OF SPECIFICATION**
