# Unions

Unions represent a value that can be one of several typed variants. They are Vexil's tagged union / sum type.

```vexil
union Shape {
    Circle    @0 : f32          # radius
    Rectangle @1 : Dimensions
    Point     @2                # no payload
}
```

## Wire encoding

A union encodes as a discriminant tag followed by the variant payload. The tag type is determined by the number of variants -- the compiler picks the smallest unsigned integer that fits.

## Non-exhaustive unions

Like enums, unions can be marked `@non_exhaustive` to allow adding variants without breaking existing decoders:

```vexil
@non_exhaustive
union Event {
    Click  @0 : ClickData
    Scroll @1 : ScrollData
}
```

## Variants with and without payloads

Variants can carry a payload type or be empty:

```vexil
union Result {
    Ok    @0 : Data
    Error @1 : string
    Empty @2
}
```

See the [language specification](https://github.com/vexil-lang/vexil/blob/main/spec/vexil-spec.md) for the full normative reference.
