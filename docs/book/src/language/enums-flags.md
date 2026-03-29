# Enums and Flags

## Enums

Enums define a closed set of named variants with a fixed-width backing type.

```vexil
enum Direction : u8 {
    North @0
    East  @1
    South @2
    West  @3
}
```

The backing type (`: u8`) determines the wire size. Variant ordinals (`@N`) are the values written to the wire.

### Non-exhaustive enums

By default, enums are exhaustive -- receiving an unknown variant is an error. Use `@non_exhaustive` to allow future additions:

```vexil
@non_exhaustive
enum Status : u8 {
    Active   @0
    Inactive @1
}
```

A non-exhaustive enum can safely add variants in newer schema versions without breaking existing decoders.

## Flags

Flags are bitmask types where each named bit occupies a specific position in a fixed-width integer.

```vexil
flags Permissions : u8 {
    Read    @0
    Write   @1
    Execute @2
}
```

Multiple flags can be set simultaneously. The ordinal (`@N`) is the bit position, not the value -- `Read @0` means bit 0 (value 1), `Write @1` means bit 1 (value 2), `Execute @2` means bit 2 (value 4).

Flags encode as their backing type on the wire. A `flags Permissions : u8` always occupies exactly 8 bits.

See the [language specification](https://github.com/vexil-lang/vexil/blob/main/spec/vexil-spec.md) for the full normative reference.
