# Language Gaps Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix three backwards-compatible language gaps: allow newtypes as map keys, preserve custom annotations through IR, and escape reserved variant names in Rust codegen.

**Architecture:** All changes are additive. Gap 1 touches the AST validator. Gap 2 adds IR types and modifies lowering. Gap 3 adds a name-escaping helper in Rust codegen. No wire format changes.

**Tech Stack:** Rust (vexil-lang core crate, vexil-codegen-rust crate). TDD with corpus files and golden tests.

**Spec reference:** `docs/superpowers/specs/2026-03-30-language-gaps-design.md`

---

## File Structure

```
crates/vexil-lang/src/
  ir/types.rs          — MODIFY: add CustomAnnotation types, extend ResolvedAnnotations
  ir/mod.rs            — MODIFY: re-export new types
  lower.rs             — MODIFY: preserve unknown annotations in catch-all
  validate.rs          — MODIFY: allow newtypes with valid terminal key types
  diagnostic.rs        — no changes (InvalidMapKey already exists)

crates/vexil-codegen-rust/src/
  enum_gen.rs          — MODIFY: add safe_variant_name helper, apply to all variant refs
  union_gen.rs         — MODIFY: apply safe_variant_name to variant name emission

corpus/
  valid/030_newtype_map_key.vexil           — NEW
  valid/031_custom_annotations.vexil        — NEW
  valid/032_reserved_variant_names.vexil    — NEW
  invalid/057_newtype_message_map_key.vexil — NEW

crates/vexil-lang/tests/corpus.rs              — MODIFY: add test entries
crates/vexil-codegen-rust/tests/golden.rs      — MODIFY: add golden test entries
crates/vexil-codegen-rust/tests/golden/032_reserved_variant_names.rs — NEW (generated)

corpus/MANIFEST.md — MODIFY: add new corpus entries
```

---

## Task 1: Newtype Map Keys — Failing Tests

**Files:**
- Create: `corpus/valid/030_newtype_map_key.vexil`
- Create: `corpus/invalid/057_newtype_message_map_key.vexil`
- Modify: `crates/vexil-lang/tests/corpus.rs`

- [ ] **Step 1: Create the valid corpus file**

Create `corpus/valid/030_newtype_map_key.vexil`:

```vexil
# §3.4: Newtypes wrapping valid map key types are valid map keys.
@version("1.0.0")
namespace test.newtype_map_key

newtype UserId : u32
newtype Label  : string

message UserProfile {
    id      @0 : UserId
    friends @1 : map<UserId, string>
    tags    @2 : map<Label, u32>
}
```

- [ ] **Step 2: Create the invalid corpus file**

Create `corpus/invalid/057_newtype_message_map_key.vexil`:

```vexil
# §3.4: Newtypes wrapping message types are NOT valid map keys.
# EXPECTED ERROR: invalid map key type
@version("1.0.0")
namespace test.newtype_msg_key

message Inner { x @0 : u8 }

newtype WrappedMsg : Inner

message Bad {
    lookup @0 : map<WrappedMsg, string>
}
```

- [ ] **Step 3: Add corpus test entries**

Add to `crates/vexil-lang/tests/corpus.rs` after the `valid_029` block (if it exists) or at the end of the valid tests:

```rust
#[test]
fn valid_030_newtype_map_key() {
    parse_valid("030_newtype_map_key.vexil");
}
```

Add to the invalid tests section:

```rust
#[test]
fn invalid_057_newtype_message_map_key() {
    parse_invalid(
        "057_newtype_message_map_key.vexil",
        ErrorClass::InvalidMapKey,
    );
}
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `cargo test -p vexil-lang valid_030 invalid_057`

Expected: `valid_030` FAILS (validator rejects newtype map key). `invalid_057` may pass or fail depending on whether the newtype-over-message is caught by `NewtypeOverNewtype` first or `InvalidMapKey` — either way confirms the test infrastructure works.

- [ ] **Step 5: Commit**

```bash
git add corpus/valid/030_newtype_map_key.vexil corpus/invalid/057_newtype_message_map_key.vexil crates/vexil-lang/tests/corpus.rs
git commit -m "test: add corpus files for newtype map key validation"
```

---

## Task 2: Newtype Map Keys — Implementation

**Files:**
- Modify: `crates/vexil-lang/src/validate.rs:454-485`

- [ ] **Step 1: Modify `check_map_key_type` to allow newtypes with valid terminal types**

In `crates/vexil-lang/src/validate.rs`, replace the `check_map_key_type` function (lines 454–485) with:

```rust
fn check_map_key_type(
    key: &Spanned<TypeExpr>,
    ctx: &ValidationContext<'_>,
    diags: &mut Vec<Diagnostic>,
) {
    if is_invalid_map_key(&key.node, ctx) {
        diags.push(Diagnostic::error(
            key.span,
            ErrorClass::InvalidMapKey,
            "invalid map key type",
        ));
    }
}

