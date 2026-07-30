use std::collections::HashSet;

use vexil_lang::ast::PrimitiveType;
use vexil_lang::codegen::portable::{
    PortableExpr, PortableExprKind, PortableFunction, PortableStatement,
};
use vexil_lang::ir::{BinOp, MessageDef, ResolvedType, TypeDef, TypeId, TypeRegistry, UnaryOp};

use crate::emit::CodeWriter;

/// Emit one checked portable impl function using the target message's Rust storage layout.
pub fn emit_function(
    w: &mut CodeWriter,
    function: &PortableFunction,
    registry: &TypeRegistry,
    message: &MessageDef,
    target_id: TypeId,
    needs_box: &HashSet<(TypeId, usize)>,
) {
    let referenced_locals = referenced_locals(function);
    let context = FunctionContext {
        registry,
        message,
        target_id,
        needs_box,
    };
    let params = function
        .params
        .iter()
        .map(|parameter| {
            format!(
                "{}: {}",
                binding_name(&parameter.name, &referenced_locals),
                crate::types::rust_type(&parameter.ty, registry, &HashSet::new(), None,)
            )
        })
        .collect::<Vec<_>>();
    let return_type = function
        .return_type
        .as_ref()
        .map(|ty| {
            format!(
                " -> {}",
                crate::types::rust_type(ty, registry, &HashSet::new(), None,)
            )
        })
        .unwrap_or_default();
    let tail = if params.is_empty() {
        String::new()
    } else {
        format!(", {}", params.join(", "))
    };
    w.line(&format!(
        "fn {}(&mut self{tail}){return_type} {{",
        function.name
    ));
    w.indent();
    for (index, statement) in function.statements.iter().enumerate() {
        emit_statement(
            w,
            statement,
            &context,
            &referenced_locals,
            index + 1 == function.statements.len(),
        );
    }
    w.dedent();
    w.line("}");
}

fn emit_statement(
    w: &mut CodeWriter,
    statement: &PortableStatement,
    context: &FunctionContext<'_>,
    referenced_locals: &HashSet<String>,
    is_last: bool,
) {
    match statement {
        PortableStatement::Let { name, ty, value } => {
            let ty = crate::types::rust_type(ty, context.registry, &HashSet::new(), None);
            let name = binding_name(name, referenced_locals);
            w.line(&format!(
                "let {name}: {ty} = {};",
                emit_expr(value, context)
            ));
        }
        PortableStatement::Return(Some(value)) if is_last => {
            w.line(&emit_expr(value, context));
        }
        PortableStatement::Return(Some(value)) => {
            w.line(&format!("return {};", emit_expr(value, context)));
        }
        PortableStatement::Return(None) if is_last => {}
        PortableStatement::Return(None) => w.line("return;"),
        PortableStatement::AssignSelfField { field, value } => {
            let field_storage = context.field_storage(field);
            if !field_storage.is_some_and(|storage| storage.is_boxed) {
                if let PortableExprKind::Binary(operator, left, right) = &value.kind {
                    if matches!(&left.kind, PortableExprKind::SelfField(name) if name == field)
                        && matches!(operator, BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div)
                    {
                        w.line(&format!(
                            "self.{field} {}= {};",
                            binary_operator(*operator),
                            emit_expr(right, context)
                        ));
                        return;
                    }
                }
            }
            let value = emit_expr(value, context);
            let value = match field_storage {
                Some(storage) if storage.is_boxed => into_storage(value, storage.ty),
                _ => value,
            };
            w.line(&format!("self.{field} = {value};"));
        }
    }
}

fn emit_expr(expression: &PortableExpr, context: &FunctionContext<'_>) -> String {
    emit_expr_with_precedence(expression, context, 0)
}

fn emit_expr_with_precedence(
    expression: &PortableExpr,
    context: &FunctionContext<'_>,
    parent_precedence: u8,
) -> String {
    match &expression.kind {
        PortableExprKind::Int(value) => {
            numeric_literal(&value.to_string(), &expression.ty, context.registry)
        }
        PortableExprKind::UInt(value) => {
            numeric_literal(&value.to_string(), &expression.ty, context.registry)
        }
        PortableExprKind::Float(value) => {
            let mut literal = value.to_string();
            if !literal.contains(['.', 'e', 'E']) {
                literal.push_str(".0");
            }
            numeric_literal(&literal, &expression.ty, context.registry)
        }
        PortableExprKind::Bool(value) => value.to_string(),
        PortableExprKind::String(value) => format!("String::from({value:?})"),
        PortableExprKind::Local(name) if is_rust_copy_type(&expression.ty, context.registry) => {
            name.to_string()
        }
        PortableExprKind::Local(name) => format!("{name}.clone()"),
        PortableExprKind::SelfRef => "self.clone()".to_string(),
        PortableExprKind::SelfField(field) => {
            if let Some(storage) = context
                .field_storage(field)
                .filter(|storage| storage.is_boxed)
            {
                from_storage(format!("self.{field}.clone()"), storage.ty)
            } else if is_rust_copy_type(&expression.ty, context.registry) {
                format!("self.{field}")
            } else {
                format!("self.{field}.clone()")
            }
        }
        PortableExprKind::Binary(operator, left, right) => {
            let precedence = binary_precedence(*operator);
            let emitted = format!(
                "{} {} {}",
                emit_expr_with_precedence(left, context, precedence),
                binary_operator(*operator),
                emit_expr_with_precedence(right, context, precedence + 1)
            );
            if precedence < parent_precedence {
                format!("({emitted})")
            } else {
                emitted
            }
        }
        PortableExprKind::Unary(operator, value) => {
            let precedence = 5;
            let emitted = format!(
                "{}{}",
                unary_operator(*operator),
                emit_expr_with_precedence(value, context, precedence)
            );
            if precedence < parent_precedence {
                format!("({emitted})")
            } else {
                emitted
            }
        }
    }
}

