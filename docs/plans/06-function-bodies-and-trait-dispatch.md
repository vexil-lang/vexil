# Function Bodies and Trait Dispatch Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Enable function bodies in impl blocks and trait method dispatch.

**Architecture:**
- Add expression and statement AST nodes
- Parse block bodies `{ ... }` in impl functions  
- Lower expressions to IR
- Generate code for function bodies in all backends
- Static trait dispatch (monomorphization) - resolve trait calls at compile time

---

## Task 1: Add Expression and Statement AST Nodes

**Files:**
- Modify: `crates/vexil-lang/src/ast/mod.rs`

**Step 1: Add Expression enum**

Add after `ImplFnBody`:

```rust
// ---------------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------------

/// A runtime expression in the Vexil AST.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Integer literal.
    Int(i64),
    /// Unsigned integer literal.
    UInt(u64),
    /// Float literal.
    Float(f64),
    /// Boolean literal.
    Bool(bool),
    /// String literal.
    String(String),
    /// Identifier reference.
    Ident(SmolStr),
    /// Field access: `obj.field`.
    FieldAccess(Box<Expr>, Spanned<SmolStr>),
    /// Function call: `fn(args)`.
    Call(Box<Expr>, Vec<Expr>),
    /// Method call: `obj.method(args)` - crucial for trait dispatch.
    MethodCall(Box<Expr>, Spanned<SmolStr>, Vec<Expr>),
    /// Binary operation: `lhs op rhs`.
    Binary(BinOpKind, Box<Expr>, Box<Expr>),
    /// Unary operation: `op expr`.
    Unary(UnaryOpKind, Box<Expr>),
    /// Self reference within impl block.
    SelfRef,
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOpKind {
    /// Negation: `-expr`.
    Neg,
    /// Logical NOT: `!expr`.
    Not,
}
```

**Step 2: Add Statement enum**

```rust
// ---------------------------------------------------------------------------
// Statements
// ---------------------------------------------------------------------------

/// A statement in a function body.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// Expression evaluated for side effects or return value.
    Expr(Expr),
    /// Variable binding: `let name: Type = value;`.
    Let {
        name: Spanned<SmolStr>,
        ty: Option<Spanned<TypeExpr>>,
        value: Expr,
    },
    /// Return statement: `return expr;`.
    Return(Option<Expr>),
    /// Assignment: `target = value;`.
    Assign { target: Expr, value: Expr },
}
```

**Step 3: Update ImplFnBody to include Block variant**

Change `ImplFnBody` to:

```rust
/// Function body in an impl block.
#[derive(Debug, Clone, PartialEq)]
pub enum ImplFnBody {
    /// External function (no body, semicolon).
    External,
    /// Block body with statements.
    Block(Vec<Statement>),
}
```

**Step 4: Build and verify**

```bash
cargo build -p vexil-lang
```

---

## Task 2: Parse Expression and Block Bodies

**Files:**
- Modify: `crates/vexil-lang/src/parser/expr.rs`
- Modify: `crates/vexil-lang/src/parser/decl.rs`

**Step 1: Add expression parsing functions**

Add to `expr.rs`:

