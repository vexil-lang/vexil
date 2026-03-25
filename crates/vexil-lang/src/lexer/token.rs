use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum TokenKind {
    // Literals
    Ident,
    StringLit,
    IntLit,

    // Punctuation
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Semicolon,
    Dot,
    Question,
    Pipe,
    Eq,
    Arrow,

    // Keywords
    KwSchema,
    KwImport,
    KwEnum,
    KwRecord,
    KwUnion,
    KwConst,

    // Trivia
    Whitespace,
    LineComment,
    BlockComment,

    // Sentinel
    Eof,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}
