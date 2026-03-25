# Vexil Reference Implementation — Milestone C Design Spec

**Date:** 2026-03-25
**Status:** Draft
**Milestone:** C — Lowering + IR + Type Checker + Validated IR

## Summary

Add an IR layer to the Vexil compiler pipeline. The IR uses ID-based type
references and a central `TypeRegistry`, resolves all names, computes encoding
metadata, and runs a type checker for wire sizes and recursive type detection.
The existing AST pipeline (`parse()`, `validate.rs`, 74 corpus tests) is
unchanged. A new `compile()` entry point drives the full pipeline.

## Decisions

| Decision | Choice | Rationale |
|---|---|---|
| IR vs AST | Separate layers | AST is source-faithful (spans, raw names); IR is resolved (TypeIds, computed encoding). Keeps parser stable as IR evolves. |
| Type references | ID-based TypeRegistry | `TypeId(u32)` avoids string comparisons, scales to multi-file (Milestone F) without restructuring. |
| Encoding representation | Layered: ResolvedType + FieldEncoding | Logical types stay clean for structural checks; encoding is a parallel field for codegen. No zipping needed. |
| Validation placement | Keep validate.rs as-is | It works, corpus proves it. IR type checker focuses on what validate.rs can't do (cross-type resolution, wire sizes, recursion). |
| API surface | Additive compile() | Existing parse() unchanged. New compile() returns both AST and IR. No churn on existing tests. |
| Cross-file references | Opaque stubs for now | Imported types get TypeIds but no definitions. Milestone F populates them. IR structure doesn't change. |

## Pipeline

```
Source → Lexer → Parser → AST → validate.rs (Milestone B, unchanged)
                                      ↓
                              lower.rs → IR → typeck.rs → Validated IR
```

`compile()` calls `parse()` internally, then if the AST has no errors, runs
lowering and type checking. If `parse()` produces errors, `compile()` returns
the AST and diagnostics but no IR.

## New Files

```
crates/vexil-lang/src/
  ir/
    mod.rs          — IR node types: CompiledSchema, TypeDef, FieldDef, etc.
    types.rs        — TypeId, TypeRegistry, ResolvedType, Encoding
  lower.rs          — AST → IR lowering pass
  typeck.rs         — Type checker on IR
```

`lib.rs` gains a `compile()` function. `Cargo.toml` unchanged (no new deps).

## Public API

```rust
pub struct CompileResult {
    pub schema: Option<Schema>,           // AST (always present if parseable)
    pub compiled: Option<CompiledSchema>, // IR (present if lowering + typeck pass)
    pub diagnostics: Vec<Diagnostic>,
}

/// Full pipeline: parse → validate → lower → type-check.
pub fn compile(source: &str) -> CompileResult {
    let parse_result = parse(source);
    if parse_result.diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return CompileResult {
            schema: parse_result.schema,
            compiled: None,
            diagnostics: parse_result.diagnostics,
        };
    }
    let schema = parse_result.schema.unwrap(); // safe: no errors means Some
    let (compiled, lower_diags) = lower::lower(&schema);
    let mut diagnostics = parse_result.diagnostics;
    diagnostics.extend(lower_diags);
    if let Some(ref compiled) = compiled {
        let check_diags = typeck::check(compiled);
        diagnostics.extend(check_diags);
    }
    // If any error-severity diagnostics were emitted (lowering or typeck),
    // still return the CompiledSchema — it may have partial wire_size info.
    // Consumers must check diagnostics before trusting the IR.
    CompileResult { schema: Some(schema), compiled, diagnostics }
}
```

## Shared Types

`PrimitiveType`, `SubByteType`, and `SemanticType` are re-exported from the AST module.
The IR does not redefine them.

## IR Types

### TypeId and TypeRegistry

```rust
/// Opaque handle to a type definition in the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(u32);

/// Central type store. All cross-references use TypeId.
pub struct TypeRegistry {
    types: Vec<TypeDef>,
    by_name: HashMap<SmolStr, TypeId>,
}

impl TypeRegistry {
    pub fn get(&self, id: TypeId) -> &TypeDef;
    pub fn lookup(&self, name: &str) -> Option<TypeId>;
    pub fn register(&mut self, name: SmolStr, def: TypeDef) -> TypeId;
}
```

### CompiledSchema (IR root)

```rust
pub struct CompiledSchema {
    pub namespace: Vec<SmolStr>,
    pub registry: TypeRegistry,
    pub declarations: Vec<TypeId>,  // in source order
}
```

### TypeDef

```rust
pub enum TypeDef {
    Message(MessageDef),
    Enum(EnumDef),
    Flags(FlagsDef),
    Union(UnionDef),
    Newtype(NewtypeDef),
    Config(ConfigDef),
}
```