```rust
use crate::ast::{Expr, BinOpKind, UnaryOpKind};
use crate::lexer::token::TokenKind;
use crate::span::{Span, Spanned};
use smol_str::SmolStr;

/// Parse a primary expression (literals, identifiers, parenthesized).
pub(crate) fn parse_primary_expr(p: &mut Parser<'_>) -> Spanned<Expr> {
    let start = p.current_offset();
    
    match p.peek_kind().clone() {
        TokenKind::DecInt(v) => {
            p.advance();
            Spanned::new(Expr::Int(v as i64), p.span_from(start))
        }
        TokenKind::HexInt(v) => {
            p.advance();
            Spanned::new(Expr::Int(v as i64), p.span_from(start))
        }
        TokenKind::Ident(s) | TokenKind::UpperIdent(s) => {
            p.advance();
            let expr = Expr::Ident(s.clone());
            
            // Check for field access or method call
            let mut expr = Spanned::new(expr, p.span_from(start));
            loop {
                if p.at(&TokenKind::Dot) {
                    p.advance();
                    let field_name = match p.peek_kind() {
                        TokenKind::Ident(s) | TokenKind::UpperIdent(s) => {
                            let name = s.clone();
                            let span = p.peek().span;
                            p.advance();
                            Spanned::new(name, span)
                        }
                        _ => {
                            p.emit(p.peek().span, ErrorClass::UnexpectedToken, "expected field name");
                            Spanned::new(SmolStr::new("__error"), Span::empty(p.current_offset()))
                        }
                    };
                    
                    // Check if this is a method call
                    if p.at(&TokenKind::LParen) {
                        let args = parse_call_args(p);
                        expr = Spanned::new(
                            Expr::MethodCall(Box::new(expr.node), field_name, args),
                            p.span_from(start)
                        );
                    } else {
                        expr = Spanned::new(
                            Expr::FieldAccess(Box::new(expr.node), field_name),
                            p.span_from(start)
                        );
                    }
                } else if p.at(&TokenKind::LParen) {
                    // Function call
                    let args = parse_call_args(p);
                    expr = Spanned::new(
                        Expr::Call(Box::new(expr.node), args),
                        p.span_from(start)
                    );
                } else {
                    break;
                }
            }
            
            expr
        }
        TokenKind::StringLit(s) => {
            p.advance();
            Spanned::new(Expr::String(s.clone()), p.span_from(start))
        }
        TokenKind::KwTrue => {
            p.advance();
            Spanned::new(Expr::Bool(true), p.span_from(start))
        }
        TokenKind::KwFalse => {
            p.advance();
            Spanned::new(Expr::Bool(false), p.span_from(start))
        }
        TokenKind::KwSelf => {
            p.advance();
            Spanned::new(Expr::SelfRef, p.span_from(start))
        }
        TokenKind::LParen => {
            p.advance();
            let expr = parse_expr(p);
            p.expect(&TokenKind::RParen);
            expr
        }
        _ => {
            p.emit(p.peek().span, ErrorClass::UnexpectedToken, "expected expression");
            Spanned::new(Expr::Int(0), Span::empty(p.current_offset()))
        }
    }
}

/// Parse function call arguments.
fn parse_call_args(p: &mut Parser<'_>) -> Vec<Expr> {
    p.expect(&TokenKind::LParen);
    let mut args = Vec::new();
    
    while !p.at(&TokenKind::RParen) && !p.at_eof() {
        args.push(parse_expr(p).node);
        
        if p.at(&TokenKind::Comma) {
            p.advance();
        } else if !p.at(&TokenKind::RParen) {
            p.emit(p.peek().span, ErrorClass::UnexpectedToken, "expected ',' or ')'");
            break;
        }
    }
    
    p.expect(&TokenKind::RParen);
    args
}

/// Parse expression with operator precedence.
pub(crate) fn parse_expr(p: &mut Parser<'_>) -> Spanned<Expr> {
    parse_binary_expr(p, 0)
}

fn parse_binary_expr(p: &mut Parser<'_>, min_prec: u8) -> Spanned<Expr> {
    let start = p.current_offset();
    let mut lhs = parse_primary_expr(p);
    
    loop {
        let op = match peek_binary_op(p) {
            Some(op) if precedence(&op) >= min_prec => op,
            _ => break,
        };
        
        let prec = precedence(&op);
        p.advance(); // consume operator
        
        let rhs = parse_binary_expr(p, prec + 1);
        
        lhs = Spanned::new(
            Expr::Binary(op, Box::new(lhs.node), Box::new(rhs.node)),
            p.span_from(start)
        );
    }
    
    lhs
}

fn peek_binary_op(p: &Parser<'_>) -> Option<BinOpKind> {
    match p.peek_kind() {
        TokenKind::Plus => Some(BinOpKind::Add),
        TokenKind::Minus => Some(BinOpKind::Sub),
        TokenKind::Star => Some(BinOpKind::Mul),
        TokenKind::Slash => Some(BinOpKind::Div),
        TokenKind::EqEq => Some(BinOpKind::Eq),
        TokenKind::Ne => Some(BinOpKind::Ne),
        TokenKind::Lt => Some(BinOpKind::Lt),
        TokenKind::Le => Some(BinOpKind::Le),
        TokenKind::Gt => Some(BinOpKind::Gt),  // Need to add Gt token kind
        TokenKind::Ge => Some(BinOpKind::Ge),
        _ => None,
    }
}

fn precedence(op: &BinOpKind) -> u8 {
    match op {
        BinOpKind::Mul | BinOpKind::Div => 4,
        BinOpKind::Add | BinOpKind::Sub => 3,
        BinOpKind::Eq | BinOpKind::Ne => 2,
        BinOpKind::Lt | BinOpKind::Le | BinOpKind::Gt | BinOpKind::Ge => 2,
        _ => 1,
    }
}
```

