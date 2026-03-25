pub mod decl;
pub mod expr;
pub mod import;

use crate::ast::{
    Annotation, AnnotationArg, AnnotationValue, Decl, ImportDecl, NamespaceDecl, Schema,
};
use crate::diagnostic::{Diagnostic, ErrorClass};
use crate::lexer::token::{Token, TokenKind};
use crate::span::{Span, Spanned};
use smol_str::SmolStr;

// ---------------------------------------------------------------------------
// Parser state
// ---------------------------------------------------------------------------

struct Parser<'s> {
    tokens: Vec<Token>,
    pos: usize,
    source: &'s str,
    diagnostics: Vec<Diagnostic>,
}

impl<'s> Parser<'s> {
    fn new(source: &'s str, tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            source,
            diagnostics: Vec::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Core primitives
    // -----------------------------------------------------------------------

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .unwrap_or_else(|| self.tokens.last().unwrap_or(&EOF_TOKEN))
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn advance(&mut self) -> Token {
        let tok = self
            .tokens
            .get(self.pos)
            .cloned()
            .unwrap_or_else(|| eof_token(self.source.len()));
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn at(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(kind)
    }

    #[allow(dead_code)]
    fn at_ident(&self, name: &str) -> bool {
        matches!(self.peek_kind(), TokenKind::Ident(s) if s.as_str() == name)
    }

    fn at_eof(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Eof)
    }

    #[allow(dead_code)]
    fn expect(&mut self, kind: &TokenKind) -> Option<Token> {
        if self.at(kind) {
            Some(self.advance())
        } else {
            let span = self.peek().span;
            self.diagnostics.push(Diagnostic::error(
                span,
                ErrorClass::UnexpectedToken,
                format!("expected {:?}, found {:?}", kind, self.peek_kind()),
            ));
            None
        }
    }

    #[allow(dead_code)]
    fn checkpoint(&self) -> (usize, usize) {
        (self.pos, self.diagnostics.len())
    }

    #[allow(dead_code)]
    fn backtrack(&mut self, cp: (usize, usize)) {
        self.pos = cp.0;
        self.diagnostics.truncate(cp.1);
    }

    fn span_from(&self, start_offset: usize) -> Span {
        let end = self.peek().span.offset as usize;
        let len = end.saturating_sub(start_offset);
        Span::new(start_offset, len)
    }

    fn current_offset(&self) -> usize {
        self.peek().span.offset as usize
    }

    // -----------------------------------------------------------------------
    // Error helpers
    // -----------------------------------------------------------------------

    fn emit(&mut self, span: Span, class: ErrorClass, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error(span, class, message));
    }
}

// Sentinel for when we're past the end.
static EOF_TOKEN: Token = Token {
    kind: TokenKind::Eof,
    span: Span { offset: 0, len: 0 },
};

