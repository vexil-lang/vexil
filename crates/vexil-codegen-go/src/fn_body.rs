use vexil_lang::codegen::portable::{
    PortableExpr, PortableExprKind, PortableFunction, PortableStatement,
};
use vexil_lang::ir::{BinOp, TypeRegistry, UnaryOp};

use crate::emit::CodeWriter;

pub fn emit_function(
    w: &mut CodeWriter,
    target: &str,
    function: &PortableFunction,
    registry: &TypeRegistry,
    import_types: Option<&std::collections::HashMap<String, String>>,
) {
    let params = function
        .params
        .iter()
        .map(|parameter| {
            format!(
                "{} {}",
                parameter.name,
                go_type(&parameter.ty, registry, import_types)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let result = function
        .return_type
        .as_ref()
        .map(|ty| format!(" {}", go_type(ty, registry, import_types)))
        .unwrap_or_default();
    w.line(&format!(
        "func (m *{target}) {}({params}){result} {{",
        crate::types::to_pascal_case(function.name.as_str())
    ));
    w.indent();
    let used_locals = referenced_locals(function);
    for statement in &function.statements {
        emit_statement(w, statement, registry, import_types, &used_locals);
    }
    w.dedent();
    w.line("}");
}

fn emit_statement(
    w: &mut CodeWriter,
    statement: &PortableStatement,
    registry: &TypeRegistry,
    import_types: Option<&std::collections::HashMap<String, String>>,
    used_locals: &std::collections::HashSet<String>,
) {
    match statement {
        PortableStatement::Let { name, ty, value } => {
            w.line(&format!(
                "var {name} {} = {}",
                go_type(ty, registry, import_types),
                emit_expr(value)
            ));
            if !used_locals.contains(name.as_str()) {
                w.line(&format!("_ = {name}"));
            }
        }
        PortableStatement::Return(Some(value)) => {
            w.line(&format!("return {}", emit_expr(value)));
        }
        PortableStatement::Return(None) => w.line("return"),
        PortableStatement::AssignSelfField { field, value } => w.line(&format!(
            "m.{} = {}",
            crate::types::to_pascal_case(field),
            emit_expr(value)
        )),
    }
}

fn referenced_locals(function: &PortableFunction) -> std::collections::HashSet<String> {
    let mut names = std::collections::HashSet::new();
    for statement in &function.statements {
        match statement {
            PortableStatement::Let { value, .. }
            | PortableStatement::Return(Some(value))
            | PortableStatement::AssignSelfField { value, .. } => {
                collect_referenced_locals(value, &mut names);
            }
            PortableStatement::Return(None) => {}
        }
    }
    names
}

fn collect_referenced_locals(
    expression: &PortableExpr,
    names: &mut std::collections::HashSet<String>,
) {
    match &expression.kind {
        PortableExprKind::Local(name) => {
            names.insert(name.to_string());
        }
        PortableExprKind::Binary(_, left, right) => {
            collect_referenced_locals(left, names);
            collect_referenced_locals(right, names);
        }
        PortableExprKind::Unary(_, value) => collect_referenced_locals(value, names),
        PortableExprKind::Int(_)
        | PortableExprKind::UInt(_)
        | PortableExprKind::Float(_)
        | PortableExprKind::Bool(_)
        | PortableExprKind::String(_)
        | PortableExprKind::SelfRef
        | PortableExprKind::SelfField(_) => {}
    }
}

pub(crate) fn go_type(
    ty: &vexil_lang::ir::ResolvedType,
    registry: &TypeRegistry,
    import_types: Option<&std::collections::HashMap<String, String>>,
) -> String {
    use vexil_lang::ir::ResolvedType;

    match ty {
        ResolvedType::Named(id) => {
            let Some(definition) = registry.get(*id) else {
                return crate::types::go_type(ty, registry);
            };
            let name = crate::type_name_of(definition);
            import_types
                .and_then(|imports| imports.get(name))
                .and_then(|path| path.rsplit('/').next())
                .map(|package| format!("{package}.{name}"))
                .unwrap_or_else(|| name.to_string())
        }
        ResolvedType::Optional(inner) => {
            format!("*{}", go_type(inner, registry, import_types))
        }
        ResolvedType::Array(inner) => {
            format!("[]{}", go_type(inner, registry, import_types))
        }
        ResolvedType::FixedArray(inner, size) => {
            format!("[{size}]{}", go_type(inner, registry, import_types))
        }
        ResolvedType::Set(inner) => {
            format!("map[{}]struct{{}}", go_type(inner, registry, import_types))
        }
        ResolvedType::Map(key, value) => format!(
            "map[{}]{}",
            go_type(key, registry, import_types),
            go_type(value, registry, import_types)
        ),
        ResolvedType::Result(ok, error) => format!(
            "Result[{}, {}]",
            go_type(ok, registry, import_types),
            go_type(error, registry, import_types)
        ),
        ResolvedType::Vec2(inner) => {
            format!("[2]{}", go_type(inner, registry, import_types))
        }
        ResolvedType::Vec3(inner) => {
            format!("[3]{}", go_type(inner, registry, import_types))
        }
        ResolvedType::Vec4(inner) | ResolvedType::Quat(inner) => {
            format!("[4]{}", go_type(inner, registry, import_types))
        }
        ResolvedType::Mat3(inner) => {
            format!("[9]{}", go_type(inner, registry, import_types))
        }
        ResolvedType::Mat4(inner) => {
            format!("[16]{}", go_type(inner, registry, import_types))
        }
        ResolvedType::Primitive(_)
        | ResolvedType::SubByte(_)
        | ResolvedType::Semantic(_)
        | ResolvedType::BitsInline(_) => crate::types::go_type(ty, registry),
        _ => crate::types::go_type(ty, registry),
    }
}

fn emit_expr(expression: &PortableExpr) -> String {
    match &expression.kind {
        PortableExprKind::Int(value) => value.to_string(),
        PortableExprKind::UInt(value) => value.to_string(),
        PortableExprKind::Float(value) => {
            let mut literal = value.to_string();
            if !literal.contains(['.', 'e', 'E']) {
                literal.push_str(".0");
            }
            literal
        }
        PortableExprKind::Bool(value) => value.to_string(),
        PortableExprKind::String(value) => quoted_string(value),
        PortableExprKind::Local(name) => name.to_string(),
        PortableExprKind::SelfRef => "*m".to_string(),
        PortableExprKind::SelfField(field) => {
            format!("m.{}", crate::types::to_pascal_case(field))
        }
        PortableExprKind::Binary(operator, left, right) => format!(
            "({} {} {})",
            emit_expr(left),
            binary_operator(*operator),
            emit_expr(right)
        ),
        PortableExprKind::Unary(operator, value) => {
            format!("({}{})", unary_operator(*operator), emit_expr(value))
        }
    }
}

fn quoted_string(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            control if control.is_control() => {
                output.push_str(&format!("\\u{:04x}", u32::from(control)));
            }
            other => output.push(other),
        }
    }
    output.push('"');
    output
}

fn binary_operator(operator: BinOp) -> &'static str {
    match operator {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
    }
}

fn unary_operator(operator: UnaryOp) -> &'static str {
    match operator {
        UnaryOp::Neg => "-",
        UnaryOp::Not => "!",
    }
}