/// Returns true if the given type expression is NOT a valid map key.
///
/// Valid keys: integer primitives, bool, string, bytes, rgb, uuid, timestamp,
/// hash, enum, flags, and newtypes that transitively wrap a valid key type.
fn is_invalid_map_key(ty: &TypeExpr, ctx: &ValidationContext<'_>) -> bool {
    match ty {
        TypeExpr::Primitive(PrimitiveType::F32 | PrimitiveType::F64 | PrimitiveType::Void) => true,
        TypeExpr::Optional(_)
        | TypeExpr::Array(_)
        | TypeExpr::Map(_, _)
        | TypeExpr::Result(_, _) => true,
        TypeExpr::Named(name) => {
            if let Some((kind, _)) = ctx.decl_map.get(name) {
                matches!(
                    kind,
                    DeclKind::Message | DeclKind::Union | DeclKind::Config
                )
            } else {
                false
            }
        }
        _ => false,
    }
}
```

This removes `DeclKind::Newtype` from the invalid set. Since newtypes cannot wrap other newtypes (enforced by `check_newtype` → `NewtypeOverNewtype` error), a newtype's inner type is always a primitive, semantic, enum, flags, message, union, or config. Messages, unions, and configs are already rejected by their own `DeclKind` match arms. So removing `Newtype` from the invalid list is sufficient — no chain-following needed.

- [ ] **Step 2: Run the tests**

Run: `cargo test -p vexil-lang valid_030 invalid_057`

Expected: Both PASS. `valid_030` compiles cleanly. `invalid_057` is rejected because the newtype wraps a message — but note: it will actually be caught by `NewtypeOverNewtype`... wait, no — `WrappedMsg` wraps `Inner` which is a `Message`, not a `Newtype`. So `check_newtype` won't reject it. But `check_map_key_type` won't reject it either because we removed `Newtype` from the invalid list.

Actually — we need to check whether the newtype's *inner type* is a valid key. The current approach just removes `Newtype` from the invalid list, but a newtype wrapping a message should still be rejected. Let me fix the implementation.

Replace the `TypeExpr::Named` arm with:

```rust
        TypeExpr::Named(name) => {
            if let Some((kind, _)) = ctx.decl_map.get(name) {
                match kind {
                    DeclKind::Message | DeclKind::Union | DeclKind::Config => true,
                    DeclKind::Newtype => {
                        // Newtypes can't nest (NewtypeOverNewtype), so we just need
                        // to check the inner type. Find it from the AST declarations.
                        // If we can't resolve it (e.g. imported newtype), allow it —
                        // the type checker will catch actual errors later.
                        ctx.newtype_inner(name)
                            .map_or(false, |inner| is_invalid_map_key(inner, ctx))
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
```

- [ ] **Step 3: Add `newtype_inner` to `ValidationContext`**

This requires the validation context to know newtype inner types. Modify the `ValidationContext` struct and its construction in `validate_impl`.

In `crates/vexil-lang/src/validate.rs`, add a field to `ValidationContext`:

```rust
struct ValidationContext<'a> {
    decl_map: &'a HashMap<&'a SmolStr, (DeclKind, Span)>,
    imported_names: &'a HashSet<&'a SmolStr>,
    has_wildcard_import: bool,
    newtype_inners: &'a HashMap<&'a SmolStr, &'a TypeExpr>,
}
```

Add a helper method:

```rust
impl ValidationContext<'_> {
    fn is_known_type(&self, name: &SmolStr) -> bool {
        self.decl_map.contains_key(name)
            || self.imported_names.contains(name)
            || self.has_wildcard_import
    }

    /// Returns the inner type of a locally-declared newtype, if known.
    fn newtype_inner(&self, name: &SmolStr) -> Option<&TypeExpr> {
        self.newtype_inners.get(name).copied()
    }
}
```

In `validate_impl`, build the newtype inner map alongside `decl_map` (after line 85):

```rust
    let mut newtype_inners: HashMap<&SmolStr, &TypeExpr> = HashMap::new();
    for decl_spanned in &schema.declarations {
        if let Decl::Newtype(d) = &decl_spanned.node {
            newtype_inners.insert(&d.name.node, &d.inner_type.node);
        }
    }
```

And pass it to the context (at line 106):

```rust
    let ctx = ValidationContext {
        decl_map: &decl_map,
        imported_names: &imported_names,
        has_wildcard_import,
        newtype_inners: &newtype_inners,
    };
```

- [ ] **Step 4: Run all tests**

Run: `cargo test -p vexil-lang`

Expected: All tests pass including `valid_030` and `invalid_057`.

- [ ] **Step 5: Commit**

```bash
git add crates/vexil-lang/src/validate.rs
git commit -m "feat(vexil-lang): allow newtypes wrapping valid key types as map keys"
```

---

## Task 3: Newtype Map Keys — Golden Test

**Files:**
- Modify: `crates/vexil-codegen-rust/tests/golden.rs`

- [ ] **Step 1: Add Rust golden test**

Add to `crates/vexil-codegen-rust/tests/golden.rs`:

```rust
#[test]
fn test_030_newtype_map_key() {
    golden_test("030_newtype_map_key");
}
```

- [ ] **Step 2: Generate the golden file**

Run: `UPDATE_GOLDEN=1 cargo test -p vexil-codegen-rust test_030`

Expected: Creates `crates/vexil-codegen-rust/tests/golden/030_newtype_map_key.rs`.

- [ ] **Step 3: Verify the golden file looks correct**

Read the generated file. The map fields should use `HashMap<u32, String>` and `HashMap<String, u32>` (unwrapped newtype keys). The newtype structs themselves should be tuple structs.

- [ ] **Step 4: Run without UPDATE_GOLDEN to confirm**

Run: `cargo test -p vexil-codegen-rust test_030`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vexil-codegen-rust/tests/golden.rs crates/vexil-codegen-rust/tests/golden/030_newtype_map_key.rs
git commit -m "test(codegen-rust): add golden test for newtype map keys"
```

---

## Task 4: Custom Annotations — IR Types

**Files:**
- Modify: `crates/vexil-lang/src/ir/types.rs:204-212`
- Modify: `crates/vexil-lang/src/ir/mod.rs:11-14`

- [ ] **Step 1: Add CustomAnnotation types to IR**

In `crates/vexil-lang/src/ir/types.rs`, add before the `ResolvedAnnotations` struct (before line 203):

```rust
/// A user-defined annotation preserved from source through to IR.
///
/// Unknown annotations (not `doc`, `deprecated`, `since`, `revision`,
/// `non_exhaustive`, `version`, or encoding annotations) are collected
/// here so SDK consumers can access custom metadata without codegen
/// needing to know about them.
#[derive(Debug, Clone, PartialEq)]
pub struct CustomAnnotation {
    pub name: SmolStr,
    pub args: Vec<CustomAnnotationArg>,
}

/// A single argument to a custom annotation.
#[derive(Debug, Clone, PartialEq)]
pub struct CustomAnnotationArg {
    pub key: Option<SmolStr>,
    pub value: CustomAnnotationValue,
}

/// Value of a custom annotation argument.
#[derive(Debug, Clone, PartialEq)]
pub enum CustomAnnotationValue {
    Int(u64),
    Hex(u64),
    Str(SmolStr),
    Bool(bool),
    Ident(SmolStr),
}
```

- [ ] **Step 2: Extend ResolvedAnnotations**

In `crates/vexil-lang/src/ir/types.rs`, add the `custom` field to `ResolvedAnnotations`:

```rust
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResolvedAnnotations {
    pub deprecated: Option<DeprecatedInfo>,
    pub since: Option<SmolStr>,
    pub doc: Vec<SmolStr>,
    pub revision: Option<u64>,
    pub non_exhaustive: bool,
    pub version: Option<SmolStr>,
    pub custom: Vec<CustomAnnotation>,
}
```

- [ ] **Step 3: Re-export from ir/mod.rs**

In `crates/vexil-lang/src/ir/mod.rs`, add to the `pub use types::` list:

```rust
pub use types::{
    CustomAnnotation, CustomAnnotationArg, CustomAnnotationValue,
    DeprecatedInfo, Encoding, FieldEncoding, ResolvedAnnotations, ResolvedType, TombstoneDef,
    TypeId, TypeRegistry, WireSize,
};
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo build -p vexil-lang`

Expected: Compiles. The `Default` derive on `ResolvedAnnotations` still works because `Vec<CustomAnnotation>` defaults to empty.

- [ ] **Step 5: Commit**

```bash
git add crates/vexil-lang/src/ir/types.rs crates/vexil-lang/src/ir/mod.rs
git commit -m "feat(vexil-lang): add CustomAnnotation IR types for user-defined annotations"
```

---

## Task 5: Custom Annotations — Lowering

**Files:**
- Modify: `crates/vexil-lang/src/lower.rs:639-669`

- [ ] **Step 1: Add the `lower_annotation_args` helper**

In `crates/vexil-lang/src/lower.rs`, add after the `extract_first_int_arg` function (after line 703):

```rust
fn lower_custom_annotation(ann: &Annotation) -> ir::CustomAnnotation {
    ir::CustomAnnotation {
        name: ann.name.node.clone(),
        args: ann
            .args
            .as_ref()
            .map(|args| {
                args.iter()
                    .map(|arg| ir::CustomAnnotationArg {
                        key: arg.key.as_ref().map(|k| k.node.clone()),
                        value: match &arg.value.node {
                            AnnotationValue::Int(v) => ir::CustomAnnotationValue::Int(*v),
                            AnnotationValue::Hex(v) => ir::CustomAnnotationValue::Hex(*v),
                            AnnotationValue::Str(s) => {
                                ir::CustomAnnotationValue::Str(SmolStr::new(s))
                            }
                            AnnotationValue::Bool(b) => ir::CustomAnnotationValue::Bool(*b),
                            AnnotationValue::Ident(s) => {
                                ir::CustomAnnotationValue::Ident(s.clone())
                            }
                            AnnotationValue::UpperIdent(s) => {
                                ir::CustomAnnotationValue::Ident(s.clone())
                            }
                        },
                    })
                    .collect()
            })
            .unwrap_or_default(),
    }
}
```

- [ ] **Step 2: Modify the catch-all in `resolve_annotations_refs`**

In `crates/vexil-lang/src/lower.rs`, replace line 665 (`_ => {}`):

```rust
            _ => {
                result.custom.push(lower_custom_annotation(ann));
            }
```

- [ ] **Step 3: Run all tests**

Run: `cargo test -p vexil-lang`

Expected: All existing tests pass. Custom annotations were previously silently dropped; now they're preserved but nothing reads them yet.

- [ ] **Step 4: Commit**

```bash
git add crates/vexil-lang/src/lower.rs
git commit -m "feat(vexil-lang): preserve custom annotations through IR lowering"
```

---

## Task 6: Custom Annotations — Test

**Files:**
- Create: `corpus/valid/031_custom_annotations.vexil`
- Modify: `crates/vexil-lang/tests/corpus.rs`
- Modify: `crates/vexil-lang/tests/compile.rs` (or create a new test file)

- [ ] **Step 1: Create corpus file**

Create `corpus/valid/031_custom_annotations.vexil`:

```vexil
# User-defined annotations are preserved through compilation.
@version("1.0.0")
namespace test.custom_annotations

@priority("Critical")
@routing("broadcast")
message Alert {
    code    @0 : u32
    message @1 : string
}

@priority("Normal")
message Heartbeat {
    seq @0 : u64
}
```

- [ ] **Step 2: Add corpus parse test**

Add to `crates/vexil-lang/tests/corpus.rs`:

```rust
#[test]
fn valid_031_custom_annotations() {
    parse_valid("031_custom_annotations.vexil");
}
```

- [ ] **Step 3: Add compile test verifying custom annotations are preserved**

Add to `crates/vexil-lang/tests/compile.rs` (look for existing compile tests to match style):

```rust
#[test]
fn custom_annotations_preserved() {
    let source = r#"
@version("1.0.0")
namespace test.custom

@priority("Critical")
@routing("broadcast")
message Alert {
    code @0 : u32
}
"#;
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.expect("should compile");
    let alert_id = compiled.declarations[0];
    let alert = compiled.registry.get(alert_id);
    if let vexil_lang::TypeDef::Message(msg) = alert {
        assert_eq!(msg.annotations.custom.len(), 2);
        assert_eq!(msg.annotations.custom[0].name.as_str(), "priority");
        assert_eq!(msg.annotations.custom[1].name.as_str(), "routing");
    } else {
        panic!("expected Message");
    }
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p vexil-lang valid_031 custom_annotations_preserved`

Expected: Both PASS.

- [ ] **Step 5: Commit**

```bash
git add corpus/valid/031_custom_annotations.vexil crates/vexil-lang/tests/corpus.rs crates/vexil-lang/tests/compile.rs
git commit -m "test(vexil-lang): verify custom annotations are preserved through compilation"
```

---

## Task 7: Reserved Variant Names — Failing Test

**Files:**
- Create: `corpus/valid/032_reserved_variant_names.vexil`
- Modify: `crates/vexil-lang/tests/corpus.rs`
- Modify: `crates/vexil-codegen-rust/tests/golden.rs`

- [ ] **Step 1: Create corpus file**

Create `corpus/valid/032_reserved_variant_names.vexil`:

```vexil
# Enum variants with names that collide with Rust prelude types.
@version("1.0.0")
namespace test.reserved_variants

enum ImageProtocol {
    None    @0
    Sixel   @1
    Kitty   @2
}

enum ParseResult {
    Ok   @0
    Err  @1
    Some @2
    None @3
}
```

- [ ] **Step 2: Add corpus test**

Add to `crates/vexil-lang/tests/corpus.rs`:

```rust
#[test]
fn valid_032_reserved_variant_names() {
    parse_valid("032_reserved_variant_names.vexil");
}
```

- [ ] **Step 3: Add golden test entry**

Add to `crates/vexil-codegen-rust/tests/golden.rs`:

```rust
#[test]
fn test_032_reserved_variant_names() {
    golden_test("032_reserved_variant_names");
}
```

- [ ] **Step 4: Generate golden file (will have incorrect output before fix)**

Run: `UPDATE_GOLDEN=1 cargo test -p vexil-codegen-rust test_032`

This generates the golden file with un-escaped `None`, `Some`, etc. We'll update it after the fix.

- [ ] **Step 5: Commit**

```bash
git add corpus/valid/032_reserved_variant_names.vexil crates/vexil-lang/tests/corpus.rs crates/vexil-codegen-rust/tests/golden.rs crates/vexil-codegen-rust/tests/golden/032_reserved_variant_names.rs
git commit -m "test: add corpus and golden tests for reserved variant names"
```

---

## Task 8: Reserved Variant Names — Implementation

**Files:**
- Modify: `crates/vexil-codegen-rust/src/enum_gen.rs`
- Modify: `crates/vexil-codegen-rust/src/union_gen.rs`

- [ ] **Step 1: Add the `safe_variant_name` helper to `enum_gen.rs`**

In `crates/vexil-codegen-rust/src/enum_gen.rs`, add after the imports (after line 4):

```rust
/// Variant names that collide with Rust prelude types or keywords.
/// These require `r#` raw identifier syntax in generated code.
const RUST_RESERVED_VARIANTS: &[&str] = &[
    "None", "Some", // Option
    "Ok", "Err",    // Result
    "Self",         // keyword
];

/// Returns the variant name escaped with `r#` if it collides with a
/// Rust keyword or prelude type, otherwise returns it unchanged.
fn safe_variant_name(name: &str) -> std::borrow::Cow<'_, str> {
    if RUST_RESERVED_VARIANTS.contains(&name) {
        std::borrow::Cow::Owned(format!("r#{name}"))
    } else {
        std::borrow::Cow::Borrowed(name)
    }
}
```

- [ ] **Step 2: Apply `safe_variant_name` in `emit_enum`**

In `crates/vexil-codegen-rust/src/enum_gen.rs`, replace every `variant.name` interpolation with `safe_variant_name(&variant.name)`:

Line 54 (non-exhaustive variant declaration):
```rust
            w.line(&format!("{},", safe_variant_name(&variant.name)));
```

Line 56 (exhaustive variant with discriminant):
```rust
            w.line(&format!("{} = {ordinal}_u64,", safe_variant_name(&variant.name)));
```

Line 77 (Pack match arm):
```rust
        w.line(&format!("Self::{} => {ordinal}_u64,", safe_variant_name(&variant.name)));
```

Line 99 (Unpack match arm):
```rust
        w.line(&format!("{ordinal}_u64 => Ok(Self::{}),", safe_variant_name(&variant.name)));
```

- [ ] **Step 3: Apply `safe_variant_name` in `union_gen.rs`**

In `crates/vexil-codegen-rust/src/union_gen.rs`, add the same constant and helper at the top (after imports), or better — extract to a shared module. For now, since it's small, duplicate it.

Add after line 8:

```rust
const RUST_RESERVED_VARIANTS: &[&str] = &[
    "None", "Some",
    "Ok", "Err",
    "Self",
];

fn safe_variant_name(name: &str) -> std::borrow::Cow<'_, str> {
    if RUST_RESERVED_VARIANTS.contains(&name) {
        std::borrow::Cow::Owned(format!("r#{name}"))
    } else {
        std::borrow::Cow::Borrowed(name)
    }
}
```

Then replace all `variant.name` / `vname` interpolations in `emit_union` where it appears in generated Rust code:

Line 119 (tombstone prefix — this is a comment identifier, not Rust code, leave as-is).

Line 141 (empty variant declaration):
```rust
            w.line(&format!("{} {{}},", safe_variant_name(&variant.name)));