fn eof_token(offset: usize) -> Token {
    Token {
        kind: TokenKind::Eof,
        span: Span::new(offset, 0),
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Parse a token stream into a Schema AST.
pub fn parse(source: &str, tokens: Vec<Token>) -> (Option<Schema>, Vec<Diagnostic>) {
    let mut p = Parser::new(source, tokens);
    let schema = parse_schema(&mut p);
    (Some(schema), p.diagnostics)
}

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

fn parse_schema(p: &mut Parser<'_>) -> Schema {
    let start = p.current_offset();

    // Schema-level annotations (before namespace).
    let annotations = parse_annotations(p);

    // Namespace (required).
    let namespace = if p.at(&TokenKind::KwNamespace) {
        Some(parse_namespace(p))
    } else if !p.at_eof() {
        p.emit(
            p.peek().span,
            ErrorClass::MissingNamespace,
            "expected `namespace` declaration",
        );
        None
    } else {
        // Empty file (or only comments) with no annotations → still missing namespace.
        if annotations.is_empty() {
            p.emit(
                Span::empty(start),
                ErrorClass::MissingNamespace,
                "expected `namespace` declaration",
            );
        } else {
            p.emit(
                Span::empty(start),
                ErrorClass::MissingNamespace,
                "annotations found but no `namespace` declaration",
            );
        }
        None
    };

    // Imports.
    let mut imports: Vec<Spanned<ImportDecl>> = Vec::new();
    while p.at(&TokenKind::KwImport) {
        let import_start = p.current_offset();
        skip_import(p);
        let span = p.span_from(import_start);
        // Placeholder import — will be properly parsed in Task 6.
        imports.push(Spanned::new(
            ImportDecl {
                kind: crate::ast::ImportKind::Wildcard,
                path: Vec::new(),
                version: None,
            },
            span,
        ));
    }

    // Declarations.
    let mut declarations: Vec<Spanned<Decl>> = Vec::new();
    while is_at_decl_keyword(p) || p.at(&TokenKind::At) {
        let decl_start = p.current_offset();
        // Consume pre-annotations.
        let _annots = parse_annotations(p);
        if is_at_decl_keyword(p) {
            skip_decl(p);
            let span = p.span_from(decl_start);
            // Placeholder — will be properly parsed in Tasks 7-9.
            declarations.push(Spanned::new(
                Decl::Message(crate::ast::MessageDecl {
                    annotations: Vec::new(),
                    name: Spanned::new(SmolStr::new("__placeholder"), span),
                    body: Vec::new(),
                }),
                span,
            ));
        } else if !p.at_eof() {
            // Annotations with no following declaration — skip token to avoid infinite loop.
            p.advance();
        } else {
            break;
        }
    }

    let span = p.span_from(start);

    Schema {
        span,
        annotations,
        namespace,
        imports,
        declarations,
    }
}

// ---------------------------------------------------------------------------
// Namespace
// ---------------------------------------------------------------------------

fn parse_namespace(p: &mut Parser<'_>) -> Spanned<NamespaceDecl> {
    let start = p.current_offset();
    p.advance(); // consume KwNamespace

    let mut path: Vec<Spanned<SmolStr>> = Vec::new();

    // First component.
    match p.peek_kind().clone() {
        TokenKind::Ident(s) => {
            let tok = p.advance();
            path.push(Spanned::new(s, tok.span));
        }
        TokenKind::UpperIdent(_) => {
            let tok = p.advance();
            p.emit(
                tok.span,
                ErrorClass::NamespaceInvalidComponent,
                "namespace components must be lowercase",
            );
        }
        _ => {
            p.emit(
                p.peek().span,
                ErrorClass::NamespaceEmpty,
                "expected namespace path",
            );
            let span = p.span_from(start);
            return Spanned::new(NamespaceDecl { path }, span);
        }
    }

    // Subsequent dot-separated components.
    while p.at(&TokenKind::Dot) {
        p.advance(); // consume Dot
        match p.peek_kind().clone() {
            TokenKind::Ident(s) => {
                let tok = p.advance();
                path.push(Spanned::new(s, tok.span));
            }
            TokenKind::UpperIdent(_) => {
                let tok = p.advance();
                p.emit(
                    tok.span,
                    ErrorClass::NamespaceInvalidComponent,
                    "namespace components must be lowercase",
                );
            }
            _ => {
                p.emit(
                    p.peek().span,
                    ErrorClass::UnexpectedToken,
                    "expected namespace component after `.`",
                );
                break;
            }
        }
    }

    let span = p.span_from(start);
    Spanned::new(NamespaceDecl { path }, span)
}

// ---------------------------------------------------------------------------
// Annotations
// ---------------------------------------------------------------------------

fn parse_annotations(p: &mut Parser<'_>) -> Vec<Annotation> {
    let mut annotations = Vec::new();

    while p.at(&TokenKind::At) {
        let start = p.current_offset();
        p.advance(); // consume At

        // Annotation name.
        let name = match p.peek_kind().clone() {
            TokenKind::Ident(s) => {
                let tok = p.advance();
                Spanned::new(s, tok.span)
            }
            _ => {
                p.emit(
                    p.peek().span,
                    ErrorClass::UnexpectedToken,
                    "expected annotation name",
                );
                continue;
            }
        };

        // Optional args.
        let args = if p.at(&TokenKind::LParen) {
            p.advance(); // consume LParen
            let args = parse_annotation_args(p);
            if !p.at(&TokenKind::RParen) {
                p.emit(
                    p.peek().span,
                    ErrorClass::UnexpectedToken,
                    "expected `)` to close annotation",
                );
            } else {
                p.advance(); // consume RParen
            }
            Some(args)
        } else {
            None
        };

        let span = p.span_from(start);
        annotations.push(Annotation { span, name, args });
    }

    annotations
}

fn parse_annotation_args(p: &mut Parser<'_>) -> Vec<AnnotationArg> {
    let mut args = Vec::new();

    while !p.at(&TokenKind::RParen) && !p.at_eof() {
        let arg_start = p.current_offset();

        // Check for named arg: Ident Colon Value.
        let key = if matches!(p.peek_kind(), TokenKind::Ident(_)) {
            // Look ahead to see if next-next is Colon.
            let maybe_key = if let Some(next) = p.tokens.get(p.pos + 1) {
                matches!(next.kind, TokenKind::Colon)
            } else {
                false
            };
            if maybe_key {
                let tok = p.advance();
                let name = match tok.kind {
                    TokenKind::Ident(s) => Spanned::new(s, tok.span),
                    _ => unreachable!(),
                };
                p.advance(); // consume Colon
                Some(name)
            } else {
                None
            }
        } else {
            None
        };

        let value = parse_annotation_value(p);
        let span = p.span_from(arg_start);

        match value {
            Some(value) => {
                args.push(AnnotationArg { span, key, value });
            }
            None => {
                // Could not parse value — skip token to avoid infinite loop.
                if !p.at(&TokenKind::RParen) && !p.at_eof() {
                    p.advance();
                }
                continue;
            }
        }

        // Optional comma separator.
        if p.at(&TokenKind::Comma) {
            p.advance();
        }
    }

    args
}

fn parse_annotation_value(p: &mut Parser<'_>) -> Option<Spanned<AnnotationValue>> {
    let tok = p.peek().clone();
    match &tok.kind {
        TokenKind::DecInt(v) => {
            let v = *v;
            let tok = p.advance();
            Some(Spanned::new(AnnotationValue::Int(v), tok.span))
        }
        TokenKind::HexInt(v) => {
            let v = *v;
            let tok = p.advance();
            Some(Spanned::new(AnnotationValue::Hex(v), tok.span))
        }
        TokenKind::StringLit(s) => {
            let s = s.clone();
            let tok = p.advance();
            Some(Spanned::new(AnnotationValue::Str(s), tok.span))
        }
        TokenKind::KwTrue => {
            let tok = p.advance();
            Some(Spanned::new(AnnotationValue::Bool(true), tok.span))
        }
        TokenKind::KwFalse => {
            let tok = p.advance();
            Some(Spanned::new(AnnotationValue::Bool(false), tok.span))
        }
        TokenKind::UpperIdent(s) => {
            let s = s.clone();
            let tok = p.advance();
            Some(Spanned::new(AnnotationValue::UpperIdent(s), tok.span))
        }
        TokenKind::Ident(s) => {
            let s = s.clone();
            let tok = p.advance();
            Some(Spanned::new(AnnotationValue::Ident(s), tok.span))
        }
        _ => {
            p.emit(
                tok.span,
                ErrorClass::UnexpectedToken,
                "expected annotation value",
            );
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Skip helpers (placeholders until Tasks 6-9)
// ---------------------------------------------------------------------------

/// Returns true if the current token is a declaration keyword.
fn is_at_decl_keyword(p: &Parser<'_>) -> bool {
    matches!(
        p.peek_kind(),
        TokenKind::KwMessage
            | TokenKind::KwEnum
            | TokenKind::KwFlags
            | TokenKind::KwUnion
            | TokenKind::KwNewtype
            | TokenKind::KwConfig
    )
}

/// Skip past an import statement. Advances until we hit a keyword that starts
/// the next statement or EOF.
fn skip_import(p: &mut Parser<'_>) {
    p.advance(); // consume KwImport
    while !p.at_eof() && !p.at(&TokenKind::KwImport) && !is_at_decl_keyword(p) && !p.at(&TokenKind::At) {
        p.advance();
    }
}

/// Skip past a declaration. Advances through nested braces until we hit the
/// next declaration keyword or EOF.
fn skip_decl(p: &mut Parser<'_>) {
    p.advance(); // consume decl keyword

    let mut brace_depth: u32 = 0;

    while !p.at_eof() {
        if p.at(&TokenKind::LBrace) {
            brace_depth += 1;
            p.advance();
        } else if p.at(&TokenKind::RBrace) {
            brace_depth = brace_depth.saturating_sub(1);
            p.advance();
            if brace_depth == 0 {
                break;
            }
        } else if brace_depth == 0 && (is_at_decl_keyword(p) || p.at(&TokenKind::At)) {
            // Next declaration starts — stop.
            break;
        } else {
            p.advance();
        }
    }
}
