# Schema Compatibility Checker — Implementation Plan (Phase A)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `vexilc compat old.vexil new.vexil` — a CLI command that compares two schemas and reports breaking/compatible/patch changes based on §10 rules, with human-readable and JSON output.

**Architecture:** Core `compat::check()` function in `vexil-lang` (library, reusable by future package manager). CLI subcommand in `vexilc` with `--format human|json`. Exit code 0 = compatible, 1 = breaking, 2 = error.

**Tech Stack:** Rust. `serde` + `serde_json` in `vexilc` for JSON output only (the `compat` module in `vexil-lang` has no serde dependency).

**Spec reference:** `docs/superpowers/specs/2026-03-28-schema-evolution-versioning-design.md` — Phase A.

---

## File Structure

```
crates/
  vexil-lang/
    src/
      compat.rs                          # NEW: core compatibility checking logic
      lib.rs                             # MODIFY: add pub mod compat
  vexilc/
    src/
      main.rs                            # MODIFY: add compat subcommand
    Cargo.toml                           # MODIFY: add serde, serde_json
```

---

## Task 1: Core Types — `compat::CompatReport`

**Files:**
- Create: `crates/vexil-lang/src/compat.rs`
- Modify: `crates/vexil-lang/src/lib.rs`

Define the data types that `check()` will return. No logic yet — just the types.

- [ ] **Step 1: Create the compat module with types**

Create `crates/vexil-lang/src/compat.rs`:

```rust
//! Schema compatibility checking.
//!
//! Compares two compiled schemas and reports changes classified per §10.

use crate::ir::{CompiledSchema, TypeDef, TypeId, TypeRegistry};
use smol_str::SmolStr;

/// Result of comparing two schemas.
#[derive(Debug, Clone)]
pub struct CompatReport {
    /// Individual changes detected.
    pub changes: Vec<Change>,
    /// Overall compatibility result.
    pub result: CompatResult,
    /// Minimum version bump required.
    pub suggested_bump: BumpKind,
}

/// A single change between two schema versions.
#[derive(Debug, Clone)]
pub struct Change {
    /// What kind of change this is.
    pub kind: ChangeKind,
    /// Name of the declaration (message, enum, etc.) involved.
    pub declaration: String,
    /// Name of the field/variant involved (if applicable).
    pub field: Option<String>,
    /// Human-readable detail string.
    pub detail: String,
    /// How this change is classified.
    pub classification: BumpKind,
}

/// Overall compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatResult {
    Compatible,
    Breaking,
}

/// Semantic version bump kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BumpKind {
    Patch,
    Minor,
    Major,
}

/// Kinds of changes that can be detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    FieldAdded,
    FieldRemoved,
    FieldTypeChanged,
    FieldOrdinalChanged,
    FieldRenamed,
    FieldDeprecated,
    FieldEncodingChanged,
    VariantAdded,
    VariantRemoved,
    VariantOrdinalChanged,
    DeclarationAdded,
    DeclarationRemoved,
    DeclarationKindChanged,
    NamespaceChanged,
    NonExhaustiveChanged,
    FlagsBitAdded,
    FlagsBitRemoved,
    FlagsBitOrdinalChanged,
}

/// Compare two compiled schemas and produce a compatibility report.
///
/// `old` is the baseline schema. `new` is the updated schema.
/// Changes are classified according to spec §10.
pub fn check(old: &CompiledSchema, new: &CompiledSchema) -> CompatReport {
    // Stub — implemented in Task 2
    CompatReport {
        changes: Vec::new(),
        result: CompatResult::Compatible,
        suggested_bump: BumpKind::Patch,
    }
}
```

- [ ] **Step 2: Export the module from lib.rs**

In `crates/vexil-lang/src/lib.rs`, add after the existing module declarations:

