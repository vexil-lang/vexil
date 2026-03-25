# Milestone B: Vexil Frontend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a complete Vexil parser that accepts all 18 valid corpus files and rejects all 56 invalid corpus files with correct `ErrorClass` diagnostics.

**Architecture:** Rust workspace with two crates — `vexil-lang` (library: lexer, parser, AST, validation) and `vexilc` (binary: CLI frontend). Hand-written recursive descent parser, corpus-driven TDD. Three layers: lexer (tokens), parser (AST), validator (semantic checks on AST).

**Tech Stack:** Rust 2021, `smol_str`, `thiserror`, `ariadne` (vexilc only), `test-case` (dev)

**Spec:** `docs/superpowers/specs/2026-03-25-reference-impl-design.md`
**Grammar:** `spec/vexil-grammar.peg`
**Corpus:** `corpus/valid/` (18 files), `corpus/invalid/` (56 files)

---

## File Map

| File | Responsibility |
|---|---|
| `Cargo.toml` | Workspace root |
| `crates/vexil-lang/Cargo.toml` | Library crate dependencies |
| `crates/vexil-lang/src/lib.rs` | Public API: `parse()`, re-exports |
| `crates/vexil-lang/src/span.rs` | `Span`, `Spanned<T>` |
| `crates/vexil-lang/src/diagnostic.rs` | `Diagnostic`, `ErrorClass`, `Severity` |
| `crates/vexil-lang/src/lexer/mod.rs` | `Lexer` struct, `pub fn lex()` |
| `crates/vexil-lang/src/lexer/token.rs` | `Token`, `TokenKind` enum |
| `crates/vexil-lang/src/parser/mod.rs` | `Parser` struct, `pub fn parse()`, core methods |
| `crates/vexil-lang/src/parser/decl.rs` | Declaration parsers: message, enum, flags, union, newtype, config |
| `crates/vexil-lang/src/parser/expr.rs` | Type expression + annotation arg parsing |
| `crates/vexil-lang/src/parser/import.rs` | Import form parsing |
| `crates/vexil-lang/src/ast/mod.rs` | All AST node types |
| `crates/vexil-lang/src/ast/visit.rs` | Visitor trait (stub for now) |
| `crates/vexil-lang/src/validate.rs` | Post-parse semantic validation pass |
| `crates/vexil-lang/tests/lexer.rs` | Lexer unit tests |
| `crates/vexil-lang/tests/corpus.rs` | Corpus-driven integration tests |
| `crates/vexilc/Cargo.toml` | Binary crate dependencies |
| `crates/vexilc/src/main.rs` | CLI entry point |

---

### Task 1: Workspace Scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `crates/vexil-lang/Cargo.toml`
- Create: `crates/vexil-lang/src/lib.rs`
- Create: `crates/vexilc/Cargo.toml`
- Create: `crates/vexilc/src/main.rs`

- [ ] **Step 1: Create workspace root `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = ["crates/vexil-lang", "crates/vexilc"]
```

- [ ] **Step 2: Create `crates/vexil-lang/Cargo.toml`**

```toml
[package]
name = "vexil-lang"
version = "0.1.0"
edition = "2021"
rust-version = "1.80"

[dependencies]
smol_str = "0.3"
thiserror = "2"

[dev-dependencies]
test-case = "3"
```

- [ ] **Step 3: Create `crates/vexil-lang/src/lib.rs`**

```rust
pub mod span;
pub mod diagnostic;
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod validate;

use diagnostic::Diagnostic;
use ast::Schema;

pub struct ParseResult {
    pub schema: Option<Schema>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Parse a Vexil schema source string.
pub fn parse(source: &str) -> ParseResult {
    let (tokens, mut diagnostics) = lexer::lex(source);
    let (schema, parse_diags) = parser::parse(source, tokens);
    diagnostics.extend(parse_diags);
    if let Some(ref schema) = schema {
        let validate_diags = validate::validate(schema);
        diagnostics.extend(validate_diags);
    }
    ParseResult { schema, diagnostics }
}
```

- [ ] **Step 4: Create `crates/vexilc/Cargo.toml`**

```toml
[package]
name = "vexilc"
version = "0.1.0"
edition = "2021"
rust-version = "1.80"

[dependencies]
vexil-lang = { path = "../vexil-lang" }
ariadne = "0.5"
```

- [ ] **Step 5: Create `crates/vexilc/src/main.rs`**

```rust
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: vexilc <file.vexil>");
        std::process::exit(1);
    }
    let source = match std::fs::read_to_string(&args[1]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {}: {e}", args[1]);
            std::process::exit(1);
        }
    };
    let result = vexil_lang::parse(&source);
    // TODO: ariadne rendering
    for diag in &result.diagnostics {
        eprintln!("{:?}", diag);
    }
    if result.diagnostics.iter().any(|d| d.severity == vexil_lang::diagnostic::Severity::Error) {
        std::process::exit(1);
    }
}
```

- [ ] **Step 6: Create stub modules so it compiles**

Create empty stubs for all modules referenced by `lib.rs`:
- `crates/vexil-lang/src/span.rs` — `Span` and `Spanned<T>` structs
- `crates/vexil-lang/src/diagnostic.rs` — `Diagnostic`, `ErrorClass`, `Severity`
- `crates/vexil-lang/src/lexer/mod.rs` — `pub fn lex()` returning empty vec
- `crates/vexil-lang/src/lexer/token.rs` — `Token`, `TokenKind`
- `crates/vexil-lang/src/ast/mod.rs` — `Schema` struct (empty)
- `crates/vexil-lang/src/ast/visit.rs` — empty
- `crates/vexil-lang/src/parser/mod.rs` — `pub fn parse()` returning None
- `crates/vexil-lang/src/parser/decl.rs` — empty
- `crates/vexil-lang/src/parser/expr.rs` — empty
- `crates/vexil-lang/src/parser/import.rs` — empty
- `crates/vexil-lang/src/validate.rs` — `pub fn validate()` returning empty vec

- [ ] **Step 7: Verify it compiles**

