use super::Parser;
use crate::ast::{
    BinOpKind, DefaultValue, Expr, PrimitiveType, SemanticType, Statement, SubByteType, TypeExpr,
    UnaryOpKind,
};
use crate::diagnostic::ErrorClass;
use crate::lexer::token::TokenKind;
use crate::span::{Span, Spanned};
use smol_str::SmolStr;

// ---------------------------------------------------------------------------
// Type expression parsing
// ---------------------------------------------------------------------------

/// Parse a type expression.
///
/// Grammar:
/// ```text
/// type_expr = "optional" "<" type_expr ">"
///           | "array"    "<" type_expr ">"
///           | "map"      "<" type_expr "," type_expr ">"
///           | "result"   "<" type_expr "," type_expr ">"
///           | "vec2"     "<" type_expr ">"
///           | "vec3"     "<" type_expr ">"
///           | "vec4"     "<" type_expr ">"
///           | "quat"     "<" type_expr ">"
///           | "mat3"     "<" type_expr ">"
///           | "mat4"     "<" type_expr ">"
///           | named_type
/// ```
pub(crate) fn parse_type_expr(p: &mut Parser<'_>) -> Spanned<TypeExpr> {
    let start = p.current_offset();

    match p.peek_kind().clone() {
        TokenKind::KwOptional => {
            p.advance();
            p.expect(&TokenKind::LAngle);
            let inner = parse_type_expr(p);
            p.expect(&TokenKind::RAngle);
            let span = p.span_from(start);
            Spanned::new(TypeExpr::Optional(Box::new(inner)), span)
        }
        TokenKind::KwArray => {
            p.advance();
            p.expect(&TokenKind::LAngle);
            let inner = parse_type_expr(p);

            // Check for fixed array syntax: array<T, N>
            let is_fixed = p.at(&TokenKind::Comma);
            if is_fixed {
                p.advance(); // consume Comma
                let size_token = p.advance();
                let size = match &size_token.kind {
                    TokenKind::DecInt(v) => *v,
                    TokenKind::HexInt(v) => *v,
                    _ => {
                        p.emit(
                            size_token.span,
                            ErrorClass::UnexpectedToken,
                            "expected integer literal for fixed array size",
                        );
                        0
                    }
                };
                p.expect(&TokenKind::RAngle);
                let span = p.span_from(start);
                Spanned::new(TypeExpr::FixedArray(Box::new(inner), size), span)
            } else {
                p.expect(&TokenKind::RAngle);
                let span = p.span_from(start);
                Spanned::new(TypeExpr::Array(Box::new(inner)), span)
            }
        }
        TokenKind::KwSet => {
            p.advance();
            p.expect(&TokenKind::LAngle);
            let inner = parse_type_expr(p);
            p.expect(&TokenKind::RAngle);
            let span = p.span_from(start);
            Spanned::new(TypeExpr::Set(Box::new(inner)), span)
        }
        TokenKind::KwMap => {
            p.advance();
            p.expect(&TokenKind::LAngle);
            let key = parse_type_expr(p);
            p.expect(&TokenKind::Comma);
            let value = parse_type_expr(p);
            p.expect(&TokenKind::RAngle);
            let span = p.span_from(start);
            Spanned::new(TypeExpr::Map(Box::new(key), Box::new(value)), span)
        }
        TokenKind::KwResult => {
            p.advance();
            p.expect(&TokenKind::LAngle);
            let ok = parse_type_expr(p);
            p.expect(&TokenKind::Comma);
            let err = parse_type_expr(p);
            p.expect(&TokenKind::RAngle);
            let span = p.span_from(start);
            Spanned::new(TypeExpr::Result(Box::new(ok), Box::new(err)), span)
        }
        TokenKind::KwVec2 => {
            p.advance();
            p.expect(&TokenKind::LAngle);
            let inner = parse_type_expr(p);
            p.expect(&TokenKind::RAngle);
            let span = p.span_from(start);
            Spanned::new(TypeExpr::Vec2(Box::new(inner)), span)
        }
        TokenKind::KwVec3 => {
            p.advance();
            p.expect(&TokenKind::LAngle);
            let inner = parse_type_expr(p);
            p.expect(&TokenKind::RAngle);
            let span = p.span_from(start);
            Spanned::new(TypeExpr::Vec3(Box::new(inner)), span)
        }
        TokenKind::KwVec4 => {
            p.advance();
            p.expect(&TokenKind::LAngle);
            let inner = parse_type_expr(p);
            p.expect(&TokenKind::RAngle);
            let span = p.span_from(start);
            Spanned::new(TypeExpr::Vec4(Box::new(inner)), span)
        }
        TokenKind::KwQuat => {
            p.advance();
            p.expect(&TokenKind::LAngle);
            let inner = parse_type_expr(p);
            p.expect(&TokenKind::RAngle);
            let span = p.span_from(start);
            Spanned::new(TypeExpr::Quat(Box::new(inner)), span)
        }
        TokenKind::KwMat3 => {
            p.advance();
            p.expect(&TokenKind::LAngle);
            let inner = parse_type_expr(p);
            p.expect(&TokenKind::RAngle);
            let span = p.span_from(start);
            Spanned::new(TypeExpr::Mat3(Box::new(inner)), span)
        }
        TokenKind::KwMat4 => {
            p.advance();
            p.expect(&TokenKind::LAngle);
            let inner = parse_type_expr(p);
            p.expect(&TokenKind::RAngle);
            let span = p.span_from(start);
            Spanned::new(TypeExpr::Mat4(Box::new(inner)), span)
        }
        TokenKind::KwBits => {
            p.advance();
            p.expect(&TokenKind::LBrace);
            let mut names = Vec::new();
            // Parse at least one identifier
            if let Some(name) = p.peek_kind().as_field_name() {
                p.advance();
                names.push(name);
                // Parse additional comma-separated identifiers
                while p.at(&TokenKind::Comma) {
                    p.advance(); // consume comma
                    if let Some(name) = p.peek_kind().as_field_name() {
                        p.advance();
                        names.push(name);
                    } else {
                        break;
                    }
                }
            }
            p.expect(&TokenKind::RBrace);
            let span = p.span_from(start);
            Spanned::new(TypeExpr::BitsInline(names), span)
        }
        _ => parse_named_type(p),
    }
}

