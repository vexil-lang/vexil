//! # Stability: Tier 2
//!
//! Type remapping utilities for cloning type definitions between registries.
//!
//! Used during multi-file compilation to copy imported types into a
//! dependent schema's registry while maintaining internal cross-references.

use std::collections::{HashMap, HashSet};

use crate::ir::{
    ConfigDef, ConfigFieldDef, FieldDef, FnParamDef, ImplDef, ImplFnDef, MessageDef, NewtypeDef,
    ResolvedType, TraitDef, TraitFieldDef, TraitFnDef, TypeDef, TypeId, TypeRegistry, UnionDef,
    UnionVariantDef,
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
                    target.clone_trait_fn_return_types(source, old_id, new_id);
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
        TypeDef::Trait(t) => {
            for f in &t.fields {
                collect_type_ids_from_resolved(&f.ty, &mut ids);
            }
            for fn_def in &t.functions {
                for p in &fn_def.params {
                    collect_type_ids_from_resolved(&p.ty, &mut ids);
                }
                if let Some(return_type) = &fn_def.return_type {
                    collect_type_ids_from_resolved(return_type, &mut ids);
                }
            }
        }
        TypeDef::Impl(i) => {
            collect_type_ids_from_resolved(&i.target_type, &mut ids);
            for ty in &i.type_args {
                collect_type_ids_from_resolved(ty, &mut ids);
            }
            for fn_def in &i.functions {
                for p in &fn_def.params {
                    collect_type_ids_from_resolved(&p.ty, &mut ids);
                }
                if let Some(return_type) = &fn_def.return_type {
                    collect_type_ids_from_resolved(return_type, &mut ids);
                }
                collect_type_ids_from_fn_body(&fn_def.body, &mut ids);
            }
        }
        TypeDef::Enum(_) | TypeDef::Flags(_) | TypeDef::GenericAlias(_) => {}
    }
    ids
}