Run: `cargo build --workspace`
Expected: compiles with no errors

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml crates/
git commit -m "feat: workspace scaffold — vexil-lang lib + vexilc bin"
```

---

### Task 2: Span, Diagnostic, and Token Types

**Files:**
- Create: `crates/vexil-lang/src/span.rs`
- Create: `crates/vexil-lang/src/diagnostic.rs`
- Create: `crates/vexil-lang/src/lexer/token.rs`

- [ ] **Step 1: Implement `span.rs`**

```rust
/// A byte-offset range in source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub offset: u32,
    pub len: u32,
}

impl Span {
    pub fn new(offset: usize, len: usize) -> Self {
        Self {
            offset: offset as u32,
            len: len as u32,
        }
    }

    pub fn empty(offset: usize) -> Self {
        Self::new(offset, 0)
    }

    pub fn range(&self) -> std::ops::Range<usize> {
        let start = self.offset as usize;
        start..start + self.len as usize
    }
}

/// A value with an associated source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }
}
```

- [ ] **Step 2: Implement `diagnostic.rs`**

The `ErrorClass` enum must have one variant per rejection condition. See the
design spec's "Parser-detectable vs semantic-only errors" section for the full
list. Implement all variants from all three categories (parser-detectable,
semantic, type-level).

```rust
use crate::span::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorClass {
    // Lexer
    InvalidCharacter,
    InvalidEscape,
    UnterminatedString,

    // Structure
    MissingNamespace,
    DuplicateNamespace,
    ImportAfterDecl,
    ImportNamedAliasedCombined,

    // Namespace
    NamespaceInvalidComponent,
    NamespaceReserved,
    NamespaceEmpty,

    // Declaration
    DeclNameInvalid,
    DeclNameDuplicate,

    // Field
    FieldNameInvalid,
    FieldNameDuplicate,
    OrdinalDuplicate,
    OrdinalTooLarge,
    OrdinalReusedAfterRemoved,

    // Type
    UnknownType,
    ConfigTypeAsField,
    NewtypeOverNewtype,
    NewtypeOverConfig,
    InvalidMapKey,

    // Config
    ConfigMissingDefault,
    ConfigHasOrdinal,
    ConfigInvalidType,
    ConfigEncodingAnnotation,

    // Enum/Flags/Union
    EnumOrdinalDuplicate,
    EnumOrdinalTooLarge,
    EnumBackingTooNarrow,
    EnumBackingInvalid,
    EnumVariantNameInvalid,
    FlagsBitTooHigh,
    UnionOrdinalDuplicate,
    UnionOrdinalTooLarge,
    UnionVariantNameInvalid,

    // Annotation
    DuplicateAnnotation,
    NonExhaustiveInvalidTarget,
    DeprecatedMissingReason,
    RemovedMissingReason,
    LimitInvalidTarget,
    LimitExceedsGlobal,
    LimitZero,
    VarintInvalidTarget,
    ZigzagInvalidTarget,
    VarintZigzagCombined,
    DeltaInvalidTarget,
    TypeValueOverflow,
    VersionAfterNamespace,
    VersionDuplicate,
    VersionInvalidSemver,

    // Generic
    UnexpectedToken,
    UnexpectedEof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub span: Span,
    pub class: ErrorClass,
    pub message: String,
}

impl Diagnostic {
    pub fn error(span: Span, class: ErrorClass, message: impl Into<String>) -> Self {
        Self { severity: Severity::Error, span, class, message: message.into() }
    }

    pub fn warning(span: Span, class: ErrorClass, message: impl Into<String>) -> Self {
        Self { severity: Severity::Warning, span, class, message: message.into() }
    }
}
```

- [ ] **Step 3: Implement `token.rs`**

```rust
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
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo build --workspace`
Expected: compiles clean

- [ ] **Step 5: Commit**

```bash
git add crates/vexil-lang/src/span.rs crates/vexil-lang/src/diagnostic.rs crates/vexil-lang/src/lexer/token.rs
git commit -m "feat: span, diagnostic, and token type definitions"
```

---

### Task 3: Lexer

**Files:**
- Create: `crates/vexil-lang/src/lexer/mod.rs`
- Create: `crates/vexil-lang/tests/lexer.rs`

- [ ] **Step 1: Write failing lexer tests**

Create `crates/vexil-lang/tests/lexer.rs`:

```rust
use vexil_lang::lexer;
use vexil_lang::lexer::token::TokenKind;

fn lex_kinds(source: &str) -> Vec<TokenKind> {
    let (tokens, _) = lexer::lex(source);
    tokens.into_iter().map(|t| t.kind).filter(|k| *k != TokenKind::Eof).collect()
}

#[test]
fn test_punctuation() {
    let kinds = lex_kinds("{ } ( ) < > : , . = ^ -");
    assert_eq!(kinds, vec![
        TokenKind::LBrace, TokenKind::RBrace,
        TokenKind::LParen, TokenKind::RParen,
        TokenKind::LAngle, TokenKind::RAngle,
        TokenKind::Colon, TokenKind::Comma,
        TokenKind::Dot, TokenKind::Eq,
        TokenKind::Caret, TokenKind::Minus,
    ]);
}

#[test]
fn test_ordinal_vs_annotation() {
    let kinds = lex_kinds("@0 @123 @name");
    assert_eq!(kinds, vec![
        TokenKind::Ordinal(0),
        TokenKind::Ordinal(123),
        TokenKind::At,
        TokenKind::Ident("name".into()),
    ]);
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
    assert_eq!(kinds, vec![
        TokenKind::KwNamespace, TokenKind::KwMessage, TokenKind::KwEnum,
        TokenKind::KwFlags, TokenKind::KwUnion, TokenKind::KwNewtype,
        TokenKind::KwConfig, TokenKind::KwImport, TokenKind::KwFrom,
        TokenKind::KwAs,
    ]);
}

#[test]
fn test_type_names_are_ident() {
    // Primitive/semantic type names are Ident, not keywords
    let kinds = lex_kinds("u8 u32 i64 f32 bool string bytes hash void");
    for k in &kinds {
        assert!(matches!(k, TokenKind::Ident(_)), "expected Ident, got {:?}", k);
    }
}

#[test]
fn test_parameterized_keywords() {
    let kinds = lex_kinds("optional array map result");
    assert_eq!(kinds, vec![
        TokenKind::KwOptional, TokenKind::KwArray,
        TokenKind::KwMap, TokenKind::KwResult,
    ]);
}