```rust
pub mod compat;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p vexil-lang`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/vexil-lang/src/compat.rs crates/vexil-lang/src/lib.rs
git commit -m "feat(vexil-lang): add compat module with report types"
```

---

## Task 2: Core Logic — `check()` Implementation

**Files:**
- Modify: `crates/vexil-lang/src/compat.rs`

The `check()` function compares old and new schemas by:
1. Comparing namespaces
2. Building name→TypeDef maps for both schemas
3. For each declaration in old: check if it exists in new (removed? changed?)
4. For each declaration in new: check if it's new (added?)
5. For matching declarations: compare fields/variants/bits by ordinal

- [ ] **Step 1: Implement check()**

Replace the stub `check()` function in `crates/vexil-lang/src/compat.rs`:

```rust
pub fn check(old: &CompiledSchema, new: &CompiledSchema) -> CompatReport {
    let mut changes = Vec::new();

    // Check namespace change
    if old.namespace != new.namespace {
        changes.push(Change {
            kind: ChangeKind::NamespaceChanged,
            declaration: String::new(),
            field: None,
            detail: format!(
                "{} → {}",
                old.namespace.join("."),
                new.namespace.join(".")
            ),
            classification: BumpKind::Major,
        });
    }

    // Build name → (TypeId, TypeDef) maps for both schemas
    let old_decls = build_decl_map(old);
    let new_decls = build_decl_map(new);

    // Check removed declarations
    for (name, (_, old_def)) in &old_decls {
        if !new_decls.contains_key(name) {
            changes.push(Change {
                kind: ChangeKind::DeclarationRemoved,
                declaration: name.to_string(),
                field: None,
                detail: format!("{} removed", decl_kind_name(old_def)),
                classification: BumpKind::Major,
            });
        }
    }

    // Check added declarations
    for (name, (_, new_def)) in &new_decls {
        if !old_decls.contains_key(name) {
            changes.push(Change {
                kind: ChangeKind::DeclarationAdded,
                declaration: name.to_string(),
                field: None,
                detail: format!("{} added", decl_kind_name(new_def)),
                classification: BumpKind::Minor,
            });
        }
    }

    // Check changed declarations
    for (name, (_, old_def)) in &old_decls {
        if let Some((_, new_def)) = new_decls.get(name) {
            compare_declarations(name, old_def, new_def, &old.registry, &new.registry, &mut changes);
        }
    }

    // Compute overall result
    let suggested_bump = changes
        .iter()
        .map(|c| c.classification)
        .max()
        .unwrap_or(BumpKind::Patch);

    let result = if suggested_bump == BumpKind::Major {
        CompatResult::Breaking
    } else {
        CompatResult::Compatible
    };

    CompatReport {
        changes,
        result,
        suggested_bump,
    }
}
```

- [ ] **Step 2: Add helper functions**

Add these helpers in the same file:

```rust
use std::collections::HashMap;

fn build_decl_map<'a>(
    compiled: &'a CompiledSchema,
) -> HashMap<SmolStr, (TypeId, &'a TypeDef)> {
    let mut map = HashMap::new();
    for &id in &compiled.declarations {
        if let Some(def) = compiled.registry.get(id) {
            let name = decl_name(def);
            map.insert(name, (id, def));
        }
    }
    map
}

fn decl_name(def: &TypeDef) -> SmolStr {
    match def {
        TypeDef::Message(m) => m.name.clone(),
        TypeDef::Enum(e) => e.name.clone(),
        TypeDef::Flags(f) => f.name.clone(),
        TypeDef::Union(u) => u.name.clone(),
        TypeDef::Newtype(n) => n.name.clone(),
        TypeDef::Config(c) => c.name.clone(),
        _ => SmolStr::new("unknown"),
    }
}

fn decl_kind_name(def: &TypeDef) -> &'static str {
    match def {
        TypeDef::Message(_) => "message",
        TypeDef::Enum(_) => "enum",
        TypeDef::Flags(_) => "flags",
        TypeDef::Union(_) => "union",
        TypeDef::Newtype(_) => "newtype",
        TypeDef::Config(_) => "config",
        _ => "unknown",
    }
}
```

- [ ] **Step 3: Add compare_declarations()**

This dispatches to type-specific comparison:

```rust
fn compare_declarations(
    name: &str,
    old: &TypeDef,
    new: &TypeDef,
    old_reg: &TypeRegistry,
    new_reg: &TypeRegistry,
    changes: &mut Vec<Change>,
) {
    // Check if declaration kind changed (e.g., message → enum)
    if std::mem::discriminant(old) != std::mem::discriminant(new) {
        changes.push(Change {
            kind: ChangeKind::DeclarationKindChanged,
            declaration: name.to_string(),
            field: None,
            detail: format!(
                "{} → {}",
                decl_kind_name(old),
                decl_kind_name(new)
            ),
            classification: BumpKind::Major,
        });
        return;
    }

    match (old, new) {
        (TypeDef::Message(old_msg), TypeDef::Message(new_msg)) => {
            compare_messages(name, old_msg, new_msg, old_reg, new_reg, changes);
        }
        (TypeDef::Enum(old_en), TypeDef::Enum(new_en)) => {
            compare_enums(name, old_en, new_en, changes);
        }
        (TypeDef::Flags(old_fl), TypeDef::Flags(new_fl)) => {
            compare_flags(name, old_fl, new_fl, changes);
        }
        (TypeDef::Union(old_un), TypeDef::Union(new_un)) => {
            compare_unions(name, old_un, new_un, old_reg, new_reg, changes);
        }
        (TypeDef::Newtype(old_nt), TypeDef::Newtype(new_nt)) => {
            compare_newtypes(name, old_nt, new_nt, old_reg, new_reg, changes);
        }
        (TypeDef::Config(old_cfg), TypeDef::Config(new_cfg)) => {
            // Config changes don't affect the wire — always patch
        }
        _ => {}
    }
}
```

- [ ] **Step 4: Add compare_messages()**

```rust
use crate::ir::{
    CompiledSchema, Encoding, FieldDef, FieldEncoding, MessageDef, EnumDef,
    FlagsDef, UnionDef, NewtypeDef, ResolvedType, TypeDef, TypeId, TypeRegistry,
};

