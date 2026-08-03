# Vexil Compliance Vectors

Golden byte cases for testing Vexil implementations against the same wire
contract. For a given schema and value, conforming implementations must emit
the expected bytes and reconstruct the expected value.

## Overview

Compliance vectors are JSON files that define test cases with:
- Schema definition (inline Vexil source)
- Input values
- Expected byte output (hex-encoded)

The wire-format specification defines the encoding. These vectors turn that
contract into executable examples shared by the runtimes and generators.

## Vector Format

### Standard Vectors

Each vector file is a JSON array of test case objects:

```json
[
  {
    "name": "human-readable test name",
    "schema": "namespace test.example\nmessage M { field @0 : u32 }",
    "type": "M",
    "value": { "field": 42 },
    "expected_bytes": "2a000000",
    "notes": "u32 42 = 0x2A in little-endian"
  }
]
```

### Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Unique identifier for the test case |
| `schema` | Yes | Complete Vexil schema as string |
| `type` | Yes | The type name to encode/decode |
| `value` | Yes | JSON value matching the schema type |
| `expected_bytes` | Yes | Hex-encoded expected wire bytes |
| `notes` | No | Human-readable explanation |

### Delta Encoding Vectors

Delta vectors use a frame-based format:

```json
{
  "name": "delta_u32_sequence",
  "schema": "namespace test.delta\nmessage M { @delta v @0 : u32 }",
  "type": "M",
  "frames": [
    { "value": { "v": 100 }, "expected_bytes": "64000000" },
    { "value": { "v": 110 }, "expected_bytes": "0a000000" },
    { "reset": true },
    { "value": { "v": 100 }, "expected_bytes": "64000000" }
  ],
  "notes": "Frames show delta encoding progression"
}
```

### Schema Evolution Vectors

Evolution vectors test backward/forward compatibility:

```json
{
  "name": "add_optional_field",
  "schema_v1": "namespace test.evol\nmessage M { a @0 : u32 }",
  "schema_v2": "namespace test.evol\nmessage M { a @0 : u32  b @1 : optional<u32> }",
  "value_v1": { "a": 1 },
  "encoded_v1": "01000000",
  "decoded_as_v2": { "a": 1, "b": null },
  "roundtrip_v2": { "a": 1, "b": null },
  "encoded_v2": "0100000000"
}
```

## Vector Files

| File | Purpose |
|------|---------|
| `primitives.json` | Scalar types: bool, integers, floats, string |
| `sub_byte.json` | Sub-byte integer packing (`u1`, `u3`, `u5`, etc.) |
| `enums.json` | Enum discriminants and bit-width selection |
| `arrays_maps.json` | Variable-length arrays and maps |
| `annotations.json` | `@varint` and `@zigzag` field encoding annotations |
| `optionals.json` | Optional presence flags |
| `results.json` | Result discriminants, void alternatives, and packed adjacency |
| `messages.json` | Message field ordering and padding |
| `unions.json` | Union discriminant + length prefix |
| `delta.json` | Delta/differential encoding |
| `evolution.json` | Schema evolution compatibility |
| `v1_types.json` | Fixed-point, geometric, set, and bit-width cases (legacy filename) |
| `generated_wire.json` | Representative shared matrix executed by generated target code |

## Verifying expected bytes

The Rust reference implementation checks its expected bytes with:

```bash
cargo test -p vexil-codegen-rust --test golden_bytes -- --nocapture
```

That command verifies hard-coded expectations; it does not rewrite vector JSON.
When adding or changing a vector, derive the bytes from the wire-format
specification, add the expected value explicitly, and review it as a contract
change.

## Cross-Implementation Testing

An implementation using the suite should:

1. **Encode Test**: For each vector, encode the `value` and assert the output matches `expected_bytes` exactly
2. **Decode Test**: For each vector, decode `expected_bytes` and assert the result equals `value`
3. **Round-trip Test**: Encode → Decode → Verify identity

### Test Pseudocode

