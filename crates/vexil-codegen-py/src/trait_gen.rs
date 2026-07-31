use vexil_lang::ast::{PrimitiveType, TypeExpr};
use vexil_lang::ir::{CompiledSchema, ImplDef, ResolvedType, TraitDef, TypeId, TypeRegistry};

use crate::emit::CodeWriter;
use crate::types::py_type_param_ident;

pub fn emit_trait(
    w: &mut CodeWriter,
    trait_id: TypeId,
    trait_def: &TraitDef,
    compiled: &CompiledSchema,
) -> Result<(), crate::CodegenError> {
    let registry = &compiled.registry;
    let params = &trait_def.type_params;
    let type_param_names: std::collections::BTreeSet<&str> = params
        .iter()
        .map(|param| param.name.node.as_str())
        .collect();
    let generic = if params.is_empty() {
        String::new()
    } else {
        format!(
            "[{}]",
            params
                .iter()
                .map(|param| py_type_param_ident(param.name.node.as_str()))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    w.line("@runtime_checkable");
    w.line(&format!("class {}(Protocol{generic}):", trait_def.name));
    if trait_def.fields.is_empty() && trait_def.functions.is_empty() {
        w.indent();
        w.line("pass");
        w.dedent();
        return Ok(());
    }
    w.indent();
    for field in &trait_def.fields {
        let ty = project_type(&field.unresolved_ty, registry, &type_param_names);
        w.line(&format!(
            "{}: {ty}",
            crate::types::py_ident(field.name.as_str())
        ));
    }
    for function in vexil_lang::codegen::portable::trait_signatures(compiled, trait_id)? {
        let mut function_params = vec!["self".to_string()];
        function_params.extend(function.params.iter().map(|parameter| {
            format!(
                "{}: {}",
                parameter.name,
                project_type(&parameter.ty, registry, &type_param_names)
            )
        }));
        let return_type = function
            .return_type
            .as_ref()
            .filter(|ty| !matches!(ty, TypeExpr::Primitive(PrimitiveType::Void)))
            .map(|ty| project_type(ty, registry, &type_param_names))
            .unwrap_or_else(|| "None".to_string());
        w.line(&format!(
            "def {}({}) -> {return_type}: ...",
            function.name,
            function_params.join(", ")
        ));
    }
    w.dedent();
    Ok(())
}

pub fn emit_impl_proof(w: &mut CodeWriter, impl_def: &ImplDef, registry: &TypeRegistry) {
    let target = crate::types::py_type(&impl_def.target_type, registry);
    let args = impl_def
        .type_args
        .iter()
        .map(|ty| crate::types::py_type(ty, registry))
        .collect::<Vec<_>>();
    let trait_name = registry
        .trait_for_impl(impl_def)
        .map(|(_, definition)| definition.name.as_str())
        .unwrap_or(impl_def.trait_name.as_str());
    let trait_ref = if args.is_empty() {
        trait_name.to_string()
    } else {
        format!("{trait_name}[{}]", args.join(", "))
    };
    let proof_name = format!(
        "_vexil_assert_{}_implements_{}",
        target.replace('.', "_"),
        trait_name
    );
    w.line(&format!(
        "def {proof_name}(value: {target}) -> {trait_ref}:  # pyright: ignore[reportUnusedFunction]"
    ));
    w.indent();
    w.line("return value");
    w.dedent();
}

fn project_type(
    expr: &TypeExpr,
    registry: &TypeRegistry,
    type_params: &std::collections::BTreeSet<&str>,
) -> String {
    match expr {
        TypeExpr::Primitive(p) => crate::types::py_type(&ResolvedType::Primitive(*p), registry),
        TypeExpr::SubByte(s) => crate::types::py_type(&ResolvedType::SubByte(*s), registry),
        TypeExpr::Semantic(s) => crate::types::py_type(&ResolvedType::Semantic(*s), registry),
        TypeExpr::Named(name) if type_params.contains(name.as_str()) => {
            py_type_param_ident(name.as_str())
        }
        TypeExpr::Named(name) => name.to_string(),
        TypeExpr::Qualified(namespace, name) => format!("{namespace}.{name}"),
        TypeExpr::Generic(name, arg) => {
            format!(
                "{}[{}]",
                name,
                project_type(&arg.node, registry, type_params)
            )
        }
        TypeExpr::Optional(inner) => {
            format!(
                "{} | None",
                project_type(&inner.node, registry, type_params)
            )
        }
        TypeExpr::Array(inner) => {
            format!("list[{}]", project_type(&inner.node, registry, type_params))
        }
        TypeExpr::FixedArray(inner, _) => {
            format!(
                "tuple[{}, ...]",
                project_type(&inner.node, registry, type_params)
            )
        }
        TypeExpr::Set(inner) => {
            format!("set[{}]", project_type(&inner.node, registry, type_params))
        }
        TypeExpr::Map(key, value) => format!(
            "dict[{}, {}]",
            project_type(&key.node, registry, type_params),
            project_type(&value.node, registry, type_params)
        ),
        TypeExpr::Result(ok, err) => format!(
            "tuple[bool, {} | {}]",
            project_type(&ok.node, registry, type_params),
            project_type(&err.node, registry, type_params)
        ),
        TypeExpr::Vec2(inner)
        | TypeExpr::Vec3(inner)
        | TypeExpr::Vec4(inner)
        | TypeExpr::Quat(inner)
        | TypeExpr::Mat3(inner)
        | TypeExpr::Mat4(inner) => format!(
            "tuple[{}, ...]",
            project_type(&inner.node, registry, type_params)
        ),
        TypeExpr::BitsInline(_) => "int".to_string(),
    }
}