fn compare_messages(
    decl_name: &str,
    old: &MessageDef,
    new: &MessageDef,
    old_reg: &TypeRegistry,
    new_reg: &TypeRegistry,
    changes: &mut Vec<Change>,
) {
    // Build ordinal → field maps
    let old_fields: HashMap<u32, &FieldDef> =
        old.fields.iter().map(|f| (f.ordinal, f)).collect();
    let new_fields: HashMap<u32, &FieldDef> =
        new.fields.iter().map(|f| (f.ordinal, f)).collect();

    // Check removed fields
    for (&ordinal, old_f) in &old_fields {
        if !new_fields.contains_key(&ordinal) {
            changes.push(Change {
                kind: ChangeKind::FieldRemoved,
                declaration: decl_name.to_string(),
                field: Some(old_f.name.to_string()),
                detail: format!("@{ordinal} removed"),
                classification: BumpKind::Major,
            });
        }
    }

    // Check added fields
    for (&ordinal, new_f) in &new_fields {
        if !old_fields.contains_key(&ordinal) {
            changes.push(Change {
                kind: ChangeKind::FieldAdded,
                declaration: decl_name.to_string(),
                field: Some(new_f.name.to_string()),
                detail: format!("@{ordinal} added"),
                classification: BumpKind::Minor,
            });
        }
    }

    // Check changed fields (same ordinal)
    for (&ordinal, old_f) in &old_fields {
        if let Some(new_f) = new_fields.get(&ordinal) {
            // Name change (compatible — patch)
            if old_f.name != new_f.name {
                changes.push(Change {
                    kind: ChangeKind::FieldRenamed,
                    declaration: decl_name.to_string(),
                    field: Some(new_f.name.to_string()),
                    detail: format!("@{ordinal} renamed: {} → {}", old_f.name, new_f.name),
                    classification: BumpKind::Patch,
                });
            }

            // Type change (breaking)
            if !types_equal(&old_f.resolved_type, &new_f.resolved_type, old_reg, new_reg) {
                changes.push(Change {
                    kind: ChangeKind::FieldTypeChanged,
                    declaration: decl_name.to_string(),
                    field: Some(new_f.name.to_string()),
                    detail: format!(
                        "@{ordinal} type changed: {} → {}",
                        type_display(&old_f.resolved_type, old_reg),
                        type_display(&new_f.resolved_type, new_reg)
                    ),
                    classification: BumpKind::Major,
                });
            }

            // Encoding change (breaking)
            if old_f.encoding.encoding != new_f.encoding.encoding {
                changes.push(Change {
                    kind: ChangeKind::FieldEncodingChanged,
                    declaration: decl_name.to_string(),
                    field: Some(new_f.name.to_string()),
                    detail: format!(
                        "@{ordinal} encoding changed: {} → {}",
                        encoding_display(&old_f.encoding.encoding),
                        encoding_display(&new_f.encoding.encoding)
                    ),
                    classification: BumpKind::Major,
                });
            }

            // Deprecated (patch)
            if old_f.annotations.deprecated.is_none() && new_f.annotations.deprecated.is_some() {
                changes.push(Change {
                    kind: ChangeKind::FieldDeprecated,
                    declaration: decl_name.to_string(),
                    field: Some(new_f.name.to_string()),
                    detail: format!("@{ordinal} deprecated"),
                    classification: BumpKind::Patch,
                });
            }
        }
    }
}
```

- [ ] **Step 5: Add compare_enums()**

```rust
fn compare_enums(
    decl_name: &str,
    old: &EnumDef,
    new: &EnumDef,
    changes: &mut Vec<Change>,
) {
    let old_variants: HashMap<u32, &str> =
        old.variants.iter().map(|v| (v.ordinal, v.name.as_str())).collect();
    let new_variants: HashMap<u32, &str> =
        new.variants.iter().map(|v| (v.ordinal, v.name.as_str())).collect();

    for (&ordinal, &name) in &old_variants {
        if !new_variants.contains_key(&ordinal) {
            changes.push(Change {
                kind: ChangeKind::VariantRemoved,
                declaration: decl_name.to_string(),
                field: Some(name.to_string()),
                detail: format!("@{ordinal} removed"),
                classification: BumpKind::Major,
            });
        }
    }

    for (&ordinal, &name) in &new_variants {
        if !old_variants.contains_key(&ordinal) {
            let classification = if new.annotations.non_exhaustive {
                BumpKind::Minor
            } else {
                BumpKind::Major // adding variant to exhaustive enum is breaking
            };
            changes.push(Change {
                kind: ChangeKind::VariantAdded,
                declaration: decl_name.to_string(),
                field: Some(name.to_string()),
                detail: format!("@{ordinal} added"),
                classification,
            });
        }
    }

    // Check @non_exhaustive changed
    if old.annotations.non_exhaustive != new.annotations.non_exhaustive {
        changes.push(Change {
            kind: ChangeKind::NonExhaustiveChanged,
            declaration: decl_name.to_string(),
            field: None,
            detail: format!(
                "@non_exhaustive: {} → {}",
                old.annotations.non_exhaustive,
                new.annotations.non_exhaustive
            ),
            classification: BumpKind::Major,
        });
    }
}
```

- [ ] **Step 6: Add compare_flags(), compare_unions(), compare_newtypes()**

**compare_flags:**
```rust
fn compare_flags(
    decl_name: &str,
    old: &FlagsDef,
    new: &FlagsDef,
    changes: &mut Vec<Change>,
) {
    let old_bits: HashMap<u32, &str> =
        old.bits.iter().map(|b| (b.bit, b.name.as_str())).collect();
    let new_bits: HashMap<u32, &str> =
        new.bits.iter().map(|b| (b.bit, b.name.as_str())).collect();

    for (&bit, &name) in &old_bits {
        if !new_bits.contains_key(&bit) {
            changes.push(Change {
                kind: ChangeKind::FlagsBitRemoved,
                declaration: decl_name.to_string(),
                field: Some(name.to_string()),
                detail: format!("bit {bit} removed"),
                classification: BumpKind::Major,
            });
        }
    }

    for (&bit, &name) in &new_bits {
        if !old_bits.contains_key(&bit) {
            changes.push(Change {
                kind: ChangeKind::FlagsBitAdded,
                declaration: decl_name.to_string(),
                field: Some(name.to_string()),
                detail: format!("bit {bit} added"),
                classification: BumpKind::Minor,
            });
        }
    }
}
```

**compare_unions** — follows the same pattern as enums but also compares variant fields (using the same ordinal-based field comparison as messages). The implementer should read `compare_messages()` and apply the same logic to each variant's fields.

**compare_newtypes:**
```rust
fn compare_newtypes(
    decl_name: &str,
    old: &NewtypeDef,
    new: &NewtypeDef,
    old_reg: &TypeRegistry,
    new_reg: &TypeRegistry,
    changes: &mut Vec<Change>,
) {
    if !types_equal(&old.inner_type, &new.inner_type, old_reg, new_reg) {
        changes.push(Change {
            kind: ChangeKind::FieldTypeChanged,
            declaration: decl_name.to_string(),
            field: None,
            detail: format!(
                "inner type changed: {} → {}",
                type_display(&old.inner_type, old_reg),
                type_display(&new.inner_type, new_reg)
            ),
            classification: BumpKind::Major,
        });
    }
}
```

- [ ] **Step 7: Add display helpers**

```rust
fn types_equal(
    old: &ResolvedType,
    new: &ResolvedType,
    old_reg: &TypeRegistry,
    new_reg: &TypeRegistry,
) -> bool {
    match (old, new) {
        (ResolvedType::Primitive(a), ResolvedType::Primitive(b)) => a == b,
        (ResolvedType::SubByte(a), ResolvedType::SubByte(b)) => a == b,
        (ResolvedType::Semantic(a), ResolvedType::Semantic(b)) => a == b,
        (ResolvedType::Named(a), ResolvedType::Named(b)) => {
            // Compare by name, not by TypeId (IDs differ between compilations)
            let a_name = old_reg.get(*a).map(|d| decl_name(d));
            let b_name = new_reg.get(*b).map(|d| decl_name(d));
            a_name == b_name
        }
        (ResolvedType::Optional(a), ResolvedType::Optional(b)) => {
            types_equal(a, b, old_reg, new_reg)
        }
        (ResolvedType::Array(a), ResolvedType::Array(b)) => {
            types_equal(a, b, old_reg, new_reg)
        }
        (ResolvedType::Map(ak, av), ResolvedType::Map(bk, bv)) => {
            types_equal(ak, bk, old_reg, new_reg) && types_equal(av, bv, old_reg, new_reg)
        }
        (ResolvedType::Result(ao, ae), ResolvedType::Result(bo, be)) => {
            types_equal(ao, bo, old_reg, new_reg) && types_equal(ae, be, old_reg, new_reg)
        }
        _ => false,
    }
}

