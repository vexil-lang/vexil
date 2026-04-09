# Vexil Trait System — Design Document

**Date:** 2026-04-09
**Status:** Draft
**Phase:** 7

---

## Overview

Traits define structural contracts that messages, enums, and other types can implement. They have zero wire impact — traits are compile-time contracts that generate code in backends.

## Syntax

```vexil
trait SensorData {
    sensor_id: u32
    timestamp: u64
}

trait Validatable {
    fn validate() -> bool
}

trait Tagged<T> {
    tag: T
    label: string
}

impl SensorData for TemperatureReading { }
impl Validatable for PlayerState { }
impl Tagged<string> for Item { }
```

## Rules

1. **Trait fields** — required fields that implementing types must have (by name and type)
2. **Trait functions** — function signatures (no body in trait, generated in impl)
3. **Trait type params** — generic traits like `Tagged<T>`
4. **Impl** — declares that a type implements a trait. Validates field compatibility at compile time.
5. **No wire impact** — traits don't exist on the wire. They're codegen contracts.
6. **Multiple traits** — a type can implement multiple traits
7. **Trait bounds** — `type SortedList<T: Ord> = array<T>` (deferred to Phase 9)

## Codegen Impact

### Rust Backend
```rust
// trait SensorData { sensor_id: u32, timestamp: u64 }
// impl SensorData for TemperatureReading { }
// Generates:

impl SensorData for TemperatureReading {
    fn sensor_id(&self) -> u32 { self.sensor_id }
    fn timestamp(&self) -> u64 { self.timestamp }
}
```

### TypeScript Backend
```typescript
// Generates interface + type guard
interface SensorData {
    sensor_id: number;
    timestamp: bigint;
}

function isSensorData(obj: unknown): obj is SensorData {
    return typeof obj === 'object' && obj !== null &&
        'sensor_id' in obj && 'timestamp' in obj;
}
```

### Go Backend
```go
// Generates interface
type SensorData interface {
    SensorId() uint32
    Timestamp() uint64
}
```

## Implementation Plan

### 1. AST (ast/mod.rs)
- `AstTraitDecl` — name, type_params, fields, functions
- `AstTraitFnDecl` — name, params, return_type
- `AstImplDecl` — trait_name, target_type, field_overrides
- `Decl::Trait(TraitDecl)` and `Decl::Impl(ImplDecl)`

### 2. Lexer (lexer/token.rs)
- `KwTrait`, `KwImpl`, `KwFor`, `KwFn`

### 3. Parser (parser/decl.rs)
- Parse `trait Name<T> { fields... fn name() -> type }`
- Parse `impl TraitName for TypeName { }`

### 4. Validate (validate.rs)
- Impl target must exist and be a message/enum
- Impl trait must exist
- Required fields must exist in target type with compatible types
- No duplicate impl (same trait for same type)

### 5. Lower (lower.rs)
- Lower TraitDecl to IR TypeDef::Trait
- Lower ImplDecl to IR TypeDef::Impl
- Store trait requirements in IR

### 6. IR (ir/mod.rs)
- `TraitDef` — name, type_params, required_fields, required_fns
- `ImplDef` — trait_id, target_id
- `TypeDef::Trait(TraitDef)`, `TypeDef::Impl(ImplDef)`

### 7. Codegen
- Rust: generate trait definition + impl blocks
- TS: generate interface + type guard
- Go: generate interface
