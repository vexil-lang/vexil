use crate::span::Span;
use smol_str::SmolStr;

/// A single token produced by the lexer, with its source span for error reporting.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// The token's kind and optional payload.
    pub kind: TokenKind,
    /// Source location of this token in the input.
    pub span: Span,
}

/// All token kinds recognized by the Vexil lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Punctuation
    /// `{` — opens a block.
    LBrace,
    /// `}` — closes a block.
    RBrace,
    /// `[` — opens a bracket.
    LBracket,
    /// `]` — closes a bracket.
    RBracket,
    /// `(` — opens parentheses.
    LParen,
    /// `)` — closes parentheses.
    RParen,
    /// `<` — opens angle bracket (generics).
    LAngle,
    /// `>` — closes angle bracket (generics).
    RAngle,
    /// `:` — type separator.
    Colon,
    /// `,` — item separator.
    Comma,
    /// `.` — dot accessor.
    Dot,
    /// `..` — inclusive range.
    DotDot,
    /// `..<` — exclusive range.
    DotDotLt,
    /// `=` — assignment or variant value separator.
    Eq,
    /// `==` — equality comparison.
    EqEq,
    /// `!` — logical NOT or field reference sigil.
    Bang,
    /// `!=` — inequality comparison.
    Ne,
    /// `<=` — less-than-or-equal comparison.
    Le,
    /// `>=` — greater-than-or-equal comparison.
    Ge,
    /// `&&` — logical AND.
    AndAnd,
    /// `||` — logical OR.
    OrOr,
    /// `#` — used in hex literals.
    Hash,
    /// `^` — caret (reserved).
    Caret,
    /// `-` — minus sign.
    Minus,
    /// `+` — plus sign.
    Plus,
    /// `*` — star (wildcard import).
    Star,
    /// `/` — forward slash (path separator).
    Slash,

    // Literals
    /// A lowercase or snake_case identifier.
    Ident(SmolStr),
    /// A PascalCase / uppercase identifier.
    UpperIdent(SmolStr),
    /// A double-quoted string literal.
    StringLit(String),
    /// A decimal integer literal.
    DecInt(u64),
    /// A hexadecimal integer literal (e.g. `0xFF`).
    HexInt(u64),
    /// A floating-point literal.
    FloatLit(f64),

    // Ordinal (@N)
    /// An ordinal marker (e.g. `@0`, `@42`).
    Ordinal(u32),

    // Annotation sigil (@)
    /// The `@` sigil introducing an annotation.
    At,

    // Keywords
    /// `namespace` keyword.
    KwNamespace,
    /// `import` keyword.
    KwImport,
    /// `from` keyword.
    KwFrom,
    /// `as` keyword.
    KwAs,
    /// `message` keyword.
    KwMessage,
    /// `enum` keyword.
    KwEnum,
    /// `flags` keyword.
    KwFlags,
    /// `bits` keyword (inline bitfields).
    KwBits,
    /// `union` keyword.
    KwUnion,
    /// `newtype` keyword.
    KwNewtype,
    /// `config` keyword.
    KwConfig,
    /// `type` keyword (aliases).
    KwType,
    /// `const` keyword.
    KwConst,
    KwTrait,
    KwImpl,
    KwFor,
    KwFn,

    /// `optional` keyword.
    KwOptional,
    /// `array` keyword.
    KwArray,
    /// `set` keyword.
    KwSet,
    /// `map` keyword.
    KwMap,
    /// `result` keyword.
    KwResult,
    /// `true` keyword.
    KwTrue,
    /// `false` keyword.
    KwFalse,
    /// `none` keyword.
    KwNone,

    // Geometric types
    /// `vec2` keyword.
    KwVec2,
    /// `vec3` keyword.
    KwVec3,
    /// `vec4` keyword.
    KwVec4,
    /// `quat` keyword.
    KwQuat,
    /// `mat3` keyword.
    KwMat3,
    /// `mat4` keyword.
    KwMat4,

    /// `where` keyword (field constraints).
    KwWhere,
    /// `in` keyword (range constraints).
    KwIn,
    /// `value` pseudo-keyword (constraint operand).
    KwValue,

    // Special
    /// End of file marker.
    Eof,
    /// An error token (invalid character or sequence).
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
