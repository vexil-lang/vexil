# Where Clauses

Where clauses add declarative validation constraints to fields. They generate validation code that runs automatically during encode and decode -- invalid data is rejected before it ever hits the wire.

## Basic Syntax

```vexil
message Player {
    health @0 : u8 where value >= 0 && value <= 100
    name   @1 : string where len(value) >= 1 && len(value) <= 32
    level  @2 : u16 where value in 1..999
}
```

The `value` keyword refers to the field's value. Constraints are boolean expressions evaluated at encode/decode time.

## Comparison Operators

| Operator | Meaning |
|----------|---------|
| `==` | Equal |
| `!=` | Not equal |
| `<` | Less than |
| `>` | Greater than |
| `<=` | Less than or equal |
| `>=` | Greater than or equal |

## Logical Operators

| Operator | Meaning |
|----------|---------|
| `&&` | Logical AND |
| `\|\|` | Logical OR |
| `!` | Logical NOT |

```vexil
message Request {
    code @0 : u16 where value >= 100 && value < 600
    flag @1 : u8 where value == 0 || value == 1 || value == 255
}
```

## Range Expressions

Check if a value falls within a range:

```vexil
# Inclusive: 0 to 100 (both endpoints included)
health @0 : u8 where value in 0..100

# Exclusive: 0 to 99 (upper bound excluded)
age @1 : u8 where value in 0..<100
```

Range bounds can be constants:

```vexil
const MaxLevel : u16 = 999

message Character {
    level @0 : u16 where value in 1..MaxLevel
}
```

## Built-in Functions

### len(value)

Returns the length of a collection, string, or byte array:

```vexil
message User {
    username @0 : string where len(value) in 3..32
    bio      @1 : string where len(value) <= 500
    tags     @2 : array<string> where len(value) <= 10
    key      @3 : bytes where len(value) == 32
}
```

`len()` is valid on: `string`, `bytes`, `array<T>`, `array<T, N>`, `map<K,V>`, `set<T>`.

## What Constraints Do NOT Do

- **Cross-field constraints are not supported**: `where amount <= balance` referencing another field is not allowed in v1.0
- **Regex patterns are not supported**: `where value matches "..."` is deferred to 1.1
- **User-defined functions are not supported**: constraints must use built-in operators and functions
- **Constraints do not affect wire format**: they are compile-time contracts, not runtime metadata

## Error Behavior

When a constraint is violated:

- **On encode**: `PackError::ConstraintViolation` is returned, no data is written
- **On decode**: `DecodeError::ConstraintViolation` is returned, partial data is discarded

This means invalid data never crosses the wire.
