use crate::ast::{
    Annotation, ConfigDecl, ConfigField, Decl, EnumBacking, EnumBodyItem, EnumDecl, EnumVariant,
    FlagsBit, FlagsBodyItem, FlagsDecl, MessageBodyItem, MessageDecl, MessageField, NewtypeDecl,
    Tombstone, TombstoneArg, UnionBodyItem, UnionDecl, UnionVariant,
};
use crate::diagnostic::ErrorClass;
use crate::lexer::token::TokenKind;
use crate::span::{Span, Spanned};
use smol_str::SmolStr;

use super::expr::{parse_literal_value, parse_type_expr};
use super::{parse_annotation_value, parse_annotations, Parser};

// ---------------------------------------------------------------------------
// Top-level dispatch
// ---------------------------------------------------------------------------

/// Parse a type declaration given its pre-annotations.
pub(crate) fn parse_type_decl(annotations: Vec<Annotation>, p: &mut Parser<'_>) -> Spanned<Decl> {
    let start = if let Some(first) = annotations.first() {
        first.span.offset as usize
    } else {
        p.current_offset()
    };

    match p.peek_kind().clone() {
        TokenKind::KwMessage => {
            let msg = parse_message_decl(annotations, p);
            let span = p.span_from(start);
            Spanned::new(Decl::Message(msg), span)
        }
        TokenKind::KwEnum => {
            let en = parse_enum_decl(annotations, p);
            let span = p.span_from(start);
            Spanned::new(Decl::Enum(en), span)
        }
        TokenKind::KwFlags => {
            let fl = parse_flags_decl(annotations, p);
            let span = p.span_from(start);
            Spanned::new(Decl::Flags(fl), span)
        }
        TokenKind::KwUnion => {
            let un = parse_union_decl(annotations, p);
            let span = p.span_from(start);
            Spanned::new(Decl::Union(un), span)
        }
        TokenKind::KwNewtype => {
            let nt = parse_newtype_decl(annotations, p);
            let span = p.span_from(start);
            Spanned::new(Decl::Newtype(nt), span)
        }
        TokenKind::KwConfig => {
            let cfg = parse_config_decl(annotations, p);
            let span = p.span_from(start);
            Spanned::new(Decl::Config(cfg), span)
        }
        _ => {
            // Unknown declaration keyword — skip to recover.
            p.emit(
                p.peek().span,
                ErrorClass::UnexpectedToken,
                "expected declaration keyword",
            );
            p.advance();
            let span = p.span_from(start);
            Spanned::new(
                Decl::Message(MessageDecl {
                    annotations,
                    name: Spanned::new(SmolStr::new("__error"), span),
                    body: Vec::new(),
                }),
                span,
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Declaration name helper
// ---------------------------------------------------------------------------

/// Consume a declaration name. Must be UpperIdent. Emits DeclNameInvalid for lowercase.
fn parse_decl_name(p: &mut Parser<'_>) -> Option<Spanned<SmolStr>> {
    match p.peek_kind().clone() {
        TokenKind::UpperIdent(s) => {
            let tok = p.advance();
            // Validate: must match [A-Z][A-Za-z0-9]* — no underscores
            if s.contains('_') {
                p.emit(
                    tok.span,
                    ErrorClass::DeclNameInvalid,
                    format!("declaration name `{s}` must match [A-Z][A-Za-z0-9]* (no underscores)"),
                );
            }
            Some(Spanned::new(s, tok.span))
        }
        TokenKind::Ident(s) => {
            let tok = p.advance();
            p.emit(
                tok.span,
                ErrorClass::DeclNameInvalid,
                format!("declaration name `{s}` must start with an uppercase letter"),
            );
            Some(Spanned::new(s, tok.span))
        }
        _ => {
            p.emit(
                p.peek().span,
                ErrorClass::UnexpectedToken,
                "expected declaration name",
            );
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Message
// ---------------------------------------------------------------------------

fn parse_message_decl(annotations: Vec<Annotation>, p: &mut Parser<'_>) -> MessageDecl {
    p.advance(); // consume KwMessage
    let name = parse_decl_name(p)
        .unwrap_or_else(|| Spanned::new(SmolStr::new("__error"), Span::empty(p.current_offset())));
    p.expect(&TokenKind::LBrace);

    let mut body = Vec::new();
    while !p.at(&TokenKind::RBrace) && !p.at_eof() {
        // Check for tombstone: @ followed by Ident("removed")
        if is_at_tombstone(p) {
            let ts = parse_tombstone(p);
            body.push(MessageBodyItem::Tombstone(ts));
            continue;
        }

        // Otherwise try to parse a field (with optional pre-annotations)
        let pre_annotations = parse_annotations(p);

        // After annotations, if we see RBrace or tombstone, annotations were trailing
        if p.at(&TokenKind::RBrace) {
            break;
        }
        if is_at_tombstone(p) {
            let ts = parse_tombstone(p);
            body.push(MessageBodyItem::Tombstone(ts));
            continue;
        }

        match parse_field(pre_annotations, p) {
            Some(field) => body.push(MessageBodyItem::Field(field)),
            None => {
                // Skip a token to avoid infinite loop
                if !p.at(&TokenKind::RBrace) && !p.at_eof() {
                    p.advance();
                }
            }
        }
    }

    p.expect(&TokenKind::RBrace);

    MessageDecl {
        annotations,
        name,
        body,
    }
}

// ---------------------------------------------------------------------------
// Field
// ---------------------------------------------------------------------------

fn parse_field(
    pre_annotations: Vec<Annotation>,
    p: &mut Parser<'_>,
) -> Option<Spanned<MessageField>> {
    let start = if let Some(first) = pre_annotations.first() {
        first.span.offset as usize
    } else {
        p.current_offset()
    };

    // Field name: accepts Ident or any keyword token
    let name = parse_field_name(p)?;

    // Validate: field name must start lowercase
    let name_str = name.node.as_str();
    if let Some(first_char) = name_str.chars().next() {
        if first_char.is_ascii_uppercase() {
            p.emit(
                name.span,
                ErrorClass::FieldNameInvalid,
                format!("field name `{name_str}` must start with a lowercase letter"),
            );
        }
    }

    // Ordinal
    let ordinal = match p.peek_kind().clone() {
        TokenKind::Ordinal(v) => {
            let tok = p.advance();
            Spanned::new(v, tok.span)
        }
        _ => {
            p.emit(
                p.peek().span,
                ErrorClass::UnexpectedToken,
                "expected ordinal (e.g. @0)",
            );
            return None;
        }
    };

    // Post-ordinal annotations
    let post_ordinal_annotations = parse_annotations(p);

    // Colon
    p.expect(&TokenKind::Colon);

    // Type expression
    let ty = parse_type_expr(p);

    // Post-type annotations (but not @removed — that's a tombstone)
    let post_type_annotations = if is_at_tombstone(p) {
        Vec::new()
    } else {
        parse_annotations(p)
    };

    let span = p.span_from(start);

    Some(Spanned::new(
        MessageField {
            pre_annotations,
            name,
            ordinal,
            post_ordinal_annotations,
            ty,
            post_type_annotations,
        },
        span,
    ))
}

/// Parse a field name — accepts Ident or any keyword (as field name).
fn parse_field_name(p: &mut Parser<'_>) -> Option<Spanned<SmolStr>> {
    match p.peek_kind().clone() {
        TokenKind::Ident(s) => {
            let tok = p.advance();
            Some(Spanned::new(s, tok.span))
        }
        ref kind if kind.is_keyword() => {
            if let Some(name) = kind.as_field_name() {
                let tok = p.advance();
                Some(Spanned::new(name, tok.span))
            } else {
                None
            }
        }
        TokenKind::UpperIdent(s) => {
            // Accept but will be flagged by name validation
            let tok = p.advance();
            Some(Spanned::new(s, tok.span))
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Tombstone
// ---------------------------------------------------------------------------

/// Check if we're at `@removed(...)` — At token followed by Ident("removed")
fn is_at_tombstone(p: &Parser<'_>) -> bool {
    if !p.at(&TokenKind::At) {
        return false;
    }
    p.tokens
        .get(p.pos + 1)
        .is_some_and(|t| matches!(&t.kind, TokenKind::Ident(s) if s == "removed"))
}

fn parse_tombstone(p: &mut Parser<'_>) -> Spanned<Tombstone> {
    let start = p.current_offset();
    p.advance(); // consume At
    p.advance(); // consume Ident("removed")

    p.expect(&TokenKind::LParen);

    // First arg: ordinal (DecInt)
    let ordinal = match p.peek_kind().clone() {
        TokenKind::DecInt(v) => {
            let tok = p.advance();
            Spanned::new(v as u32, tok.span)
        }
        _ => {
            p.emit(
                p.peek().span,
                ErrorClass::UnexpectedToken,
                "expected ordinal number in @removed",
            );
            Spanned::new(0, p.peek().span)
        }
    };

    // Optional comma + named args
    let mut args = Vec::new();
    if p.at(&TokenKind::Comma) {
        p.advance();
        // Parse named args: key: value, ...
        while !p.at(&TokenKind::RParen) && !p.at_eof() {
            let arg_start = p.current_offset();
            let key = match p.peek_kind().clone() {
                TokenKind::Ident(s) => {
                    let tok = p.advance();
                    Spanned::new(s, tok.span)
                }
                _ => {
                    break;
                }
            };

            p.expect(&TokenKind::Colon);

            let value = match parse_annotation_value(p) {
                Some(v) => v,
                None => break,
            };

            let span = p.span_from(arg_start);
            args.push(TombstoneArg { span, key, value });

            if p.at(&TokenKind::Comma) {
                p.advance();
            }
        }
    }

    p.expect(&TokenKind::RParen);

    // Check for missing reason
    let has_reason = args.iter().any(|a| a.key.node == "reason");
    if !has_reason {
        let span = p.span_from(start);
        p.emit(
            span,
            ErrorClass::RemovedMissingReason,
            "@removed must include a `reason` argument",
        );
    }

    // Optional original type annotation: `: u32`, `: string`, etc.
    let original_type = if p.at(&TokenKind::Colon) {
        p.advance(); // consume ':'
        let ty = parse_type_expr(p);
        Some(ty)
    } else {
        None
    };

    let span = p.span_from(start);
    Spanned::new(
        Tombstone {
            ordinal,
            args,
            original_type,
        },
        span,
    )
}

// ---------------------------------------------------------------------------
// Enum
// ---------------------------------------------------------------------------

fn parse_enum_decl(annotations: Vec<Annotation>, p: &mut Parser<'_>) -> EnumDecl {
    p.advance(); // consume KwEnum
    let name = parse_decl_name(p)
        .unwrap_or_else(|| Spanned::new(SmolStr::new("__error"), Span::empty(p.current_offset())));

    // Optional backing type: `: u8` / `: u16` / `: u32` / `: u64`
    let backing = if p.at(&TokenKind::Colon) {
        let backing_start = p.current_offset();
        p.advance(); // consume Colon
        match p.peek_kind().clone() {
            TokenKind::Ident(s) => {
                let tok = p.advance();
                let b = match s.as_str() {
                    "u8" => Some(EnumBacking::U8),
                    "u16" => Some(EnumBacking::U16),
                    "u32" => Some(EnumBacking::U32),
                    "u64" => Some(EnumBacking::U64),
                    _ => {
                        p.emit(
                            tok.span,
                            ErrorClass::EnumBackingInvalid,
                            format!("invalid enum backing type `{s}`, expected u8/u16/u32/u64"),
                        );
                        None
                    }
                };
                b.map(|b| {
                    let span = p.span_from(backing_start);
                    Spanned::new(b, span)
                })
            }
            _ => {
                p.emit(
                    p.peek().span,
                    ErrorClass::UnexpectedToken,
                    "expected backing type (u8, u16, u32, or u64)",
                );
                None
            }
        }
    } else {
        None
    };

    p.expect(&TokenKind::LBrace);

    let mut body = Vec::new();
    while !p.at(&TokenKind::RBrace) && !p.at_eof() {
        // Tombstone
        if is_at_tombstone(p) {
            let ts = parse_tombstone(p);
            body.push(EnumBodyItem::Tombstone(ts));
            continue;
        }

        // Annotations + variant
        let annotations = parse_annotations(p);

        if p.at(&TokenKind::RBrace) {
            break;
        }
        if is_at_tombstone(p) {
            let ts = parse_tombstone(p);
            body.push(EnumBodyItem::Tombstone(ts));
            continue;
        }

        match parse_enum_variant(annotations, p) {
            Some(v) => body.push(EnumBodyItem::Variant(v)),
            None => {
                if !p.at(&TokenKind::RBrace) && !p.at_eof() {
                    p.advance();
                }
            }
        }
    }

    p.expect(&TokenKind::RBrace);

    EnumDecl {
        annotations,
        name,
        backing,
        body,
    }
}

fn parse_enum_variant(
    annotations: Vec<Annotation>,
    p: &mut Parser<'_>,
) -> Option<Spanned<EnumVariant>> {
    let start = if let Some(first) = annotations.first() {
        first.span.offset as usize
    } else {
        p.current_offset()
    };

    let name = match p.peek_kind().clone() {
        TokenKind::UpperIdent(s) => {
            let tok = p.advance();
            Spanned::new(s, tok.span)
        }
        TokenKind::Ident(s) => {
            let tok = p.advance();
            p.emit(
                tok.span,
                ErrorClass::EnumVariantNameInvalid,
                format!("enum variant `{s}` must start with an uppercase letter"),
            );
            Spanned::new(s, tok.span)
        }
        _ => {
            return None;
        }
    };

    let ordinal = match p.peek_kind().clone() {
        TokenKind::Ordinal(v) => {
            let tok = p.advance();
            Spanned::new(v, tok.span)
        }
        _ => {
            p.emit(
                p.peek().span,
                ErrorClass::UnexpectedToken,
                "expected ordinal (e.g. @0) for enum variant",
            );
            return None;
        }
    };

    let span = p.span_from(start);
    Some(Spanned::new(
        EnumVariant {
            annotations,
            name,
            ordinal,
        },
        span,
    ))
}

// ---------------------------------------------------------------------------
// Flags
// ---------------------------------------------------------------------------

fn parse_flags_decl(annotations: Vec<Annotation>, p: &mut Parser<'_>) -> FlagsDecl {
    p.advance(); // consume KwFlags
    let name = parse_decl_name(p)
        .unwrap_or_else(|| Spanned::new(SmolStr::new("__error"), Span::empty(p.current_offset())));

    p.expect(&TokenKind::LBrace);

    let mut body = Vec::new();
    while !p.at(&TokenKind::RBrace) && !p.at_eof() {
        // Tombstone
        if is_at_tombstone(p) {
            let ts = parse_tombstone(p);
            body.push(FlagsBodyItem::Tombstone(ts));
            continue;
        }

        // Annotations + bit
        let annotations = parse_annotations(p);

        if p.at(&TokenKind::RBrace) {
            break;
        }
        if is_at_tombstone(p) {
            let ts = parse_tombstone(p);
            body.push(FlagsBodyItem::Tombstone(ts));
            continue;
        }

        match parse_flags_bit(annotations, p) {
            Some(b) => body.push(FlagsBodyItem::Bit(b)),
            None => {
                if !p.at(&TokenKind::RBrace) && !p.at_eof() {
                    p.advance();
                }
            }
        }
    }

    p.expect(&TokenKind::RBrace);

    FlagsDecl {
        annotations,
        name,
        body,
    }
}

fn parse_flags_bit(annotations: Vec<Annotation>, p: &mut Parser<'_>) -> Option<Spanned<FlagsBit>> {
    let start = if let Some(first) = annotations.first() {
        first.span.offset as usize
    } else {
        p.current_offset()
    };

    let name = match p.peek_kind().clone() {
        TokenKind::UpperIdent(s) => {
            let tok = p.advance();
            Spanned::new(s, tok.span)
        }
        TokenKind::Ident(s) => {
            let tok = p.advance();
            p.emit(
                tok.span,
                ErrorClass::EnumVariantNameInvalid,
                format!("flags bit `{s}` must start with an uppercase letter"),
            );
            Spanned::new(s, tok.span)
        }
        _ => {
            return None;
        }
    };

    let ordinal = match p.peek_kind().clone() {
        TokenKind::Ordinal(v) => {
            let tok = p.advance();
            Spanned::new(v, tok.span)
        }
        _ => {
            p.emit(
                p.peek().span,
                ErrorClass::UnexpectedToken,
                "expected ordinal (e.g. @0) for flags bit",
            );
            return None;
        }
    };

    let span = p.span_from(start);
    Some(Spanned::new(
        FlagsBit {
            annotations,
            name,
            ordinal,
        },
        span,
    ))
}

// ---------------------------------------------------------------------------
// Union
// ---------------------------------------------------------------------------

fn parse_union_decl(annotations: Vec<Annotation>, p: &mut Parser<'_>) -> UnionDecl {
    p.advance(); // consume KwUnion
    let name = parse_decl_name(p)
        .unwrap_or_else(|| Spanned::new(SmolStr::new("__error"), Span::empty(p.current_offset())));
    p.expect(&TokenKind::LBrace);

    let mut body = Vec::new();
    while !p.at(&TokenKind::RBrace) && !p.at_eof() {
        // Check for tombstone
        if is_at_tombstone(p) {
            let ts = parse_tombstone(p);
            body.push(UnionBodyItem::Tombstone(ts));
            continue;
        }

        // Annotations + variant
        let pre_annotations = parse_annotations(p);

        if p.at(&TokenKind::RBrace) {
            break;
        }
        if is_at_tombstone(p) {
            let ts = parse_tombstone(p);
            body.push(UnionBodyItem::Tombstone(ts));
            continue;
        }

        match parse_union_variant(pre_annotations, p) {
            Some(v) => body.push(UnionBodyItem::Variant(v)),
            None => {
                if !p.at(&TokenKind::RBrace) && !p.at_eof() {
                    p.advance();
                }
            }
        }
    }

    p.expect(&TokenKind::RBrace);

    UnionDecl {
        annotations,
        name,
        body,
    }
}

fn parse_union_variant(
    annotations: Vec<Annotation>,
    p: &mut Parser<'_>,
) -> Option<Spanned<UnionVariant>> {
    let start = if let Some(first) = annotations.first() {
        first.span.offset as usize
    } else {
        p.current_offset()
    };

    // Variant name: must be UpperIdent
    let name = match p.peek_kind().clone() {
        TokenKind::UpperIdent(s) => {
            let tok = p.advance();
            Spanned::new(s, tok.span)
        }
        TokenKind::Ident(s) => {
            let tok = p.advance();
            p.emit(
                tok.span,
                ErrorClass::UnionVariantNameInvalid,
                format!("union variant `{s}` must start with an uppercase letter"),
            );
            Spanned::new(s, tok.span)
        }
        _ => {
            return None;
        }
    };

    // Ordinal
    let ordinal = match p.peek_kind().clone() {
        TokenKind::Ordinal(v) => {
            let tok = p.advance();
            Spanned::new(v, tok.span)
        }
        _ => {
            p.emit(
                p.peek().span,
                ErrorClass::UnexpectedToken,
                "expected ordinal (e.g. @0) for union variant",
            );
            return None;
        }
    };

    // Optional body: if LBrace follows, parse fields/tombstones like a message body
    let fields = if p.at(&TokenKind::LBrace) {
        p.advance(); // consume LBrace
        let mut items = Vec::new();
        while !p.at(&TokenKind::RBrace) && !p.at_eof() {
            if is_at_tombstone(p) {
                let ts = parse_tombstone(p);
                items.push(MessageBodyItem::Tombstone(ts));
                continue;
            }

            let field_annotations = parse_annotations(p);

            if p.at(&TokenKind::RBrace) {
                break;
            }
            if is_at_tombstone(p) {
                let ts = parse_tombstone(p);
                items.push(MessageBodyItem::Tombstone(ts));
                continue;
            }

            match parse_field(field_annotations, p) {
                Some(field) => items.push(MessageBodyItem::Field(field)),
                None => {
                    if !p.at(&TokenKind::RBrace) && !p.at_eof() {
                        p.advance();
                    }
                }
            }
        }
        p.expect(&TokenKind::RBrace);
        items
    } else {
        Vec::new()
    };

    let span = p.span_from(start);
    Some(Spanned::new(
        UnionVariant {
            annotations,
            name,
            ordinal,
            fields,
        },
        span,
    ))
}

// ---------------------------------------------------------------------------
// Newtype
// ---------------------------------------------------------------------------

fn parse_newtype_decl(annotations: Vec<Annotation>, p: &mut Parser<'_>) -> NewtypeDecl {
    p.advance(); // consume KwNewtype
    let name = parse_decl_name(p)
        .unwrap_or_else(|| Spanned::new(SmolStr::new("__error"), Span::empty(p.current_offset())));

    p.expect(&TokenKind::Colon);

    let inner_type = parse_type_expr(p);

    NewtypeDecl {
        annotations,
        name,
        inner_type,
    }
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

fn parse_config_decl(annotations: Vec<Annotation>, p: &mut Parser<'_>) -> ConfigDecl {
    p.advance(); // consume KwConfig
    let name = parse_decl_name(p)
        .unwrap_or_else(|| Spanned::new(SmolStr::new("__error"), Span::empty(p.current_offset())));
    p.expect(&TokenKind::LBrace);

    let mut fields = Vec::new();
    while !p.at(&TokenKind::RBrace) && !p.at_eof() {
        let field_annotations = parse_annotations(p);

        if p.at(&TokenKind::RBrace) {
            break;
        }

        match parse_config_field(field_annotations, p) {
            Some(field) => fields.push(field),
            None => {
                if !p.at(&TokenKind::RBrace) && !p.at_eof() {
                    p.advance();
                }
            }
        }
    }

    p.expect(&TokenKind::RBrace);

    ConfigDecl {
        annotations,
        name,
        fields,
    }
}

fn parse_config_field(
    annotations: Vec<Annotation>,
    p: &mut Parser<'_>,
) -> Option<Spanned<ConfigField>> {
    let start = if let Some(first) = annotations.first() {
        first.span.offset as usize
    } else {
        p.current_offset()
    };

    // Check for ordinal (invalid in config)
    if let TokenKind::Ordinal(_) = p.peek_kind() {
        p.emit(
            p.peek().span,
            ErrorClass::ConfigHasOrdinal,
            "config fields must not have ordinals",
        );
        p.advance(); // skip the ordinal
    }

    // Field name
    let name = parse_field_name(p)?;

    // Validate: field name must start lowercase
    let name_str = name.node.as_str();
    if let Some(first_char) = name_str.chars().next() {
        if first_char.is_ascii_uppercase() {
            p.emit(
                name.span,
                ErrorClass::FieldNameInvalid,
                format!("field name `{name_str}` must start with a lowercase letter"),
            );
        }
    }

    // Check for ordinal after name (invalid in config)
    if let TokenKind::Ordinal(_) = p.peek_kind() {
        p.emit(
            p.peek().span,
            ErrorClass::ConfigHasOrdinal,
            "config fields must not have ordinals",
        );
        p.advance(); // skip the ordinal
    }

    // Colon
    p.expect(&TokenKind::Colon);

    // Type expression
    let ty = parse_type_expr(p);

    // Expect `=` for default value
    if !p.at(&TokenKind::Eq) {
        p.emit(
            p.peek().span,
            ErrorClass::ConfigMissingDefault,
            format!("config field `{name_str}` must have a default value"),
        );
        let span = p.span_from(start);
        return Some(Spanned::new(
            ConfigField {
                annotations,
                name,
                ty,
                default_value: Spanned::new(
                    crate::ast::DefaultValue::None,
                    Span::empty(p.current_offset()),
                ),
            },
            span,
        ));
    }
    p.advance(); // consume Eq

    let default_value = parse_literal_value(p);

    // Post-default annotations (e.g. `= 0 @varint`)
    let post_annotations = parse_annotations(p);
    let mut all_annotations = annotations;
    all_annotations.extend(post_annotations);

    let span = p.span_from(start);
    Some(Spanned::new(
        ConfigField {
            annotations: all_annotations,
            name,
            ty,
            default_value,
        },
        span,
    ))
}
