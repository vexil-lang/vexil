use vexil_lang::ast::TypeExpr;
use vexil_lang::ir::{ImplDef, ResolvedType, TraitDef, TypeDef, TypeRegistry};

use crate::emit::CodeWriter;

pub fn emit_trait(w: &mut CodeWriter, trait_def: &TraitDef, registry: &TypeRegistry) {
    let params = &trait_def.type_params;
    let generic = if params.is_empty() {
        String::new()
    } else {
        format!(
            "[{} any]",
            params
                .iter()
                .map(|p| p.name.node.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    w.line(&format!("type {}{generic} interface {{", trait_def.name));
    w.indent();
    for field in &trait_def.fields {
        let ty = project_type(&field.unresolved_ty, params, None, registry);
        w.line(&format!(
            "Get{}() {ty}",
            crate::types::to_pascal_case(field.name.as_str())
        ));
    }
    w.dedent();
    w.line("}");
}

pub fn emit_impl(
    w: &mut CodeWriter,
    impl_def: &ImplDef,
    registry: &TypeRegistry,
    import_types: Option<&std::collections::HashMap<String, String>>,
) {
    let Some(TypeDef::Message(message)) = target_def(&impl_def.target_type, registry) else {
        return;
    };
    let target = crate::types::go_type(&impl_def.target_type, registry);
    for trait_field in trait_fields(impl_def, registry) {
        let Some(message_field) = message.fields.iter().find(|f| f.name == trait_field.name) else {
            continue;
        };
        let ty = substituted_type(trait_field, impl_def, registry);
        let method = crate::types::to_pascal_case(message_field.name.as_str());
        let field = crate::types::to_pascal_case(message_field.name.as_str());
        w.line(&format!(
            "func (m *{target}) Get{method}() {ty} {{ return m.{field} }}"
        ));
    }
    let args = impl_def
        .type_args
        .iter()
        .map(|t| crate::types::go_type(t, registry))
        .collect::<Vec<_>>();
    let trait_name = import_types
        .and_then(|imports| imports.get(impl_def.trait_name.as_str()))
        .and_then(|path| path.rsplit('/').next())
        .map(|package| format!("{package}.{}", impl_def.trait_name))
        .unwrap_or_else(|| impl_def.trait_name.to_string());
    let trait_ref = if args.is_empty() {
        trait_name
    } else {
        format!("{trait_name}[{}]", args.join(", "))
    };
    w.line(&format!("var _ {trait_ref} = (*{target})(nil)"));
}

fn target_def<'a>(ty: &ResolvedType, registry: &'a TypeRegistry) -> Option<&'a TypeDef> {
    match ty {
        ResolvedType::Named(id) => registry.get(*id),
        _ => None,
    }
}
fn trait_fields<'a>(
    impl_def: &ImplDef,
    registry: &'a TypeRegistry,
) -> Vec<&'a vexil_lang::ir::TraitFieldDef> {
    registry
        .iter()
        .find_map(|(_, def)| match def {
            TypeDef::Trait(t) if t.name == impl_def.trait_name => Some(t.fields.iter().collect()),
            _ => None,
        })
        .unwrap_or_default()
}
fn substituted_type(
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
        TypeExpr::Primitive(p) => crate::types::go_type(&ResolvedType::Primitive(*p), registry),
        TypeExpr::SubByte(s) => crate::types::go_type(&ResolvedType::SubByte(*s), registry),
        TypeExpr::Semantic(s) => crate::types::go_type(&ResolvedType::Semantic(*s), registry),
        TypeExpr::Named(name) => params
            .iter()
            .position(|p| p.name.node == *name)
            .and_then(|i| args.and_then(|args| args.get(i)))
            .map(|ty| crate::types::go_type(ty, registry))
            .unwrap_or_else(|| name.to_string()),
        TypeExpr::Qualified(namespace, name) => format!("{namespace}.{name}"),
        TypeExpr::Generic(name, arg) => format!(
            "{}[{}]",
            name,
            project_type(&arg.node, params, args, registry)
        ),
        TypeExpr::Optional(inner) => {
            format!("*{}", project_type(&inner.node, params, args, registry))
        }
        TypeExpr::Array(inner) => {
            format!("[]{}", project_type(&inner.node, params, args, registry))
        }
        TypeExpr::FixedArray(inner, size) => format!(
            "[{size}]{}",
            project_type(&inner.node, params, args, registry)
        ),
        TypeExpr::Set(inner) => format!(
            "map[{}]struct{{}}",
            project_type(&inner.node, params, args, registry)
        ),
        TypeExpr::Map(key, value) => format!(
            "map[{}]{}",
            project_type(&key.node, params, args, registry),
            project_type(&value.node, params, args, registry)
        ),
        TypeExpr::Result(ok, err) => format!(
            "Result[{}, {}]",
            project_type(&ok.node, params, args, registry),
            project_type(&err.node, params, args, registry)
        ),
        TypeExpr::Vec2(inner) => {
            format!("[2]{}", project_type(&inner.node, params, args, registry))
        }
        TypeExpr::Vec3(inner) => {
            format!("[3]{}", project_type(&inner.node, params, args, registry))
        }
        TypeExpr::Vec4(inner) | TypeExpr::Quat(inner) => {
            format!("[4]{}", project_type(&inner.node, params, args, registry))
        }
        TypeExpr::Mat3(inner) => {
            format!("[9]{}", project_type(&inner.node, params, args, registry))
        }
        TypeExpr::Mat4(inner) => {
            format!("[16]{}", project_type(&inner.node, params, args, registry))
        }
        TypeExpr::BitsInline(names) => {
            crate::types::containing_int_type(names.len() as u8).to_string()
        }
    }
}
