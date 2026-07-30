use vexil_lang::ast::PrimitiveType;
use vexil_lang::ast::TypeExpr;
use vexil_lang::ir::{
    CompiledSchema, ImplDef, ResolvedType, TraitDef, TypeDef, TypeId, TypeRegistry,
};

use crate::emit::CodeWriter;

pub fn emit_trait(
    w: &mut CodeWriter,
    trait_id: TypeId,
    trait_def: &TraitDef,
    compiled: &CompiledSchema,
    import_types: Option<&std::collections::HashMap<String, String>>,
) -> Result<(), crate::CodegenError> {
    let registry = &compiled.registry;
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
        let ty = project_type(&field.unresolved_ty, params, None, registry, import_types);
        w.line(&format!(
            "Get{}() {ty}",
            crate::types::to_pascal_case(field.name.as_str())
        ));
    }
    for function in vexil_lang::codegen::portable::trait_signatures(compiled, trait_id)? {
        let function_params = function
            .params
            .iter()
            .map(|parameter| {
                format!(
                    "{} {}",
                    parameter.name,
                    project_type(&parameter.ty, params, None, registry, import_types)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let result = function
            .return_type
            .as_ref()
            .filter(|ty| !matches!(ty, TypeExpr::Primitive(PrimitiveType::Void)))
            .map(|ty| {
                format!(
                    " {}",
                    project_type(ty, params, None, registry, import_types)
                )
            })
            .unwrap_or_default();
        w.line(&format!(
            "{}({function_params}){result}",
            crate::types::to_pascal_case(function.name.as_str())
        ));
    }
    w.dedent();
    w.line("}");
    Ok(())
}

pub fn emit_impl(
    w: &mut CodeWriter,
    impl_def: &ImplDef,
    compiled: &CompiledSchema,
    import_types: Option<&std::collections::HashMap<String, String>>,
) -> Result<(), crate::CodegenError> {
    let registry = &compiled.registry;
    let Some(TypeDef::Message(message)) = target_def(&impl_def.target_type, registry) else {
        return Ok(());
    };
    let target = crate::types::go_type(&impl_def.target_type, registry);
    for trait_field in trait_fields(impl_def, registry) {
        let Some(message_field) = message.fields.iter().find(|f| f.name == trait_field.name) else {
            continue;
        };
        let ty = substituted_type(trait_field, impl_def, registry, import_types);
        let method = crate::types::to_pascal_case(message_field.name.as_str());
        let field = crate::types::to_pascal_case(message_field.name.as_str());
        w.line(&format!(
            "func (m *{target}) Get{method}() {ty} {{ return m.{field} }}"
        ));
    }
    for function in vexil_lang::codegen::portable::project_impl(compiled, impl_def)? {
        crate::fn_body::emit_function(w, &target, &function, registry, import_types);
    }
    let args = impl_def
        .type_args
        .iter()
        .map(|t| crate::fn_body::go_type(t, registry, import_types))
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
    Ok(())
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
    import_types: Option<&std::collections::HashMap<String, String>>,
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
        import_types,
    )
}

fn project_type(
    expr: &TypeExpr,
    params: &[vexil_lang::ast::TypeParam],
    args: Option<&[ResolvedType]>,
    registry: &TypeRegistry,
    import_types: Option<&std::collections::HashMap<String, String>>,
) -> String {
    match expr {
        TypeExpr::Primitive(p) => crate::types::go_type(&ResolvedType::Primitive(*p), registry),
        TypeExpr::SubByte(s) => crate::types::go_type(&ResolvedType::SubByte(*s), registry),
        TypeExpr::Semantic(s) => crate::types::go_type(&ResolvedType::Semantic(*s), registry),
        TypeExpr::Named(name) => params
            .iter()
            .position(|p| p.name.node == *name)
            .and_then(|i| args.and_then(|args| args.get(i)))
            .map(|ty| crate::fn_body::go_type(ty, registry, import_types))
            .unwrap_or_else(|| qualify_imported_name(name, import_types)),
        TypeExpr::Qualified(_, name) => qualify_imported_name(name, import_types),
        TypeExpr::Generic(name, arg) => format!(
            "{}[{}]",
            qualify_imported_name(name, import_types),
            project_type(&arg.node, params, args, registry, import_types)
        ),
        TypeExpr::Optional(inner) => {
            format!(
                "*{}",
                project_type(&inner.node, params, args, registry, import_types)
            )
        }
        TypeExpr::Array(inner) => {
            format!(
                "[]{}",
                project_type(&inner.node, params, args, registry, import_types)
            )
        }
        TypeExpr::FixedArray(inner, size) => format!(
            "[{size}]{}",
            project_type(&inner.node, params, args, registry, import_types)
        ),
        TypeExpr::Set(inner) => format!(
            "map[{}]struct{{}}",
            project_type(&inner.node, params, args, registry, import_types)
        ),
        TypeExpr::Map(key, value) => format!(
            "map[{}]{}",
            project_type(&key.node, params, args, registry, import_types),
            project_type(&value.node, params, args, registry, import_types)
        ),
        TypeExpr::Result(ok, err) => format!(
            "Result[{}, {}]",
            project_type(&ok.node, params, args, registry, import_types),
            project_type(&err.node, params, args, registry, import_types)
        ),
        TypeExpr::Vec2(inner) => {
            format!(
                "[2]{}",
                project_type(&inner.node, params, args, registry, import_types)
            )
        }
        TypeExpr::Vec3(inner) => {
            format!(
                "[3]{}",
                project_type(&inner.node, params, args, registry, import_types)
            )
        }
        TypeExpr::Vec4(inner) | TypeExpr::Quat(inner) => {
            format!(
                "[4]{}",
                project_type(&inner.node, params, args, registry, import_types)
            )
        }
        TypeExpr::Mat3(inner) => {
            format!(
                "[9]{}",
                project_type(&inner.node, params, args, registry, import_types)
            )
        }
        TypeExpr::Mat4(inner) => {
            format!(
                "[16]{}",
                project_type(&inner.node, params, args, registry, import_types)
            )
        }
        TypeExpr::BitsInline(names) => {
            crate::types::containing_int_type(names.len() as u8).to_string()
        }
    }
}

fn qualify_imported_name(
    name: &str,
    import_types: Option<&std::collections::HashMap<String, String>>,
) -> String {
    import_types
        .and_then(|imports| imports.get(name))
        .and_then(|path| path.rsplit('/').next())
        .map(|package| format!("{package}.{name}"))
        .unwrap_or_else(|| name.to_string())
}
