use crate::ast::{ImportDecl, ImportKind};
use crate::diagnostic::ErrorClass;
use crate::lexer::token::TokenKind;
use crate::span::Spanned;
use smol_str::SmolStr;

use super::Parser;

/// Parse a single import declaration.
///
/// Grammar (left-factored):
/// ```text
/// import_decl = "import" ( named_import | path_import )
/// named_import = "{" UpperIdent ("," UpperIdent)* "}" "from" namespace_path version?
/// path_import  = namespace_path version? ("as" UpperIdent)?
/// namespace_path = Ident ("." Ident)*
/// version = "@" "^" DecInt "." DecInt "." DecInt
/// ```
pub(crate) fn parse_import(p: &mut Parser<'_>) -> Spanned<ImportDecl> {
    let start = p.current_offset();
    p.advance(); // consume KwImport

    // Dispatch: named form starts with `{`, otherwise path-based.
    if p.at(&TokenKind::LBrace) {
        parse_named_import(p, start)
    } else {
        parse_path_import(p, start)
    }
}

/// Parse `{ Name, Name, ... } from namespace.path version?`
fn parse_named_import(p: &mut Parser<'_>, start: usize) -> Spanned<ImportDecl> {
    p.advance(); // consume LBrace

    let mut names: Vec<Spanned<SmolStr>> = Vec::new();

    // Parse comma-separated UpperIdents.
    while !p.at(&TokenKind::RBrace) && !p.at_eof() {
        match p.peek_kind().clone() {
            TokenKind::UpperIdent(s) => {
                let tok = p.advance();
                names.push(Spanned::new(s, tok.span));
            }
            _ => {
                p.emit(
                    p.peek().span,
                    ErrorClass::UnexpectedToken,
                    "expected type name (UpperCase) in import list",
                );
                // Skip one token to avoid infinite loop.
                if !p.at(&TokenKind::RBrace) && !p.at_eof() {
                    p.advance();
                }
                continue;
            }
        }
        if p.at(&TokenKind::Comma) {
            p.advance();
        }
    }

    // Consume RBrace.
    if p.at(&TokenKind::RBrace) {
        p.advance();
    } else {
        p.emit(
            p.peek().span,
            ErrorClass::UnexpectedToken,
            "expected `}` to close import list",
        );
    }

    // Expect `from`.
    if !p.at(&TokenKind::KwFrom) {
        p.emit(
            p.peek().span,
            ErrorClass::UnexpectedToken,
            "expected `from` after named import list",
        );
        let span = p.span_from(start);
        return Spanned::new(
            ImportDecl {
                kind: ImportKind::Named { names },
                path: Vec::new(),
                version: None,
            },
            span,
        );
    }
    p.advance(); // consume KwFrom

    // Parse namespace path.
    let path = parse_namespace_path(p);

    // Optional version.
    let version = parse_optional_version(p);

    // Named + aliased is an error.
    if p.at(&TokenKind::KwAs) {
        let as_tok = p.advance();
        p.emit(
            as_tok.span,
            ErrorClass::ImportNamedAliasedCombined,
            "named imports cannot be combined with `as` alias",
        );
        // Consume the alias UpperIdent if present.
        if matches!(p.peek_kind(), TokenKind::UpperIdent(_)) {
            p.advance();
        }
    }

    let span = p.span_from(start);
    Spanned::new(
        ImportDecl {
            kind: ImportKind::Named { names },
            path,
            version,
        },
        span,
    )
}

/// Parse `namespace.path version? ("as" UpperIdent)?`
fn parse_path_import(p: &mut Parser<'_>, start: usize) -> Spanned<ImportDecl> {
    let path = parse_namespace_path(p);

    // Optional version.
    let version = parse_optional_version(p);

    // Optional alias.
    let kind = if p.at(&TokenKind::KwAs) {
        p.advance(); // consume KwAs
        match p.peek_kind().clone() {
            TokenKind::UpperIdent(s) => {
                let tok = p.advance();
                ImportKind::Aliased {
                    alias: Spanned::new(s, tok.span),
                }
            }
            _ => {
                p.emit(
                    p.peek().span,
                    ErrorClass::UnexpectedToken,
                    "expected alias name (UpperCase) after `as`",
                );
                ImportKind::Wildcard
            }
        }
    } else {
        ImportKind::Wildcard
    };

    let span = p.span_from(start);
    Spanned::new(
        ImportDecl {
            kind,
            path,
            version,
        },
        span,
    )
}

/// Parse dot-separated lowercase identifier path: `ident.ident.ident`
///
/// Keywords that overlap with lowercase identifiers (e.g. `config`, `message`,
/// `flags`) are accepted as path components via `as_field_name()`.
fn parse_namespace_path(p: &mut Parser<'_>) -> Vec<Spanned<SmolStr>> {
    let mut path: Vec<Spanned<SmolStr>> = Vec::new();

    // First component: Ident or keyword-as-ident.
    match try_consume_lowercase_component(p) {
        Some(spanned) => path.push(spanned),
        None => {
            p.emit(
                p.peek().span,
                ErrorClass::UnexpectedToken,
                "expected namespace path",
            );
            return path;
        }
    }

    // Subsequent dot-separated components.
    while p.at(&TokenKind::Dot) {
        p.advance(); // consume Dot
        match try_consume_lowercase_component(p) {
            Some(spanned) => path.push(spanned),
            None => {
                p.emit(
                    p.peek().span,
                    ErrorClass::UnexpectedToken,
                    "expected identifier after `.` in import path",
                );
                break;
            }
        }
    }

    path
}

