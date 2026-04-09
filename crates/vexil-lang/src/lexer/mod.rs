//! # Stability: Tier 3 (internal)
//!
//! Lexer: tokenises Vexil source text into a flat token stream.
//!
//! The lexer is intentionally simple -- it produces tokens without
//! any lookahead beyond single-character peek. All error recovery
//! happens in the parser.

pub mod token;

use crate::diagnostic::{Diagnostic, ErrorClass};
use crate::span::Span;
use smol_str::SmolStr;
use token::{Token, TokenKind};

struct Lexer<'a> {
    source: &'a [u8],
    pos: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source: source.as_bytes(),
            pos: 0,
            diagnostics: Vec::new(),
        }
    }

    fn peek(&self) -> Option<u8> {
        self.source.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.source.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let ch = self.source.get(self.pos).copied();
        if ch.is_some() {
            self.pos += 1;
        }
        ch
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // Skip whitespace
            while let Some(ch) = self.peek() {
                if ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r' {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            // Skip line comments
            if self.peek() == Some(b'#') {
                while let Some(ch) = self.peek() {
                    self.pos += 1;
                    if ch == b'\n' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    fn make_token(&self, kind: TokenKind, start: usize) -> Token {
        Token {
            kind,
            span: Span::new(start, self.pos - start),
        }
    }

    fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();

        let start = self.pos;

        let ch = match self.advance() {
            Some(ch) => ch,
            None => {
                return Token {
                    kind: TokenKind::Eof,
                    span: Span::empty(self.pos),
                }
            }
        };

        match ch {
            b'{' => self.make_token(TokenKind::LBrace, start),
            b'}' => self.make_token(TokenKind::RBrace, start),
            b'[' => self.make_token(TokenKind::LBracket, start),
            b']' => self.make_token(TokenKind::RBracket, start),
            b'(' => self.make_token(TokenKind::LParen, start),
            b')' => self.make_token(TokenKind::RParen, start),
            b'<' => {
                if self.peek() == Some(b'=') {
                    self.pos += 1;
                    self.make_token(TokenKind::Le, start)
                } else {
                    self.make_token(TokenKind::LAngle, start)
                }
            }
            b'>' => {
                if self.peek() == Some(b'=') {
                    self.pos += 1;
                    self.make_token(TokenKind::Ge, start)
                } else {
                    self.make_token(TokenKind::RAngle, start)
                }
            }
            b':' => self.make_token(TokenKind::Colon, start),
            b',' => self.make_token(TokenKind::Comma, start),
            b'^' => self.make_token(TokenKind::Caret, start),
            b'-' => self.make_token(TokenKind::Minus, start),
            b'+' => self.make_token(TokenKind::Plus, start),
            b'*' => self.make_token(TokenKind::Star, start),
            b'/' => self.make_token(TokenKind::Slash, start),

            // Multi-character punctuation
            b'.' => {
                if self.peek() == Some(b'.') {
                    self.pos += 1;
                    if self.peek() == Some(b'<') {
                        self.pos += 1;
                        self.make_token(TokenKind::DotDotLt, start)
                    } else {
                        self.make_token(TokenKind::DotDot, start)
                    }
                } else {
                    self.make_token(TokenKind::Dot, start)
                }
            }
            b'=' => {
                if self.peek() == Some(b'=') {
                    self.pos += 1;
                    self.make_token(TokenKind::EqEq, start)
                } else {
                    self.make_token(TokenKind::Eq, start)
                }
            }
            b'!' => {
                if self.peek() == Some(b'=') {
                    self.pos += 1;
                    self.make_token(TokenKind::Ne, start)
                } else {
                    self.make_token(TokenKind::Bang, start)
                }
            }
            b'&' => {
                if self.peek() == Some(b'&') {
                    self.pos += 1;
                    self.make_token(TokenKind::AndAnd, start)
                } else {
                    self.diagnostics.push(Diagnostic::error(
                        Span::new(start, 1),
                        ErrorClass::InvalidCharacter,
                        "unexpected character: &",
                    ));
                    self.make_token(TokenKind::Error, start)
                }
            }
            b'|' => {
                if self.peek() == Some(b'|') {
                    self.pos += 1;
                    self.make_token(TokenKind::OrOr, start)
                } else {
                    self.diagnostics.push(Diagnostic::error(
                        Span::new(start, 1),
                        ErrorClass::InvalidCharacter,
                        "unexpected character: |",
                    ));
                    self.make_token(TokenKind::Error, start)
                }
            }

            b'"' => self.lex_string(start),

            b'@' => {
                if let Some(next) = self.peek() {
                    if next.is_ascii_digit() {
                        return self.lex_ordinal(start);
                    }
                }
                self.make_token(TokenKind::At, start)
            }

            b'0' if matches!(self.peek(), Some(b'x' | b'X')) => {
                self.pos += 1; // consume 'x'/'X'
                self.lex_hex_int(start)
            }

            b'0'..=b'9' => self.lex_number(start),

            b'A'..=b'Z' => self.lex_upper_ident(start),

            b'a'..=b'z' | b'_' => self.lex_word(start),

            _ => {
                self.diagnostics.push(Diagnostic::error(
                    Span::new(start, 1),
                    ErrorClass::InvalidCharacter,
                    format!("unexpected character: {:?}", ch as char),
                ));
                self.make_token(TokenKind::Error, start)
            }
        }
    }

    fn lex_string(&mut self, start: usize) -> Token {
        let mut value = String::new();
        loop {
            match self.advance() {
                None => {
                    self.diagnostics.push(Diagnostic::error(
                        Span::new(start, self.pos - start),
                        ErrorClass::UnterminatedString,
                        "unterminated string literal",
                    ));
                    return self.make_token(TokenKind::StringLit(value), start);
                }
                Some(b'"') => {
                    return self.make_token(TokenKind::StringLit(value), start);
                }
                Some(b'\\') => {
                    let esc_start = self.pos - 1;
                    match self.advance() {
                        Some(b'n') => value.push('\n'),
                        Some(b't') => value.push('\t'),
                        Some(b'r') => value.push('\r'),
                        Some(b'\\') => value.push('\\'),
                        Some(b'"') => value.push('"'),
                        Some(other) => {
                            self.diagnostics.push(Diagnostic::error(
                                Span::new(esc_start, 2),
                                ErrorClass::InvalidEscape,
                                format!("invalid escape sequence: \\{}", other as char),
                            ));
                            value.push(other as char);
                        }
                        None => {
                            self.diagnostics.push(Diagnostic::error(
                                Span::new(start, self.pos - start),
                                ErrorClass::UnterminatedString,
                                "unterminated string literal",
                            ));
                            return self.make_token(TokenKind::StringLit(value), start);
                        }
                    }
                }
                Some(ch) => {
                    value.push(ch as char);
                }
            }
        }
    }

    fn lex_ordinal(&mut self, start: usize) -> Token {
        // pos is right after '@', and we know next char is a digit
        let num_start = self.pos;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        let num_str = std::str::from_utf8(&self.source[num_start..self.pos]).unwrap_or("0");
        let val = num_str.parse::<u32>().unwrap_or(0);
        self.make_token(TokenKind::Ordinal(val), start)
    }

    fn lex_hex_int(&mut self, start: usize) -> Token {
        let hex_start = self.pos;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_hexdigit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        let hex_str = std::str::from_utf8(&self.source[hex_start..self.pos]).unwrap_or("0");
        let val = u64::from_str_radix(hex_str, 16).unwrap_or(0);
        self.make_token(TokenKind::HexInt(val), start)
    }

    fn lex_number(&mut self, start: usize) -> Token {
        // We already consumed the first digit; scan remaining digits
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }

        // Check for float: '.' followed by a digit
        if self.peek() == Some(b'.') {
            if let Some(next) = self.peek_at(1) {
                if next.is_ascii_digit() {
                    self.pos += 1; // consume '.'
                    while let Some(ch) = self.peek() {
                        if ch.is_ascii_digit() {
                            self.pos += 1;
                        } else {
                            break;
                        }
                    }
                    let text = std::str::from_utf8(&self.source[start..self.pos]).unwrap_or("0.0");
                    let val = text.parse::<f64>().unwrap_or(0.0);
                    return self.make_token(TokenKind::FloatLit(val), start);
                }
            }
        }

        let text = std::str::from_utf8(&self.source[start..self.pos]).unwrap_or("0");
        let val = text.parse::<u64>().unwrap_or(0);
        self.make_token(TokenKind::DecInt(val), start)
    }

    fn lex_upper_ident(&mut self, start: usize) -> Token {
        // First char (A-Z) already consumed
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        let text = std::str::from_utf8(&self.source[start..self.pos]).unwrap_or("");
        self.make_token(TokenKind::UpperIdent(SmolStr::new(text)), start)
    }

    fn lex_word(&mut self, start: usize) -> Token {
        // First char (a-z or _) already consumed. Scan [a-z0-9_]*
        while let Some(ch) = self.peek() {
            if ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }

        let text = std::str::from_utf8(&self.source[start..self.pos]).unwrap_or("");

        let kind = match text {
            "namespace" => TokenKind::KwNamespace,
            "import" => TokenKind::KwImport,
            "from" => TokenKind::KwFrom,
            "as" => TokenKind::KwAs,
            "message" => TokenKind::KwMessage,
            "enum" => TokenKind::KwEnum,
            "flags" => TokenKind::KwFlags,
            "bits" => TokenKind::KwBits,
            "union" => TokenKind::KwUnion,
            "newtype" => TokenKind::KwNewtype,
            "config" => TokenKind::KwConfig,
            "type" => TokenKind::KwType,
            "const" => TokenKind::KwConst,
            "trait" => TokenKind::KwTrait,
            "impl" => TokenKind::KwImpl,
            "for" => TokenKind::KwFor,
            "fn" => TokenKind::KwFn,
            "optional" => TokenKind::KwOptional,
            "array" => TokenKind::KwArray,
            "set" => TokenKind::KwSet,
            "map" => TokenKind::KwMap,
            "result" => TokenKind::KwResult,
            "true" => TokenKind::KwTrue,
            "false" => TokenKind::KwFalse,
            "none" => TokenKind::KwNone,
            "where" => TokenKind::KwWhere,
            "in" => TokenKind::KwIn,
            "value" => TokenKind::KwValue,
            "vec2" => TokenKind::KwVec2,
            "vec3" => TokenKind::KwVec3,
            "vec4" => TokenKind::KwVec4,
            "quat" => TokenKind::KwQuat,
            "mat3" => TokenKind::KwMat3,
            "mat4" => TokenKind::KwMat4,
            _ => TokenKind::Ident(SmolStr::new(text)),
        };

        self.make_token(kind, start)
    }
}

/// Tokenise source into a flat token list plus any lexer diagnostics.
pub fn lex(source: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    let mut lexer = Lexer::new(source);
    let mut tokens = Vec::new();

    loop {
        let tok = lexer.next_token();
        let is_eof = tok.kind == TokenKind::Eof;
        tokens.push(tok);
        if is_eof {
            break;
        }
    }

    (tokens, lexer.diagnostics)
}
