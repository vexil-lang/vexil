//! # Stability: Tier 2
//!
//! Schema compatibility checker for detecting breaking and compatible changes
//! between two compiled Vexil schemas (spec section 10).
//!
//! Given an "old" and "new" [`CompiledSchema`], `check()` produces a
//! `CompatReport` listing every change, its classification, and the overall
//! suggested version bump.

use crate::ir::{
    CompiledSchema, Encoding, EnumDef, FieldDef, FlagsDef, MessageDef, NewtypeDef, ResolvedType,
    TypeDef, TypeId, TypeRegistry, UnionDef,
};
use smol_str::SmolStr;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// The overall compatibility verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatResult {
    /// All changes are backward-compatible (or there are none).
    Compatible,
    /// At least one change breaks wire compatibility.
    Breaking,
}

/// Suggested semantic-version bump kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BumpKind {
    /// Bug fix / cosmetic change only.
    Patch,
    /// New features, backward-compatible additions.
    Minor,
    /// Breaking wire-format changes.
    Major,
}

/// Classification of an individual schema change.
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

/// A single detected change between two schema versions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// What kind of change this is.
    pub kind: ChangeKind,
    /// The declaration name this change applies to (or `""` for schema-level).
    pub declaration: String,
    /// The field/variant/bit name, when applicable.
    pub field: Option<String>,
    /// Human-readable description of the change.
    pub detail: String,
    /// How this change affects version compatibility.
    pub classification: BumpKind,
}

