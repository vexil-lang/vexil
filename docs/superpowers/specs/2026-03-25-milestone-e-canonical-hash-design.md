# Milestone E: Canonical Form + BLAKE3 Schema Hash — Design Spec

> **For agentic workers:** Use superpowers:subagent-driven-development or superpowers:executing-plans to implement from the corresponding plan.

## Goal

Implement spec §7 (Canonical Form) and §8 (Schema Hash) for single-file schemas. Given a `CompiledSchema`, produce a deterministic UTF-8 canonical form string and its BLAKE3 hash. Wire the hash into codegen as `pub const SCHEMA_HASH: [u8; 32]`.

## Scope

- **In scope:** Single-file canonical form, BLAKE3 hash, codegen integration, `vexilc check` hash output.
- **Out of scope:** Multi-file transitive closure (§7.6) — deferred to Milestone F when import resolution lands. The algorithm has a clear extension point for this.

---

## Architecture

### New module: `vexil-lang::canonical`

Two public functions:

```rust
/// Compute the canonical form of a single-file schema per spec §7.
/// Returns a deterministic UTF-8 string.
pub fn canonical_form(compiled: &CompiledSchema) -> String

/// Compute the BLAKE3 hash of the canonical form.
pub fn schema_hash(compiled: &CompiledSchema) -> [u8; 32]
```

`schema_hash` is trivially:
```rust
pub fn schema_hash(compiled: &CompiledSchema) -> [u8; 32] {
    let form = canonical_form(compiled);
    *blake3::hash(form.as_bytes()).as_bytes()
}
```

### Dependency

`blake3 = "1"` added to `crates/vexil-lang/Cargo.toml` dependencies. Default features (std + SIMD detection).

### Codegen integration

`vexil-codegen::generate()` calls `vexil_lang::canonical::schema_hash()` and emits:

```rust
pub const SCHEMA_HASH: [u8; 32] = [0xab, 0xcd, /* ... 32 bytes ... */];
pub const SCHEMA_VERSION: &str = "1.2.0";
```

`SCHEMA_HASH` is always emitted (every schema has a hash). `SCHEMA_VERSION` follows if the schema declares a version. This matches the spec §8.2 example ordering.

---

## Canonical Form Algorithm

Walks the `CompiledSchema` IR and emits a normalized text representation. The spec (§7.1) says canonical form is computed "over the resolved schema graph, not the raw source text."

### Output format

The canonical form is a **single space-delimited string** with no newlines. This follows directly from §7.1 step 2: "Normalize all whitespace sequences to a single space character. Strip leading and trailing whitespace." Since the canonical form is the output of this normalization, all internal whitespace (including what would be line breaks) is a single space.

The segments below are described on separate lines for readability, but in the actual output they are concatenated with single space separators:

```
namespace {ns.path.here} {schema_annotations} {declaration_1} {declaration_2} ...
```

No comments. No newlines. Single spaces as separators throughout.

### Segment rules

#### 1. Namespace

```
namespace {path}
```

Where `{path}` is `compiled.namespace` joined with `.`.

#### 2. Schema-level annotations

Immediately after the namespace, emit `compiled.annotations` using the annotation format defined in §3 below. These are the schema-level annotations (e.g., `@version("1.0.0")`). If no schema-level annotations exist, nothing is emitted.

#### 3. Declarations (source order)

Iterate `compiled.declarations` in order (source order is preserved in the IR). For each `TypeDef`:

**Message:**
```
{annotations} message {Name} { {fields} {tombstones} }
```

**Enum:**
```
{annotations} enum {Name} [: {backing}] { {variants} {tombstones} }
```

Where `{backing}` is emitted only if `EnumDef.backing` is `Some(...)`. Backing type strings: `u8`, `u16`, `u32`, `u64` (lowercase, matching primitive type names).

**Flags:**
```
{annotations} flags {Name} { {bits} {tombstones} }
```

**Union:**
```
{annotations} union {Name} { {variants} {tombstones} }
```

Each union variant is:
```
{variant_annotations} {Name} @{ordinal} { {fields} {variant_tombstones} }
```

Union variants are written in ascending ordinal order. Variant-level tombstones (from `UnionVariantDef.tombstones`) are emitted after variant fields, within the variant's `{ ... }` block.

**Newtype:**
```
{annotations} newtype {Name} = {inner_type}
```

**Config:**
```
{annotations} config {Name} { {fields} }
```

Config fields are written in ascending **lexicographic order by name** (config fields have no ordinals).

#### 4. Annotations (lexicographic by name)

