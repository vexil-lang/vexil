# Messages

Messages are the primary data type in Vexil -- ordered, typed fields with explicit ordinals.

```text
message SensorReading {
    channel  @0 : u4
    kind     @1 : SensorKind
    value    @2 : u16
    sequence @3 : u32 @varint
}
```

Fields are encoded in ordinal order. Each field has a name, an ordinal (`@N`), and a type.

## Field ordinals

Ordinals determine wire order. They must be unique within a message but do not
need to be sequential. Gaps can reserve positions and make a schema easier to
read, but filling a gap or appending a field still changes the message contract.

```text
message Config {
    name    @0 : string
    # @1 was removed
    timeout @2 : u32
    retries @3 : u8
}
```

## Field annotations

Fields can carry encoding annotations:

```text
message Packet {
    sequence @0 : u32 @varint     # LEB128 variable-length encoding
    delta    @1 : i32 @zigzag     # ZigZag encoding for signed values
    payload  @2 : bytes
}
```

## Wire encoding

Fields are packed in ordinal order with LSB-first bit packing. Sub-byte fields (like `u4`) pack tightly -- two `u4` fields occupy a single byte. After all fields, the encoder flushes to a byte boundary.

## Unknown fields

Generated message types carry target-specific storage for unknown bytes, but
decoders do not populate it. The storage is empty after every decode. Vexil
does not currently provide lossless unknown-field round-tripping.

Message values are not internally length-delimited. A decoder therefore cannot
identify unknown trailing fields safely when the message is nested inside a
parent or inline aggregate. Adding a field is classified as breaking. See
[schema evolution](./evolution.md) for the boundary and migration guidance.

See the [language specification](https://github.com/vexil-lang/vexil/blob/main/spec/language.md) for the full normative reference.
