//! # Stability: Tier 2
//!
//! Type remapping utilities for cloning type definitions between registries.
//!
//! Used during multi-file compilation to copy imported types into a
//! dependent schema's registry while maintaining internal cross-references.

use std::collections::{HashMap, HashSet};

use crate::ir::{
    ConfigDef, ConfigFieldDef, FieldDef, MessageDef, NewtypeDef, ResolvedType, TypeDef, TypeId,
    TypeRegistry, UnionDef, UnionVariantDef,
};

/// Clone type definitions from `source` into `target`, assigning fresh `TypeId`s.
///
/// Transitively discovers all `TypeId`s referenced by the given `declarations`
/// and clones them too, so that internal cross-references remain valid.
///
/// Returns a map from old (source) `TypeId`s to new (target) `TypeId`s.
pub fn clone_types_into(
    source: &TypeRegistry,
    declarations: &[TypeId],
    target: &mut TypeRegistry,
) -> HashMap<TypeId, TypeId> {
    // Phase 0: collect all transitively referenced TypeIds.
    let all_ids = collect_transitive_ids(source, declarations);

    // Phase 1: register stubs in target, building the id map.
    let mut id_map = HashMap::new();
    for &old_id in &all_ids {
        if let Some(def) = source.get(old_id) {
            // Skip if already in target by name (diamond dedup).
            let name = type_def_name(def);
            if let Some(existing_id) = target.lookup(name) {
                id_map.insert(old_id, existing_id);
            } else {
                let new_id = target.register_stub(name.into());
                id_map.insert(old_id, new_id);
            }
        }
    }

    // Phase 2: remap and fill each stub (skip already-filled from diamond dedup).
    for &old_id in &all_ids {
        if let Some(def) = source.get(old_id) {
            if let Some(&new_id) = id_map.get(&old_id) {
                if target.is_stub(new_id) {
                    let remapped = remap_type_def(def, &id_map);
                    target.fill_stub(new_id, remapped);
                }
            }
        }
    }

    id_map
}

/// Walk all types transitively referenced by `declarations` in the source registry.
fn collect_transitive_ids(source: &TypeRegistry, declarations: &[TypeId]) -> Vec<TypeId> {
    let mut visited = HashSet::new();
    let mut order = Vec::new();

    for &id in declarations {
        collect_ids_recursive(source, id, &mut visited, &mut order);
    }

    order
}

fn collect_ids_recursive(
    source: &TypeRegistry,
    id: TypeId,
    visited: &mut HashSet<TypeId>,
    order: &mut Vec<TypeId>,
) {
    if !visited.insert(id) {
        return;
    }
    // Visit dependencies first (depth-first) so they get stubs before dependents.
    if let Some(def) = source.get(id) {
        for referenced_id in referenced_type_ids(def) {
            collect_ids_recursive(source, referenced_id, visited, order);
        }
    }
    order.push(id);
}

/// Extract all `TypeId`s directly referenced by a `TypeDef`.
fn referenced_type_ids(def: &TypeDef) -> Vec<TypeId> {
    let mut ids = Vec::new();
    match def {
        TypeDef::Message(m) => {
            for f in &m.fields {
                collect_type_ids_from_resolved(&f.resolved_type, &mut ids);
            }
        }
        TypeDef::Union(u) => {
            for v in &u.variants {
                for f in &v.fields {
                    collect_type_ids_from_resolved(&f.resolved_type, &mut ids);
                }
            }
        }
        TypeDef::Newtype(n) => {
            collect_type_ids_from_resolved(&n.inner_type, &mut ids);
            collect_type_ids_from_resolved(&n.terminal_type, &mut ids);
        }
        TypeDef::Config(c) => {
            for f in &c.fields {
                collect_type_ids_from_resolved(&f.resolved_type, &mut ids);
            }
        }
        TypeDef::Enum(_) | TypeDef::Flags(_) | TypeDef::GenericAlias(_) => {}
    }
    ids
}

fn collect_type_ids_from_resolved(ty: &ResolvedType, ids: &mut Vec<TypeId>) {
    match ty {
        ResolvedType::Named(id) => ids.push(*id),
        ResolvedType::Optional(inner) | ResolvedType::Array(inner) => {
            collect_type_ids_from_resolved(inner, ids);
        }
        ResolvedType::FixedArray(inner, _) => {
            collect_type_ids_from_resolved(inner, ids);
        }
        ResolvedType::Set(inner) => {
            collect_type_ids_from_resolved(inner, ids);
        }
        ResolvedType::Map(k, v) | ResolvedType::Result(k, v) => {
            collect_type_ids_from_resolved(k, ids);
            collect_type_ids_from_resolved(v, ids);
        }
        ResolvedType::Vec2(inner)
        | ResolvedType::Vec3(inner)
        | ResolvedType::Vec4(inner)
        | ResolvedType::Quat(inner)
        | ResolvedType::Mat3(inner)
        | ResolvedType::Mat4(inner) => {
            collect_type_ids_from_resolved(inner, ids);
        }
        ResolvedType::Primitive(_)
        | ResolvedType::SubByte(_)
        | ResolvedType::Semantic(_)
        | ResolvedType::BitsInline(_) => {}
    }
}