**Step 2: Add statement parsing**

```rust
/// Parse a statement.
pub(crate) fn parse_statement(p: &mut Parser<'_>) -> Option<Statement> {
    match p.peek_kind() {
        TokenKind::KwLet => Some(parse_let_stmt(p)),
        TokenKind::KwReturn => Some(parse_return_stmt(p)),
        _ => {
            // Try as expression statement or assignment
            let expr = parse_expr(p);
            if p.at(&TokenKind::Eq) {
                p.advance();
                let value = parse_expr(p);
                Some(Statement::Assign { 
                    target: expr.node, 
                    value: value.node 
                })
            } else {
                Some(Statement::Expr(expr.node))
            }
        }
    }
}

fn parse_let_stmt(p: &mut Parser<'_>) -> Statement {
    p.advance(); // consume 'let'
    
    let name = match p.peek_kind() {
        TokenKind::Ident(s) => {
            let name = s.clone();
            let span = p.peek().span;
            p.advance();
            Spanned::new(name, span)
        }
        _ => {
            p.emit(p.peek().span, ErrorClass::UnexpectedToken, "expected identifier");
            Spanned::new(SmolStr::new("__error"), Span::empty(p.current_offset()))
        }
    };
    
    // Optional type annotation
    let ty = if p.at(&TokenKind::Colon) {
        p.advance();
        Some(parse_type_expr(p))
    } else {
        None
    };
    
    p.expect(&TokenKind::Eq);
    let value = parse_expr(p).node;
    
    Statement::Let { name, ty, value }
}

fn parse_return_stmt(p: &mut Parser<'_>) -> Statement {
    p.advance(); // consume 'return'
    
    let value = if !p.at(&TokenKind::RBrace) && !p.at_eof() {
        Some(parse_expr(p).node)
    } else {
        None
    };
    
    Statement::Return(value)
}
```

**Step 3: Update parse_impl_fn_decl to handle block bodies**

In `decl.rs`, update `parse_impl_fn_decl`:

```rust
fn parse_impl_fn_decl(p: &mut Parser<'_>) -> ImplFnDecl {
    p.advance(); // consume KwFn

    // Function name
    let name = parse_field_name(p)
        .unwrap_or_else(|| Spanned::new(SmolStr::new("__error"), Span::empty(p.current_offset())));

    // Parameter list: (name: Type, ...)
    p.expect(&TokenKind::LParen);

    let mut params = Vec::new();
    while !p.at(&TokenKind::RParen) && !p.at_eof() {
        params.push(parse_fn_param(p));

        // Optional comma separator
        if p.at(&TokenKind::Comma) {
            p.advance();
        } else if !p.at(&TokenKind::RParen) {
            p.emit(
                p.peek().span,
                ErrorClass::UnexpectedToken,
                "expected ',' or ')' in parameter list",
            );
            break;
        }
    }

    p.expect(&TokenKind::RParen);

    // Optional return type: -> Type
    let return_type = if p.at(&TokenKind::Minus) {
        // Check for -> (minus followed by greater-than)
        if p.tokens
            .get(p.pos + 1)
            .is_some_and(|t| matches!(t.kind, TokenKind::RAngle))
        {
            p.advance(); // consume Minus
            p.advance(); // consume RAngle (which acts as '>' in this context)
            Some(parse_type_expr(p))
        } else {
            None
        }
    } else {
        None
    };

    // Parse body: either external (no body) or block
    let body = if p.at(&TokenKind::LBrace) {
        p.advance(); // consume LBrace
        
        let mut statements = Vec::new();
        while !p.at(&TokenKind::RBrace) && !p.at_eof() {
            if let Some(stmt) = parse_statement(p) {
                statements.push(stmt);
            }
        }
        
        p.expect(&TokenKind::RBrace);
        ImplFnBody::Block(statements)
    } else {
        // External function - no body needed
        ImplFnBody::External
    };

    ImplFnDecl {
        annotations: Vec::new(),
        name,
        params,
        return_type,
        body,
    }
}
```

