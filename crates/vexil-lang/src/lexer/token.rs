use crate::span::Span;
use smol_str::SmolStr;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Punctuation
    LBrace,
    RBrace,
    LParen,
    RParen,
    LAngle,
    RAngle,
    Colon,
    Comma,
    Dot,
    Eq,
    Hash,
    Caret,
    Minus,

    // Literals
    Ident(SmolStr),
    UpperIdent(SmolStr),
    StringLit(String),
    DecInt(u64),
    HexInt(u64),
    FloatLit(f64),

    // Ordinal (@N)
    Ordinal(u32),

    // Annotation sigil (@)
    At,

    // Keywords
    KwNamespace,
    KwImport,
    KwFrom,
    KwAs,
    KwMessage,
    KwEnum,
    KwFlags,
    KwUnion,
    KwNewtype,
    KwConfig,
    KwOptional,
    KwArray,
    KwMap,
    KwResult,
    KwTrue,
    KwFalse,
    KwNone,

    // Special
    Eof,
    Error,
}

impl TokenKind {
    /// Returns true if this token is a keyword that can appear as a field name.
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            TokenKind::KwNamespace
                | TokenKind::KwImport
                | TokenKind::KwFrom
                | TokenKind::KwAs
                | TokenKind::KwMessage
                | TokenKind::KwEnum
                | TokenKind::KwFlags
                | TokenKind::KwUnion
                | TokenKind::KwNewtype
                | TokenKind::KwConfig
                | TokenKind::KwOptional
                | TokenKind::KwArray
                | TokenKind::KwMap
                | TokenKind::KwResult
                | TokenKind::KwTrue
                | TokenKind::KwFalse
                | TokenKind::KwNone
        )
    }

    /// For keyword tokens, returns the keyword string. For Ident, returns the value.
    pub fn as_field_name(&self) -> Option<SmolStr> {
        match self {
            TokenKind::Ident(s) => Some(s.clone()),
            TokenKind::KwNamespace => Some("namespace".into()),
            TokenKind::KwImport => Some("import".into()),
            TokenKind::KwFrom => Some("from".into()),
            TokenKind::KwAs => Some("as".into()),
            TokenKind::KwMessage => Some("message".into()),
            TokenKind::KwEnum => Some("enum".into()),
            TokenKind::KwFlags => Some("flags".into()),
            TokenKind::KwUnion => Some("union".into()),
            TokenKind::KwNewtype => Some("newtype".into()),
            TokenKind::KwConfig => Some("config".into()),
            TokenKind::KwOptional => Some("optional".into()),
            TokenKind::KwArray => Some("array".into()),
            TokenKind::KwMap => Some("map".into()),
            TokenKind::KwResult => Some("result".into()),
            TokenKind::KwTrue => Some("true".into()),
            TokenKind::KwFalse => Some("false".into()),
            TokenKind::KwNone => Some("none".into()),
            _ => None,
        }
    }
}
