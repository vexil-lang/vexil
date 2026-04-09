use super::Parser;
use crate::ast::{DefaultValue, PrimitiveType, SemanticType, SubByteType, TypeExpr};
use crate::diagnostic::ErrorClass;
use crate::lexer::token::TokenKind;
use crate::span::Spanned;

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
