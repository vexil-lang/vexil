use vexil_lang::ast::PrimitiveType;
use vexil_lang::codegen::portable::{
    PortableExpr, PortableExprKind, PortableFunction, PortableStatement,
};
use vexil_lang::ir::{BinOp, ResolvedType, TypeRegistry, UnaryOp};

use crate::emit::CodeWriter;
use crate::types::{ts_type_with_symbols, TypeScriptSymbols};

pub fn method_signature(function: &PortableFunction, registry: &TypeRegistry) -> String {
    method_signature_with_symbols(function, registry, &TypeScriptSymbols::default())
}

pub(crate) fn method_signature_with_symbols(
    function: &PortableFunction,
    registry: &TypeRegistry,
    symbols: &TypeScriptSymbols,
) -> String {
    let params = function
        .params
        .iter()
        .map(|parameter| {
            format!(
                "{}: {}",
                parameter.name,
                ts_type_with_symbols(&parameter.ty, registry, symbols)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let return_type = function
        .return_type
        .as_ref()
        .map(|ty| ts_type_with_symbols(ty, registry, symbols))
        .unwrap_or_else(|| "void".to_string());
    format!("{}({params}): {return_type}", function.name)
}

pub fn emit_object_method(
    w: &mut CodeWriter,
    function: &PortableFunction,
    registry: &TypeRegistry,
) {
    emit_object_method_with_symbols(w, function, registry, &TypeScriptSymbols::default());
}

pub(crate) fn emit_object_method_with_symbols(
    w: &mut CodeWriter,
    function: &PortableFunction,
    registry: &TypeRegistry,
    symbols: &TypeScriptSymbols,
) {
    w.open_block(&method_signature_with_symbols(function, registry, symbols));
    for statement in &function.statements {
        emit_statement(w, statement, registry, symbols);
    }
    w.dedent();
    w.line("},");
}

fn emit_statement(
    w: &mut CodeWriter,
    statement: &PortableStatement,
    registry: &TypeRegistry,
    symbols: &TypeScriptSymbols,
) {
    match statement {
        PortableStatement::Let { name, ty, value } => {
            w.line(&format!(
                "const {name}: {} = {};",
                ts_type_with_symbols(ty, registry, symbols),
                emit_expr(value)
            ));
        }
        PortableStatement::Return(Some(value)) => {
            w.line(&format!("return {};", emit_expr(value)));
        }
        PortableStatement::Return(None) => w.line("return;"),
        PortableStatement::AssignSelfField { field, value } => {
            w.line(&format!("this.{field} = {};", emit_expr(value)));
        }
    }
}

fn emit_expr(expression: &PortableExpr) -> String {
    match &expression.kind {
        PortableExprKind::Int(value) => integer_literal(&value.to_string(), &expression.ty),
        PortableExprKind::UInt(value) => integer_literal(&value.to_string(), &expression.ty),
        PortableExprKind::Float(value) => value.to_string(),
        PortableExprKind::Bool(value) => value.to_string(),
        PortableExprKind::String(value) => quoted_string(value),
        PortableExprKind::Local(name) => name.to_string(),
        PortableExprKind::SelfRef => "this".to_string(),
        PortableExprKind::SelfField(field) => format!("this.{field}"),
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

fn integer_literal(value: &str, ty: &ResolvedType) -> String {
    if matches!(
        ty,
        ResolvedType::Primitive(PrimitiveType::U64 | PrimitiveType::I64)
    ) {
        format!("{value}n")
    } else {
        value.to_string()
    }
}

fn binary_operator(operator: BinOp) -> &'static str {
    match operator {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Eq => "===",
        BinOp::Ne => "!==",
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