/// Parse a named/primitive/semantic/sub-byte type.
fn parse_named_type(p: &mut Parser<'_>) -> Spanned<TypeExpr> {
    let start = p.current_offset();

    match p.peek_kind().clone() {
        TokenKind::UpperIdent(name) => {
            let tok = p.advance();
            // Check for qualified: UpperIdent.UpperIdent
            let is_qualified = p.at(&TokenKind::Dot)
                && p.tokens
                    .get(p.pos + 1)
                    .is_some_and(|t| matches!(t.kind, TokenKind::UpperIdent(_)));
            if is_qualified {
                p.advance(); // consume Dot
                if let TokenKind::UpperIdent(member) = p.peek_kind().clone() {
                    p.advance();
                    let span = p.span_from(start);
                    return Spanned::new(TypeExpr::Qualified(name, member), span);
                }
            }
            // Check for generic type instantiation: Name<TypeArg>
            let is_generic = p.at(&TokenKind::LAngle);
            if is_generic {
                p.advance(); // consume LAngle
                let arg = parse_type_expr(p);
                p.expect(&TokenKind::RAngle);
                let span = p.span_from(start);
                return Spanned::new(TypeExpr::Generic(name, Box::new(arg)), span);
            }
            Spanned::new(TypeExpr::Named(name), tok.span)
        }
        TokenKind::Ident(s) => {
            let s_ref = s.as_str();
            let ty = match s_ref {
                "bool" => TypeExpr::Primitive(PrimitiveType::Bool),
                "u8" => TypeExpr::Primitive(PrimitiveType::U8),
                "u16" => TypeExpr::Primitive(PrimitiveType::U16),
                "u32" => TypeExpr::Primitive(PrimitiveType::U32),
                "u64" => TypeExpr::Primitive(PrimitiveType::U64),
                "i8" => TypeExpr::Primitive(PrimitiveType::I8),
                "i16" => TypeExpr::Primitive(PrimitiveType::I16),
                "i32" => TypeExpr::Primitive(PrimitiveType::I32),
                "i64" => TypeExpr::Primitive(PrimitiveType::I64),
                "f32" => TypeExpr::Primitive(PrimitiveType::F32),
                "f64" => TypeExpr::Primitive(PrimitiveType::F64),
                "fixed32" => TypeExpr::Primitive(PrimitiveType::Fixed32),
                "fixed64" => TypeExpr::Primitive(PrimitiveType::Fixed64),
                "void" => TypeExpr::Primitive(PrimitiveType::Void),
                "string" => TypeExpr::Semantic(SemanticType::String),
                "bytes" => TypeExpr::Semantic(SemanticType::Bytes),
                "rgb" => TypeExpr::Semantic(SemanticType::Rgb),
                "uuid" => TypeExpr::Semantic(SemanticType::Uuid),
                "timestamp" => TypeExpr::Semantic(SemanticType::Timestamp),
                "hash" => TypeExpr::Semantic(SemanticType::Hash),
                _ => {
                    // Check for sub-byte pattern: u/i followed by digits
                    if let Some(ty) = try_parse_sub_byte(s_ref) {
                        ty
                    } else {
                        // User-defined type (forward ref as lowercase)
                        TypeExpr::Named(s.clone())
                    }
                }
            };
            let tok = p.advance();
            Spanned::new(ty, tok.span)
        }
        _ => {
            let span = p.peek().span;
            p.emit(
                span,
                ErrorClass::UnexpectedToken,
                "expected type expression",
            );
            Spanned::new(TypeExpr::Primitive(PrimitiveType::Void), span)
        }
    }
}

