use vexil_lang::ast::TypeExpr;
use vexil_lang::ir::{ImplDef, ResolvedType, TraitDef, TypeDef, TypeRegistry};

use crate::emit::CodeWriter;

pub fn emit_trait(w: &mut CodeWriter, trait_def: &TraitDef, registry: &TypeRegistry) {
    let params = &trait_def.type_params;
    let generic = if params.is_empty() {
        String::new()
    } else {
        format!(
            "<{}>",
            params
                .iter()
                .map(|p| p.name.node.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    w.line(&format!("pub trait {}{generic} {{", trait_def.name));
    w.indent();
    for field in &trait_def.fields {
        w.line(&format!(
            "fn {}(&self) -> &{};",
            field.name,
            project_type(&field.unresolved_ty, params, None, registry)
        ));
    }
    w.dedent();
    w.line("}");
}

pub fn emit_impl(w: &mut CodeWriter, impl_def: &ImplDef, registry: &TypeRegistry) {
    let trait_args = impl_def
        .type_args
        .iter()
        .map(|ty| rust_type(ty, registry))
        .collect::<Vec<_>>();
    let trait_ref = if trait_args.is_empty() {
        impl_def.trait_name.to_string()
    } else {
        format!("{}<{}>", impl_def.trait_name, trait_args.join(", "))
    };
    let target = rust_type(&impl_def.target_type, registry);
    let Some(TypeDef::Message(message)) = target_def(&impl_def.target_type, registry) else {
        return;
    };
    w.line(&format!("impl {trait_ref} for {target} {{"));
    w.indent();
    for field in &message.fields {
        if let Some(trait_field) = trait_field_for_impl(impl_def, field.name.as_str(), registry) {
            let ty = resolved_trait_field_type(trait_field, impl_def, registry);
            w.line(&format!("fn {}(&self) -> &{ty} {{", field.name));
            w.indent();
            w.line(&format!("&self.{}", field.name));
            w.dedent();
            w.line("}");
        }
    }
    w.dedent();
    w.line("}");
}

fn target_def<'a>(ty: &ResolvedType, registry: &'a TypeRegistry) -> Option<&'a TypeDef> {
    match ty {
        ResolvedType::Named(id) => registry.get(*id),
        _ => None,
    }
}

fn trait_field_for_impl<'a>(
    impl_def: &ImplDef,
    name: &str,
    registry: &'a TypeRegistry,
) -> Option<&'a vexil_lang::ir::TraitFieldDef> {
    registry.iter().find_map(|(_, def)| match def {
        TypeDef::Trait(trait_def) if trait_def.name == impl_def.trait_name => {
            trait_def.fields.iter().find(|f| f.name == name)
        }
        _ => None,
    })
}

fn resolved_trait_field_type(
    field: &vexil_lang::ir::TraitFieldDef,
    impl_def: &ImplDef,
    registry: &TypeRegistry,
) -> String {
    let params = registry
        .iter()
        .find_map(|(_, def)| match def {
            TypeDef::Trait(t) if t.name == impl_def.trait_name => Some(&t.type_params),
            _ => None,
        })
        .map(|params| params.as_slice())
        .unwrap_or(&[]);
    project_type(
        &field.unresolved_ty,
        params,
        Some(&impl_def.type_args),
        registry,
    )
}

fn project_type(
    expr: &TypeExpr,
    params: &[vexil_lang::ast::TypeParam],
    args: Option<&[ResolvedType]>,
    registry: &TypeRegistry,
) -> String {
    match expr {
        TypeExpr::Primitive(p) => rust_type(&ResolvedType::Primitive(*p), registry),
        TypeExpr::SubByte(s) => rust_type(&ResolvedType::SubByte(*s), registry),
        TypeExpr::Semantic(s) => rust_type(&ResolvedType::Semantic(*s), registry),
        TypeExpr::Named(name) => params
            .iter()
            .position(|p| p.name.node == *name)
            .and_then(|index| args.and_then(|args| args.get(index)))
            .map(|ty| rust_type(ty, registry))
            .unwrap_or_else(|| name.to_string()),
        TypeExpr::Qualified(namespace, name) => format!("{namespace}::{name}"),
        TypeExpr::Generic(name, arg) => format!(
            "{name}<{}>",
            project_type(&arg.node, params, args, registry)
        ),
        TypeExpr::Optional(inner) => format!(
            "Option<{}>",
            project_type(&inner.node, params, args, registry)
        ),
        TypeExpr::Array(inner) => {
            format!("Vec<{}>", project_type(&inner.node, params, args, registry))
        }
        TypeExpr::FixedArray(inner, size) => format!(
            "[{}; {size}]",
            project_type(&inner.node, params, args, registry)
        ),
        TypeExpr::Set(inner) => format!(
            "std::collections::BTreeSet<{}>",
            project_type(&inner.node, params, args, registry)
        ),
        TypeExpr::Map(key, value) => format!(
            "BTreeMap<{}, {}>",
            project_type(&key.node, params, args, registry),
            project_type(&value.node, params, args, registry)
        ),
        TypeExpr::Result(ok, err) => format!(
            "Result<{}, {}>",
            project_type(&ok.node, params, args, registry),
            project_type(&err.node, params, args, registry)
        ),
        TypeExpr::Vec2(inner) => format!(
            "vexil_runtime::Vec2<{}>",
            project_type(&inner.node, params, args, registry)
        ),
        TypeExpr::Vec3(inner) => format!(
            "vexil_runtime::Vec3<{}>",
            project_type(&inner.node, params, args, registry)
        ),
        TypeExpr::Vec4(inner) => format!(
            "vexil_runtime::Vec4<{}>",
            project_type(&inner.node, params, args, registry)
        ),
        TypeExpr::Quat(inner) => format!(
            "vexil_runtime::Quat<{}>",
            project_type(&inner.node, params, args, registry)
        ),
        TypeExpr::Mat3(inner) => format!(
            "vexil_runtime::Mat3<{}>",
            project_type(&inner.node, params, args, registry)
        ),
        TypeExpr::Mat4(inner) => format!(
            "vexil_runtime::Mat4<{}>",
            project_type(&inner.node, params, args, registry)
        ),
        TypeExpr::BitsInline(names) => {
            rust_type(&ResolvedType::BitsInline(names.clone()), registry)
        }
    }
}

fn rust_type(ty: &ResolvedType, registry: &TypeRegistry) -> String {
    crate::types::rust_type(ty, registry, &std::collections::HashSet::new(), None)
}
