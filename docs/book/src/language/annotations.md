# Annotations

Annotations modify the behavior of types, fields, and declarations. They are prefixed with `@`.

## Encoding annotations

These change how a field is encoded on the wire:

| Annotation | Applies to | Effect |
|-----------|-----------|--------|
| `@varint` | unsigned integers | LEB128 variable-length encoding |
| `@zigzag` | signed integers | ZigZag encoding (small magnitudes use fewer bytes) |
| `@delta` | numeric fields in arrays | Delta encoding (store differences, not absolute values) |

```vexil
message Packet {
    sequence @0 : u32 @varint
    offset   @1 : i32 @zigzag
}
```

## Declaration annotations

| Annotation | Applies to | Effect |
|-----------|-----------|--------|
| `@non_exhaustive` | enum, union | Allows adding variants without breaking decoders |
| `@deprecated` | fields, variants | Marks as deprecated in generated code |
| `@removed(ordinal, reason: "...")` | message fields | Typed tombstone for removed fields |

```vexil
@non_exhaustive
enum Status : u8 {
    Active     @0
    @deprecated
    Legacy     @1
    Suspended  @2
}
```

## Removed fields

When evolving a schema, use `@removed` to leave a typed tombstone. This allows decoders to skip the correct number of bytes for the removed field:

```vexil
message Config {
    name       @0 : string
    @removed(1, reason: "migrated to timeout_ms") : u32
    timeout_ms @2 : u64
}
```

See the [language specification](https://github.com/vexil-lang/vexil/blob/main/spec/vexil-spec.md) for the full normative reference.