### MessageDef

```rust
pub struct MessageDef {
    pub name: SmolStr,
    pub span: Span,                   // source location for diagnostics
    pub fields: Vec<FieldDef>,
    pub tombstones: Vec<TombstoneDef>,
    pub annotations: ResolvedAnnotations,
    pub wire_size: Option<WireSize>,  // computed by type checker
}
```

### FieldDef

```rust
pub struct FieldDef {
    pub name: SmolStr,
    pub span: Span,                   // source location for diagnostics
    pub ordinal: u32,
    pub resolved_type: ResolvedType,
    pub encoding: FieldEncoding,
    pub annotations: ResolvedAnnotations,
}
```

### EnumDef

```rust
pub struct EnumDef {
    pub name: SmolStr,
    pub span: Span,
    pub backing: PrimitiveType,      // resolved: default u32 if unspecified
    pub variants: Vec<EnumVariantDef>,
    pub tombstones: Vec<TombstoneDef>,
    pub annotations: ResolvedAnnotations,
}

pub struct EnumVariantDef {
    pub name: SmolStr,
    pub ordinal: u32,
    pub annotations: ResolvedAnnotations,
}
```

### FlagsDef

```rust
pub struct FlagsDef {
    pub name: SmolStr,
    pub span: Span,
    pub bits: Vec<FlagsBitDef>,
    pub tombstones: Vec<TombstoneDef>,
    pub annotations: ResolvedAnnotations,
}
// Wire size: always Fixed(64 bits). Flags use a u64 bitfield on the wire
// regardless of how many bits are defined. Configurable width is out of scope.

pub struct FlagsBitDef {
    pub name: SmolStr,
    pub bit: u32,
    pub annotations: ResolvedAnnotations,
}
```

### UnionDef

```rust
pub struct UnionDef {
    pub name: SmolStr,
    pub span: Span,
    pub variants: Vec<UnionVariantDef>,
    pub annotations: ResolvedAnnotations,
    pub wire_size: Option<WireSize>,
}

pub struct UnionVariantDef {
    pub name: SmolStr,
    pub ordinal: u32,
    pub fields: Vec<FieldDef>,
    pub tombstones: Vec<TombstoneDef>,
    pub annotations: ResolvedAnnotations,
}
```

### NewtypeDef

```rust
pub struct NewtypeDef {
    pub name: SmolStr,
    pub span: Span,
    pub inner_type: ResolvedType,
    pub terminal_type: ResolvedType,  // follows newtype chains to base type
    pub annotations: ResolvedAnnotations,
}
```

### ConfigDef

```rust
pub struct ConfigDef {
    pub name: SmolStr,
    pub span: Span,
    pub fields: Vec<ConfigFieldDef>,
    pub annotations: ResolvedAnnotations,
}
// ConfigFieldDef.default_value reuses ast::DefaultValue. Named references
// (Ident/UpperIdent variants) are NOT resolved to TypeIds in the IR — they
// remain string-based. Resolution to ordinals is a codegen concern (Milestone D).

pub struct ConfigFieldDef {
    pub name: SmolStr,
    pub resolved_type: ResolvedType,
    pub default_value: DefaultValue,  // reuse AST's DefaultValue
    pub annotations: ResolvedAnnotations,
}
```

### ResolvedType

```rust
/// Fully resolved type — no string references, only TypeIds.
pub enum ResolvedType {
    Primitive(PrimitiveType),
    SubByte(SubByteType),
    Semantic(SemanticType),
    Named(TypeId),
    Optional(Box<ResolvedType>),
    Array(Box<ResolvedType>),
    Map(Box<ResolvedType>, Box<ResolvedType>),
    Result(Box<ResolvedType>, Box<ResolvedType>),
}
```

### Encoding

```rust
/// Wire encoding strategy for a field.
pub enum Encoding {
    /// Default for the logical type (fixed-width primitives, length-prefixed strings, etc.)
    Default,
    /// @varint — LEB128 variable-length encoding for u16/u32/u64
    Varint,
    /// @zigzag — ZigZag + LEB128 for i16/i32/i64
    ZigZag,
    /// @delta — delta encoding on numeric fields (wraps base encoding)
    Delta(Box<Encoding>),
}

pub struct FieldEncoding {
    pub encoding: Encoding,
    pub limit: Option<u64>,  // @limit(N) — max length for string/bytes/array/map
}
```

### WireSize

```rust
/// Computed wire size for a type.
pub enum WireSize {
    /// Exact known size in bits.
    Fixed(u64),
    /// Variable size with optional bounds.
    Variable { min_bits: u64, max_bits: Option<u64> },
}
```