#[test]
fn test_keyword_prefix_not_consumed() {
    // "messages" should be UpperIdent or Ident, not KwMessage
    let kinds = lex_kinds("messages");
    assert_eq!(kinds, vec![TokenKind::Ident("messages".into())]);
}

#[test]
fn test_upper_ident() {
    let kinds = lex_kinds("Foo MyType SessionId");
    assert_eq!(kinds, vec![
        TokenKind::UpperIdent("Foo".into()),
        TokenKind::UpperIdent("MyType".into()),
        TokenKind::UpperIdent("SessionId".into()),
    ]);
}

#[test]
fn test_integers() {
    let kinds = lex_kinds("42 0 0xFF 0x00");
    assert_eq!(kinds, vec![
        TokenKind::DecInt(42), TokenKind::DecInt(0),
        TokenKind::HexInt(0xFF), TokenKind::HexInt(0),
    ]);
}

#[test]
fn test_float() {
    let kinds = lex_kinds("3.14");
    assert_eq!(kinds, vec![TokenKind::FloatLit(3.14)]);
}

#[test]
fn test_string_escapes() {
    let (tokens, diags) = lexer::lex(r#""hello\nworld" "tab\there" "quote\"end" "back\\slash""#);
    assert!(diags.is_empty());
    let strings: Vec<_> = tokens.iter().filter_map(|t| match &t.kind {
        TokenKind::StringLit(s) => Some(s.clone()),
        _ => None,
    }).collect();
    assert_eq!(strings, vec![
        "hello\nworld".to_string(),
        "tab\there".to_string(),
        "quote\"end".to_string(),
        "back\\slash".to_string(),
    ]);
}

#[test]
fn test_invalid_escape() {
    let (_, diags) = lexer::lex(r#""\a""#);
    assert!(!diags.is_empty());
}

#[test]
fn test_comments_discarded() {
    let kinds = lex_kinds("namespace # comment\ntest");
    assert_eq!(kinds, vec![TokenKind::KwNamespace, TokenKind::Ident("test".into())]);
}

#[test]
fn test_booleans_and_none() {
    let kinds = lex_kinds("true false none");
    assert_eq!(kinds, vec![TokenKind::KwTrue, TokenKind::KwFalse, TokenKind::KwNone]);
}

#[test]
fn test_void_is_ident() {
    // void is a type name, not a keyword — must be Ident
    let kinds = lex_kinds("void");
    assert_eq!(kinds, vec![TokenKind::Ident("void".into())]);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p vexil-lang --test lexer`
Expected: FAIL (lex() is a stub)

- [ ] **Step 3: Implement lexer**

Implement `crates/vexil-lang/src/lexer/mod.rs`. The lexer is a `Lexer` struct
with fields `source: &str`, `pos: usize`, `diagnostics: Vec<Diagnostic>`. Key
implementation notes:

- Main `lex()` function: loop calling `next_token()` until EOF.
- `next_token()`: skip whitespace and comments, then match first char:
  - `{` `}` `(` `)` `<` `>` `:` `,` `.` `=` `^` `-` → punctuation tokens
  - `#` → skip to newline (comment)
  - `"` → `lex_string()`: consume chars, handle `\n` `\t` `\r` `\\` `\"` escapes,
    reject unknown escapes with `InvalidEscape` diagnostic
  - `@` → peek next: if digit → `lex_ordinal()`, else → `At`
  - `0` followed by `x`/`X` → `lex_hex_int()`
  - digit → `lex_number()`: scan digits, if `.` followed by digit → `FloatLit`,
    else → `DecInt`. Note: the Vexil grammar does not support exponent notation
    (e.g., `1e10`); only `digits.digits` is a valid float literal.
  - `A-Z` → `lex_upper_ident()`
  - `a-z` → `lex_word()`: scan `[a-z0-9_]*`, check `!ident-continue`, then match
    against keyword table. Keywords: `namespace`, `import`, `from`, `as`,
    `message`, `enum`, `flags`, `union`, `newtype`, `config`, `optional`, `array`,
    `map`, `result`, `true`, `false`, `none`. Anything else → `Ident`.
    Note: `void`, `bool`, `string`, `bytes`, etc. are NOT keywords — they stay as `Ident`.
    The `-` character is always lexed as a standalone `Minus` token. The lexer never
    combines `-` with following digits into a single `DecInt` — negative values are
    assembled by the parser from `Minus` + `DecInt`.
  - anything else → `InvalidCharacter` diagnostic + `Error` token

`ident-continue` check: after scanning an identifier/keyword, peek next char. If
it's `[A-Za-z0-9_]`, the token continues and should NOT match the keyword. Example:
`messages` starts with `message` but the next char is `s`, so it's `Ident("messages")`.

For the keyword table, only the 18 declaration/structure keywords listed in the
design spec's `TokenKind` become keyword tokens. Type names (`u8`, `string`, etc.)
stay as `Ident`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p vexil-lang --test lexer`
Expected: all PASS

- [ ] **Step 5: Commit**

```bash
git add crates/vexil-lang/src/lexer/ crates/vexil-lang/tests/lexer.rs
git commit -m "feat: lexer — tokenizes all Vexil syntax"
```

---

### Task 4: AST Types

**Files:**
- Create: `crates/vexil-lang/src/ast/mod.rs`
- Create: `crates/vexil-lang/src/ast/visit.rs`

- [ ] **Step 1: Implement AST types**

Implement `crates/vexil-lang/src/ast/mod.rs` with all node types from the design
spec's AST section. Every node gets `#[derive(Debug, Clone, PartialEq)]`.

Key types to implement:
- `Schema { span, annotations, namespace, imports, declarations }`
- `NamespaceDecl { span, path: Vec<Spanned<SmolStr>> }`
- `ImportDecl { span, kind: ImportKind, path, version }`
- `ImportKind { Wildcard, Named { names }, Aliased { alias } }`
- `Decl` enum: `Message`, `Enum`, `Flags`, `Union`, `Newtype`, `Config`
- `MessageDecl { span, annotations, name, body: Vec<MessageBodyItem> }`
- `MessageBodyItem` enum: `Field(MessageField)`, `Tombstone(Tombstone)`
- `MessageField { span, pre_annotations, name, ordinal, post_ordinal_annotations, ty, post_type_annotations }`
- `EnumDecl { span, annotations, name, backing, body: Vec<EnumBodyItem> }`
- `EnumBodyItem` enum: `Variant(EnumVariant)`, `Tombstone(Tombstone)`
- `EnumVariant { span, annotations, name, ordinal }`
- `FlagsDecl { span, annotations, name, body: Vec<FlagsBodyItem> }`
- `FlagsBodyItem` enum: `Bit(FlagsBit)`, `Tombstone(Tombstone)`
- `FlagsBit { span, annotations, name, ordinal }`
- `UnionDecl { span, annotations, name, body: Vec<UnionVariant> }`
- `UnionVariant { span, annotations, name, ordinal, fields: Vec<MessageBodyItem> }`
- `NewtypeDecl { span, annotations, name, inner_type }`
- `ConfigDecl { span, annotations, name, fields: Vec<ConfigField> }`
- `ConfigField { span, annotations, name, ty, default_value }`
- `TypeExpr` enum: `Primitive(PrimitiveType)`, `SubByte(SubByteType)`, `Semantic(SemanticType)`, `Named(SmolStr)`, `Qualified(SmolStr, SmolStr)`, `Optional(Box<Spanned<TypeExpr>>)`, `Array(Box<Spanned<TypeExpr>>)`, `Map(Box<Spanned<TypeExpr>>, Box<Spanned<TypeExpr>>)`, `Result(Box<Spanned<TypeExpr>>, Box<Spanned<TypeExpr>>)`
- `PrimitiveType` enum: `Bool`, `U8`, `U16`, `U32`, `U64`, `I8`, `I16`, `I32`, `I64`, `F32`, `F64`, `Void`
- `SubByteType { signed: bool, bits: u8 }`
- `SemanticType` enum: `String`, `Bytes`, `Rgb`, `Uuid`, `Timestamp`, `Hash`
- `Annotation { span, name, args: Option<Vec<AnnotationArg>> }`
- `AnnotationArg { span, key: Option<Spanned<SmolStr>>, value: Spanned<AnnotationValue> }`
- `AnnotationValue` enum: `Int(u64)`, `Hex(u64)`, `Str(String)`, `Bool(bool)`, `Ident(SmolStr)`, `UpperIdent(SmolStr)`
- `DefaultValue` enum: `None`, `Bool(bool)`, `Int(i64)`, `UInt(u64)`, `Float(f64)`, `Str(String)`, `Ident(SmolStr)`, `UpperIdent(SmolStr)`, `Array(Vec<Spanned<DefaultValue>>)`
- `Tombstone { span, ordinal: Spanned<u32>, args: Vec<TombstoneArg> }`
- `TombstoneArg { span, key: Spanned<SmolStr>, value: Spanned<AnnotationValue> }`
- `EnumBacking` enum: `U8`, `U16`, `U32`, `U64`

- [ ] **Step 2: Create `visit.rs` stub**

```rust
// Visitor trait — stub for future passes.
// Will be populated when IR lowering is implemented.
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build -p vexil-lang`
Expected: compiles clean

- [ ] **Step 4: Commit**

```bash
git add crates/vexil-lang/src/ast/
git commit -m "feat: AST node types — all declaration kinds, types, annotations"
```

---

### Task 5: Parser Core + Namespace + Minimal Corpus Test

**Files:**
- Create: `crates/vexil-lang/src/parser/mod.rs`
- Create: `crates/vexil-lang/tests/corpus.rs`

- [ ] **Step 1: Write failing corpus test for `001_minimal.vexil`**

Create `crates/vexil-lang/tests/corpus.rs`:

```rust
use vexil_lang::diagnostic::Severity;

fn parse_valid(file: &str) {
    let path = format!("{}/../../corpus/valid/{file}", env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
    let result = vexil_lang::parse(&source);
    let errors: Vec<_> = result.diagnostics.iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "expected no errors in {file}, got: {errors:#?}");
}

#[test]
fn valid_001_minimal() {
    parse_valid("001_minimal.vexil");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vexil-lang --test corpus valid_001`
Expected: FAIL

- [ ] **Step 3: Implement parser core + `parse_schema()` + `parse_namespace()`**

Implement `crates/vexil-lang/src/parser/mod.rs`:

- `Parser` struct with `tokens`, `pos`, `source`, `diagnostics`
- Core methods: `peek()`, `advance()`, `expect()`, `at()`, `at_ident()`,
  `checkpoint()`, `backtrack()`, `span_from()` (creates span from saved start to current)
- `pub fn parse()` — constructs `Parser`, calls `parse_schema()`
- `parse_schema()`:
  1. Call `parse_annotations()` to collect schema-level annotations
  2. Expect `KwNamespace` → call `parse_namespace()`
  3. If missing, emit `MissingNamespace` error
  4. Loop: while `KwImport`, call `parse_import()` (stub for now, returns dummy)
  5. Loop: while declaration keyword, call `parse_type_decl()` (stub for now)
  6. Check for trailing tokens (emit error if not EOF)
- `parse_namespace()`: expect `KwNamespace`, then parse dot-separated components.
  Each component is an `Ident` (lowercase). `UpperIdent` at component position → `NamespaceInvalidComponent`.
- `parse_annotations()`: loop while `At` token, call `parse_annotation()`:
  consume `At`, expect `Ident` for name, if `LParen` follows parse annotation args, return `Annotation`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vexil-lang --test corpus valid_001`
Expected: PASS

- [ ] **Step 5: Add failing test for `001_missing_namespace.vexil`**

Add to `corpus.rs`:

```rust
use vexil_lang::diagnostic::ErrorClass;

fn parse_invalid(file: &str, expected: ErrorClass) {
    let path = format!("{}/../../corpus/invalid/{file}", env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
    let result = vexil_lang::parse(&source);
    let has_expected = result.diagnostics.iter()
        .any(|d| d.class == expected && d.severity == Severity::Error);
    assert!(has_expected,
        "expected {expected:?} in {file}, got: {:#?}", result.diagnostics);
}

#[test]
fn invalid_001_missing_namespace() {
    parse_invalid("001_missing_namespace.vexil", ErrorClass::MissingNamespace);
}
```

- [ ] **Step 6: Run test to verify it passes** (MissingNamespace should already work)

Run: `cargo test -p vexil-lang --test corpus invalid_001`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add crates/vexil-lang/src/parser/mod.rs crates/vexil-lang/tests/corpus.rs
git commit -m "feat: parser core + namespace parsing + first corpus tests"
```

---

### Task 6: Import Parsing

**Files:**
- Modify: `crates/vexil-lang/src/parser/import.rs`
- Modify: `crates/vexil-lang/tests/corpus.rs`

- [ ] **Step 1: Add corpus tests for imports**

Add to `corpus.rs`:
```rust
#[test]
fn valid_012_imports() { parse_valid("012_imports.vexil"); }

#[test]
fn invalid_023_import_after_decl() {
    parse_invalid("023_import_after_decl.vexil", ErrorClass::ImportAfterDecl);
}

#[test]
fn invalid_024_import_named_aliased() {
    parse_invalid("024_import_named_aliased.vexil", ErrorClass::ImportNamedAliasedCombined);
}

#[test]
fn invalid_042_version_not_semver() {
    parse_invalid("042_version_not_semver.vexil", ErrorClass::VersionInvalidSemver);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p vexil-lang --test corpus`
Expected: import tests FAIL

- [ ] **Step 3: Implement import parsing**

Implement `crates/vexil-lang/src/parser/import.rs` with `parse_import()`:

Left-factored dispatch per design spec:
1. Expect `KwImport`
2. If `LBrace` → named: parse `{ UpperIdent, UpperIdent }`, expect `KwFrom`, parse namespace, optional version
3. Else → parse namespace path + optional version constraint
4. After namespace+version: if `KwAs` → aliased, else → wildcard
5. For named path: if `KwAs` also follows → emit `ImportNamedAliasedCombined`
6. Version constraint: `At` + `Caret` + three `DecInt` separated by `Dot`. Missing patch → `VersionInvalidSemver`

Also update `parse_schema()` to detect imports appearing after declarations → `ImportAfterDecl`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p vexil-lang --test corpus`
Expected: all PASS

- [ ] **Step 5: Commit**

```bash
git add crates/vexil-lang/src/parser/import.rs crates/vexil-lang/tests/corpus.rs
git commit -m "feat: import parsing — all 6 forms + version constraints"
```

---

### Task 7: Type Expression Parsing

**Files:**
- Modify: `crates/vexil-lang/src/parser/expr.rs`
- Modify: `crates/vexil-lang/tests/corpus.rs`

- [ ] **Step 1: Add corpus tests**

```rust
#[test]
fn valid_002_primitives() { parse_valid("002_primitives.vexil"); }
#[test]
fn valid_003_sub_byte() { parse_valid("003_sub_byte.vexil"); }
#[test]
fn valid_004_semantic_types() { parse_valid("004_semantic_types.vexil"); }
#[test]
fn valid_005_parameterized() { parse_valid("005_parameterized.vexil"); }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p vexil-lang --test corpus valid_00`
Expected: FAIL

- [ ] **Step 3: Implement type expression parsing**

Implement `parse_type_expr()` in `crates/vexil-lang/src/parser/expr.rs`:

1. If `KwOptional` → consume, expect `<`, recurse `parse_type_expr()`, expect `>`
2. If `KwArray` → same pattern, single type param
3. If `KwMap` → consume, expect `<`, type, `,`, type, `>`
4. If `KwResult` → same as map pattern
5. Else → `parse_named_type()`:
   a. If `UpperIdent` followed by `Dot` + `UpperIdent` → `Qualified`
   b. If `UpperIdent` → `Named`
   c. If `Ident` → match string value:
      - `"bool"`, `"u8"`, `"u16"`, `"u32"`, `"u64"`, `"i8"`, `"i16"`, `"i32"`, `"i64"`, `"f32"`, `"f64"`, `"void"` → `Primitive`
      - `"string"`, `"bytes"`, `"rgb"`, `"uuid"`, `"timestamp"`, `"hash"` → `Semantic`
      - Pattern `u` + digits (not 8/16/32/64) → `SubByte { signed: false, bits }`
      - Pattern `i` + digits (not 8/16/32/64) → `SubByte { signed: true, bits }`
      - Anything else → `Named` (user-defined type, forward ref)

Also implement `parse_annotation_args()` and `parse_annotation_value()` here:
- `annotation_value`: try `HexInt` → `Hex`, `DecInt` → `Int`, `StringLit` → `Str`,
  `KwTrue`/`KwFalse` → `Bool`, `UpperIdent` → `UpperIdent`, `Ident` → `Ident`
- `annotation_arg`: if `Ident` followed by `Colon` → named, else → positional

Also implement `parse_literal_value()` for config defaults:
- `KwNone` → `None`, `KwTrue`/`KwFalse` → `Bool`, `LBracket` → array literal,
  `HexInt` → `UInt`, `FloatLit` → `Float`, `Minus` + `DecInt` → `Int` (negative),
  `Minus` + `FloatLit` → `Float` (negative), `DecInt` → `UInt`,
  `StringLit` → `Str`, `UpperIdent` → `UpperIdent`, `Ident` → `Ident`

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p vexil-lang --test corpus`
Expected: all PASS

- [ ] **Step 5: Commit**

```bash
git add crates/vexil-lang/src/parser/expr.rs crates/vexil-lang/tests/corpus.rs
git commit -m "feat: type expression + literal value parsing"
```

---

### Task 8: Declaration Parsing — Message, Enum, Flags

**Files:**
- Modify: `crates/vexil-lang/src/parser/decl.rs`
- Modify: `crates/vexil-lang/tests/corpus.rs`

- [ ] **Step 1: Add corpus tests**

```rust
#[test]
fn valid_006_message() { parse_valid("006_message.vexil"); }
#[test]
fn valid_007_enum() { parse_valid("007_enum.vexil"); }
#[test]
fn valid_008_flags() { parse_valid("008_flags.vexil"); }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p vexil-lang --test corpus valid_00`
Expected: FAIL

- [ ] **Step 3: Implement declaration parsing**

Implement in `crates/vexil-lang/src/parser/decl.rs`:

**`parse_type_decl()`**: match current keyword → dispatch to specific parser.

**`parse_message_decl()`**:
1. Annotations already collected by caller
2. Consume `KwMessage`, expect `UpperIdent` (name), expect `LBrace`
3. Loop `parse_message_body()`: try tombstone (At + Ident("removed")), then field
4. Expect `RBrace`

**`parse_field()`**:
1. `pre_annotations` = annotations already parsed
2. `parse_field_name()` — accepts `Ident` OR any keyword token
3. Expect `Ordinal`
4. `post_ordinal_annotations` = parse annotations
5. Expect `Colon`
6. `parse_type_expr()`
7. `post_type_annotations` = parse annotations

**`parse_tombstone()`**:
1. Consume `At` + `Ident("removed")`
2. Expect `LParen`, `DecInt` (ordinal), `Comma`
3. Loop: parse named args (`Ident : annotation-value`)
4. Expect `RParen`

**`parse_enum_decl()`**:
1. Consume `KwEnum`, expect `UpperIdent`
2. Optional backing: if `Colon` → expect `Ident` matching `u8`/`u16`/`u32`/`u64`
3. Expect `LBrace`
4. Loop body items: tombstone or enum variant (`UpperIdent` + `Ordinal`)
5. Expect `RBrace`

**`parse_flags_decl()`**: Same as enum but no backing type. Bits = `UpperIdent` + `Ordinal`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p vexil-lang --test corpus`
Expected: all PASS

- [ ] **Step 5: Commit**

```bash
git add crates/vexil-lang/src/parser/decl.rs crates/vexil-lang/tests/corpus.rs
git commit -m "feat: message, enum, flags declaration parsing"
```

---

### Task 9: Declaration Parsing — Union, Newtype, Config

**Files:**
- Modify: `crates/vexil-lang/src/parser/decl.rs`
- Modify: `crates/vexil-lang/tests/corpus.rs`

- [ ] **Step 1: Add corpus tests**

```rust
#[test]
fn valid_009_union() { parse_valid("009_union.vexil"); }
#[test]
fn valid_010_newtype() { parse_valid("010_newtype.vexil"); }
#[test]
fn valid_011_config() { parse_valid("011_config.vexil"); }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p vexil-lang --test corpus`
Expected: new tests FAIL

- [ ] **Step 3: Implement remaining declaration parsers**

**`parse_union_decl()`**:
1. Consume `KwUnion`, expect `UpperIdent`, expect `LBrace`
2. Loop variants: annotations + `UpperIdent` + `Ordinal` + `LBrace` + variant body + `RBrace`
3. Variant body: same as message body (tombstone / field)

**`parse_newtype_decl()`**:
1. Consume `KwNewtype`, expect `UpperIdent`, expect `Colon`, `parse_type_expr()`

**`parse_config_decl()`**:
1. Consume `KwConfig`, expect `UpperIdent`, expect `LBrace`
2. Loop config fields: annotations + `parse_field_name()` + `:` + type + `=` + literal value
3. Expect `RBrace`

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p vexil-lang --test corpus`
Expected: all PASS

- [ ] **Step 5: Commit**

```bash
git add crates/vexil-lang/src/parser/decl.rs crates/vexil-lang/tests/corpus.rs
git commit -m "feat: union, newtype, config declaration parsing"
```

---

### Task 10: Remaining Valid Corpus Tests

**Files:**
- Modify: `crates/vexil-lang/tests/corpus.rs`

- [ ] **Step 1: Add all remaining valid corpus tests**

```rust
#[test]
fn valid_013_annotations() { parse_valid("013_annotations.vexil"); }
#[test]
fn valid_014_keywords_as_fields() { parse_valid("014_keywords_as_fields.vexil"); }
#[test]
fn valid_015_forward_refs() { parse_valid("015_forward_refs.vexil"); }
#[test]
fn valid_016_recursive() { parse_valid("016_recursive.vexil"); }
#[test]
fn valid_017_escapes() { parse_valid("017_escapes.vexil"); }
#[test]
fn valid_018_comments() { parse_valid("018_comments.vexil"); }
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p vexil-lang --test corpus valid_`
Expected: all 18 PASS. If any fail, fix the parser/lexer.

- [ ] **Step 3: Commit**

```bash
git add crates/vexil-lang/tests/corpus.rs
git commit -m "test: all 18 valid corpus tests passing"
```

---

### Task 11: Parser-Detectable Invalid Corpus Tests

**Files:**
- Modify: `crates/vexil-lang/tests/corpus.rs`

These invalid tests should already pass from the parser implementation in Tasks 5–9.

- [ ] **Step 1: Add parser-detectable invalid tests**

```rust
// Lexer errors
#[test] fn invalid_021() { parse_invalid("021_invalid_escape.vexil", ErrorClass::InvalidEscape); }

// Structure errors
#[test] fn invalid_001() { parse_invalid("001_missing_namespace.vexil", ErrorClass::MissingNamespace); }
#[test] fn invalid_002() { parse_invalid("002_duplicate_namespace.vexil", ErrorClass::DuplicateNamespace); }
#[test] fn invalid_005() { parse_invalid("005_namespace_empty.vexil", ErrorClass::NamespaceEmpty); }
#[test] fn invalid_023() { parse_invalid("023_import_after_decl.vexil", ErrorClass::ImportAfterDecl); }
#[test] fn invalid_024() { parse_invalid("024_import_named_aliased.vexil", ErrorClass::ImportNamedAliasedCombined); }
#[test] fn invalid_042() { parse_invalid("042_version_not_semver.vexil", ErrorClass::VersionInvalidSemver); }

// Namespace errors
#[test] fn invalid_003() { parse_invalid("003_namespace_invalid_component.vexil", ErrorClass::NamespaceInvalidComponent); }

// Name validation
#[test] fn invalid_006() { parse_invalid("006_decl_name_lowercase.vexil", ErrorClass::DeclNameInvalid); }
#[test] fn invalid_007() { parse_invalid("007_decl_name_underscore.vexil", ErrorClass::DeclNameInvalid); }
#[test] fn invalid_009() { parse_invalid("009_field_name_uppercase.vexil", ErrorClass::FieldNameInvalid); }
#[test] fn invalid_039() { parse_invalid("039_union_variant_lowercase.vexil", ErrorClass::UnionVariantNameInvalid); }
#[test] fn invalid_047() { parse_invalid("047_enum_variant_lowercase.vexil", ErrorClass::EnumVariantNameInvalid); }

// Config parse errors
#[test] fn invalid_016() { parse_invalid("016_config_missing_default.vexil", ErrorClass::ConfigMissingDefault); }
#[test] fn invalid_017() { parse_invalid("017_config_with_ordinal.vexil", ErrorClass::ConfigHasOrdinal); }

// Tombstone
#[test] fn invalid_040() { parse_invalid("040_removed_missing_reason.vexil", ErrorClass::RemovedMissingReason); }

// Version
#[test] fn invalid_055() { parse_invalid("055_namespace_before_version.vexil", ErrorClass::VersionAfterNamespace); }
```

- [ ] **Step 2: Run tests — all should already pass**

Run: `cargo test -p vexil-lang --test corpus invalid_`
Expected: all PASS (these are detected during parsing). Fix any that fail.

- [ ] **Step 3: Commit**

```bash
git add crates/vexil-lang/tests/corpus.rs
git commit -m "test: parser-detectable invalid corpus tests — all passing"
```

---

### Task 12: Semantic Validation Pass

**Files:**
- Create: `crates/vexil-lang/src/validate.rs`
- Modify: `crates/vexil-lang/tests/corpus.rs`

- [ ] **Step 1: Add semantic/type invalid corpus tests**

These tests should all FAIL before implementing `validate.rs`:

```rust
// Namespace semantic
#[test] fn invalid_004() { parse_invalid("004_namespace_reserved.vexil", ErrorClass::NamespaceReserved); }

// Declaration semantic
#[test] fn invalid_008() { parse_invalid("008_decl_name_duplicate.vexil", ErrorClass::DeclNameDuplicate); }

// Ordinal semantic
#[test] fn invalid_010() { parse_invalid("010_duplicate_ordinal.vexil", ErrorClass::OrdinalDuplicate); }
#[test] fn invalid_011() { parse_invalid("011_ordinal_too_large.vexil", ErrorClass::OrdinalTooLarge); }
#[test] fn invalid_012() { parse_invalid("012_duplicate_field_name.vexil", ErrorClass::FieldNameDuplicate); }
#[test] fn invalid_041() { parse_invalid("041_removed_reuses_ordinal.vexil", ErrorClass::OrdinalReusedAfterRemoved); }

// Enum/flags/union semantic
#[test] fn invalid_033() { parse_invalid("033_enum_duplicate_ordinal.vexil", ErrorClass::EnumOrdinalDuplicate); }
#[test] fn invalid_034() { parse_invalid("034_enum_ordinal_overflow.vexil", ErrorClass::EnumOrdinalTooLarge); }
#[test] fn invalid_035() { parse_invalid("035_enum_backing_too_narrow.vexil", ErrorClass::EnumBackingTooNarrow); }
#[test] fn invalid_036() { parse_invalid("036_flags_bit_too_high.vexil", ErrorClass::FlagsBitTooHigh); }
#[test] fn invalid_037() { parse_invalid("037_union_duplicate_ordinal.vexil", ErrorClass::UnionOrdinalDuplicate); }
#[test] fn invalid_038() { parse_invalid("038_union_ordinal_overflow.vexil", ErrorClass::UnionOrdinalTooLarge); }
#[test] fn invalid_051() { parse_invalid("051_enum_backing_invalid_type.vexil", ErrorClass::EnumBackingInvalid); }

// Annotation semantic
#[test] fn invalid_022() { parse_invalid("022_duplicate_annotation.vexil", ErrorClass::DuplicateAnnotation); }
#[test] fn invalid_056() { parse_invalid("056_duplicate_version.vexil", ErrorClass::VersionDuplicate); }
#[test] fn invalid_054() { parse_invalid("054_limit_zero.vexil", ErrorClass::LimitZero); }
#[test] fn invalid_045() { parse_invalid("045_limit_exceeds_global.vexil", ErrorClass::LimitExceedsGlobal); }
#[test] fn invalid_050() { parse_invalid("050_type_domain_bad_arg.vexil", ErrorClass::TypeValueOverflow); }

// Type-level errors
#[test] fn invalid_046() { parse_invalid("046_type_unknown.vexil", ErrorClass::UnknownType); }
#[test] fn invalid_013() { parse_invalid("013_field_references_config.vexil", ErrorClass::ConfigTypeAsField); }
#[test] fn invalid_014() { parse_invalid("014_newtype_over_newtype.vexil", ErrorClass::NewtypeOverNewtype); }
#[test] fn invalid_015() { parse_invalid("015_newtype_over_config.vexil", ErrorClass::NewtypeOverConfig); }
#[test] fn invalid_029() { parse_invalid("029_map_invalid_key.vexil", ErrorClass::InvalidMapKey); }
#[test] fn invalid_030() { parse_invalid("030_map_void_key.vexil", ErrorClass::InvalidMapKey); }
#[test] fn invalid_031() { parse_invalid("031_map_message_key.vexil", ErrorClass::InvalidMapKey); }
#[test] fn invalid_032() { parse_invalid("032_map_optional_key.vexil", ErrorClass::InvalidMapKey); }
#[test] fn invalid_018() { parse_invalid("018_config_map_type.vexil", ErrorClass::ConfigInvalidType); }
#[test] fn invalid_019() { parse_invalid("019_config_result_type.vexil", ErrorClass::ConfigInvalidType); }

// Annotation-target errors
#[test] fn invalid_043() { parse_invalid("043_non_exhaustive_on_message.vexil", ErrorClass::NonExhaustiveInvalidTarget); }
#[test] fn invalid_025() { parse_invalid("025_varint_on_subbyte.vexil", ErrorClass::VarintInvalidTarget); }
#[test] fn invalid_027() { parse_invalid("027_varint_on_signed.vexil", ErrorClass::VarintInvalidTarget); }
#[test] fn invalid_053() { parse_invalid("053_varint_on_float.vexil", ErrorClass::VarintInvalidTarget); }
#[test] fn invalid_026() { parse_invalid("026_zigzag_on_unsigned.vexil", ErrorClass::ZigzagInvalidTarget); }
#[test] fn invalid_052() { parse_invalid("052_zigzag_on_subbyte.vexil", ErrorClass::ZigzagInvalidTarget); }
#[test] fn invalid_028() { parse_invalid("028_varint_zigzag_combined.vexil", ErrorClass::VarintZigzagCombined); }
#[test] fn invalid_049() { parse_invalid("049_delta_on_string.vexil", ErrorClass::DeltaInvalidTarget); }
#[test] fn invalid_044() { parse_invalid("044_limit_on_invalid_type.vexil", ErrorClass::LimitInvalidTarget); }
#[test] fn invalid_048() { parse_invalid("048_deprecated_missing_reason.vexil", ErrorClass::DeprecatedMissingReason); }
#[test] fn invalid_020() { parse_invalid("020_config_encoding_annotation.vexil", ErrorClass::ConfigEncodingAnnotation); }
```

- [ ] **Step 2: Run tests to verify they all fail**

Run: `cargo test -p vexil-lang --test corpus invalid_`
Expected: all new semantic/type tests FAIL (validation not yet implemented)

- [ ] **Step 3: Implement `validate.rs`**

Implement `pub fn validate(schema: &Schema) -> Vec<Diagnostic>`. This walks the
AST and checks every semantic/type rule from the design spec:

**Namespace checks:**
- `NamespaceReserved`: path starts with "vexil"

**Declaration-level:**
- `DeclNameDuplicate`: collect all decl names, check uniqueness
- `VersionDuplicate`: count `@version` in schema annotations
- `DuplicateAnnotation`: for each element, check no annotation name repeats
  (except `@doc` which allows repetition)

**Per-message/union-variant:**
- `OrdinalDuplicate`: check field ordinals unique (include tombstone ordinals)
- `OrdinalTooLarge`: any ordinal > 65535
- `OrdinalReusedAfterRemoved`: field ordinal matches a tombstone ordinal
- `FieldNameDuplicate`: check field names unique

**Enum-specific:**
- `EnumOrdinalDuplicate`, `EnumOrdinalTooLarge`
- `EnumBackingTooNarrow`: if backing is u8 and max ordinal > 255, etc.
- `EnumBackingInvalid`: backing type string not in {u8, u16, u32, u64}

**Flags-specific:**
- `FlagsBitTooHigh`: bit position > 63

**Union-specific:**
- `UnionOrdinalDuplicate`, `UnionOrdinalTooLarge`

**Type-level checks (requires knowing declaration kinds):**
Build a map of `decl_name → DeclKind` from the schema, then:
- `UnknownType`: `Named` type not in decl map and not a primitive/semantic/sub-byte
- `ConfigTypeAsField`: field references a config decl
- `NewtypeOverNewtype`: newtype's inner type references another newtype
- `NewtypeOverConfig`: newtype's inner type references a config
- `InvalidMapKey`: map key type is f32/f64/void/optional/array/map/result/message/union/newtype/config
- `ConfigInvalidType`: config field type is map or result

**Annotation-target checks:**
- `NonExhaustiveInvalidTarget`: `@non_exhaustive` on anything other than enum/union
- `VarintInvalidTarget`: `@varint` not on u8/u16/u32/u64
- `ZigzagInvalidTarget`: `@zigzag` not on i8/i16/i32/i64
- `VarintZigzagCombined`: field has both
- `DeltaInvalidTarget`: `@delta` on non-numeric type
- `LimitInvalidTarget`: `@limit` not on string/bytes/array/map
- `LimitExceedsGlobal`: `@limit(N)` where N > 16M for array/map or > 64M for string/bytes
- `LimitZero`: `@limit(0)`
- `TypeValueOverflow`: `@type(N)` where N > 255
- `DeprecatedMissingReason`: `@deprecated` without `reason` arg
- `ConfigEncodingAnnotation`: `@varint`/`@zigzag`/`@delta` on config field

- [ ] **Step 4: Run all tests**

Run: `cargo test -p vexil-lang --test corpus`
Expected: all 74 tests PASS (18 valid + 56 invalid)

- [ ] **Step 5: Commit**

```bash
git add crates/vexil-lang/src/validate.rs crates/vexil-lang/tests/corpus.rs
git commit -m "feat: semantic validation — all 74 corpus tests passing"
```

---

### Task 13: CLI Polish + Final Verification

**Files:**
- Modify: `crates/vexilc/src/main.rs`

- [ ] **Step 1: Implement ariadne error rendering in vexilc**

Update `main.rs` to convert `Diagnostic` to ariadne `Report`:
- Map `Span` → ariadne `Source` + byte range
- Render with colors, underlines, error class name
- Print to stderr

```rust
use ariadne::{Color, Label, Report, ReportKind, Source};
use vexil_lang::diagnostic::{Severity, Diagnostic};

fn render_diagnostic(filename: &str, source: &str, diag: &Diagnostic) {
    let kind = match diag.severity {
        Severity::Error => ReportKind::Error,
        Severity::Warning => ReportKind::Warning,
    };
    let range = diag.span.range();
    Report::build(kind, filename, range.start)
        .with_message(&diag.message)
        .with_label(
            Label::new((filename, range))
                .with_message(format!("{:?}", diag.class))
                .with_color(Color::Red),
        )
        .finish()
        .eprint((filename, Source::from(source)))
        .ok();
}
```

- [ ] **Step 2: Run full test suite**

Run: `cargo test --workspace`
Expected: all tests PASS

- [ ] **Step 3: Run clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings

- [ ] **Step 4: Run fmt**

Run: `cargo fmt --all -- --check`
Expected: no formatting issues

- [ ] **Step 5: Test vexilc on a valid file**

Run: `cargo run -p vexilc -- corpus/valid/006_message.vexil`
Expected: no output, exit code 0

- [ ] **Step 6: Test vexilc on an invalid file**

Run: `cargo run -p vexilc -- corpus/invalid/001_missing_namespace.vexil`
Expected: colored error output with span underline, exit code 1

- [ ] **Step 7: Commit**

```bash
git add crates/vexilc/src/main.rs
git commit -m "feat: vexilc CLI with ariadne error rendering"
```

---

## Completion Criteria

All of the following must be true:

1. `cargo test --workspace` — all tests pass
2. `cargo clippy --workspace --all-targets -- -D warnings` — clean
3. `cargo fmt --all -- --check` — clean
4. All 18 valid corpus files parse without errors
5. All 56 invalid corpus files are rejected with the correct `ErrorClass`
6. `vexilc` renders errors with file/line/column via ariadne
