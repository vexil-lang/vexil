# Type Aliases and Constants

## Type Aliases

A type alias gives an existing type a new name. It's transparent — same wire encoding, same codegen, just a different name in the schema.

```vexil
type UserId = u64
type Token = bytes
type DFixed = fixed64
```

`UserId` and `u64` produce identical bytes. The alias exists only in the schema source, making fields more readable.

### Rules

- The target must be a concrete type, not another alias
- Alias chains are rejected: `type A = u64` then `type B = A` won't compile
- Aliases can be imported: `import { UserId } from my.types`

## Constants

Constants are named compile-time values. They don't exist on the wire — they're resolved during compilation and disappear.

```vexil
const MaxHealth : u32 = 100
const TickRate : u32 = 64
```

### Where You Can Use Them

- Array sizes: `array<u8, MaxHealth>`
- Where clause bounds: `where value in 0..MaxHealth`
- Other constant expressions (see below)

### Cross-References

Constants can reference each other with `+`, `-`, `*`, `/`:

```vexil
const TicksPerSec : u32 = 64
const TickMs : u32 = 1000 / TicksPerSec   # 15
const TwoSecTicks : u32 = TicksPerSec * 2  # 128
```

Division truncates toward zero (integer division). Division by zero and circular dependencies are caught at compile time — you'll get a diagnostic, not a runtime panic.

### Supported Types

`bool`, `u8`–`u64`, `i8`–`i64`, `f32`, `f64`, `fixed32`, `fixed64`. Messages, enums, unions — none of those. Constants are values, not types.
