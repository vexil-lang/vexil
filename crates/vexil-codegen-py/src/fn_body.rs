use vexil_lang::ast::PrimitiveType;
use vexil_lang::codegen::portable::{
    PortableExpr, PortableExprKind, PortableFunction, PortableStatement,
};
use vexil_lang::ir::{BinOp, ResolvedType, TypeRegistry, UnaryOp};

use crate::emit::CodeWriter;

pub fn emit_function(w: &mut CodeWriter, function: &PortableFunction, registry: &TypeRegistry) {
    let mut params = vec!["self".to_string()];
    params.extend(function.params.iter().map(|parameter| {
        format!(
            "{}: {}",
            parameter.name,
            crate::types::py_type(&parameter.ty, registry)
        )
    }));
    let return_type = function
        .return_type
        .as_ref()
        .map(|ty| crate::types::py_type(ty, registry))
        .unwrap_or_else(|| "None".to_string());
    w.line(&format!(
        "def {}({}) -> {return_type}:",
        function.name,
        params.join(", ")
    ));
    w.indent();
    if function.statements.is_empty() {
        w.line("pass");
    } else {
        for statement in &function.statements {
            emit_statement(w, statement, registry);
        }
    }
    w.dedent();
}

fn emit_statement(w: &mut CodeWriter, statement: &PortableStatement, registry: &TypeRegistry) {
    match statement {
        PortableStatement::Let { name, ty, value } => w.line(&format!(
            "{name}: {} = {}",
            crate::types::py_type(ty, registry),
            emit_expr(value)
        )),
        PortableStatement::Return(Some(value)) => {
            w.line(&format!("return {}", emit_expr(value)));
        }
        PortableStatement::Return(None) => w.line("return"),
        PortableStatement::AssignSelfField { field, value } => {
            w.line(&format!("self.{field} = {}", emit_expr(value)));
        }
    }
}

fn emit_expr(expression: &PortableExpr) -> String {
    match &expression.kind {
        PortableExprKind::Int(value) => numeric_literal(&value.to_string(), &expression.ty),
        PortableExprKind::UInt(value) => numeric_literal(&value.to_string(), &expression.ty),
        PortableExprKind::Float(value) => {
            let mut literal = value.to_string();
            if !literal.contains(['.', 'e', 'E']) {
                literal.push_str(".0");
            }
            literal
        }
        PortableExprKind::Bool(value) => {
            if *value {
                "True".to_string()
            } else {
                "False".to_string()
            }
        }
        PortableExprKind::String(value) => quoted_string(value),
        PortableExprKind::Local(name) => name.to_string(),
        PortableExprKind::SelfRef => "self".to_string(),
        PortableExprKind::SelfField(field) => format!("self.{field}"),
        PortableExprKind::Binary(operator, left, right) => format!(
            "({} {} {})",
            emit_expr(left),
            binary_operator(*operator, &left.ty),
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

fn numeric_literal(value: &str, ty: &ResolvedType) -> String {
    if matches!(
        ty,
        ResolvedType::Primitive(PrimitiveType::F32 | PrimitiveType::F64)
    ) {
        format!("{value}.0")
    } else {
        value.to_string()
    }
}

fn binary_operator(operator: BinOp, operand_type: &ResolvedType) -> &'static str {
    match operator {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div if is_integer(operand_type) => "//",
        BinOp::Div => "/",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
    }
}

fn is_integer(ty: &ResolvedType) -> bool {
    matches!(
        ty,
        ResolvedType::Primitive(
            PrimitiveType::U8
                | PrimitiveType::U16
                | PrimitiveType::U32
                | PrimitiveType::U64
                | PrimitiveType::I8
                | PrimitiveType::I16
                | PrimitiveType::I32
                | PrimitiveType::I64
                | PrimitiveType::Fixed32
                | PrimitiveType::Fixed64
        ) | ResolvedType::SubByte(_)
    )
}

fn unary_operator(operator: UnaryOp) -> &'static str {
    match operator {
        UnaryOp::Neg => "-",
        UnaryOp::Not => "not ",
    }
}
