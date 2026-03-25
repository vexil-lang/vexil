use vexil_lang::lexer;
use vexil_lang::lexer::token::TokenKind;

fn lex_kinds(source: &str) -> Vec<TokenKind> {
    let (tokens, _) = lexer::lex(source);
    tokens
        .into_iter()
        .map(|t| t.kind)
        .filter(|k| *k != TokenKind::Eof)
        .collect()
}

#[test]
fn test_punctuation() {
    let kinds = lex_kinds("{ } ( ) < > : , . = ^ -");
    assert_eq!(
        kinds,
        vec![
            TokenKind::LBrace,
            TokenKind::RBrace,
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::LAngle,
            TokenKind::RAngle,
            TokenKind::Colon,
            TokenKind::Comma,
            TokenKind::Dot,
            TokenKind::Eq,
            TokenKind::Caret,
            TokenKind::Minus,
        ]
    );
}

#[test]
fn test_ordinal_vs_annotation() {
    let kinds = lex_kinds("@0 @123 @name");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Ordinal(0),
            TokenKind::Ordinal(123),
            TokenKind::At,
            TokenKind::Ident("name".into()),
        ]
    );
}

#[test]
fn test_at_followed_by_space() {
    // version constraint: @ ^1.0.0
    let kinds = lex_kinds("@ ^");
    assert_eq!(kinds, vec![TokenKind::At, TokenKind::Caret]);
}

#[test]
fn test_keywords() {
    let kinds = lex_kinds("namespace message enum flags union newtype config import from as");
    assert_eq!(
        kinds,
        vec![
            TokenKind::KwNamespace,
            TokenKind::KwMessage,
            TokenKind::KwEnum,
            TokenKind::KwFlags,
            TokenKind::KwUnion,
            TokenKind::KwNewtype,
            TokenKind::KwConfig,
            TokenKind::KwImport,
            TokenKind::KwFrom,
            TokenKind::KwAs,
        ]
    );
}

#[test]
fn test_type_names_are_ident() {
    // Primitive/semantic type names are Ident, not keywords
    let kinds = lex_kinds("u8 u32 i64 f32 bool string bytes hash void");
    for k in &kinds {
        assert!(
            matches!(k, TokenKind::Ident(_)),
            "expected Ident, got {:?}",
            k
        );
    }
}

#[test]
fn test_parameterized_keywords() {
    let kinds = lex_kinds("optional array map result");
    assert_eq!(
        kinds,
        vec![
            TokenKind::KwOptional,
            TokenKind::KwArray,
            TokenKind::KwMap,
            TokenKind::KwResult,
        ]
    );
}

#[test]
fn test_keyword_prefix_not_consumed() {
    // "messages" should be Ident, not KwMessage
    let kinds = lex_kinds("messages");
    assert_eq!(kinds, vec![TokenKind::Ident("messages".into())]);
}

#[test]
fn test_upper_ident() {
    let kinds = lex_kinds("Foo MyType SessionId");
    assert_eq!(
        kinds,
        vec![
            TokenKind::UpperIdent("Foo".into()),
            TokenKind::UpperIdent("MyType".into()),
            TokenKind::UpperIdent("SessionId".into()),
        ]
    );
}

#[test]
fn test_integers() {
    let kinds = lex_kinds("42 0 0xFF 0x00");
    assert_eq!(
        kinds,
        vec![
            TokenKind::DecInt(42),
            TokenKind::DecInt(0),
            TokenKind::HexInt(0xFF),
            TokenKind::HexInt(0),
        ]
    );
}

#[test]
#[allow(clippy::approx_constant)]
fn test_float() {
    let kinds = lex_kinds("3.14");
    assert_eq!(kinds, vec![TokenKind::FloatLit(3.14)]);
}

#[test]
fn test_string_escapes() {
    let (tokens, diags) = lexer::lex(r#""hello\nworld" "tab\there" "quote\"end" "back\\slash""#);
    assert!(diags.is_empty());
    let strings: Vec<_> = tokens
        .iter()
        .filter_map(|t| match &t.kind {
            TokenKind::StringLit(s) => Some(s.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        strings,
        vec![
            "hello\nworld".to_string(),
            "tab\there".to_string(),
            "quote\"end".to_string(),
            "back\\slash".to_string(),
        ]
    );
}

#[test]
fn test_invalid_escape() {
    let (_, diags) = lexer::lex(r#""\a""#);
    assert!(!diags.is_empty());
}

#[test]
fn test_comments_discarded() {
    let kinds = lex_kinds("namespace # comment\ntest");
    assert_eq!(
        kinds,
        vec![TokenKind::KwNamespace, TokenKind::Ident("test".into())]
    );
}

#[test]
fn test_booleans_and_none() {
    let kinds = lex_kinds("true false none");
    assert_eq!(
        kinds,
        vec![TokenKind::KwTrue, TokenKind::KwFalse, TokenKind::KwNone]
    );
}

#[test]
fn test_void_is_ident() {
    // void is a type name, not a keyword — must be Ident
    let kinds = lex_kinds("void");
    assert_eq!(kinds, vec![TokenKind::Ident("void".into())]);
}