/// Full compatibility report comparing two schema versions.
#[derive(Debug, Clone)]
pub struct CompatReport {
    /// Every individual change detected.
    pub changes: Vec<Change>,
    /// Overall compatibility verdict.
    pub result: CompatResult,
    /// The highest bump kind across all changes.
    pub suggested_bump: BumpKind,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compare two compiled schemas and produce a compatibility report.
///
/// The `old` schema is the baseline; `new` is the proposed revision.
/// The report lists every detected change and its classification per spec
/// section 10 rules.
pub fn check(old: &CompiledSchema, new: &CompiledSchema) -> CompatReport {
    let mut changes = Vec::new();

    // 1. Namespace change
    if old.namespace != new.namespace {
        changes.push(Change {
            kind: ChangeKind::NamespaceChanged,
            declaration: String::new(),
            field: None,
            detail: format!(
                "namespace changed from '{}' to '{}'",
                old.namespace.join("."),
                new.namespace.join(".")
            ),
            classification: BumpKind::Major,
        });
    }

    // 2. Build name -> TypeDef maps
    let old_map = build_decl_map(old);
    let new_map = build_decl_map(new);

    // 3. Removed declarations
    for (name, (_id, def)) in &old_map {
        if !new_map.contains_key(name) {
            changes.push(Change {
                kind: ChangeKind::DeclarationRemoved,
                declaration: name.to_string(),
                field: None,
                detail: format!("{} '{}' was removed", decl_kind_name(def), name),
                classification: BumpKind::Major,
            });
        }
    }

    // 4. Added declarations
    for (name, (_id, def)) in &new_map {
        if !old_map.contains_key(name) {
            changes.push(Change {
                kind: ChangeKind::DeclarationAdded,
                declaration: name.to_string(),
                field: None,
                detail: format!("{} '{}' was added", decl_kind_name(def), name),
                classification: BumpKind::Minor,
            });
        }
    }

    // 5. Matching declarations — compare in detail
    for (name, (_old_id, old_def)) in &old_map {
        if let Some((_new_id, new_def)) = new_map.get(name) {
            compare_decls(
                name,
                old_def,
                new_def,
                &old.registry,
                &new.registry,
                &mut changes,
            );
        }
    }

    // Compute overall result
    let suggested_bump = changes
        .iter()
        .map(|c| c.classification)
        .max()
        .unwrap_or(BumpKind::Patch);

    let result = if suggested_bump >= BumpKind::Major {
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_decl_map(compiled: &CompiledSchema) -> HashMap<SmolStr, (TypeId, &TypeDef)> {
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
        TypeDef::Message(d) => d.name.clone(),
        TypeDef::Enum(d) => d.name.clone(),
        TypeDef::Flags(d) => d.name.clone(),
        TypeDef::Union(d) => d.name.clone(),
        TypeDef::Newtype(d) => d.name.clone(),
        TypeDef::Config(d) => d.name.clone(),
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
    }
}

/// Recursively compare two resolved types by structure, resolving Named types
/// by their name in their respective registries rather than by TypeId.
fn types_equal(
    old_ty: &ResolvedType,
    new_ty: &ResolvedType,
    old_reg: &TypeRegistry,
    new_reg: &TypeRegistry,
) -> bool {
    match (old_ty, new_ty) {
        (ResolvedType::Primitive(a), ResolvedType::Primitive(b)) => a == b,
        (ResolvedType::SubByte(a), ResolvedType::SubByte(b)) => a == b,
        (ResolvedType::Semantic(a), ResolvedType::Semantic(b)) => a == b,
        (ResolvedType::Named(old_id), ResolvedType::Named(new_id)) => {
            // Compare by name, not by TypeId
            let old_name = old_reg.get(*old_id).map(decl_name);
            let new_name = new_reg.get(*new_id).map(decl_name);
            old_name == new_name
        }
        (ResolvedType::Optional(a), ResolvedType::Optional(b)) => {
            types_equal(a, b, old_reg, new_reg)
        }
        (ResolvedType::Array(a), ResolvedType::Array(b)) => types_equal(a, b, old_reg, new_reg),
        (ResolvedType::Map(ak, av), ResolvedType::Map(bk, bv)) => {
            types_equal(ak, bk, old_reg, new_reg) && types_equal(av, bv, old_reg, new_reg)
        }
        (ResolvedType::Result(ao, ae), ResolvedType::Result(bo, be)) => {
            types_equal(ao, bo, old_reg, new_reg) && types_equal(ae, be, old_reg, new_reg)
        }
        _ => false,
    }
}

/// Human-readable type display.
fn type_display(ty: &ResolvedType, reg: &TypeRegistry) -> String {
    match ty {
        ResolvedType::Primitive(p) => format!("{:?}", p).to_lowercase(),
        ResolvedType::SubByte(s) => {
            if s.signed {
                format!("i{}", s.bits)
            } else {
                format!("u{}", s.bits)
            }
        }
        ResolvedType::Semantic(s) => format!("{:?}", s).to_lowercase(),
        ResolvedType::Named(id) => reg
            .get(*id)
            .map(|d| decl_name(d).to_string())
            .unwrap_or_else(|| "<unknown>".to_string()),
        ResolvedType::Optional(inner) => format!("optional<{}>", type_display(inner, reg)),
        ResolvedType::Array(inner) => format!("array<{}>", type_display(inner, reg)),
        ResolvedType::Map(k, v) => {
            format!("map<{}, {}>", type_display(k, reg), type_display(v, reg))
        }
        ResolvedType::Result(ok, err) => {
            format!(
                "result<{}, {}>",
                type_display(ok, reg),
                type_display(err, reg)
            )
        }
    }
}

fn encoding_display(enc: &Encoding) -> String {
    match enc {
        Encoding::Default => "default".to_string(),
        Encoding::Varint => "varint".to_string(),
        Encoding::ZigZag => "zigzag".to_string(),
        Encoding::Delta(inner) => format!("delta({})", encoding_display(inner)),
    }
}

// ---------------------------------------------------------------------------
// Declaration-level comparison dispatch
// ---------------------------------------------------------------------------

fn compare_decls(
    name: &SmolStr,
    old_def: &TypeDef,
    new_def: &TypeDef,
    old_reg: &TypeRegistry,
    new_reg: &TypeRegistry,
    changes: &mut Vec<Change>,
) {
    // If the declaration kind changed entirely, that's major
    if std::mem::discriminant(old_def) != std::mem::discriminant(new_def) {
        changes.push(Change {
            kind: ChangeKind::DeclarationKindChanged,
            declaration: name.to_string(),
            field: None,
            detail: format!(
                "'{}' changed from {} to {}",
                name,
                decl_kind_name(old_def),
                decl_kind_name(new_def)
            ),
            classification: BumpKind::Major,
        });
        return;
    }

    match (old_def, new_def) {
        (TypeDef::Message(old_msg), TypeDef::Message(new_msg)) => {
            compare_messages(name, old_msg, new_msg, old_reg, new_reg, changes);
        }
        (TypeDef::Enum(old_e), TypeDef::Enum(new_e)) => {
            compare_enums(name, old_e, new_e, changes);
        }
        (TypeDef::Flags(old_f), TypeDef::Flags(new_f)) => {
            compare_flags(name, old_f, new_f, changes);
        }
        (TypeDef::Union(old_u), TypeDef::Union(new_u)) => {
            compare_unions(name, old_u, new_u, old_reg, new_reg, changes);
        }
        (TypeDef::Newtype(old_n), TypeDef::Newtype(new_n)) => {
            compare_newtypes(name, old_n, new_n, old_reg, new_reg, changes);
        }
        (TypeDef::Config(_), TypeDef::Config(_)) => {
            // Config has no wire format, skip
        }
        _ => unreachable!("discriminant check above guarantees matching variants"),
    }
}

// ---------------------------------------------------------------------------
// Message comparison
// ---------------------------------------------------------------------------

fn compare_messages(
    decl_name: &SmolStr,
    old_msg: &MessageDef,
    new_msg: &MessageDef,
    old_reg: &TypeRegistry,
    new_reg: &TypeRegistry,
    changes: &mut Vec<Change>,
) {
    let old_by_ord = fields_by_ordinal(&old_msg.fields);
    let new_by_ord = fields_by_ordinal(&new_msg.fields);

    compare_field_sets(
        decl_name,
        &old_by_ord,
        &new_by_ord,
        old_reg,
        new_reg,
        changes,
    );

    // Check @deprecated on the message itself
    compare_deprecated(
        decl_name,
        None,
        &old_msg.annotations,
        &new_msg.annotations,
        changes,
    );
}

fn fields_by_ordinal(fields: &[FieldDef]) -> HashMap<u32, &FieldDef> {
    fields.iter().map(|f| (f.ordinal, f)).collect()
}

fn compare_field_sets(
    decl_name: &SmolStr,
    old_fields: &HashMap<u32, &FieldDef>,
    new_fields: &HashMap<u32, &FieldDef>,
    old_reg: &TypeRegistry,
    new_reg: &TypeRegistry,
    changes: &mut Vec<Change>,
) {
    // Fields removed (in old but not in new)
    for (&ord, old_f) in old_fields {
        if !new_fields.contains_key(&ord) {
            changes.push(Change {
                kind: ChangeKind::FieldRemoved,
                declaration: decl_name.to_string(),
                field: Some(old_f.name.to_string()),
                detail: format!("field '{}' @{} was removed", old_f.name, ord),
                classification: BumpKind::Major,
            });
        }
    }

    // Fields added (in new but not in old)
    for (&ord, new_f) in new_fields {
        if !old_fields.contains_key(&ord) {
            changes.push(Change {
                kind: ChangeKind::FieldAdded,
                declaration: decl_name.to_string(),
                field: Some(new_f.name.to_string()),
                detail: format!("field '{}' @{} was added", new_f.name, ord),
                classification: BumpKind::Minor,
            });
        }
    }

    // Fields at same ordinal — compare details
    for (&ord, old_f) in old_fields {
        if let Some(new_f) = new_fields.get(&ord) {
            // Name changed
            if old_f.name != new_f.name {
                changes.push(Change {
                    kind: ChangeKind::FieldRenamed,
                    declaration: decl_name.to_string(),
                    field: Some(new_f.name.to_string()),
                    detail: format!(
                        "field @{} renamed from '{}' to '{}'",
                        ord, old_f.name, new_f.name
                    ),
                    classification: BumpKind::Patch,
                });
            }

            // Type changed
            if !types_equal(&old_f.resolved_type, &new_f.resolved_type, old_reg, new_reg) {
                changes.push(Change {
                    kind: ChangeKind::FieldTypeChanged,
                    declaration: decl_name.to_string(),
                    field: Some(new_f.name.to_string()),
                    detail: format!(
                        "field '{}' @{} type changed from {} to {}",
                        new_f.name,
                        ord,
                        type_display(&old_f.resolved_type, old_reg),
                        type_display(&new_f.resolved_type, new_reg)
                    ),
                    classification: BumpKind::Major,
                });
            }

            // Encoding changed
            if old_f.encoding != new_f.encoding {
                changes.push(Change {
                    kind: ChangeKind::FieldEncodingChanged,
                    declaration: decl_name.to_string(),
                    field: Some(new_f.name.to_string()),
                    detail: format!(
                        "field '{}' @{} encoding changed from {} to {}",
                        new_f.name,
                        ord,
                        encoding_display(&old_f.encoding.encoding),
                        encoding_display(&new_f.encoding.encoding)
                    ),
                    classification: BumpKind::Major,
                });
            }

            // Deprecated changed
            compare_deprecated(
                decl_name,
                Some(&new_f.name),
                &old_f.annotations,
                &new_f.annotations,
                changes,
            );
        }
    }
}

fn compare_deprecated(
    decl_name: &SmolStr,
    field_name: Option<&SmolStr>,
    old_ann: &crate::ir::ResolvedAnnotations,
    new_ann: &crate::ir::ResolvedAnnotations,
    changes: &mut Vec<Change>,
) {
    if old_ann.deprecated.is_none() && new_ann.deprecated.is_some() {
        let target = field_name
            .map(|f| format!("field '{}'", f))
            .unwrap_or_else(|| "declaration".to_string());
        changes.push(Change {
            kind: ChangeKind::FieldDeprecated,
            declaration: decl_name.to_string(),
            field: field_name.map(|f| f.to_string()),
            detail: format!("{} was marked @deprecated", target),
            classification: BumpKind::Patch,
        });
    }
}

// ---------------------------------------------------------------------------
// Enum comparison
// ---------------------------------------------------------------------------

fn compare_enums(decl_name: &SmolStr, old_e: &EnumDef, new_e: &EnumDef, changes: &mut Vec<Change>) {
    // @non_exhaustive changed
    if old_e.annotations.non_exhaustive != new_e.annotations.non_exhaustive {
        changes.push(Change {
            kind: ChangeKind::NonExhaustiveChanged,
            declaration: decl_name.to_string(),
            field: None,
            detail: format!(
                "@non_exhaustive changed from {} to {}",
                old_e.annotations.non_exhaustive, new_e.annotations.non_exhaustive
            ),
            classification: BumpKind::Major,
        });
    }

    let old_by_ord: HashMap<u32, &crate::ir::EnumVariantDef> =
        old_e.variants.iter().map(|v| (v.ordinal, v)).collect();
    let new_by_ord: HashMap<u32, &crate::ir::EnumVariantDef> =
        new_e.variants.iter().map(|v| (v.ordinal, v)).collect();

    // Removed variants
    for (&ord, old_v) in &old_by_ord {
        if !new_by_ord.contains_key(&ord) {
            changes.push(Change {
                kind: ChangeKind::VariantRemoved,
                declaration: decl_name.to_string(),
                field: Some(old_v.name.to_string()),
                detail: format!("variant '{}' @{} was removed", old_v.name, ord),
                classification: BumpKind::Major,
            });
        }
    }

    // Added variants
    for (&ord, new_v) in &new_by_ord {
        if !old_by_ord.contains_key(&ord) {
            let bump = if new_e.annotations.non_exhaustive {
                BumpKind::Minor
            } else {
                BumpKind::Major
            };
            changes.push(Change {
                kind: ChangeKind::VariantAdded,
                declaration: decl_name.to_string(),
                field: Some(new_v.name.to_string()),
                detail: format!("variant '{}' @{} was added", new_v.name, ord),
                classification: bump,
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Flags comparison
// ---------------------------------------------------------------------------

fn compare_flags(
    decl_name: &SmolStr,
    old_f: &FlagsDef,
    new_f: &FlagsDef,
    changes: &mut Vec<Change>,
) {
    let old_by_bit: HashMap<u32, &crate::ir::FlagsBitDef> =
        old_f.bits.iter().map(|b| (b.bit, b)).collect();
    let new_by_bit: HashMap<u32, &crate::ir::FlagsBitDef> =
        new_f.bits.iter().map(|b| (b.bit, b)).collect();

    for (&bit, old_b) in &old_by_bit {
        if !new_by_bit.contains_key(&bit) {
            changes.push(Change {
                kind: ChangeKind::FlagsBitRemoved,
                declaration: decl_name.to_string(),
                field: Some(old_b.name.to_string()),
                detail: format!("bit '{}' @{} was removed", old_b.name, bit),
                classification: BumpKind::Major,
            });
        }
    }

    for (&bit, new_b) in &new_by_bit {
        if !old_by_bit.contains_key(&bit) {
            changes.push(Change {
                kind: ChangeKind::FlagsBitAdded,
                declaration: decl_name.to_string(),
                field: Some(new_b.name.to_string()),
                detail: format!("bit '{}' @{} was added", new_b.name, bit),
                classification: BumpKind::Minor,
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Union comparison
// ---------------------------------------------------------------------------

fn compare_unions(
    decl_name: &SmolStr,
    old_u: &UnionDef,
    new_u: &UnionDef,
    old_reg: &TypeRegistry,
    new_reg: &TypeRegistry,
    changes: &mut Vec<Change>,
) {
    let old_by_ord: HashMap<u32, &crate::ir::UnionVariantDef> =
        old_u.variants.iter().map(|v| (v.ordinal, v)).collect();
    let new_by_ord: HashMap<u32, &crate::ir::UnionVariantDef> =
        new_u.variants.iter().map(|v| (v.ordinal, v)).collect();

    // Removed variants
    for (&ord, old_v) in &old_by_ord {
        if !new_by_ord.contains_key(&ord) {
            changes.push(Change {
                kind: ChangeKind::VariantRemoved,
                declaration: decl_name.to_string(),
                field: Some(old_v.name.to_string()),
                detail: format!("variant '{}' @{} was removed", old_v.name, ord),
                classification: BumpKind::Major,
            });
        }
    }

    // Added variants
    for (&ord, new_v) in &new_by_ord {
        if !old_by_ord.contains_key(&ord) {
            changes.push(Change {
                kind: ChangeKind::VariantAdded,
                declaration: decl_name.to_string(),
                field: Some(new_v.name.to_string()),
                detail: format!("variant '{}' @{} was added", new_v.name, ord),
                classification: BumpKind::Minor,
            });
        }
    }

    // Matching variants — compare their fields
    for (&ord, old_v) in &old_by_ord {
        if let Some(new_v) = new_by_ord.get(&ord) {
            let old_fields = fields_by_ordinal(&old_v.fields);
            let new_fields = fields_by_ordinal(&new_v.fields);
            // Use a synthetic name "Decl::Variant" for context
            let variant_decl = SmolStr::new(format!("{}::{}", decl_name, old_v.name));
            compare_field_sets(
                &variant_decl,
                &old_fields,
                &new_fields,
                old_reg,
                new_reg,
                changes,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Newtype comparison
// ---------------------------------------------------------------------------

fn compare_newtypes(
    decl_name: &SmolStr,
    old_n: &NewtypeDef,
    new_n: &NewtypeDef,
    old_reg: &TypeRegistry,
    new_reg: &TypeRegistry,
    changes: &mut Vec<Change>,
) {
    if !types_equal(&old_n.inner_type, &new_n.inner_type, old_reg, new_reg) {
        changes.push(Change {
            kind: ChangeKind::FieldTypeChanged,
            declaration: decl_name.to_string(),
            field: None,
            detail: format!(
                "newtype '{}' inner type changed from {} to {}",
                decl_name,
                type_display(&old_n.inner_type, old_reg),
                type_display(&new_n.inner_type, new_reg)
            ),
            classification: BumpKind::Major,
        });
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: compile a schema string and return the CompiledSchema.
    fn compile_schema(source: &str) -> CompiledSchema {
        let result = crate::compile(source);
        assert!(
            result
                .diagnostics
                .iter()
                .all(|d| d.severity != crate::Severity::Error),
            "compilation errors: {:?}",
            result.diagnostics
        );
        result.compiled.expect("compilation should produce IR")
    }

    #[test]
    fn identical_schemas_are_compatible() {
        let src = r#"
            namespace test
            message Point { x @0 : f32  y @1 : f32 }
        "#;
        let old = compile_schema(src);
        let new = compile_schema(src);
        let report = check(&old, &new);
        assert!(report.changes.is_empty());
        assert_eq!(report.result, CompatResult::Compatible);
    }

    #[test]
    fn field_added_is_minor() {
        let old = compile_schema(
            r#"
            namespace test
            message Point { x @0 : f32 }
        "#,
        );
        let new = compile_schema(
            r#"
            namespace test
            message Point { x @0 : f32  y @1 : f32 }
        "#,
        );
        let report = check(&old, &new);
        assert_eq!(report.suggested_bump, BumpKind::Minor);
        assert_eq!(report.result, CompatResult::Compatible);
        assert!(report
            .changes
            .iter()
            .any(|c| c.kind == ChangeKind::FieldAdded));
    }

    #[test]
    fn field_removed_is_major() {
        let old = compile_schema(
            r#"
            namespace test
            message Point { x @0 : f32  y @1 : f32 }
        "#,
        );
        let new = compile_schema(
            r#"
            namespace test
            message Point { x @0 : f32 }
        "#,
        );
        let report = check(&old, &new);
        assert_eq!(report.suggested_bump, BumpKind::Major);
        assert_eq!(report.result, CompatResult::Breaking);
        assert!(report
            .changes
            .iter()
            .any(|c| c.kind == ChangeKind::FieldRemoved));
    }

    #[test]
    fn field_type_changed_is_major() {
        let old = compile_schema(
            r#"
            namespace test
            message Point { x @0 : u32 }
        "#,
        );
        let new = compile_schema(
            r#"
            namespace test
            message Point { x @0 : u64 }
        "#,
        );
        let report = check(&old, &new);
        assert_eq!(report.suggested_bump, BumpKind::Major);
        assert_eq!(report.result, CompatResult::Breaking);
        assert!(report
            .changes
            .iter()
            .any(|c| c.kind == ChangeKind::FieldTypeChanged));
    }

    #[test]
    fn field_renamed_is_patch() {
        let old = compile_schema(
            r#"
            namespace test
            message Point { x_coord @0 : f32 }
        "#,
        );
        let new = compile_schema(
            r#"
            namespace test
            message Point { x @0 : f32 }
        "#,
        );
        let report = check(&old, &new);
        assert_eq!(report.suggested_bump, BumpKind::Patch);
        assert_eq!(report.result, CompatResult::Compatible);
        assert!(report
            .changes
            .iter()
            .any(|c| c.kind == ChangeKind::FieldRenamed));
    }

    #[test]
    fn required_to_optional_is_major() {
        let old = compile_schema(
            r#"
            namespace test
            message Point { x @0 : u32 }
        "#,
        );
        let new = compile_schema(
            r#"
            namespace test
            message Point { x @0 : optional<u32> }
        "#,
        );
        let report = check(&old, &new);
        assert_eq!(report.suggested_bump, BumpKind::Major);
        assert_eq!(report.result, CompatResult::Breaking);
        assert!(report
            .changes
            .iter()
            .any(|c| c.kind == ChangeKind::FieldTypeChanged));
    }

    #[test]
    fn declaration_added_is_minor() {
        let old = compile_schema(
            r#"
            namespace test
            message Point { x @0 : f32 }
        "#,
        );
        let new = compile_schema(
            r#"
            namespace test
            message Point { x @0 : f32 }
            message Color { r @0 : u8 }
        "#,
        );
        let report = check(&old, &new);
        assert_eq!(report.suggested_bump, BumpKind::Minor);
        assert_eq!(report.result, CompatResult::Compatible);
        assert!(report
            .changes
            .iter()
            .any(|c| c.kind == ChangeKind::DeclarationAdded));
    }

    #[test]
    fn declaration_removed_is_major() {
        let old = compile_schema(
            r#"
            namespace test
            message Point { x @0 : f32 }
            message Color { r @0 : u8 }
        "#,
        );
        let new = compile_schema(
            r#"
            namespace test
            message Point { x @0 : f32 }
        "#,
        );
        let report = check(&old, &new);
        assert_eq!(report.suggested_bump, BumpKind::Major);
        assert_eq!(report.result, CompatResult::Breaking);
        assert!(report
            .changes
            .iter()
            .any(|c| c.kind == ChangeKind::DeclarationRemoved));
    }

    #[test]
    fn namespace_changed_is_major() {
        let old = compile_schema(
            r#"
            namespace test.v1
            message Point { x @0 : f32 }
        "#,
        );
        let new = compile_schema(
            r#"
            namespace test.v2
            message Point { x @0 : f32 }
        "#,
        );
        let report = check(&old, &new);
        assert_eq!(report.suggested_bump, BumpKind::Major);
        assert_eq!(report.result, CompatResult::Breaking);
        assert!(report
            .changes
            .iter()
            .any(|c| c.kind == ChangeKind::NamespaceChanged));
    }

    #[test]
    fn field_deprecated_is_patch() {
        let old = compile_schema(
            r#"
            namespace test
            message Point { x @0 : f32 }
        "#,
        );
        let new = compile_schema(
            r#"
            namespace test
            message Point { @deprecated(reason: "use y") x @0 : f32 }
        "#,
        );
        let report = check(&old, &new);
        assert_eq!(report.suggested_bump, BumpKind::Patch);
        assert_eq!(report.result, CompatResult::Compatible);
        assert!(report
            .changes
            .iter()
            .any(|c| c.kind == ChangeKind::FieldDeprecated));
    }

    #[test]
    fn enum_variant_added_non_exhaustive_is_minor() {
        let old = compile_schema(
            r#"
            namespace test
            @non_exhaustive
            enum Color { Red @0  Green @1 }
        "#,
        );
        let new = compile_schema(
            r#"
            namespace test
            @non_exhaustive
            enum Color { Red @0  Green @1  Blue @2 }
        "#,
        );
        let report = check(&old, &new);
        assert_eq!(report.suggested_bump, BumpKind::Minor);
        assert_eq!(report.result, CompatResult::Compatible);
        assert!(report
            .changes
            .iter()
            .any(|c| c.kind == ChangeKind::VariantAdded));
    }

    #[test]
    fn enum_variant_removed_is_major() {
        let old = compile_schema(
            r#"
            namespace test
            enum Color { Red @0  Green @1  Blue @2 }
        "#,
        );
        let new = compile_schema(
            r#"
            namespace test
            enum Color { Red @0  Green @1 }
        "#,
        );
        let report = check(&old, &new);
        assert_eq!(report.suggested_bump, BumpKind::Major);
        assert_eq!(report.result, CompatResult::Breaking);
        assert!(report
            .changes
            .iter()
            .any(|c| c.kind == ChangeKind::VariantRemoved));
    }

    #[test]
    fn multiple_changes_take_highest_bump() {
        let old = compile_schema(
            r#"
            namespace test
            message Point { x @0 : f32  y @1 : f32 }
        "#,
        );
        // Remove field y (major) and add field z (minor)
        let new = compile_schema(
            r#"
            namespace test
            message Point { x @0 : f32  z @2 : f32 }
        "#,
        );
        let report = check(&old, &new);
        assert_eq!(report.suggested_bump, BumpKind::Major);
        assert_eq!(report.result, CompatResult::Breaking);
        // Should have both a removal and an addition
        assert!(report
            .changes
            .iter()
            .any(|c| c.kind == ChangeKind::FieldRemoved));
        assert!(report
            .changes
            .iter()
            .any(|c| c.kind == ChangeKind::FieldAdded));
    }
}