Annotations are emitted from `ResolvedAnnotations` in sorted order by annotation name. The canonical names and their formats:

| Annotation | Condition | Output |
|---|---|---|
| `@deprecated` | `deprecated.is_some()` | `@deprecated(reason: "{reason}")` or `@deprecated(reason: "{reason}", since: "{since}")` |
| `@doc` | `doc.is_empty() == false` | `@doc("{text}")` — multiple instances in source order |
| `@non_exhaustive` | `non_exhaustive == true` | `@non_exhaustive` |
| `@revision` | `revision.is_some()` | `@revision({n})` |
| `@since` | `since.is_some()` | `@since("{version}")` |
| `@version` | `version.is_some()` | `@version("{version}")` |

Sorted lexicographically: `deprecated`, `doc`, `non_exhaustive`, `revision`, `since`, `version`.

`@deprecated` always uses **named argument form** for consistency: `@deprecated(reason: "...")` even when `since` is absent.

Per spec §7.1: `@doc` is the only annotation permitting repetition. Multiple `@doc` instances are written in source order within the sorted annotation block.

Annotations are emitted as a space-separated prefix before the declaration keyword, e.g.:
```
@deprecated(reason: "use Foo2") @doc("A message") @since("1.0") message Foo { ... }
```

Field-level annotations use the same format, prefixed to the field within the body.

#### 5. Fields (ascending ordinal order)

For message and union variant fields:
```
{field_annotations} {name} @{ordinal} : {type} {encoding}
```

Where:
- `{type}` is the canonical type string (see below)
- `{encoding}` is the encoding suffix chain (see below)

Encoding suffix rules (in order):

| `Encoding` variant | Canonical suffix |
|---|---|
| `Default` | *(empty — no suffix)* |
| `Varint` | `@varint` |
| `ZigZag` | `@zigzag` |
| `Delta(Default)` | `@delta` |
| `Delta(Varint)` | `@delta @varint` |
| `Delta(ZigZag)` | `@delta @zigzag` |

For any other `Encoding` variant (including unknown `#[non_exhaustive]` arms): omit the encoding suffix and emit a `// WARNING: unknown encoding` comment to stderr during compilation (not in the canonical form itself).

After encoding suffixes, if `FieldEncoding.limit` is `Some(n)`, append `@limit({n})`. The `@limit` always comes last:
```
{name} @{ordinal} : {type} @delta @varint @limit(256)
```

Fields are separated by spaces within `{ ... }`.

#### 6. Type strings

| ResolvedType | Canonical string |
|---|---|
| `Primitive(Bool)` | `bool` |
| `Primitive(U8)` | `u8` |
| `Primitive(U16)` | `u16` |
| `Primitive(U32)` | `u32` |
| `Primitive(U64)` | `u64` |
| `Primitive(I8)` | `i8` |
| `Primitive(I16)` | `i16` |
| `Primitive(I32)` | `i32` |
| `Primitive(I64)` | `i64` |
| `Primitive(F32)` | `f32` |
| `Primitive(F64)` | `f64` |
| `Primitive(Void)` | `void` |
| `SubByte { bits: N, signed: false }` | `u{N}` |
| `SubByte { bits: N, signed: true }` | `i{N}` |
| `Semantic(String)` | `string` |
| `Semantic(Bytes)` | `bytes` |
| `Semantic(Rgb)` | `rgb` |
| `Semantic(Uuid)` | `uuid` |
| `Semantic(Timestamp)` | `timestamp` |
| `Semantic(Hash)` | `hash` |
| `Named(id)` | The type's name from the registry (see error handling below) |
| `Optional(inner)` | `optional<{inner}>` |
| `Array(inner)` | `array<{inner}>` |
| `Map(k, v)` | `map<{k}, {v}>` |
| `Result(ok, err)` | `result<{ok}, {err}>` |

**Error handling for `Named(id)`:** If the registry returns `None` for the id (including `POISON_TYPE_ID`), this is a compiler bug — the canonical form function should panic with `debug_assert!` in debug builds and emit `"<unresolved>"` in release builds. In practice this should never occur because `canonical_form` is called on validated IR.

#### 7. Enum variants (ascending ordinal order)

```
{variant_annotations} {Name} = {ordinal}
```

#### 8. Flags bits (ascending bit index)

```
{bit_annotations} {Name} = {bit}
```

#### 9. Tombstones (ascending ordinal order)

```
@removed({ordinal}, "{reason}")
```

Or with since: `@removed({ordinal}, "{reason}", since: "{since}")`

