# Where Clauses

Where clauses add validation constraints to fields. They run automatically on encode and decode — if the data violates the constraint, you get an error and nothing goes on the wire.

## Basic Syntax

```vexil
message Player {
    health @0 : u8 where value >= 0 && value <= 100
    name   @1 : string where len(value) >= 1 && len(value) <= 32
    level  @2 : u16 where value in 1..999
}
```

`value` refers to the field being validated. The expression must evaluate to a boolean.

## Comparison Operators

`==`, `!=`, `<`, `>`, `<=`, `>=`. They work like you'd expect.

## Logical Operators

`&&`, `||`, `!`. Standard boolean logic.

```vexil
message Request {
    code @0 : u16 where value >= 100 && value < 600
    flag @1 : u8 where value == 0 || value == 1 || value == 255
}
```

## Range Expressions

```vexil
# Inclusive: 0 to 100 (both endpoints included)
health @0 : u8 where value in 0..100

# Exclusive: 0 to 99 (upper bound excluded)
age @1 : u8 where value in 0..<100
```

Bounds can be constants:

```vexil
const MaxLevel : u16 = 999

level @0 : u16 where value in 1..MaxLevel
```

## len()

Returns the length of a string, bytes, array, map, or set:

```vexil
message User {
    username @0 : string where len(value) in 3..32
    tags     @1 : array<string> where len(value) <= 10
    key      @2 : bytes where len(value) == 32
}
```

## What's Not Supported (Yet)

- **Cross-field constraints**: `where amount <= balance` can't reference other fields. Deferred to 1.1.
- **Regex**: `where value matches "..."` doesn't exist. Use a length check and validate regex in application code.
- **User-defined functions**: you can only use the built-in operators and `len()`.

## Error Behavior

On constraint violation:
- Encode: returns `PackError::ConstraintViolation`, nothing written
- Decode: returns `DecodeError::ConstraintViolation`, partial data discarded

Invalid data never makes it onto the wire. That's the whole point.