fn collect_type_ids_from_fn_body(body: &crate::ir::FnBody, ids: &mut Vec<TypeId>) {
    let crate::ir::FnBody::Block(statements) = body else {
        return;
    };
    for statement in statements {
        if let crate::ir::Statement::Let { ty: Some(ty), .. } = statement {
            collect_type_ids_from_resolved(ty, ids);
        }
    }
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
        TypeDef::Trait(t) => TypeDef::Trait(remap_trait_def(t, id_map)),
        TypeDef::Impl(i) => TypeDef::Impl(remap_impl_def(i, id_map)),
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
        TypeDef::Trait(t) => t.name.as_str(),
        TypeDef::Impl(_) => "", // Impls don't have a simple name
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

fn remap_trait_def(t: &TraitDef, id_map: &HashMap<TypeId, TypeId>) -> TraitDef {
    TraitDef {
        name: t.name.clone(),
        type_params: t.type_params.clone(),
        fields: t
            .fields
            .iter()
            .map(|f| TraitFieldDef {
                name: f.name.clone(),
                ty: remap_resolved_type(&f.ty, id_map),
                unresolved_ty: f.unresolved_ty.clone(),
                ordinal: f.ordinal,
                annotations: f.annotations.clone(),
            })
            .collect(),
        functions: t
            .functions
            .iter()
            .map(|f| TraitFnDef {
                name: f.name.clone(),
                params: f
                    .params
                    .iter()
                    .map(|p| FnParamDef {
                        name: p.name.clone(),
                        ty: remap_resolved_type(&p.ty, id_map),
                        unresolved_ty: p.unresolved_ty.clone(),
                    })
                    .collect(),
                return_type: f
                    .return_type
                    .as_ref()
                    .map(|t| remap_resolved_type(t, id_map)),
            })
            .collect(),
        annotations: t.annotations.clone(),
        span: t.span,
    }
}

fn remap_impl_def(i: &ImplDef, id_map: &HashMap<TypeId, TypeId>) -> ImplDef {
    ImplDef {
        trait_name: i.trait_name.clone(),
        target_type: remap_resolved_type(&i.target_type, id_map),
        type_args: i
            .type_args
            .iter()
            .map(|t| remap_resolved_type(t, id_map))
            .collect(),
        functions: i
            .functions
            .iter()
            .map(|f| ImplFnDef {
                name: f.name.clone(),
                params: f
                    .params
                    .iter()
                    .map(|p| FnParamDef {
                        name: p.name.clone(),
                        ty: remap_resolved_type(&p.ty, id_map),
                        unresolved_ty: p.unresolved_ty.clone(),
                    })
                    .collect(),
                return_type: f
                    .return_type
                    .as_ref()
                    .map(|t| remap_resolved_type(t, id_map)),
                body: remap_fn_body(&f.body, id_map),
            })
            .collect(),
        annotations: i.annotations.clone(),
        span: i.span,
    }
}

fn remap_fn_body(body: &crate::ir::FnBody, id_map: &HashMap<TypeId, TypeId>) -> crate::ir::FnBody {
    match body {
        crate::ir::FnBody::External => crate::ir::FnBody::External,
        crate::ir::FnBody::Block(statements) => crate::ir::FnBody::Block(
            statements
                .iter()
                .map(|statement| remap_statement(statement, id_map))
                .collect(),
        ),
    }
}

fn remap_statement(
    statement: &crate::ir::Statement,
    id_map: &HashMap<TypeId, TypeId>,
) -> crate::ir::Statement {
    match statement {
        crate::ir::Statement::Expr(expr) => crate::ir::Statement::Expr(remap_expr(expr)),
        crate::ir::Statement::Let { name, ty, value } => crate::ir::Statement::Let {
            name: name.clone(),
            ty: ty.as_ref().map(|ty| remap_resolved_type(ty, id_map)),
            value: remap_expr(value),
        },
        crate::ir::Statement::Return(value) => {
            crate::ir::Statement::Return(value.as_ref().map(remap_expr))
        }
        crate::ir::Statement::Assign { target, value } => crate::ir::Statement::Assign {
            target: remap_expr(target),
            value: remap_expr(value),
        },
    }
}

fn remap_expr(expr: &crate::ir::Expr) -> crate::ir::Expr {
    use crate::ir::Expr;
    match expr {
        Expr::Int(value) => Expr::Int(*value),
        Expr::UInt(value) => Expr::UInt(*value),
        Expr::Float(value) => Expr::Float(*value),
        Expr::Bool(value) => Expr::Bool(*value),
        Expr::String(value) => Expr::String(value.clone()),
        Expr::Local(name) => Expr::Local(name.clone()),
        Expr::SelfRef => Expr::SelfRef,
        Expr::FieldAccess(receiver, field) => {
            Expr::FieldAccess(Box::new(remap_expr(receiver)), field.clone())
        }
        Expr::Call(name, args) => Expr::Call(name.clone(), args.iter().map(remap_expr).collect()),
        Expr::TraitMethodCall {
            trait_name,
            method_name,
            receiver,
            args,
        } => Expr::TraitMethodCall {
            trait_name: trait_name.clone(),
            method_name: method_name.clone(),
            receiver: Box::new(remap_expr(receiver)),
            args: args.iter().map(remap_expr).collect(),
        },
        Expr::Binary(operator, left, right) => Expr::Binary(
            *operator,
            Box::new(remap_expr(left)),
            Box::new(remap_expr(right)),
        ),
        Expr::Unary(operator, value) => Expr::Unary(*operator, Box::new(remap_expr(value))),
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

    #[test]
    fn remap_clones_trait_function_return_dependencies_transitively() {
        let source = r#"
namespace test.function_return
message Leaf { value @0 : u32 }
message Payload { leaf @0 : Leaf }
trait Producer { fn produce() -> Payload }
"#;
        let result = crate::compile(source);
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        let compiled = result.compiled.expect("compiled schema");
        let (trait_id, _) = compiled.find_type("Producer").expect("producer trait");
        let (payload_id, _) = compiled.find_type("Payload").expect("payload message");
        let (leaf_id, _) = compiled.find_type("Leaf").expect("leaf message");

        let mut target = crate::ir::TypeRegistry::new();
        let id_map = clone_types_into(&compiled.registry, &[trait_id], &mut target);

        assert_eq!(id_map.len(), 3);
        assert!(id_map.contains_key(&payload_id));
        assert!(id_map.contains_key(&leaf_id));
        let Some(TypeDef::Trait(trait_def)) = target.get(id_map[&trait_id]) else {
            panic!("expected remapped trait");
        };
        assert_eq!(
            trait_def.functions[0].return_type,
            Some(ResolvedType::Named(id_map[&payload_id]))
        );
    }

    #[test]
    fn remap_trait_function_return_diamond_reuses_existing_dependencies() {
        let source = r#"
namespace test.function_return_diamond
message Leaf { value @0 : u32 }
message Payload { leaf @0 : Leaf }
trait Producer { fn produce() -> Payload }
"#;
        let compiled = crate::compile(source).compiled.expect("compiled schema");
        let (trait_id, _) = compiled.find_type("Producer").expect("producer trait");
        let (payload_id, _) = compiled.find_type("Payload").expect("payload message");
        let (leaf_id, _) = compiled.find_type("Leaf").expect("leaf message");

        let mut target = crate::ir::TypeRegistry::new();
        let first_map = clone_types_into(&compiled.registry, &[payload_id], &mut target);
        let second_map = clone_types_into(&compiled.registry, &[trait_id], &mut target);

        assert_eq!(second_map[&payload_id], first_map[&payload_id]);
        assert_eq!(second_map[&leaf_id], first_map[&leaf_id]);
        let Some(TypeDef::Trait(trait_def)) = target.get(second_map[&trait_id]) else {
            panic!("expected remapped trait");
        };
        assert_eq!(
            trait_def.functions[0].return_type,
            Some(ResolvedType::Named(first_map[&payload_id]))
        );
    }

    #[test]
    fn remap_generic_trait_function_preserves_concrete_return_dependency() {
        let source = r#"
namespace test.generic_function_dependency
message Failure { code @0 : u32 }
trait Resolver<T> {
    fn resolve(candidate: T) -> result<T, Failure>
}
"#;
        let result = crate::compile(source);
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        let compiled = result.compiled.expect("compiled schema");
        let (trait_id, _) = compiled.find_type("Resolver").expect("resolver trait");
        let (failure_id, _) = compiled.find_type("Failure").expect("failure message");

        let mut target = crate::ir::TypeRegistry::new();
        let first_map = clone_types_into(&compiled.registry, &[failure_id], &mut target);
        let second_map = clone_types_into(&compiled.registry, &[trait_id], &mut target);

        assert_eq!(second_map[&failure_id], first_map[&failure_id]);
        let Some(TypeDef::Trait(trait_def)) = target.get(second_map[&trait_id]) else {
            panic!("expected remapped trait");
        };
        let Some(ResolvedType::Result(ok, error)) = &trait_def.functions[0].return_type else {
            panic!("expected result return type");
        };
        assert_eq!(**ok, ResolvedType::Named(crate::ir::POISON_TYPE_ID));
        assert_eq!(**error, ResolvedType::Named(first_map[&failure_id]));
    }

    #[test]
    fn remap_impl_function_return_and_local_annotation_types() {
        let source = r#"
namespace test.impl_function_types
trait Producer { fn produce() -> Payload }
message Payload { value @0 : bool }
message Leaf { value @0 : u32 }
message Host { value @0 : bool }
impl Producer for Host {
    fn produce() -> Payload {
        let leaf: Leaf = self
        return self
    }
}
"#;
        let compiled = crate::compile(source).compiled.expect("compiled schema");
        let (payload_id, _) = compiled.find_type("Payload").expect("payload message");
        let (leaf_id, _) = compiled.find_type("Leaf").expect("leaf message");
        let (impl_id, _) = compiled
            .registry
            .iter()
            .find(|(_, definition)| matches!(definition, TypeDef::Impl(_)))
            .expect("impl definition");

        let mut target = crate::ir::TypeRegistry::new();
        let id_map = clone_types_into(&compiled.registry, &[impl_id], &mut target);
        let Some(TypeDef::Impl(implementation)) = target.get(id_map[&impl_id]) else {
            panic!("expected remapped impl");
        };

        assert_eq!(
            implementation.functions[0].return_type,
            Some(ResolvedType::Named(id_map[&payload_id]))
        );
        let crate::ir::FnBody::Block(statements) = &implementation.functions[0].body else {
            panic!("expected block body");
        };
        let crate::ir::Statement::Let {
            ty: Some(local_type),
            ..
        } = &statements[0]
        else {
            panic!("expected typed local");
        };
        assert_eq!(*local_type, ResolvedType::Named(id_map[&leaf_id]));
    }
}
