use vexil_lang::ast::TypeExpr;
use vexil_lang::ir::{CompiledSchema, ImplDef, ResolvedType, TraitDef, TypeId, TypeRegistry};

use crate::emit::CodeWriter;
use crate::types::{ts_type_with_symbols, TypeScriptSymbols};

pub fn emit_trait(
    w: &mut CodeWriter,
    trait_id: TypeId,
    trait_def: &TraitDef,
    compiled: &CompiledSchema,
) -> Result<(), crate::CodegenError> {
    emit_trait_with_symbols(
        w,
        trait_id,
        trait_def,
        compiled,
        &TypeScriptSymbols::default(),
    )
}

pub(crate) fn emit_trait_with_symbols(
    w: &mut CodeWriter,
    trait_id: TypeId,
    trait_def: &TraitDef,
    compiled: &CompiledSchema,
    symbols: &TypeScriptSymbols,
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
        let ty = project_type(&field.unresolved_ty, params, None, registry, symbols);
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
                    project_type(&parameter.ty, params, None, registry, symbols)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let return_type = function
            .return_type
            .as_ref()
            .map(|ty| project_type(ty, params, None, registry, symbols))
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
    symbols: &TypeScriptSymbols,
) -> String {
    match expr {
        TypeExpr::Primitive(p) => {
            ts_type_with_symbols(&ResolvedType::Primitive(*p), registry, symbols)
        }
        TypeExpr::SubByte(s) => ts_type_with_symbols(&ResolvedType::SubByte(*s), registry, symbols),
        TypeExpr::Semantic(s) => {
            ts_type_with_symbols(&ResolvedType::Semantic(*s), registry, symbols)
        }
        TypeExpr::Named(name) => params
            .iter()
            .position(|p| p.name.node == *name)
            .and_then(|i| args.and_then(|args| args.get(i)))
            .map(|ty| ts_type_with_symbols(ty, registry, symbols))
            .or_else(|| {
                registry
                    .lookup(name)
                    .map(|id| symbols.type_ref(id, registry))
            })
            .unwrap_or_else(|| name.to_string()),
        TypeExpr::Qualified(namespace, name) => registry
            .lookup(&format!("{namespace}.{name}"))
            .map(|id| symbols.type_ref(id, registry))
            .unwrap_or_else(|| format!("{namespace}.{name}")),
        TypeExpr::Generic(name, arg) => {
            let base = registry
                .lookup(name)
                .map(|id| symbols.type_ref(id, registry))
                .unwrap_or_else(|| name.to_string());
            format!(
                "{base}<{}>",
                project_type(&arg.node, params, args, registry, symbols)
            )
        }
        TypeExpr::Optional(inner) => {
            let inner_type = project_type(&inner.node, params, args, registry, symbols);
            if matches!(inner.node, TypeExpr::Optional(_)) {
                format!("[{inner_type}] | null")
            } else {
                format!("{inner_type} | null")
            }
        }
        TypeExpr::Array(inner) | TypeExpr::FixedArray(inner, _) => {
            format!(
                "{}[]",
                project_type(&inner.node, params, args, registry, symbols)
            )
        }
        TypeExpr::Set(inner) => {
            format!(
                "Set<{}>",
                project_type(&inner.node, params, args, registry, symbols)
            )
        }
        TypeExpr::Map(key, value) => format!(
            "Map<{}, {}>",
            project_type(&key.node, params, args, registry, symbols),
            project_type(&value.node, params, args, registry, symbols)
        ),
        TypeExpr::Result(ok, err) => format!(
            "{{ ok: {} }} | {{ err: {} }}",
            project_type(&ok.node, params, args, registry, symbols),
            project_type(&err.node, params, args, registry, symbols)
        ),
        TypeExpr::Vec2(inner) => {
            let inner = project_type(&inner.node, params, args, registry, symbols);
            format!("[{inner}, {inner}]")
        }
        TypeExpr::Vec3(inner) => {
            let inner = project_type(&inner.node, params, args, registry, symbols);
            format!("[{inner}, {inner}, {inner}]")
        }
        TypeExpr::Vec4(inner) | TypeExpr::Quat(inner) => {
            let inner = project_type(&inner.node, params, args, registry, symbols);
            format!("[{inner}, {inner}, {inner}, {inner}]")
        }
        TypeExpr::Mat3(inner) | TypeExpr::Mat4(inner) => {
            format!(
                "{}[]",
                project_type(&inner.node, params, args, registry, symbols)
            )
        }
        TypeExpr::BitsInline(_) => "number".to_string(),
    }
}

pub fn emit_impl_assertion(w: &mut CodeWriter, impl_def: &ImplDef, registry: &TypeRegistry) {
    emit_impl_assertion_with_symbols(w, impl_def, registry, &TypeScriptSymbols::default());
}

pub(crate) fn emit_impl_assertion_with_symbols(
    w: &mut CodeWriter,
    impl_def: &ImplDef,
    registry: &TypeRegistry,
    symbols: &TypeScriptSymbols,
) {
    let args = impl_def
        .type_args
        .iter()
        .map(|t| ts_type_with_symbols(t, registry, symbols))
        .collect::<Vec<_>>();
    let (trait_id, trait_name) = registry
        .trait_for_impl(impl_def)
        .map(|(id, definition)| (Some(id), definition.name.as_str()))
        .unwrap_or((None, impl_def.trait_name.as_str()));
    let trait_type = trait_id
        .map(|id| symbols.type_ref(id, registry))
        .unwrap_or_else(|| trait_name.to_string());
    let assertion_trait = trait_type.replace(['.', '$'], "_");
    let trait_ref = if args.is_empty() {
        trait_type
    } else {
        format!("{trait_type}<{}>", args.join(", "))
    };
    let target = ts_type_with_symbols(&impl_def.target_type, registry, symbols);
    w.line(&format!(
        "export type _{target}Implements{} = _VexilAssertAssignable<{target}, {trait_ref}>;",
        assertion_trait
    ));
}