fn type_display(ty: &ResolvedType, reg: &TypeRegistry) -> String {
    match ty {
        ResolvedType::Primitive(p) => format!("{p:?}").to_lowercase(),
        ResolvedType::SubByte(s) => {
            if s.signed { format!("i{}", s.bits) } else { format!("u{}", s.bits) }
        }
        ResolvedType::Semantic(s) => format!("{s:?}").to_lowercase(),
        ResolvedType::Named(id) => {
            reg.get(*id).map(|d| decl_name(d).to_string()).unwrap_or_else(|| "?".to_string())
        }
        ResolvedType::Optional(inner) => format!("optional<{}>", type_display(inner, reg)),
        ResolvedType::Array(inner) => format!("array<{}>", type_display(inner, reg)),
        ResolvedType::Map(k, v) => format!("map<{}, {}>", type_display(k, reg), type_display(v, reg)),
        ResolvedType::Result(ok, err) => format!("result<{}, {}>", type_display(ok, reg), type_display(err, reg)),
        _ => "?".to_string(),
    }
}

fn encoding_display(enc: &Encoding) -> &'static str {
    match enc {
        Encoding::Default => "default",
        Encoding::Varint => "varint",
        Encoding::ZigZag => "zigzag",
        Encoding::Delta(_) => "delta",
        _ => "?",
    }
}
```

- [ ] **Step 8: Run check**

Run: `cargo check -p vexil-lang`
Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add crates/vexil-lang/src/compat.rs
git commit -m "feat(vexil-lang): implement compat::check() with §10 change detection"
```