/// Recursively remap `TypeId` references within a `ResolvedType`.
pub fn remap_resolved_type(ty: &ResolvedType, id_map: &HashMap<TypeId, TypeId>) -> ResolvedType {
    match ty {
        ResolvedType::Named(id) => ResolvedType::Named(id_map.get(id).copied().unwrap_or(*id)),
        ResolvedType::Optional(inner) => {
            ResolvedType::Optional(Box::new(remap_resolved_type(inner, id_map)))
        }
        ResolvedType::Array(inner) => {
            ResolvedType::Array(Box::new(remap_resolved_type(inner, id_map)))
        }
        ResolvedType::FixedArray(inner, size) => {
            ResolvedType::FixedArray(Box::new(remap_resolved_type(inner, id_map)), *size)
        }
        ResolvedType::Set(inner) => ResolvedType::Set(Box::new(remap_resolved_type(inner, id_map))),
        ResolvedType::Map(k, v) => ResolvedType::Map(
            Box::new(remap_resolved_type(k, id_map)),
            Box::new(remap_resolved_type(v, id_map)),
        ),
        ResolvedType::Result(ok, err) => ResolvedType::Result(
            Box::new(remap_resolved_type(ok, id_map)),
            Box::new(remap_resolved_type(err, id_map)),
        ),
        ResolvedType::Vec2(inner) => {
            ResolvedType::Vec2(Box::new(remap_resolved_type(inner, id_map)))
        }
        ResolvedType::Vec3(inner) => {
            ResolvedType::Vec3(Box::new(remap_resolved_type(inner, id_map)))
        }
        ResolvedType::Vec4(inner) => {
            ResolvedType::Vec4(Box::new(remap_resolved_type(inner, id_map)))
        }
        ResolvedType::Quat(inner) => {
            ResolvedType::Quat(Box::new(remap_resolved_type(inner, id_map)))
        }
        ResolvedType::Mat3(inner) => {
            ResolvedType::Mat3(Box::new(remap_resolved_type(inner, id_map)))
        }
        ResolvedType::Mat4(inner) => {
            ResolvedType::Mat4(Box::new(remap_resolved_type(inner, id_map)))
        }
        ResolvedType::Primitive(_) | ResolvedType::SubByte(_) | ResolvedType::Semantic(_) => {
            ty.clone()
        }
        ResolvedType::BitsInline(_) => ty.clone(),
    }
}

/// Remap all `TypeId` references within a `TypeDef`.
pub fn remap_type_def(def: &TypeDef, id_map: &HashMap<TypeId, TypeId>) -> TypeDef {
    match def {
        TypeDef::Message(m) => TypeDef::Message(remap_message_def(m, id_map)),
        TypeDef::Union(u) => TypeDef::Union(remap_union_def(u, id_map)),
        TypeDef::Newtype(n) => TypeDef::Newtype(remap_newtype_def(n, id_map)),
        TypeDef::Config(c) => TypeDef::Config(remap_config_def(c, id_map)),
        TypeDef::Enum(e) => TypeDef::Enum(e.clone()),
        TypeDef::Flags(f) => TypeDef::Flags(f.clone()),
        TypeDef::GenericAlias(a) => TypeDef::GenericAlias(a.clone()),
    }
}