/// Try to consume the current token as a lowercase path component.
/// Accepts `Ident` and any keyword that maps to a field name (lowercase keywords).
fn try_consume_lowercase_component(p: &mut Parser<'_>) -> Option<Spanned<SmolStr>> {
    match p.peek_kind().clone() {
        TokenKind::Ident(s) => {
            let tok = p.advance();
            Some(Spanned::new(s, tok.span))
        }
        ref kind if kind.is_keyword() => {
            // Keywords like `config`, `message`, etc. can appear as path components.
            if let Some(name) = kind.as_field_name() {
                let tok = p.advance();
                Some(Spanned::new(name, tok.span))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Parse optional version constraint: `@ ^ major . minor . patch`
///
/// The lexer greedily parses `1.0` as `FloatLit(1.0)` when a dot is followed
/// by a digit, so we must handle two token patterns:
///   - `FloatLit(major.minor) Dot DecInt(patch)` → valid semver
///   - `FloatLit(major.minor)` with no following `.patch` → incomplete semver
///   - `DecInt Dot DecInt Dot DecInt` → also valid (if lexer changes)
fn parse_optional_version(p: &mut Parser<'_>) -> Option<Spanned<String>> {
    if !p.at(&TokenKind::At) {
        return None;
    }

    let start = p.current_offset();
    p.advance(); // consume At

    // Expect Caret.
    if !p.at(&TokenKind::Caret) {
        p.emit(
            p.peek().span,
            ErrorClass::VersionInvalidSemver,
            "expected `^` after `@` in version constraint",
        );
        return None;
    }
    p.advance(); // consume Caret

    // The lexer may produce FloatLit for "major.minor" or DecInt for just "major".
    match p.peek_kind().clone() {
        TokenKind::FloatLit(_) => {
            // major.minor was lexed as a float. Extract major and minor.
            // Use the source text directly from the span, since Display for f64
            // may drop trailing zeros (e.g. 1.0 → "1").
            let tok = p.advance(); // consume FloatLit
            let float_str = p.source_text(tok.span).to_string();

            let (major_str, minor_str) = match float_str.split_once('.') {
                Some((maj, min)) => (maj.to_string(), min.to_string()),
                None => {
                    p.emit(
                        p.span_from(start),
                        ErrorClass::VersionInvalidSemver,
                        "invalid version format",
                    );
                    return None;
                }
            };

            // Now expect Dot + DecInt for patch.
            if !p.at(&TokenKind::Dot) {
                let span = p.span_from(start);
                p.emit(
                    span,
                    ErrorClass::VersionInvalidSemver,
                    format!(
                        "incomplete semver: ^{major_str}.{minor_str} — patch component required"
                    ),
                );
                return None;
            }
            p.advance(); // consume Dot

            let patch = match p.peek_kind() {
                TokenKind::DecInt(v) => {
                    let v = *v;
                    p.advance();
                    v
                }
                _ => {
                    p.emit(
                        p.peek().span,
                        ErrorClass::VersionInvalidSemver,
                        "expected patch version number",
                    );
                    return None;
                }
            };

            let version_string = format!("^{major_str}.{minor_str}.{patch}");
            let span = p.span_from(start);
            Some(Spanned::new(version_string, span))
        }
        TokenKind::DecInt(major) => {
            // major is a plain integer. Expect Dot minor Dot patch.
            p.advance(); // consume DecInt

            if !p.at(&TokenKind::Dot) {
                p.emit(
                    p.peek().span,
                    ErrorClass::VersionInvalidSemver,
                    "expected `.` after major version",
                );
                return None;
            }
            p.advance(); // consume Dot

            let minor = match p.peek_kind() {
                TokenKind::DecInt(v) => {
                    let v = *v;
                    p.advance();
                    v
                }
                _ => {
                    p.emit(
                        p.peek().span,
                        ErrorClass::VersionInvalidSemver,
                        "expected minor version number",
                    );
                    return None;
                }
            };

            if !p.at(&TokenKind::Dot) {
                let span = p.span_from(start);
                p.emit(
                    span,
                    ErrorClass::VersionInvalidSemver,
                    format!("incomplete semver: ^{major}.{minor} — patch component required"),
                );
                return None;
            }
            p.advance(); // consume Dot

            let patch = match p.peek_kind() {
                TokenKind::DecInt(v) => {
                    let v = *v;
                    p.advance();
                    v
                }
                _ => {
                    p.emit(
                        p.peek().span,
                        ErrorClass::VersionInvalidSemver,
                        "expected patch version number",
                    );
                    return None;
                }
            };

            let version_string = format!("^{major}.{minor}.{patch}");
            let span = p.span_from(start);
            Some(Spanned::new(version_string, span))
        }
        _ => {
            p.emit(
                p.peek().span,
                ErrorClass::VersionInvalidSemver,
                "expected version number after `^`",
            );
            None
        }
    }
}