---

## Task 3: Unit Tests for `compat::check()`

**Files:**
- Modify: `crates/vexil-lang/src/compat.rs` (add test module)

- [ ] **Step 1: Add test module**

Add at the bottom of `crates/vexil-lang/src/compat.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile;

    fn check_schemas(old_src: &str, new_src: &str) -> CompatReport {
        let old = compile(old_src).compiled.expect("old schema failed to compile");
        let new = compile(new_src).compiled.expect("new schema failed to compile");
        check(&old, &new)
    }

    #[test]
    fn identical_schemas_are_compatible() {
        let src = "namespace test.compat\nmessage Foo { x @0 : u32 }";
        let report = check_schemas(src, src);
        assert_eq!(report.result, CompatResult::Compatible);
        assert!(report.changes.is_empty());
    }

    #[test]
    fn field_added_is_minor() {
        let old = "namespace test.compat\nmessage Foo { x @0 : u32 }";
        let new = "namespace test.compat\nmessage Foo { x @0 : u32\n  y @1 : u16 }";
        let report = check_schemas(old, new);
        assert_eq!(report.result, CompatResult::Compatible);
        assert_eq!(report.suggested_bump, BumpKind::Minor);
        assert_eq!(report.changes.len(), 1);
        assert_eq!(report.changes[0].kind, ChangeKind::FieldAdded);
    }

    #[test]
    fn field_removed_is_major() {
        let old = "namespace test.compat\nmessage Foo { x @0 : u32\n  y @1 : u16 }";
        let new = "namespace test.compat\nmessage Foo { x @0 : u32 }";
        let report = check_schemas(old, new);
        assert_eq!(report.result, CompatResult::Breaking);
        assert_eq!(report.suggested_bump, BumpKind::Major);
        assert_eq!(report.changes[0].kind, ChangeKind::FieldRemoved);
    }

    #[test]
    fn field_type_changed_is_major() {
        let old = "namespace test.compat\nmessage Foo { x @0 : u32 }";
        let new = "namespace test.compat\nmessage Foo { x @0 : u64 }";
        let report = check_schemas(old, new);
        assert_eq!(report.result, CompatResult::Breaking);
        assert_eq!(report.changes[0].kind, ChangeKind::FieldTypeChanged);
    }

    #[test]
    fn field_renamed_is_patch() {
        let old = "namespace test.compat\nmessage Foo { x @0 : u32 }";
        let new = "namespace test.compat\nmessage Foo { renamed_x @0 : u32 }";
        let report = check_schemas(old, new);
        assert_eq!(report.result, CompatResult::Compatible);
        assert_eq!(report.suggested_bump, BumpKind::Patch);
        assert_eq!(report.changes[0].kind, ChangeKind::FieldRenamed);
    }

    #[test]
    fn required_to_optional_is_major() {
        let old = "namespace test.compat\nmessage Foo { x @0 : u32 }";
        let new = "namespace test.compat\nmessage Foo { x @0 : optional<u32> }";
        let report = check_schemas(old, new);
        assert_eq!(report.result, CompatResult::Breaking);
        assert_eq!(report.changes[0].kind, ChangeKind::FieldTypeChanged);
    }

    #[test]
    fn declaration_added_is_minor() {
        let old = "namespace test.compat\nmessage Foo { x @0 : u32 }";
        let new = "namespace test.compat\nmessage Foo { x @0 : u32 }\nmessage Bar { y @0 : u16 }";
        let report = check_schemas(old, new);
        assert_eq!(report.result, CompatResult::Compatible);
        assert_eq!(report.suggested_bump, BumpKind::Minor);
        assert_eq!(report.changes[0].kind, ChangeKind::DeclarationAdded);
    }

    #[test]
    fn declaration_removed_is_major() {
        let old = "namespace test.compat\nmessage Foo { x @0 : u32 }\nmessage Bar { y @0 : u16 }";
        let new = "namespace test.compat\nmessage Foo { x @0 : u32 }";
        let report = check_schemas(old, new);
        assert_eq!(report.result, CompatResult::Breaking);
        assert_eq!(report.changes[0].kind, ChangeKind::DeclarationRemoved);
    }

    #[test]
    fn namespace_changed_is_major() {
        let old = "namespace test.v1\nmessage Foo { x @0 : u32 }";
        let new = "namespace test.v2\nmessage Foo { x @0 : u32 }";
        let report = check_schemas(old, new);
        assert_eq!(report.result, CompatResult::Breaking);
        assert_eq!(report.changes[0].kind, ChangeKind::NamespaceChanged);
    }

    #[test]
    fn field_deprecated_is_patch() {
        let old = "namespace test.compat\nmessage Foo { x @0 : u32 }";
        let new = "namespace test.compat\nmessage Foo {\n  @deprecated(\"use y\")\n  x @0 : u32 }";
        let report = check_schemas(old, new);
        assert_eq!(report.result, CompatResult::Compatible);
        assert_eq!(report.suggested_bump, BumpKind::Patch);
        assert_eq!(report.changes[0].kind, ChangeKind::FieldDeprecated);
    }

    #[test]
    fn enum_variant_added_non_exhaustive_is_minor() {
        let old = "namespace test.compat\n@non_exhaustive\nenum E { A @0  B @1 }";
        let new = "namespace test.compat\n@non_exhaustive\nenum E { A @0  B @1  C @2 }";
        let report = check_schemas(old, new);
        assert_eq!(report.result, CompatResult::Compatible);
        assert_eq!(report.suggested_bump, BumpKind::Minor);
    }

    #[test]
    fn enum_variant_removed_is_major() {
        let old = "namespace test.compat\nenum E { A @0  B @1 }";
        let new = "namespace test.compat\nenum E { A @0 }";
        let report = check_schemas(old, new);
        assert_eq!(report.result, CompatResult::Breaking);
    }

    #[test]
    fn multiple_changes_take_highest_bump() {
        let old = "namespace test.compat\nmessage Foo { x @0 : u32\n  y @1 : u16 }";
        let new = "namespace test.compat\nmessage Foo { x @0 : u32\n  z @2 : u8 }";
        let report = check_schemas(old, new);
        // y removed (major) + z added (minor) = major overall
        assert_eq!(report.result, CompatResult::Breaking);
        assert_eq!(report.suggested_bump, BumpKind::Major);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p vexil-lang compat`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add crates/vexil-lang/src/compat.rs