/// Extract the name from a `TypeDef`.
pub fn type_def_name(def: &TypeDef) -> &str {
    match def {
        TypeDef::Message(m) => m.name.as_str(),
        TypeDef::Enum(e) => e.name.as_str(),
        TypeDef::Flags(f) => f.name.as_str(),
        TypeDef::Union(u) => u.name.as_str(),
        TypeDef::Newtype(n) => n.name.as_str(),
        TypeDef::Config(c) => c.name.as_str(),
        TypeDef::GenericAlias(a) => a.name.as_str(),
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn remap_field_def(f: &FieldDef, id_map: &HashMap<TypeId, TypeId>) -> FieldDef {
    FieldDef {
        name: f.name.clone(),
        span: f.span,
        ordinal: f.ordinal,
        resolved_type: remap_resolved_type(&f.resolved_type, id_map),
        encoding: f.encoding.clone(),
        annotations: f.annotations.clone(),
        constraint: f.constraint.clone(),
    }
}

fn remap_message_def(m: &MessageDef, id_map: &HashMap<TypeId, TypeId>) -> MessageDef {
    MessageDef {
        name: m.name.clone(),
        span: m.span,
        fields: m
            .fields
            .iter()
            .map(|f| remap_field_def(f, id_map))
            .collect(),
        tombstones: m.tombstones.clone(),
        annotations: m.annotations.clone(),
        wire_size: m.wire_size.clone(),
    }
}

fn remap_union_variant_def(
    v: &UnionVariantDef,
    id_map: &HashMap<TypeId, TypeId>,
) -> UnionVariantDef {
    UnionVariantDef {
        name: v.name.clone(),
        span: v.span,
        ordinal: v.ordinal,
        fields: v
            .fields
            .iter()
            .map(|f| remap_field_def(f, id_map))
            .collect(),
        tombstones: v.tombstones.clone(),
        annotations: v.annotations.clone(),
    }
}

fn remap_union_def(u: &UnionDef, id_map: &HashMap<TypeId, TypeId>) -> UnionDef {
    UnionDef {
        name: u.name.clone(),
        span: u.span,
        variants: u
            .variants
            .iter()
            .map(|v| remap_union_variant_def(v, id_map))
            .collect(),
        tombstones: u.tombstones.clone(),
        annotations: u.annotations.clone(),
        wire_size: u.wire_size.clone(),
    }
}

fn remap_newtype_def(n: &NewtypeDef, id_map: &HashMap<TypeId, TypeId>) -> NewtypeDef {
    NewtypeDef {
        name: n.name.clone(),
        span: n.span,
        inner_type: remap_resolved_type(&n.inner_type, id_map),
        terminal_type: remap_resolved_type(&n.terminal_type, id_map),
        annotations: n.annotations.clone(),
    }
}

fn remap_config_field_def(f: &ConfigFieldDef, id_map: &HashMap<TypeId, TypeId>) -> ConfigFieldDef {
    ConfigFieldDef {
        name: f.name.clone(),
        span: f.span,
        resolved_type: remap_resolved_type(&f.resolved_type, id_map),
        default_value: f.default_value.clone(),
        annotations: f.annotations.clone(),
    }
}

fn remap_config_def(c: &ConfigDef, id_map: &HashMap<TypeId, TypeId>) -> ConfigDef {
    ConfigDef {
        name: c.name.clone(),
        span: c.span,
        fields: c
            .fields
            .iter()
            .map(|f| remap_config_field_def(f, id_map))
            .collect(),
        annotations: c.annotations.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remap_clones_types_with_new_ids() {
        let source = "namespace test.remap\nmessage Foo { x @0 : u32 }";
        let result = crate::compile(source);
        let compiled = result.compiled.unwrap();
        let foo_id = compiled.declarations[0];

        let mut target = crate::ir::TypeRegistry::new();
        let id_map = clone_types_into(&compiled.registry, &compiled.declarations, &mut target);

        assert_eq!(id_map.len(), 1);
        let new_id = id_map[&foo_id];
        assert!(!target.is_stub(new_id));
        if let Some(crate::ir::TypeDef::Message(m)) = target.get(new_id) {
            assert_eq!(m.name.as_str(), "Foo");
        } else {
            panic!("expected Message");
        }
    }

    #[test]
    fn remap_transitively_clones_referenced_types() {
        // Compile a schema where Bar references Foo — cloning only [Bar]
        // should transitively pull in Foo.
        let source = "namespace test.trans\nmessage Foo { x @0 : u32 }\nmessage Bar { f @0 : Foo }";
        let result = crate::compile(source);
        let compiled = result.compiled.unwrap();

        // Find Bar's TypeId (second declaration).
        let bar_id = compiled.declarations[1];

        let mut target = crate::ir::TypeRegistry::new();
        let id_map = clone_types_into(&compiled.registry, &[bar_id], &mut target);

        // Both Bar and Foo should be cloned.
        assert_eq!(id_map.len(), 2, "expected Foo + Bar, got {:?}", id_map);

        // Bar's field should reference the new Foo, not the old one.
        let new_bar_id = id_map[&bar_id];
        if let Some(crate::ir::TypeDef::Message(m)) = target.get(new_bar_id) {
            assert_eq!(m.name.as_str(), "Bar");
            if let ResolvedType::Named(ref_id) = &m.fields[0].resolved_type {
                assert!(
                    id_map.values().any(|v| v == ref_id),
                    "Bar's field should reference the new Foo TypeId, not the old one"
                );
                assert!(!target.is_stub(*ref_id), "Foo should not be a stub");
            } else {
                panic!("expected Named type for Bar.f");
            }
        } else {
            panic!("expected Message Bar");
        }
    }

    #[test]
    fn remap_diamond_dedup_skips_existing() {
        // Simulate diamond: target already has Foo, cloning Bar that references Foo
        // should reuse the existing Foo.
        let source =
            "namespace test.diamond\nmessage Foo { x @0 : u32 }\nmessage Bar { f @0 : Foo }";
        let result = crate::compile(source);
        let compiled = result.compiled.unwrap();
        let foo_id = compiled.declarations[0];
        let bar_id = compiled.declarations[1];

        let mut target = crate::ir::TypeRegistry::new();
        // Pre-populate target with Foo (simulating first import path in diamond).
        let first_map = clone_types_into(&compiled.registry, &[foo_id], &mut target);
        let existing_foo_id = first_map[&foo_id];

        // Now clone Bar — transitive discovery finds Foo, but it already exists.
        let second_map = clone_types_into(&compiled.registry, &[bar_id], &mut target);

        // Foo should map to the existing ID, not a new one.
        assert_eq!(
            second_map[&foo_id], existing_foo_id,
            "diamond dedup should reuse existing Foo"
        );
    }
}
