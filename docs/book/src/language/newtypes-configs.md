# Newtypes and Configs

## Newtypes

A newtype wraps an existing type with a distinct name. On the wire, it encodes identically to the underlying type.

```vexil
newtype UserId = u64
newtype Temperature = f32
```

Newtypes provide type safety in generated code -- a `UserId` and a raw `u64` are different types even though they have the same wire representation.

### Newtypes with annotations

```vexil
newtype CompactId = u64 @varint
```

The annotation applies to the wire encoding of the underlying type.

## Configs

Configs are compile-time constant declarations. They do not appear on the wire but are available in generated code as constants.

```vexil
config MAX_PACKET_SIZE : u32 = 1500
config VERSION : string = "1.0.0"
```

Configs are useful for sharing magic numbers and version strings between schema and application code without encoding them in every message.

See the [language specification](https://github.com/vexil-lang/vexil/blob/main/spec/vexil-spec.md) for the full normative reference.