#[derive(Clone, Copy)]
struct FieldStorage<'a> {
    ty: &'a ResolvedType,
    is_boxed: bool,
}

struct FunctionContext<'a> {
    registry: &'a TypeRegistry,
    message: &'a MessageDef,
    target_id: TypeId,
    needs_box: &'a HashSet<(TypeId, usize)>,
}

impl<'a> FunctionContext<'a> {
    fn field_storage(&self, name: &str) -> Option<FieldStorage<'a>> {
        self.message
            .fields
            .iter()
            .enumerate()
            .find(|(_, field)| field.name == name)
            .map(|(index, field)| FieldStorage {
                ty: &field.resolved_type,
                is_boxed: self.needs_box.contains(&(self.target_id, index)),
            })
    }
}

fn binding_name(name: &str, referenced_locals: &HashSet<String>) -> String {
    if referenced_locals.contains(name) {
        name.to_string()
    } else {
        format!("_{name}")
    }
}

fn referenced_locals(function: &PortableFunction) -> HashSet<String> {
    let mut names = HashSet::new();
    for statement in &function.statements {
        match statement {
            PortableStatement::Let { value, .. }
            | PortableStatement::Return(Some(value))
            | PortableStatement::AssignSelfField { value, .. } => {
                collect_local_references(value, &mut names);
            }
            PortableStatement::Return(None) => {}
        }
    }
    names
}

fn collect_local_references(expression: &PortableExpr, names: &mut HashSet<String>) {
    match &expression.kind {
        PortableExprKind::Local(name) => {
            names.insert(name.to_string());
        }
        PortableExprKind::Binary(_, left, right) => {
            collect_local_references(left, names);
            collect_local_references(right, names);
        }
        PortableExprKind::Unary(_, value) => collect_local_references(value, names),
        PortableExprKind::Int(_)
        | PortableExprKind::UInt(_)
        | PortableExprKind::Float(_)
        | PortableExprKind::Bool(_)
        | PortableExprKind::String(_)
        | PortableExprKind::SelfRef
        | PortableExprKind::SelfField(_) => {}
    }
}

fn into_storage(value: String, ty: &ResolvedType) -> String {
    match ty {
        ResolvedType::Named(_) => format!("Box::new({value})"),
        ResolvedType::Optional(inner) => {
            let mapped = into_storage("value".to_string(), inner);
            if mapped == "Box::new(value)" {
                format!("{value}.map(Box::new)")
            } else if mapped == "value" {
                value
            } else {
                format!("{value}.map(|value| {mapped})")
            }
        }
        ResolvedType::Result(ok, err) => {
            let mapped_ok = into_storage("value".to_string(), ok);
            let mapped_err = into_storage("error".to_string(), err);
            let value = if mapped_ok == "value" {
                value
            } else {
                format!("{value}.map(|value| {mapped_ok})")
            };
            if mapped_err == "error" {
                value
            } else {
                format!("{value}.map_err(|error| {mapped_err})")
            }
        }
        _ => value,
    }
}

fn from_storage(value: String, ty: &ResolvedType) -> String {
    match ty {
        ResolvedType::Named(_) => format!("*{value}"),
        ResolvedType::Optional(inner) => {
            let mapped = from_storage("value".to_string(), inner);
            if mapped == "*value" {
                format!("{value}.map(|value| *value)")
            } else if mapped == "value" {
                value
            } else {
                format!("{value}.map(|value| {mapped})")
            }
        }
        ResolvedType::Result(ok, err) => {
            let mapped_ok = from_storage("value".to_string(), ok);
            let mapped_err = from_storage("error".to_string(), err);
            let value = if mapped_ok == "value" {
                value
            } else {
                format!("{value}.map(|value| {mapped_ok})")
            };
            if mapped_err == "error" {
                value
            } else {
                format!("{value}.map_err(|error| {mapped_err})")
            }
        }
        _ => value,
    }
}

fn is_rust_copy_type(ty: &ResolvedType, registry: &TypeRegistry) -> bool {
    match ty {
        ResolvedType::Primitive(_) | ResolvedType::SubByte(_) | ResolvedType::BitsInline(_) => true,
        ResolvedType::Semantic(semantic) => !matches!(
            semantic,
            vexil_lang::ast::SemanticType::String | vexil_lang::ast::SemanticType::Bytes
        ),
        ResolvedType::Named(id) => matches!(
            registry.get(*id),
            Some(TypeDef::Enum(_) | TypeDef::Flags(_))
        ),
        ResolvedType::Optional(inner) | ResolvedType::FixedArray(inner, _) => {
            is_rust_copy_type(inner, registry)
        }
        _ => false,
    }
}

fn numeric_literal(value: &str, ty: &ResolvedType, registry: &TypeRegistry) -> String {
    let suffix = crate::types::rust_type(ty, registry, &std::collections::HashSet::new(), None);
    match ty {
        ResolvedType::Primitive(PrimitiveType::Fixed32) => format!("{value}i32"),
        ResolvedType::Primitive(PrimitiveType::Fixed64) => format!("{value}i64"),
        _ => format!("{value}{suffix}"),
    }
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

fn binary_precedence(operator: BinOp) -> u8 {
    match operator {
        BinOp::Eq | BinOp::Ne => 1,
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => 2,
        BinOp::Add | BinOp::Sub => 3,
        BinOp::Mul | BinOp::Div => 4,
    }
}

fn unary_operator(operator: UnaryOp) -> &'static str {
    match operator {
        UnaryOp::Neg => "-",
        UnaryOp::Not => "!",
    }
}
