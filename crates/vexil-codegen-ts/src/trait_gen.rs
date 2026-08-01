use vexil_lang::ast::TypeExpr;
use vexil_lang::ir::{CompiledSchema, ImplDef, ResolvedType, TraitDef, TypeId, TypeRegistry};

use crate::emit::CodeWriter;

pub fn emit_trait(
    w: &mut CodeWriter,
    trait_id: TypeId,
    trait_def: &TraitDef,
    compiled: &CompiledSchema,
) -> Result<(), crate::CodegenError> {
    let registry = &compiled.registry;
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
    w.line(&format!("export interface {}{generic} {{", trait_def.name));
    w.indent();
    for field in &trait_def.fields {
        let ty = project_type(&field.unresolved_ty, params, None, registry);
        w.line(&format!("readonly {}: {ty};", field.name));
    }
    for function in vexil_lang::codegen::portable::trait_signatures(compiled, trait_id)? {
        let function_params = function
            .params
            .iter()
            .map(|parameter| {
                format!(
                    "{}: {}",
                    parameter.name,
                    project_type(&parameter.ty, params, None, registry)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let return_type = function
            .return_type
            .as_ref()
            .map(|ty| project_type(ty, params, None, registry))
            .unwrap_or_else(|| "void".to_string());
        w.line(&format!(
            "{}({function_params}): {return_type};",
            function.name
        ));
    }
    w.dedent();
    w.line("}");
    w.blank();
    w.line(&format!(
        "export function is{}{}(value: unknown): value is {}{} {{",
        trait_def.name, generic, trait_def.name, generic
    ));
    w.indent();
    let checks = trait_def
        .fields
        .iter()
        .map(|f| format!("'{}' in value", f.name))
        .chain(trait_def.functions.iter().map(|function| {
            format!(
                "'{}' in value && typeof value.{} === 'function'",
                function.name, function.name
            )
        }))
        .collect::<Vec<_>>();
    let condition = if checks.is_empty() {
        "true".to_string()
    } else {
        checks.join(" && ")
    };
    w.line(&format!(
        "return typeof value === 'object' && value !== null && {condition};"
    ));
    w.dedent();
    w.line("}");
    Ok(())
}

fn project_type(
    expr: &TypeExpr,
    params: &[vexil_lang::ast::TypeParam],
    args: Option<&[ResolvedType]>,
    registry: &TypeRegistry,
) -> String {
    match expr {
        TypeExpr::Primitive(p) => crate::types::ts_type(&ResolvedType::Primitive(*p), registry),
        TypeExpr::SubByte(s) => crate::types::ts_type(&ResolvedType::SubByte(*s), registry),
        TypeExpr::Semantic(s) => crate::types::ts_type(&ResolvedType::Semantic(*s), registry),
        TypeExpr::Named(name) => params
            .iter()
            .position(|p| p.name.node == *name)
            .and_then(|i| args.and_then(|args| args.get(i)))
            .map(|ty| crate::types::ts_type(ty, registry))
            .unwrap_or_else(|| name.to_string()),
        TypeExpr::Qualified(namespace, name) => format!("{namespace}.{name}"),
        TypeExpr::Generic(name, arg) => format!(
            "{name}<{}>",
            project_type(&arg.node, params, args, registry)
        ),
        TypeExpr::Optional(inner) => {
            let inner_type = project_type(&inner.node, params, args, registry);
            if matches!(inner.node, TypeExpr::Optional(_)) {
                format!("[{inner_type}] | null")
            } else {
                format!("{inner_type} | null")
            }
        }
        TypeExpr::Array(inner) | TypeExpr::FixedArray(inner, _) => {
            format!("{}[]", project_type(&inner.node, params, args, registry))
        }
        TypeExpr::Set(inner) => {
            format!("Set<{}>", project_type(&inner.node, params, args, registry))
        }
        TypeExpr::Map(key, value) => format!(
            "Map<{}, {}>",
            project_type(&key.node, params, args, registry),
            project_type(&value.node, params, args, registry)
        ),
        TypeExpr::Result(ok, err) => format!(
            "{{ ok: {} }} | {{ err: {} }}",
            project_type(&ok.node, params, args, registry),
            project_type(&err.node, params, args, registry)
        ),
        TypeExpr::Vec2(inner) => {
            let inner = project_type(&inner.node, params, args, registry);
            format!("[{inner}, {inner}]")
        }
        TypeExpr::Vec3(inner) => {
            let inner = project_type(&inner.node, params, args, registry);
            format!("[{inner}, {inner}, {inner}]")
        }
        TypeExpr::Vec4(inner) | TypeExpr::Quat(inner) => {
            let inner = project_type(&inner.node, params, args, registry);
            format!("[{inner}, {inner}, {inner}, {inner}]")
        }
        TypeExpr::Mat3(inner) | TypeExpr::Mat4(inner) => {
            format!("{}[]", project_type(&inner.node, params, args, registry))
        }
        TypeExpr::BitsInline(_) => "number".to_string(),
    }
}

pub fn emit_impl_assertion(w: &mut CodeWriter, impl_def: &ImplDef, registry: &TypeRegistry) {
    let args = impl_def
        .type_args
        .iter()
        .map(|t| crate::types::ts_type(t, registry))
        .collect::<Vec<_>>();
    let trait_name = registry
        .trait_for_impl(impl_def)
        .map(|(_, definition)| definition.name.as_str())
        .unwrap_or(impl_def.trait_name.as_str());
    let trait_ref = if args.is_empty() {
        trait_name.to_string()
    } else {
        format!("{trait_name}<{}>", args.join(", "))
    };
    let target = crate::types::ts_type(&impl_def.target_type, registry);
    w.line(&format!(
        "export type _{target}Implements{} = _VexilAssertAssignable<{target}, {trait_ref}>;",
        trait_name.replace('.', "_")
    ));
}