### ResolvedAnnotations

```rust
/// Structured annotations — parsed from raw annotation bags during lowering.
pub struct ResolvedAnnotations {
    pub deprecated: Option<SmolStr>,
    pub since: Option<SmolStr>,
    pub doc: Vec<SmolStr>,
    pub revision: Option<u64>,
    pub non_exhaustive: bool,
    pub version: Option<SmolStr>,
}
```

### TombstoneDef

```rust
pub struct TombstoneDef {
    pub ordinal: u32,
    pub reason: SmolStr,
    pub since: Option<SmolStr>,
}
```

## New ErrorClass Variants

```rust
// Added to diagnostic.rs ErrorClass enum:
RecursiveTypeInfinite,    // direct cycle with no indirection
EncodingTypeMismatch,     // @varint on non-integer, etc. (defensive re-check)
UnresolvedType,           // type name not found during lowering (poison)
```

## Lowering Pass (lower.rs)

Takes `&Schema` (AST), returns `(Option<CompiledSchema>, Vec<Diagnostic>)`.

**Steps in order:**

1. **Register all declarations** — Walk `schema.declarations`, assign a
   `TypeId` to each name via `TypeRegistry::register()`. This forward pass
   ensures mutual references resolve.

2. **Resolve type expressions** — Convert `ast::TypeExpr` → `ir::ResolvedType`.
   `Named(SmolStr)` looks up the registry. If not found (shouldn't happen after
   validate.rs, but defensive), emit diagnostic and use a poison placeholder.
   `Qualified(ns, name)` is left as `Named(TypeId)` if the import is registered
   as a stub, or flagged if unknown.

3. **Compute field encodings** — For each field, read annotations and the
   resolved type to produce `FieldEncoding`. Default encoding is inferred from
   the logical type. `@varint` → `Encoding::Varint`, `@zigzag` →
   `Encoding::ZigZag`. `@delta` wraps whatever base encoding was resolved:
   `@delta @varint u32` → `Delta(Varint)`, `@delta u32` → `Delta(Default)`.
   Annotations are processed in order: resolve base encoding first, then wrap
   with delta if present. `@limit(N)` is extracted into `FieldEncoding.limit`.

4. **Resolve annotations** — Convert `Vec<Annotation>` → `ResolvedAnnotations`.
   Extract known annotations by name, parse their arguments into typed fields.

5. **Build defs** — Assemble `MessageDef`, `EnumDef`, etc. from resolved
   components. Enum backing defaults to `u32` if unspecified. Tombstones are
   extracted from `ast::Tombstone.args`: look up `reason` key for
   `TombstoneDef.reason` (emit diagnostic if missing), look up `since` key for
   `TombstoneDef.since`. Spans are copied from the AST nodes into each IR def.

**Imports handling (pre-Milestone F):**
When the AST has imports, lowering registers stub `TypeId`s for imported names
(from named imports) or marks the namespace as wildcard-imported. Type resolution
against these stubs produces `Named(TypeId)` with no backing `TypeDef`. The type
checker skips validation on stub types. This is sufficient for single-file
schemas that reference imported types — the IR acknowledges them without resolving
their internals.

**Error handling:**
Lowering accumulates diagnostics but continues as far as possible. If a type
can't be resolved, it's replaced with a poison value. The caller checks for
error-severity diagnostics to decide whether the `CompiledSchema` is usable.

## Type Checker (typeck.rs)

Takes `&CompiledSchema`, returns `Vec<Diagnostic>`.

### Wire Size Computation

Compute `WireSize` for each `TypeDef`:

| Type | Wire Size |
|---|---|
| `bool` | Fixed(1 bit) |
| `u8`/`i8` | Fixed(8 bits) |
| `u16`/`i16` | Fixed(16 bits) |
| `u32`/`i32`/`f32` | Fixed(32 bits) |
| `u64`/`i64`/`f64` | Fixed(64 bits) |
| `void` | Fixed(0 bits) |
| Sub-byte `uN`/`iN` | Fixed(N bits) |
| `string`/`bytes` | Variable(min=0) |
| `rgb` | Fixed(24 bits) |
| `uuid` | Fixed(128 bits) |
| `timestamp` | Fixed(64 bits) |
| `hash` | Fixed(256 bits) |
| `optional<T>` | Variable(min=1 bit, max=if T.max is Some then Some(1+T.max) else None) |
| `array<T>` | Variable(min=LEB128 header) |
| `map<K,V>` | Variable(min=LEB128 header) |
| `result<T,E>` | Variable(min=1 bit + min(T.min_bits, E.min_bits), max=if both have max then Some(1+max(T.max,E.max)) else None) |
| Message | sum of field wire sizes (pure bitpack concatenation, no framing headers — framing is a transport concern) |
| Enum | Fixed(backing type size) |
| Flags | Fixed(64 bits) |
| Union | Variable (tag + largest variant) |
| Newtype | same as terminal type |
| Config | not wire-encoded (skip) |