#### 10. Config fields (ascending lexicographic order by name)

```
{field_annotations} {name} : {type} = {default_value}
```

Where `{default_value}` uses the canonical representation:
- `Bool(true)` → `true`, `Bool(false)` → `false`
- `Int(n)` → `{n}` (decimal)
- `UInt(n)` → `{n}` (decimal)
- `Float(f)` → formatted with `format!("{:?}", f)` (Rust Debug format, which uses enough precision to round-trip: e.g., `1.0`, `3.14`, `1e-10`)
- `Str(s)` → `"{s}"`
- `None` → `none`
- `Ident(s)` / `UpperIdent(s)` → `{s}`
- `Array(items)` → `[{item1}, {item2}]` (recursive)

The `{:?}` Debug format for f64 is used because it round-trips all values faithfully and is stable across Rust versions (it uses Grisu3/Dragon4 which always produces the shortest representation that round-trips).

---

## Codegen Changes

In `vexil-codegen::generate()`, emit `SCHEMA_HASH` before `SCHEMA_VERSION`, matching the spec §8.2 example:

```rust
// Always emit SCHEMA_HASH
let hash = vexil_lang::canonical::schema_hash(compiled);
let hash_str = hash.iter()
    .map(|b| format!("0x{b:02x}"))
    .collect::<Vec<_>>()
    .join(", ");
w.line(&format!("pub const SCHEMA_HASH: [u8; 32] = [{hash_str}];"));

// Then SCHEMA_VERSION if present
if let Some(ref version) = compiled.annotations.version {
    w.line(&format!("pub const SCHEMA_VERSION: &str = \"{version}\";"));
}
```

---

## CLI Integration

No new subcommand. Extend `vexilc check` to print the schema hash when compilation succeeds:

```
$ vexilc check corpus/valid/006_message.vexil
schema hash: abcdef0123456789...  (64 hex chars)
```

This helps developers verify hash values. The `codegen` subcommand already emits the hash via the generated `SCHEMA_HASH` constant.

---

## Testing Strategy

### 1. Unit tests in `canonical` module

- **Determinism:** Compile the same source twice → identical canonical form.
- **Annotation sorting:** Schema with `@since` before `@doc` in source → canonical form has `@doc` before `@since`.
- **Field ordering:** Schema with fields in non-ordinal order → canonical form has fields in ascending ordinal order.
- **Whitespace invariance:** Two schemas identical except for whitespace/comments → identical canonical form.
- **Each type kind:** One test per declaration kind (message, enum, flags, union, newtype, config) verifying the exact canonical form output.
- **Encoding annotations:** Fields with `@varint`, `@zigzag`, `@delta`, `@limit` → correct canonical encoding suffix.
- **Schema-level annotations:** Schema with `@version` → appears in canonical form after namespace.
- **Union variant tombstones:** Union with variant-level tombstones → tombstones appear within variant body.
- **Config field ordering:** Config with fields in non-alphabetical source order → canonical form sorts by name.

### 2. Hash stability tests

For each corpus file (006–016), compute the hash and assert the exact `[u8; 32]`. Hash values are pinned after the first green test run — if the canonical form algorithm changes, these tests break intentionally.

### 3. Golden file update

Re-generate golden files for `vexil-codegen` tests since they now include `SCHEMA_HASH`. Run `UPDATE_GOLDEN=1 cargo test -p vexil-codegen --test golden`.

### 4. Round-trip property test

Write the same schema with different comment/whitespace patterns → verify all produce identical hashes.

---

## File Changes Summary

| File | Change |
|---|---|
| `crates/vexil-lang/Cargo.toml` | Add `blake3 = "1"` |
| `crates/vexil-lang/src/canonical.rs` | New: `canonical_form()`, `schema_hash()`, helpers, tests |
| `crates/vexil-lang/src/lib.rs` | Add `pub mod canonical;` |
| `crates/vexil-codegen/src/lib.rs` | Emit `SCHEMA_HASH` constant in `generate()`, reorder to HASH then VERSION |
| `crates/vexil-codegen/tests/golden/*.rs` | Updated golden files (now include SCHEMA_HASH) |
| `crates/vexilc/src/main.rs` | Print schema hash in `check` output |

---

## Future Extension (Milestone F)

When multi-file imports land, `canonical_form()` will accept an additional parameter (the resolved import graph) and prepend dependency canonical forms using Kahn's algorithm with lexicographic tie-breaking, per §7.6. The single-file output format doesn't change — it just gets prefixed with dependency content.