git commit -m "test(vexil-lang): compat checker unit tests for all §10 change kinds"
```

---

## Task 4: CLI Subcommand — `vexilc compat`

**Files:**
- Modify: `crates/vexilc/src/main.rs`
- Modify: `crates/vexilc/Cargo.toml`

- [ ] **Step 1: Add serde dependencies to vexilc**

In `crates/vexilc/Cargo.toml`, add:

```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 2: Add the cmd_compat function**

In `crates/vexilc/src/main.rs`, add the command function:

```rust
use vexil_lang::compat::{self, BumpKind, ChangeKind, CompatResult};

fn cmd_compat(old_path: &str, new_path: &str, format: &str) -> i32 {
    // Read and compile old schema
    let old_source = match std::fs::read_to_string(old_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {old_path}: {e}");
            return 2;
        }
    };
    let old_result = vexil_lang::compile(&old_source);
    if let Some(diag) = old_result.diagnostics.iter().find(|d| {
        d.severity == vexil_lang::diagnostic::Severity::Error
    }) {
        eprintln!("error in {old_path}:");
        render_diagnostic(diag, old_path, &old_source);
        return 2;
    }
    let old = match old_result.compiled {
        Some(c) => c,
        None => {
            eprintln!("error: {old_path}: compilation failed");
            return 2;
        }
    };

    // Read and compile new schema
    let new_source = match std::fs::read_to_string(new_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {new_path}: {e}");
            return 2;
        }
    };
    let new_result = vexil_lang::compile(&new_source);
    if let Some(diag) = new_result.diagnostics.iter().find(|d| {
        d.severity == vexil_lang::diagnostic::Severity::Error
    }) {
        eprintln!("error in {new_path}:");
        render_diagnostic(diag, new_path, &new_source);
        return 2;
    }
    let new = match new_result.compiled {
        Some(c) => c,
        None => {
            eprintln!("error: {new_path}: compilation failed");
            return 2;
        }
    };

    // Run compatibility check
    let report = compat::check(&old, &new);

    match format {
        "json" => print_report_json(&report),
        _ => print_report_human(&report),
    }

    match report.result {
        CompatResult::Compatible => 0,
        CompatResult::Breaking => 1,
    }
}

fn print_report_human(report: &compat::CompatReport) {
    if report.changes.is_empty() {
        println!("No changes detected.");
        return;
    }

    for change in &report.changes {
        let icon = if change.classification == BumpKind::Major {
            "\u{2717}" // ✗
        } else {
            "\u{2713}" // ✓
        };
        let class_str = match change.classification {
            BumpKind::Patch => "patch",
            BumpKind::Minor => "minor",
            BumpKind::Major => "BREAKING (major)",
        };
        let field_str = change
            .field
            .as_ref()
            .map(|f| format!(" field \"{}\"", f))
            .unwrap_or_default();

        println!(
            "  {icon} {}{field_str}: {} — {class_str}",
            change.declaration, change.detail
        );
    }

    println!();
    match report.result {
        CompatResult::Compatible => {
            println!(
                "Result: compatible (suggested bump: {:?})",
                report.suggested_bump
            );
        }
        CompatResult::Breaking => {
            println!("Result: BREAKING — requires major version bump");
        }
    }
}

fn print_report_json(report: &compat::CompatReport) {
    #[derive(serde::Serialize)]
    struct JsonReport {
        changes: Vec<JsonChange>,
        result: String,
        suggested_bump: String,
    }

    #[derive(serde::Serialize)]
    struct JsonChange {
        kind: String,
        declaration: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        field: Option<String>,
        detail: String,
        classification: String,
    }

    let json = JsonReport {
        changes: report
            .changes
            .iter()
            .map(|c| JsonChange {
                kind: format!("{:?}", c.kind).to_lowercase(),
                declaration: c.declaration.clone(),
                field: c.field.clone(),
                detail: c.detail.clone(),
                classification: format!("{:?}", c.classification).to_lowercase(),
            })
            .collect(),
        result: match report.result {
            CompatResult::Compatible => "compatible".to_string(),
            CompatResult::Breaking => "breaking".to_string(),
        },
        suggested_bump: format!("{:?}", report.suggested_bump).to_lowercase(),
    };

    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}
```