/// Try to parse a sub-byte type from an identifier string like "u3" or "i7".
fn try_parse_sub_byte(s: &str) -> Option<TypeExpr> {
    let (prefix, rest) = if let Some(rest) = s.strip_prefix('u') {
        (false, rest)
    } else if let Some(rest) = s.strip_prefix('i') {
        (true, rest)
    } else {
        return None;
    };

    // Must be all digits after prefix
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }

    let bits: u8 = rest.parse().ok()?;

    // Standard widths are handled as primitives, not sub-byte
    if matches!(bits, 8 | 16 | 32 | 64) {
        return None;
    }

    Some(TypeExpr::SubByte(SubByteType {
        signed: prefix,
        bits,
    }))
}

// ---------------------------------------------------------------------------
// Literal / default value parsing (for config fields)
// ---------------------------------------------------------------------------

/// Parse a literal/default value for config fields.
pub(crate) fn parse_literal_value(p: &mut Parser<'_>) -> Spanned<DefaultValue> {
    let start = p.current_offset();

    match p.peek_kind().clone() {
        TokenKind::KwNone => {
            let tok = p.advance();
            Spanned::new(DefaultValue::None, tok.span)
        }
        TokenKind::KwTrue => {
            let tok = p.advance();
            Spanned::new(DefaultValue::Bool(true), tok.span)
        }
        TokenKind::KwFalse => {
            let tok = p.advance();
            Spanned::new(DefaultValue::Bool(false), tok.span)
        }
        TokenKind::LBracket => {
            p.advance(); // consume [
            let mut items = Vec::new();
            while !p.at(&TokenKind::RBracket) && !p.at_eof() {
                items.push(parse_literal_value(p));
                if p.at(&TokenKind::Comma) {
                    p.advance();
                }
            }
            p.expect(&TokenKind::RBracket);
            let span = p.span_from(start);
            Spanned::new(DefaultValue::Array(items), span)
        }
        TokenKind::HexInt(v) => {
            let tok = p.advance();
            Spanned::new(DefaultValue::UInt(v), tok.span)
        }
        TokenKind::FloatLit(v) => {
            let tok = p.advance();
            Spanned::new(DefaultValue::Float(v), tok.span)
        }
        TokenKind::Minus => {
            p.advance(); // consume Minus
            match p.peek_kind().clone() {
                TokenKind::DecInt(v) => {
                    p.advance();
                    let span = p.span_from(start);
                    Spanned::new(DefaultValue::Int(-(v as i64)), span)
                }
                TokenKind::FloatLit(v) => {
                    p.advance();
                    let span = p.span_from(start);
                    Spanned::new(DefaultValue::Float(-v), span)
                }
                _ => {
                    let span = p.span_from(start);
                    p.emit(
                        span,
                        ErrorClass::UnexpectedToken,
                        "expected number after `-`",
                    );
                    Spanned::new(DefaultValue::Int(0), span)
                }
            }
        }
        TokenKind::DecInt(v) => {
            let tok = p.advance();
            Spanned::new(DefaultValue::UInt(v), tok.span)
        }
        TokenKind::StringLit(s) => {
            let tok = p.advance();
            Spanned::new(DefaultValue::Str(s), tok.span)
        }
        TokenKind::UpperIdent(s) => {
            let tok = p.advance();
            Spanned::new(DefaultValue::UpperIdent(s), tok.span)
        }
        TokenKind::Ident(s) => {
            let tok = p.advance();
            Spanned::new(DefaultValue::Ident(s), tok.span)
        }
        _ => {
            let span = p.peek().span;
            p.emit(span, ErrorClass::UnexpectedToken, "expected default value");
            Spanned::new(DefaultValue::None, span)
        }
    }
}

