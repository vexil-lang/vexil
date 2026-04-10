//! Function body code generation for Rust backend.
//!
//! Emits Rust code for impl function bodies.

use vexil_lang::ir::{BinOp, Expr, FnBody, Statement, UnaryOp};

use crate::emit::CodeWriter;

/// Generate code for a function body.
pub fn emit_fn_body(w: &mut CodeWriter, body: &FnBody) {
    match body {
        FnBody::External => {
            // External function - generate unimplemented!() or FFI placeholder
            w.line("unimplemented!(\"external function\")");
        }
        FnBody::Block(stmts) => {
            w.indent();
            for stmt in stmts {
                emit_statement(w, stmt);
            }
            w.dedent();
        }
    }
}

/// Generate a simple Rust type string for basic types (used in let statements).
fn simple_rust_type(ty: &vexil_lang::ir::ResolvedType) -> String {
    use vexil_lang::ast::{PrimitiveType, SemanticType};
    use vexil_lang::ir::ResolvedType;

    match ty {
        ResolvedType::Primitive(p) => match p {
            PrimitiveType::Bool => "bool".to_string(),
            PrimitiveType::U8 => "u8".to_string(),
            PrimitiveType::U16 => "u16".to_string(),
            PrimitiveType::U32 => "u32".to_string(),
            PrimitiveType::U64 => "u64".to_string(),
            PrimitiveType::I8 => "i8".to_string(),
            PrimitiveType::I16 => "i16".to_string(),
            PrimitiveType::I32 => "i32".to_string(),
            PrimitiveType::I64 => "i64".to_string(),
            PrimitiveType::F32 => "f32".to_string(),
            PrimitiveType::F64 => "f64".to_string(),
            PrimitiveType::Fixed32 => "i32".to_string(),
            PrimitiveType::Fixed64 => "i64".to_string(),
            PrimitiveType::Void => "()".to_string(),
        },
        ResolvedType::Semantic(s) => match s {
            SemanticType::String => "String".to_string(),
            SemanticType::Bytes => "Vec<u8>".to_string(),
            SemanticType::Rgb => "Rgb".to_string(),
            SemanticType::Uuid => "[u8; 16]".to_string(),
            SemanticType::Timestamp => "i64".to_string(),
            SemanticType::Hash => "[u8; 32]".to_string(),
        },
        ResolvedType::Optional(inner) => {
            let inner_str = simple_rust_type(inner);
            format!("Option<{}>", inner_str)
        }
        ResolvedType::Array(inner) => {
            let inner_str = simple_rust_type(inner);
            format!("Vec<{}>", inner_str)
        }
        ResolvedType::FixedArray(inner, size) => {
            let inner_str = simple_rust_type(inner);
            format!("[{}; {}]", inner_str, size)
        }
        ResolvedType::Named(_) => "/* Named type */".to_string(),
        _ => "/* Complex type */".to_string(),
    }
}

/// Generate code for a statement.
fn emit_statement(w: &mut CodeWriter, stmt: &Statement) {
    match stmt {
        Statement::Expr(expr) => {
            let code = emit_expr(expr);
            w.line(&format!("{};", code));
        }
        Statement::Let { name, ty, value } => {
            let val_code = emit_expr(value);
            if let Some(t) = ty {
                let ty_code = simple_rust_type(t);
                w.line(&format!("let {}: {} = {};", name, ty_code, val_code));
            } else {
                w.line(&format!("let {} = {};", name, val_code));
            }
        }
        Statement::Return(None) => {
            w.line("return;");
        }
        Statement::Return(Some(expr)) => {
            let val = emit_expr(expr);
            w.line(&format!("return {};", val));
        }
        Statement::Assign { target, value } => {
            let target_code = emit_expr(target);
            let val_code = emit_expr(value);
            w.line(&format!("{} = {};", target_code, val_code));
        }
    }
}

/// Generate code for an expression.
fn emit_expr(expr: &Expr) -> String {
    match expr {
        Expr::Int(v) => v.to_string(),
        Expr::UInt(v) => v.to_string(),
        Expr::Float(v) => v.to_string(),
        Expr::Bool(v) => v.to_string(),
        Expr::String(s) => format!("\"{}\".to_string()", s),
        Expr::Local(name) => name.to_string(),
        Expr::SelfRef => "self".to_string(),
        Expr::FieldAccess(obj, field) => {
            let obj_code = emit_expr(obj);
            format!("{}.{}", obj_code, field)
        }
        Expr::Call(name, args) => {
            let args_code: Vec<_> = args.iter().map(emit_expr).collect();
            format!("{}({})", name, args_code.join(", "))
        }
        Expr::TraitMethodCall {
            trait_name: _,
            method_name,
            receiver,
            args,
        } => {
            // Static dispatch: generate direct call to impl function
            // The receiver is passed as first argument for methods
            let recv_code = emit_expr(receiver);
            let args_code: Vec<_> = args.iter().map(emit_expr).collect();
            if args_code.is_empty() {
                format!("{}({})", method_name, recv_code)
            } else {
                format!("{}({}, {})", method_name, recv_code, args_code.join(", "))
            }
        }
        Expr::Binary(op, lhs, rhs) => {
            let op_str = match op {
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
            };
            let lhs_code = emit_expr(lhs);
            let rhs_code = emit_expr(rhs);
            format!("({} {} {})", lhs_code, op_str, rhs_code)
        }
        Expr::Unary(op, expr) => {
            let op_str = match op {
                UnaryOp::Neg => "-",
                UnaryOp::Not => "!",
            };
            let expr_code = emit_expr(expr);
            format!("{}{}", op_str, expr_code)
        }
    }
}
