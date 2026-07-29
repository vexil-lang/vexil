use vexil_lang::ast::TypeExpr;
use vexil_lang::ir::{ResolvedType, TraitDef, TypeRegistry};

use crate::emit::CodeWriter;

pub fn emit_trait(w: &mut CodeWriter, trait_def: &TraitDef, registry: &TypeRegistry) {
    let params = &trait_def.type_params;
    let generic = if params.is_empty() {
        String::new()
    } else {
        format!(
            "[{}]",
            params
                .iter()
                .map(|p| p.name.node.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    w.line("@runtime_checkable");
    w.line(&format!("class {}(Protocol{generic}):", trait_def.name));
    if trait_def.fields.is_empty() {
        w.indent();
        w.line("pass");
        w.dedent();
        return;
    }
    w.indent();
    for field in &trait_def.fields {
        let ty = project_type(&field.unresolved_ty, registry);
        w.line(&format!("{}: {ty}", field.name));
    }
    w.dedent();
}

fn project_type(expr: &TypeExpr, registry: &TypeRegistry) -> String {
    match expr {
        TypeExpr::Primitive(p) => crate::types::py_type(&ResolvedType::Primitive(*p), registry),
        TypeExpr::SubByte(s) => crate::types::py_type(&ResolvedType::SubByte(*s), registry),
        TypeExpr::Semantic(s) => crate::types::py_type(&ResolvedType::Semantic(*s), registry),
        TypeExpr::Named(name) => name.to_string(),
        TypeExpr::Qualified(namespace, name) => format!("{namespace}.{name}"),
        TypeExpr::Generic(name, arg) => {
            format!("{}[{}]", name, project_type(&arg.node, registry))
        }
        TypeExpr::Optional(inner) => {
            format!("{} | None", project_type(&inner.node, registry))
        }
        TypeExpr::Array(inner) => format!("list[{}]", project_type(&inner.node, registry)),
        TypeExpr::FixedArray(inner, _) => {
            format!("tuple[{}, ...]", project_type(&inner.node, registry))
        }
        TypeExpr::Set(inner) => format!("set[{}]", project_type(&inner.node, registry)),
        TypeExpr::Map(key, value) => format!(
            "dict[{}, {}]",
            project_type(&key.node, registry),
            project_type(&value.node, registry)
        ),
        TypeExpr::Result(ok, err) => format!(
            "tuple[bool, {} | {}]",
            project_type(&ok.node, registry),
            project_type(&err.node, registry)
        ),
        TypeExpr::Vec2(inner)
        | TypeExpr::Vec3(inner)
        | TypeExpr::Vec4(inner)
        | TypeExpr::Quat(inner)
        | TypeExpr::Mat3(inner)
        | TypeExpr::Mat4(inner) => format!("tuple[{}, ...]", project_type(&inner.node, registry)),
        TypeExpr::BitsInline(_) => "int".to_string(),
    }
}
