# Language Gaps Design — Newtype Map Keys, Custom Annotations, Variant Name Safety

> Three backwards-compatible improvements to vexil-lang discovered during MALT/VNP schema design.
> Source: `VEXIL_GAPS.md` in the malt repo (Gaps 1, 2, 3).

---

## Overview

| Gap | Summary | Scope | Breaking? |
|-----|---------|-------|-----------|
| 1 | Newtype map keys | Type checker | No |
| 2 | Custom annotation pass-through | IR + lowering | No |
| 3 | Reserved variant names in Rust codegen | Rust codegen | No |

All changes are additive. No wire format changes. No spec revision required.

---

## Gap 1: Newtype Map Keys

### Problem

`map<PaneId, PersistedPane>` is rejected because `validate.rs:check_map_key_type()` unconditionally marks `DeclKind::Newtype` as an invalid key type. Newtypes wrapping valid key types (integers, string, uuid, enum, etc.) should be allowed — the wire encoding is identical to the inner type.

### Design

**File:** `crates/vexil-lang/src/validate.rs` — `check_map_key_type()` (lines 454–485)

Currently the function checks `DeclKind::Newtype` in the invalid set:

```rust
TypeExpr::Named(name) => {
    if let Some((kind, _)) = ctx.decl_map.get(name) {
        matches!(kind, DeclKind::Message | DeclKind::Union | DeclKind::Newtype | DeclKind::Config)
    } else {
        false
    }
}
```

**Change:** Remove `DeclKind::Newtype` from the invalid set. Instead, when a named type resolves to a newtype, follow the inner-type chain to the terminal type and validate *that*.

This validation runs on AST (`TypeExpr`) before lowering, so `terminal_type` from the IR is not available. The validation context's `decl_map` maps names to `(DeclKind, Span)`. To follow the chain, extend the validation context (or add a parallel map) to carry the newtype's inner `TypeExpr`. Chain-following is safe because circular newtypes are already rejected during parsing.

A newtype is a valid map key if and only if its terminal type is a valid map key type.

### Codegen impact

None. Newtypes already unwrap to their inner type on the wire in all backends.

### Test plan

- Corpus file: `corpus/valid/0XX_newtype_map_key.vexil` — message with `map<MyId, SomeMessage>` where `MyId` is a newtype over `u32`.
- Negative test: newtype wrapping a message is still rejected as a map key.
- Codegen golden tests for Rust, Go, TS showing the generated map type uses the unwrapped key.

---

## Gap 2: Custom Annotation Pass-through

### Problem

The parser already accepts arbitrary `@name(args)` annotations, but `lower.rs:resolve_annotations_refs()` silently drops unknown annotations (the `_ => {}` catch-all at line 665). Consumers like MALT want to attach protocol metadata (`@priority("Critical")`, `@routing("broadcast")`) and access it via the compiler API.

### Design

**New IR types** in `crates/vexil-lang/src/ir/types.rs`:

```rust
/// A user-defined annotation preserved from source through to IR.
#[derive(Debug, Clone, PartialEq)]
pub struct CustomAnnotation {
    pub name: SmolStr,
    pub args: Vec<CustomAnnotationArg>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CustomAnnotationArg {
    pub key: Option<SmolStr>,
    pub value: CustomAnnotationValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CustomAnnotationValue {
    Int(u64),
    Hex(u64),
    Str(SmolStr),
    Bool(bool),
    Ident(SmolStr),
}
```

Mirrors the AST `AnnotationValue` but in IR form (no `Span`s, `SmolStr` for strings). `UpperIdent` and `Ident` collapse into one `Ident` variant — casing distinction is irrelevant for metadata consumers.

**Extension to `ResolvedAnnotations`** in `crates/vexil-lang/src/ir/types.rs`:

```rust
pub struct ResolvedAnnotations {
    pub deprecated: Option<DeprecatedInfo>,
    pub since: Option<SmolStr>,
    pub doc: Vec<SmolStr>,
    pub revision: Option<u64>,
    pub non_exhaustive: bool,
    pub version: Option<SmolStr>,
    pub custom: Vec<CustomAnnotation>,  // NEW
}
```

**Lowering change** in `crates/vexil-lang/src/lower.rs` — `resolve_annotations_refs()`:

The `_ => {}` catch-all becomes:

```rust
_ => {
    result.custom.push(CustomAnnotation {
        name: ann.name.node.clone(),
        args: lower_annotation_args(&ann.args),
    });
}
```

Where `lower_annotation_args` converts `Option<Vec<ast::AnnotationArg>>` → `Vec<CustomAnnotationArg>`, mapping `ast::AnnotationValue` to `CustomAnnotationValue`.

**SDK exposure:** `CustomAnnotation` is `pub` in `vexil_lang::ir`, accessible via `type_def.annotations.custom` for any SDK consumer.

### What this does NOT do

- No validation of custom annotation names or values. A future "declared annotations" feature (approach B) may add this.
- No codegen output. Backends ignore `custom` annotations.
- No spec change. This is implementation-level.

### Test plan

- Unit test: parse a schema with `@priority("Critical")` on a message, compile, verify `annotations.custom` contains the expected `CustomAnnotation`.
- Verify existing schemas compile identically (custom field is empty by default).

---

## Gap 3: Reserved Variant Name Safety (Rust Codegen)

### Problem

Enum variants named `None`, `Some`, `Ok`, `Err`, or `Self` generate ambiguous Rust code. `Self::None` in a match arm could be interpreted as `Option::None` rather than the enum's `None` variant.

### Design

**Collision list** — Rust names that are valid Vexil identifiers (uppercase-initial) but shadow prelude types or are keywords:

```rust
const RUST_RESERVED_VARIANTS: &[&str] = &[
    "None", "Some",  // Option variants
    "Ok", "Err",     // Result variants
    "Self",          // keyword
];
```

Standard Rust keywords (`match`, `type`, `fn`, etc.) cannot appear as Vexil variant names because variants must start with an uppercase letter. The risk is specifically prelude names.

**Helper function** in `crates/vexil-codegen-rust/src/enum_gen.rs`:

```rust
fn safe_variant_name(name: &str) -> String {
    if RUST_RESERVED_VARIANTS.contains(&name) {
        format!("r#{name}")
    } else {
        name.to_string()
    }
}
```

**Application points:**
- `enum_gen.rs`: variant declarations (line 54, 56), Pack match arms (line 77), Unpack match arms (line 99)
- `union_gen.rs`: union variants use the same name emission pattern — apply the same helper

**Go and TS backends:** No changes needed. Go prefixes variant names with the type name (`DirectionNone`). TS uses string literals (`'None'`).

### Test plan

- New corpus file: `corpus/valid/0XX_reserved_variant_names.vexil` — enum with `None`, `Some`, `Ok`, `Err` variants.
- Rust golden test showing `r#None`, `r#Some`, etc. in generated output.
- Go and TS golden tests showing no change in their output.
- Compile-test the generated Rust code to verify `r#None` works.

---

## Future Work

- **Gap 2 extension (approach B):** Declared annotation types in the schema language (`annotation priority { ... }`) with typed values and compile-time validation. Deferred — requires spec RFC.
- **Gap 4 (stdlib types):** `vexil.std` with `Duration`, `IpAddr`, `SemVer`, etc. Deferred until the package manager (Milestone G) provides a distribution mechanism.