// ---------------------------------------------------------------------------
// Expression parsing
// ---------------------------------------------------------------------------

/// Parse a primary expression (literals, identifiers, self, parens).
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
        TokenKind::FloatLit(v) => {
            p.advance();
            Spanned::new(Expr::Float(v), p.span_from(start))
        }
        TokenKind::Ident(s) | TokenKind::UpperIdent(s) => {
            p.advance();
            let expr = Expr::Ident(s.clone());

            // Check for field access or method call
            let mut expr = Spanned::new(expr, p.span_from(start));
            loop {
                if p.at(&TokenKind::Dot) {
                    p.advance();
                    let field_name = match p.peek_kind().as_field_name() {
                        Some(name) => {
                            let span = p.peek().span;
                            p.advance();
                            Spanned::new(name, span)
                        }
                        None => {
                            p.emit(
                                p.peek().span,
                                ErrorClass::UnexpectedToken,
                                "expected field name",
                            );
                            Spanned::new(SmolStr::new("__error"), Span::empty(p.current_offset()))
                        }
                    };

                    // Check if this is a method call
                    if p.at(&TokenKind::LParen) {
                        let args = parse_call_args(p);
                        expr = Spanned::new(
                            Expr::MethodCall(Box::new(expr.node), field_name, args),
                            p.span_from(start),
                        );
                    } else {
                        expr = Spanned::new(
                            Expr::FieldAccess(Box::new(expr.node), field_name),
                            p.span_from(start),
                        );
                    }
                } else if p.at(&TokenKind::LParen) {
                    // Function call
                    let args = parse_call_args(p);
                    expr = Spanned::new(Expr::Call(Box::new(expr.node), args), p.span_from(start));
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
        TokenKind::KwResult => {
            // 'result' keyword can be used as an identifier (e.g., variable name)
            p.advance();
            Spanned::new(Expr::Ident(SmolStr::new("result")), p.span_from(start))
        }
        TokenKind::KwSelf => {
            p.advance();
            let mut expr = Spanned::new(Expr::SelfRef, p.span_from(start));
            // Handle field access and method calls on self
            loop {
                if p.at(&TokenKind::Dot) {
                    p.advance();
                    let field_name = match p.peek_kind().as_field_name() {
                        Some(name) => {
                            let span = p.peek().span;
                            p.advance();
                            Spanned::new(name, span)
                        }
                        None => {
                            p.emit(
                                p.peek().span,
                                ErrorClass::UnexpectedToken,
                                "expected field name",
                            );
                            Spanned::new(SmolStr::new("__error"), Span::empty(p.current_offset()))
                        }
                    };

                    // Check if this is a method call
                    if p.at(&TokenKind::LParen) {
                        let args = parse_call_args(p);
                        expr = Spanned::new(
                            Expr::MethodCall(Box::new(expr.node), field_name, args),
                            p.span_from(start),
                        );
                    } else {
                        expr = Spanned::new(
                            Expr::FieldAccess(Box::new(expr.node), field_name),
                            p.span_from(start),
                        );
                    }
                } else {
                    break;
                }
            }
            expr
        }
        TokenKind::LParen => {
            p.advance();
            let expr = parse_expr(p);
            p.expect(&TokenKind::RParen);
            expr
        }
        TokenKind::Minus => {
            p.advance();
            let expr = parse_primary_expr(p);
            Spanned::new(
                Expr::Unary(UnaryOpKind::Neg, Box::new(expr.node)),
                p.span_from(start),
            )
        }
        TokenKind::Bang => {
            p.advance();
            let expr = parse_primary_expr(p);
            Spanned::new(
                Expr::Unary(UnaryOpKind::Not, Box::new(expr.node)),
                p.span_from(start),
            )
        }
        _ => {
            p.emit(
                p.peek().span,
                ErrorClass::UnexpectedToken,
                "expected expression",
            );
            Spanned::new(Expr::Int(0), Span::empty(p.current_offset()))
        }
    }
}

/// Parse function call arguments.
pub(crate) fn parse_call_args(p: &mut Parser<'_>) -> Vec<Expr> {
    p.expect(&TokenKind::LParen);
    let mut args = Vec::new();

    while !p.at(&TokenKind::RParen) && !p.at_eof() {
        args.push(parse_expr(p).node);

        if p.at(&TokenKind::Comma) {
            p.advance();
        } else if !p.at(&TokenKind::RParen) {
            p.emit(
                p.peek().span,
                ErrorClass::UnexpectedToken,
                "expected ',' or ')'",
            );
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
            p.span_from(start),
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
        TokenKind::LAngle => Some(BinOpKind::Lt),
        TokenKind::Le => Some(BinOpKind::Le),
        TokenKind::RAngle => Some(BinOpKind::Gt),
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
    }
}

// ---------------------------------------------------------------------------
// Statement parsing
// ---------------------------------------------------------------------------

/// Parse a statement.
pub(crate) fn parse_statement(p: &mut Parser<'_>) -> Option<Statement> {
    match p.peek_kind() {
        TokenKind::KwLet => Some(parse_let_stmt(p)),
        TokenKind::KwReturn => Some(parse_return_stmt(p)),
        _ => {
            // Try as expression statement or assignment
            let start_pos = p.current_offset();
            let expr = parse_expr(p);
            // Ensure we always make progress to prevent infinite loops
            if p.current_offset() == start_pos {
                // Expression parsing didn't advance - skip the problematic token
                p.advance();
                return None;
            }
            if p.at(&TokenKind::Eq) {
                p.advance();
                let value = parse_expr(p);
                Some(Statement::Assign {
                    target: expr.node,
                    value: value.node,
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
        // Keywords that can be used as identifiers (variable names)
        TokenKind::KwResult => {
            let span = p.peek().span;
            p.advance();
            Spanned::new(SmolStr::new("result"), span)
        }
        _ => {
            p.emit(
                p.peek().span,
                ErrorClass::UnexpectedToken,
                "expected identifier",
            );
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