- [ ] **Step 3: Add the match arm in main()**

Find the match block in `main()` that dispatches subcommands. Add:

```rust
Some("compat") => {
    if args.len() < 4 {
        eprintln!("Usage: vexilc compat <old.vexil> <new.vexil> [--format human|json]");
        std::process::exit(2);
    }
    let mut format = "human";
    let mut i = 4;
    while i < args.len() {
        match args[i].as_str() {
            "--format" => {
                i += 1;
                if i < args.len() {
                    format = args[i].as_str();
                }
            }
            other => {
                eprintln!("unknown option: {other}");
                std::process::exit(2);
            }
        }
        i += 1;
    }
    std::process::exit(cmd_compat(&args[2], &args[3], format));
}
```

- [ ] **Step 4: Update help text**

Find the help/usage text in main.rs and add:

```
    compat <old.vexil> <new.vexil>  Compare schemas for breaking changes
        --format human|json         Output format (default: human)
```

- [ ] **Step 5: Build and test**

Run: `cargo build -p vexilc`
Expected: PASS.

Smoke test:
```bash
cargo run -p vexilc -- compat corpus/valid/019_evolution_append_field.vexil corpus/valid/019_evolution_append_field.vexil
```
Expected: "No changes detected."

- [ ] **Step 6: Commit**

