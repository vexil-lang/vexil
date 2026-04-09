# Type Aliases and Constants

## Type Aliases

A type alias gives an existing type a new name. Aliases are transparent -- they have zero wire impact and produce the same encoding as the underlying type.

```vexil
type UserId = u64
type Token = bytes
type DFixed = fixed64
```

Aliases make schemas more readable by replacing raw types with meaningful names. A field `sender @0 : UserId` is clearer than `sender @0 : u64`.

### Rules

- The target must be a concrete type (not another alias)
- Alias chains are rejected: `type A = u64` then `type B = A` is invalid
- Aliases do not create distinct wire types -- `UserId` and `u64` produce identical bytes
- Aliases can be exported via `import { UserId } from my.types`

## Constants

Constants define named compile-time values. They have no wire impact -- they exist only during compilation.

```vexil
const MaxHealth : u32 = 100
const TickRate : u32 = 64
const DefaultPos : fixed64 = 0.0
```

### Using Constants

Constants can be used in:

- **Array sizes**: `array<u8, MaxHealth>`
- **Where clause bounds**: `where value in 0..MaxHealth`
- **Other constant expressions**: see below

### Cross-References

Constants can reference other constants using simple arithmetic:

```vexil
const TicksPerSec : u32 = 64
const TickMs : u32 = 1000 / TicksPerSec   # evaluates to 15
const TwoSecTicks : u32 = TicksPerSec * 2  # evaluates to 128
```

Supported operators: `+`, `-`, `*`, `/` (integer division, truncates toward zero).

Circular dependencies are rejected at compile time:

```vexil
# INVALID: circular dependency
const A : u32 = B + 1
const B : u32 = A + 1
```

Division by zero is also caught at compile time.

### Supported Types

Constants can be declared with these types: `bool`, `u8`--`u64`, `i8`--`i64`, `f32`, `f64`, `fixed32`, `fixed64`.