```python
for vector in load_vectors("primitives.json"):
    # Encode test
    encoded = encode(vector.schema, vector.type, vector.value)
    assert hex(encoded) == vector.expected_bytes
    
    # Decode test
    decoded = decode(vector.schema, vector.type, hex_to_bytes(vector.expected_bytes))
    assert decoded == vector.value
    
    # Round-trip
    re_encoded = encode(vector.schema, vector.type, decoded)
    assert re_encoded == encoded
```

### Value Representation

Special value formats in JSON:

| Type | JSON Representation | Example |
|------|---------------------|---------|
| `bool` | `true` / `false` | `true` |
| integers | Number | `42`, `-17` |
| `f32`/`f64` | Number or string | `3.14`, `"NaN"`, `"-0.0"` |
| `string` | String | `"hello"` |
| `bytes` | Array of numbers | `[0xDE, 0xAD]` |
| `enum` | String (variant name) | `"Active"` |
| `flags` | Object of bools | `{"Read": true, "Write": false}` |
| `optional<T>` | `null` or value | `null`, `42` |
| `result<T,E>` | Object with exactly one `ok` or `err` key | `{"ok": 42}`, `{"err": "oops"}` |
| `array<T>` | Array | `[1, 2, 3]` |
| `map<K,V>` | Object (string keys) | `{"key": "value"}` |
| `set<T>` | Array (sorted, unique) | `["a", "b"]` |
| `union` | Object with `variant` key; unknown open variants include `unknown`, `discriminant`, and byte-array `data` | `{"variant": "Circle", "radius": 1.5}` |
| `void` | `null` | `null` |
| `newtype` | Inner value | `42` |
| `vec2`/`vec3`/`vec4` | Array | `[1.0, 2.0, 3.0]` |
| `quat` | Array | `[0.0, 0.0, 0.0, 1.0]` |
| `mat3`/`mat4` | Array (column-major) | `[...9 or 16 elements...]` |
| `fixed32`/`fixed64` | Number (raw integer) | `65536` for 1.0 in Q16.16 |

### Float Special Values

Floating-point values can be represented as strings for special cases:
- `"NaN"` - Not a Number (will be canonicalized)
- `"+Inf"` / `"Infinity"` - Positive infinity
- `"-Inf"` / `"-Infinity"` - Negative infinity
- `"+0.0"` - Positive zero
- `"-0.0"` - Negative zero (distinct from positive)

## Hex Encoding Rules

- Lowercase hex digits (`a-f` not `A-F`)
- No `0x` prefix
- No spaces between bytes
- Even number of characters (whole bytes)

**Valid:** `"2a00"`  
**Invalid:** `"2A00"`, `"0x2a00"`, `"2a 00"`, `"2a0"`

## Failure Reporting

A conformance failure MUST report:
1. Vector file name
2. Test case `name`
3. Operation (encode/decode/roundtrip)
4. Expected bytes/value
5. Actual bytes/value
6. Difference explanation

## Reference Implementation

The Rust implementation in `vexil-runtime` is the repository's reference
implementation; the wire-format specification remains normative:
- `BitWriter` produces the canonical byte sequence
- `BitReader` parses the canonical byte sequence
- Golden tests in `crates/vexil-codegen-rust/tests/golden_bytes.rs`

Rust and TypeScript cover the broad vector suite. Go and Python cover a
representative shared matrix; see the
[support matrix](../../docs/book/src/getting-started/support-matrix.md) for the
current boundary.

## Adding New Vectors

1. Create a minimal schema demonstrating the feature
2. Generate expected bytes using the Rust reference
3. Document the expected encoding in `notes`
4. Add to the appropriate JSON file or create new file
5. Update this README with the new file

### Vector Contribution Checklist

- [ ] Schema is minimal (only types needed for test)
- [ ] Test case name is descriptive and unique
- [ ] `expected_bytes` is verified against Rust reference
- [ ] `notes` explain the encoding
- [ ] JSON is valid and formatted
- [ ] File is listed in this README

## Versioning

Vectors evolve with the specification. The `v1_types.json` name predates the
current versioning guidance and does not claim that Vexil 1.0 has shipped.

---

**See also:** [Wire-Format Specification](/spec/wire-format.md) — Complete binary format specification
