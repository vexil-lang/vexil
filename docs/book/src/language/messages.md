# Messages

Messages are the primary data type in Vexil -- ordered, typed fields with explicit ordinals.

```vexil
message SensorReading {
    channel  @0 : u4
    kind     @1 : SensorKind
    value    @2 : u16
    sequence @3 : u32 @varint
}
```

Fields are encoded in ordinal order. Each field has a name, an ordinal (`@N`), and a type.

## Field ordinals

Ordinals determine wire order. They must be unique within a message but don't need to be sequential. Gaps are allowed -- this is important for schema evolution, since you can add new fields at new ordinals without disturbing existing ones.

```vexil
message Config {
    name    @0 : string
    # @1 was removed
    timeout @2 : u32
    retries @3 : u8
}
```

## Field annotations

Fields can carry encoding annotations:

```vexil
message Packet {
    sequence @0 : u32 @varint     # LEB128 variable-length encoding
    delta    @1 : i32 @zigzag     # ZigZag encoding for signed values
    payload  @2 : bytes
}
```

## Wire encoding

Fields are packed in ordinal order with LSB-first bit packing. Sub-byte fields (like `u4`) pack tightly -- two `u4` fields occupy a single byte. After all fields, the encoder flushes to a byte boundary.

## Unknown fields

Every generated message struct has an `_unknown` field that captures trailing bytes from newer schema versions. When a v1 decoder reads v2-encoded data, the extra bytes are preserved. Re-encoding includes them, enabling forward-compatible round-tripping with no data loss.

See the [language specification](https://github.com/vexil-lang/vexil/blob/main/spec/vexil-spec.md) for the full normative reference.