**Step 4: Build and verify**

```bash
cargo build -p vexil-lang
```

---

## Task 3: Add Expression IR and Lowering

**Files:**
- Modify: `crates/vexil-lang/src/ir/mod.rs`
- Modify: `crates/vexil-lang/src/lower.rs`

**Step 1: Add expression IR types**

```rust
// Add to ir/mod.rs after FnBody

/// IR expression.
#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    UInt(u64),
    Float(f64),
    Bool(bool),
    String(String),
    /// Local variable or parameter reference.
    Local(SmolStr),
    /// Field access on self or another expression.
    FieldAccess(Box<Expr>, SmolStr),
    /// Function call (resolved to specific function).
    Call(SmolStr, Vec<Expr>),
    /// Trait method call - will be resolved to specific impl.
    TraitMethodCall {
        trait_name: SmolStr,
        method_name: SmolStr,
        receiver: Box<Expr>,
        args: Vec<Expr>,
    },
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Unary(UnaryOp, Box<Expr>),
    /// Self reference.
    SelfRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add, Sub, Mul, Div,
    Eq, Ne, Lt, Le, Gt, Ge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg, Not,
}

/// IR statement.
#[derive(Debug, Clone)]
pub enum Statement {
    Expr(Expr),
    Let { name: SmolStr, ty: Option<ResolvedType>, value: Expr },
    Return(Option<Expr>),
    Assign { target: Expr, value: Expr },
}
```

**Step 2: Update FnBody to use IR statements**

```rust
pub enum FnBody {
    External,
    Block(Vec<Statement>),
}
```

**Step 3: Add lowering for expressions**

In `lower.rs`, add `lower_expr` function:

```rust
fn lower_expr(expr: &crate::ast::Expr, ctx: &mut LowerCtx) -> crate::ir::Expr {
    use crate::ast::Expr as AstExpr;
    
    match expr {
        AstExpr::Int(v) => crate::ir::Expr::Int(*v),
        AstExpr::UInt(v) => crate::ir::Expr::UInt(*v),
        AstExpr::Float(v) => crate::ir::Expr::Float(*v),
        AstExpr::Bool(v) => crate::ir::Expr::Bool(*v),
        AstExpr::String(s) => crate::ir::Expr::String(s.clone()),
        AstExpr::Ident(name) => crate::ir::Expr::Local(name.clone()),
        AstExpr::SelfRef => crate::ir::Expr::SelfRef,
        AstExpr::FieldAccess(obj, field) => {
            let obj = lower_expr(obj, ctx);
            crate::ir::Expr::FieldAccess(Box::new(obj), field.node.clone())
        }
        AstExpr::Call(func, args) => {
            let func_name = match func.as_ref() {
                AstExpr::Ident(name) => name.clone(),
                _ => SmolStr::new("__error"),
            };
            let args = args.iter().map(|a| lower_expr(a, ctx)).collect();
            crate::ir::Expr::Call(func_name, args)
        }
        AstExpr::MethodCall(receiver, method, args) => {
            let receiver = lower_expr(receiver, ctx);
            let args: Vec<_> = args.iter().map(|a| lower_expr(a, ctx)).collect();
            
            // For now, emit trait method call - resolution happens later
            crate::ir::Expr::TraitMethodCall {
                trait_name: SmolStr::new("__unresolved"), // filled in by typeck
                method_name: method.node.clone(),
                receiver: Box::new(receiver),
                args,
            }
        }
        AstExpr::Binary(op, lhs, rhs) => {
            let lhs = lower_expr(lhs, ctx);
            let rhs = lower_expr(rhs, ctx);
            let ir_op = lower_bin_op(*op);
            crate::ir::Expr::Binary(ir_op, Box::new(lhs), Box::new(rhs))
        }
        AstExpr::Unary(op, expr) => {
            let expr = lower_expr(expr, ctx);
            let ir_op = lower_unary_op(*op);
            crate::ir::Expr::Unary(ir_op, Box::new(expr))
        }
    }
}

fn lower_bin_op(op: crate::ast::BinOpKind) -> crate::ir::BinOp {
    use crate::ast::BinOpKind as Ast;
    use crate::ir::BinOp as Ir;
    
    match op {
        Ast::Add => Ir::Add,
        Ast::Sub => Ir::Sub,
        Ast::Mul => Ir::Mul,
        Ast::Div => Ir::Div,
        Ast::Eq => Ir::Eq,
        Ast::Ne => Ir::Ne,
        Ast::Lt => Ir::Lt,
        Ast::Le => Ir::Le,
        Ast::Gt => Ir::Gt,
        Ast::Ge => Ir::Ge,
    }
}

fn lower_unary_op(op: crate::ast::UnaryOpKind) -> crate::ir::UnaryOp {
    use crate::ast::UnaryOpKind as Ast;
    use crate::ir::UnaryOp as Ir;
    
    match op {
        Ast::Neg => Ir::Neg,
        Ast::Not => Ir::Not,
    }
}
```