Varint/ZigZag encoding makes fixed types become variable:
`@varint u32` → Variable(min=8 bits, max=40 bits).

### Recursive Type Detection

Walk the type graph from each `TypeId`, tracking visited IDs. A cycle is valid
if every path through it passes through an indirection point. Indirection points
are: `Optional`, `Array`, `Map`, `Result`, and **union dispatch** (because unions
are tag + variable-size payload — the tag selects a variant, so embedding a union
is not infinite recursion; a non-recursive variant like `Literal` terminates it).

A cycle through only direct message embedding (no indirection) is infinite —
emit diagnostic.

Algorithm: for each message type, DFS through field types. Maintain a "direct"
flag that starts true and becomes false when passing through any indirection
point (Optional/Array/Map/Result/Union). If we revisit a TypeId while `direct`
is true → error. When entering a union's variants, set `direct = false` for the
recursive walk of variant fields.

### Newtype Chain Resolution

Follow `NewtypeDef.inner_type`. If it's `Named(TypeId)` pointing to another
newtype, follow again. Store the terminal non-newtype type in
`NewtypeDef.terminal_type`. (Currently validate.rs rejects newtype-over-newtype,
so chains are always length 1. But the resolver is ready for when that rule
relaxes.)

### Encoding Compatibility (defensive)

Re-verify encoding matches resolved type. This is redundant with validate.rs
for single-file schemas but becomes necessary when Milestone F introduces
imported types whose definitions weren't available during AST validation.

## Testing Strategy

**Existing tests (unchanged):**
- 74 corpus tests via `parse()` — remain as-is, prove no regression.
- 15 lexer tests — unchanged.

**New tests:**

`crates/vexil-lang/tests/compile.rs`:

1. **Lowering round-trip tests** — For each valid corpus file, call `compile()`,
   assert `compiled.is_some()` and zero error diagnostics. Verify the
   `TypeRegistry` has the expected number of types.

2. **Type resolution tests** — Compile a schema with inter-type references
   (message field → enum, newtype → primitive). Assert `ResolvedType::Named`
   points to the correct `TypeId`. Verify `registry.get(id)` returns the
   expected `TypeDef` variant.

3. **Wire size tests** — Compile schemas with known layouts. Assert computed
   `WireSize` values match expected:
   - `message { a @0 : u32, b @1 : bool }` → Fixed(33 bits)
   - `message { s @0 : string }` → Variable
   - `message { v @0 : optional<u8> }` → Variable(min=1, max=9)

4. **Recursive type tests** — Valid recursion through optional (no error).
   Invalid direct recursion (error diagnostic).

5. **Newtype terminal resolution** — `newtype Foo : u64` → terminal is
   `Primitive(U64)`.

6. **Encoding computation tests** — Field with `@varint` on `u32` → encoding
   is `Varint`. Field with no annotations → `Default`. Field with `@limit(100)`
   on `array<u8>` → limit is `Some(100)`.

7. **Invalid corpus through compile()** — For each invalid corpus file, call
   `compile()`, assert `compiled.is_none()` (errors prevent IR creation).

8. **Import stub tests** — Compile a schema with imports, verify stub TypeIds
   are created in the registry, verify typeck skips stub types without error.

## Exit Criteria

1. `cargo test --workspace` — all tests pass (existing + new)
2. `cargo clippy --workspace --all-targets -- -D warnings` — clean
3. `cargo fmt --all -- --check` — clean
4. All 18 valid corpus files produce `CompiledSchema` with correct type counts
5. All 56 invalid corpus files produce error diagnostics (parse errors → `None` compiled; validate-only errors caught before lowering → `None`)
6. Wire size computation correct for primitive, composite, and variable types
7. Recursive type detection catches direct cycles, allows indirect cycles
8. `compile()` is a documented public API

## Dependencies

No new crate dependencies. Only `smol_str` (already present) is needed for
the IR string fields. `HashMap` from std for the registry.

## Future Milestones (not in scope)

- **Milestone D:** Codegen reads `CompiledSchema` + `WireSize` to emit Rust types
- **Milestone E:** Canonical serialization of `CompiledSchema` → BLAKE3 hash
- **Milestone F:** Multi-file: populate `TypeRegistry` from resolved imports