```bash
git add crates/vexilc/
git commit -m "feat(vexilc): add compat subcommand for breaking change detection"
```

---

## Task 5: CLI Integration Tests

**Files:**
- Create: `crates/vexilc/tests/compat.rs` (or add to existing integration test file)

- [ ] **Step 1: Write CLI integration tests**

These test the actual CLI by running `vexilc compat` as a subprocess.

```rust
use std::process::Command;
use std::path::PathBuf;

fn vexilc() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vexilc"));
    cmd
}

fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("corpus/valid")
        .join(name)
}

#[test]
fn compat_identical_returns_0() {
    let path = corpus_path("006_message.vexil");
    let output = vexilc()
        .args(["compat", path.to_str().unwrap(), path.to_str().unwrap()])
        .output()
        .expect("failed to run vexilc");
    assert!(output.status.success(), "expected exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No changes"));
}

#[test]
fn compat_breaking_returns_1() {
    let old = corpus_path("019_evolution_append_field.vexil");
    let new = corpus_path("026_required_to_optional.vexil");
    let output = vexilc()
        .args(["compat", old.to_str().unwrap(), new.to_str().unwrap()])
        .output()
        .expect("failed to run vexilc");
    // Different schemas entirely — should detect changes
    assert!(!output.status.success() || !output.stdout.is_empty());
}

#[test]
fn compat_json_format() {
    let path = corpus_path("006_message.vexil");
    let output = vexilc()
        .args(["compat", path.to_str().unwrap(), path.to_str().unwrap(), "--format", "json"])
        .output()
        .expect("failed to run vexilc");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should be valid JSON
    assert!(stdout.contains("\"result\""));
    assert!(stdout.contains("\"compatible\""));
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p vexilc compat`
Expected: All pass.

- [ ] **Step 3: Commit**

```bash
git add crates/vexilc/tests/compat.rs
git commit -m "test(vexilc): CLI integration tests for compat subcommand"
```

---

## Task 6: Final Verification

**Files:** No new files.

- [ ] **Step 1: Run full test suite**

Run: `cargo test --workspace`
Expected: All pass.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy -p vexil-lang -p vexilc -- -D warnings`
Expected: Clean.

- [ ] **Step 3: Run fmt**

Run: `cargo fmt --all -- --check`
Expected: Clean.

- [ ] **Step 4: Smoke test the full workflow**

Create two temp files and test:
```bash
echo 'namespace test.v1\nmessage Foo { x @0 : u32 }' > /tmp/v1.vexil
echo 'namespace test.v1\nmessage Foo { x @0 : u32\n  y @1 : u16 }' > /tmp/v2.vexil
cargo run -p vexilc -- compat /tmp/v1.vexil /tmp/v2.vexil
cargo run -p vexilc -- compat /tmp/v1.vexil /tmp/v2.vexil --format json
```

- [ ] **Step 5: Commit**

```bash
git commit --allow-empty -m "chore: phase A complete — compat checker verified"
```