**Step 4: Update lower_impl to handle block bodies**

```rust
// In lower_impl, when lowering functions:
let body = match &f.body {
    crate::ast::ImplFnBody::External => crate::ir::FnBody::External,
    crate::ast::ImplFnBody::Block(stmts) => {
        let ir_stmts = stmts.iter().map(|s| lower_statement(s, ctx)).collect();
        crate::ir::FnBody::Block(ir_stmts)
    }
};
```

Add `lower_statement`:

```rust
fn lower_statement(
    stmt: &crate::ast::Statement,
    ctx: &mut LowerCtx,
) -> crate::ir::Statement {
    use crate::ast::Statement as Ast;
    
    match stmt {
        Ast::Expr(e) => crate::ir::Statement::Expr(lower_expr(e, ctx)),
        Ast::Let { name, ty, value } => crate::ir::Statement::Let {
            name: name.node.clone(),
            ty: ty.as_ref().map(|t| resolve_type_expr(&t.node, name.span, ctx)),
            value: lower_expr(value, ctx),
        },
        Ast::Return(v) => crate::ir::Statement::Return(v.as_ref().map(|e| lower_expr(e, ctx))),
        Ast::Assign { target, value } => crate::ir::Statement::Assign {
            target: lower_expr(target, ctx),
            value: lower_expr(value, ctx),
        },
    }
}
```

---

## Task 4: Implement Static Trait Dispatch (Monomorphization)

**Files:**
- Modify: `crates/vexil-lang/src/typeck.rs`

**Step 1: Resolve trait method calls**

Add to typeck:

```rust
/// Resolve trait method calls to specific impl functions.
pub fn resolve_trait_calls(ctx: &mut TypeckContext, diags: &mut Vec<Diagnostic>) {
    // For each impl, check that method calls resolve to the right trait
    for &id in &ctx.schema.declarations {
        if let Some(type_def) = ctx.schema.registry.get(id) {
            if let ir::TypeDef::Impl(impl_def) = type_def {
                resolve_impl_trait_calls(impl_def, ctx, diags);
            }
        }
    }
}

fn resolve_impl_trait_calls(
    impl_def: &ir::ImplDef,
    ctx: &TypeckContext,
    diags: &mut Vec<Diagnostic>,
) {
    // Find the trait definition
    let trait_def = ctx.schema.declarations.iter()
        .filter_map(|&id| ctx.schema.registry.get(id))
        .find_map(|d| match d {
            ir::TypeDef::Trait(t) if t.name == impl_def.trait_name => Some(t),
            _ => None,
        });
    
    let Some(trait_def) = trait_def else {
        return; // Error already reported during conformance checking
    };
    
    // For each function in the impl, verify it matches a trait function
    for impl_fn in &impl_def.functions {
        let trait_fn = trait_def.functions.iter()
            .find(|f| f.name == impl_fn.name);
        
        if trait_fn.is_none() {
            diags.push(Diagnostic::error(
                impl_def.span,
                ErrorClass::TypeMismatch,
                format!("impl function '{}' not found in trait '{}'", impl_fn.name, impl_def.trait_name),
            ));
        }
    }
}
```

---

## Task 5: Generate Function Body Code (Rust Backend)

**Files:**
- Modify: `crates/vexil-codegen-rust/src/message.rs` or new file

Add function body generation:

