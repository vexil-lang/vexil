use std::collections::HashMap;

use crate::ir::{
    ConfigDef, ConfigFieldDef, FieldDef, MessageDef, NewtypeDef, ResolvedType, TypeDef, TypeId,
    TypeRegistry, UnionDef, UnionVariantDef,
};

/// Clone type definitions from `source` into `target`, assigning fresh `TypeId`s.
///
/// Returns a map from old (source) `TypeId`s to new (target) `TypeId`s.
pub fn clone_types_into(
    source: &TypeRegistry,
    declarations: &[TypeId],
    target: &mut TypeRegistry,
) -> HashMap<TypeId, TypeId> {
    // Phase 1: register stubs in target, building the id map.
    let mut id_map = HashMap::new();
    for &old_id in declarations {
        if let Some(def) = source.get(old_id) {
            let name = type_def_name(def);
            let new_id = target.register_stub(name.into());
            id_map.insert(old_id, new_id);
        }
    }

    // Phase 2: remap and fill each stub.
    for &old_id in declarations {
        if let Some(def) = source.get(old_id) {
            let remapped = remap_type_def(def, &id_map);
            if let Some(&new_id) = id_map.get(&old_id) {
                target.fill_stub(new_id, remapped);
            }
        }
    }

    id_map
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
        ResolvedType::Map(k, v) => ResolvedType::Map(
            Box::new(remap_resolved_type(k, id_map)),
            Box::new(remap_resolved_type(v, id_map)),
        ),
        ResolvedType::Result(ok, err) => ResolvedType::Result(
            Box::new(remap_resolved_type(ok, id_map)),
            Box::new(remap_resolved_type(err, id_map)),
        ),
        ResolvedType::Primitive(_) | ResolvedType::SubByte(_) | ResolvedType::Semantic(_) => {
            ty.clone()
        }
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
}