```

Line 143 (variant with fields):
```rust
            w.line(&format!("{} {{ {} }},", safe_variant_name(&variant.name), fields_str));
```

Line 161 — `vname` is used for generated Rust code. Change the binding:
```rust
        let vname = safe_variant_name(variant.name.as_str());
```

This flows through to lines 165, 176, 227, 249 where `vname` is interpolated into `Self::{vname}`.

- [ ] **Step 4: Regenerate the golden file**

Run: `UPDATE_GOLDEN=1 cargo test -p vexil-codegen-rust test_032`

Expected: Golden file now contains `r#None`, `r#Some`, `r#Ok`, `r#Err`.

- [ ] **Step 5: Verify all existing golden tests still pass**

Run: `cargo test -p vexil-codegen-rust`

Expected: All pass. Existing corpus files don't have reserved variant names, so their golden output is unchanged.

- [ ] **Step 6: Commit**

```bash
git add crates/vexil-codegen-rust/src/enum_gen.rs crates/vexil-codegen-rust/src/union_gen.rs crates/vexil-codegen-rust/tests/golden/032_reserved_variant_names.rs
git commit -m "feat(codegen-rust): escape reserved variant names with r# syntax"
```

---

## Task 9: Corpus Manifest + Full Test Suite

**Files:**
- Modify: `corpus/MANIFEST.md`

- [ ] **Step 1: Add entries to MANIFEST.md**

Add to the valid section of `corpus/MANIFEST.md`:

```markdown
| 030 | `030_newtype_map_key.vexil` | §3.4 | Newtypes wrapping valid key types as map keys |
| 031 | `031_custom_annotations.vexil` | — | User-defined annotations preserved through compilation |
| 032 | `032_reserved_variant_names.vexil` | — | Enum variants with Rust-reserved names |
```

Add to the invalid section:

```markdown
| 057 | `057_newtype_message_map_key.vexil` | §3.4 | Newtype wrapping message as map key |
```

- [ ] **Step 2: Run full workspace tests**

Run: `cargo test --workspace`

Expected: All tests pass.

- [ ] **Step 3: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`

Expected: Clean.

- [ ] **Step 4: Commit**

```bash
git add corpus/MANIFEST.md
git commit -m "docs: add new corpus entries to MANIFEST.md"
```
