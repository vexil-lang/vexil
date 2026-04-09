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
    LBracket,
    RBracket,
    LParen,
    RParen,
    LAngle,
    RAngle,
    Colon,
    Comma,
    Dot,
    DotDot,   // ..
    DotDotLt, // ..<
    Eq,
    EqEq,   // ==
    Bang,   // !
    Ne,     // !=
    Le,     // <=
    Ge,     // >=
    AndAnd, // &&
    OrOr,   // ||
    Hash,
    Caret,
    Minus,
    Plus,
    Star,
    Slash,

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
    KwBits,
    KwUnion,
    KwNewtype,
    KwConfig,
    KwType,
    KwConst,

    KwOptional,
    KwArray,
    KwSet,
    KwMap,
    KwResult,
    KwTrue,
    KwFalse,
    KwNone,

    // Geometric types
    KwVec2,
    KwVec3,
    KwVec4,
    KwQuat,
    KwMat3,
    KwMat4,

    KwWhere, // where
    KwIn,    // in
    KwValue, // value (pseudo-keyword for constraints)

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
                | TokenKind::KwBits
                | TokenKind::KwUnion
                | TokenKind::KwNewtype
                | TokenKind::KwConfig
                | TokenKind::KwType
                | TokenKind::KwConst
                | TokenKind::KwOptional
                | TokenKind::KwArray
                | TokenKind::KwSet
                | TokenKind::KwMap
                | TokenKind::KwResult
                | TokenKind::KwTrue
                | TokenKind::KwFalse
                | TokenKind::KwNone
                | TokenKind::KwWhere
                | TokenKind::KwIn
                | TokenKind::KwValue
                | TokenKind::KwVec2
                | TokenKind::KwVec3
                | TokenKind::KwVec4
                | TokenKind::KwQuat
                | TokenKind::KwMat3
                | TokenKind::KwMat4
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
            TokenKind::KwBits => Some("bits".into()),
            TokenKind::KwUnion => Some("union".into()),
            TokenKind::KwNewtype => Some("newtype".into()),
            TokenKind::KwConfig => Some("config".into()),
            TokenKind::KwType => Some("type".into()),
            TokenKind::KwConst => Some("const".into()),
            TokenKind::KwOptional => Some("optional".into()),
            TokenKind::KwArray => Some("array".into()),
            TokenKind::KwSet => Some("set".into()),
            TokenKind::KwMap => Some("map".into()),
            TokenKind::KwResult => Some("result".into()),
            TokenKind::KwTrue => Some("true".into()),
            TokenKind::KwFalse => Some("false".into()),
            TokenKind::KwNone => Some("none".into()),
            TokenKind::KwWhere => Some("where".into()),
            TokenKind::KwIn => Some("in".into()),
            TokenKind::KwValue => Some("value".into()),
            TokenKind::KwVec2 => Some("vec2".into()),
            TokenKind::KwVec3 => Some("vec3".into()),
            TokenKind::KwVec4 => Some("vec4".into()),
            TokenKind::KwQuat => Some("quat".into()),
            TokenKind::KwMat3 => Some("mat3".into()),
            TokenKind::KwMat4 => Some("mat4".into()),
            _ => None,
        }
    }
}