```rust
/// Generate function body code.
fn emit_fn_body(body: &ir::FnBody, ctx: &mut EmitCtx) {
    match body {
        ir::FnBody::External => {
            // External function - generate unimplemented!() or FFI placeholder
            ctx.emit_line("unimplemented!(\"external function\")");
        }
        ir::FnBody::Block(stmts) => {
            ctx.emit_line("{");
            ctx.indent();
            for stmt in stmts {
                emit_statement(stmt, ctx);
            }
            ctx.dedent();
            ctx.emit_line("}");
        }
    }
}

fn emit_statement(stmt: &ir::Statement, ctx: &mut EmitCtx) {
    match stmt {
        ir::Statement::Expr(expr) => {
            let code = emit_expr(expr);
            ctx.emit_line(&format!("{};", code));
        }
        ir::Statement::Let { name, ty, value } => {
            let val_code = emit_expr(value);
            if let Some(t) = ty {
                let ty_code = rust_type(t);
                ctx.emit_line(&format!("let {}: {} = {};", name, ty_code, val_code));
            } else {
                ctx.emit_line(&format!("let {} = {};", name, val_code));
            }
        }
        ir::Statement::Return(None) => {
            ctx.emit_line("return;");
        }
        ir::Statement::Return(Some(expr)) => {
            let val = emit_expr(expr);
            ctx.emit_line(&format!("return {};", val));
        }
        ir::Statement::Assign { target, value } => {
            let target_code = emit_expr(target);
            let val_code = emit_expr(value);
            ctx.emit_line(&format!("{} = {};", target_code, val_code));
        }
    }
}

fn emit_expr(expr: &ir::Expr) -> String {
    match expr {
        ir::Expr::Int(v) => v.to_string(),
        ir::Expr::UInt(v) => v.to_string(),
        ir::Expr::Float(v) => v.to_string(),
        ir::Expr::Bool(v) => v.to_string(),
        ir::Expr::String(s) => format!("\"{}\".to_string()", s),
        ir::Expr::Local(name) => name.to_string(),
        ir::Expr::SelfRef => "self".to_string(),
        ir::Expr::FieldAccess(obj, field) => {
            let obj_code = emit_expr(obj);
            format!("{}.{}", obj_code, field)
        }
        ir::Expr::Call(name, args) => {
            let args_code: Vec<_> = args.iter().map(emit_expr).collect();
            format!("{}({})", name, args_code.join(", "))
        }
        ir::Expr::TraitMethodCall { trait_name: _, method_name, receiver, args } => {
            // Static dispatch: generate direct call to impl function
            let recv_code = emit_expr(receiver);
            let args_code: Vec<_> = args.iter().map(emit_expr).collect();
            format!("{}({})", method_name, args_code.join(", "))
        }
        ir::Expr::Binary(op, lhs, rhs) => {
            let op_str = match op {
                ir::BinOp::Add => "+",
                ir::BinOp::Sub => "-",
                ir::BinOp::Mul => "*",
                ir::BinOp::Div => "/",
                ir::BinOp::Eq => "==",
                ir::BinOp::Ne => "!=",
                ir::BinOp::Lt => "<",
                ir::BinOp::Le => "<=",
                ir::BinOp::Gt => ">",
                ir::BinOp::Ge => ">=",
            };
            let lhs_code = emit_expr(lhs);
            let rhs_code = emit_expr(rhs);
            format!("({} {} {})", lhs_code, op_str, rhs_code)
        }
        ir::Expr::Unary(op, expr) => {
            let op_str = match op {
                ir::UnaryOp::Neg => "-",
                ir::UnaryOp::Not => "!",
            };
            let expr_code = emit_expr(expr);
            format!("{}{}", op_str, expr_code)
        }
    }
}
```

---

## Task 6: Test End-to-End

Create test schema:

```vexil
trait Calculable {
    value @1 : i32
    
    fn calculate(x: i32) -> i32
}

message Point {
    value @1 : i32
    y @2 : i32
}

impl Calculable for Point {
    fn calculate(x: i32) -> i32 {
        let result = self.value + x
        return result
    }
}
```

Verify:
1. Parser accepts the syntax
2. Lowering creates IR with function body
3. Typeck validates trait conformance
4. Rust codegen generates working code
5. Tests pass

---

**Estimated Time:** 4-6 hours with subagent delegation
