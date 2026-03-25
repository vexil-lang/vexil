# Vexil Reference Implementation — Design Spec

**Date:** 2026-03-25
**Status:** Approved
**Milestone:** B — Frontend only (Lexer + Parser + AST + corpus passing)

## Summary

Hand-written recursive descent parser for the Vexil schema language, packaged as
a Rust workspace with one library crate (`vexil-lang`) and one binary crate
(`vexilc`). The first milestone targets parsing correctness: all 74 corpus files
(18 valid, 56 invalid) must pass. IR, type checker, and codegen are deferred to
later milestones.

## Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Crate structure | Hybrid: `vexil-lang` lib + `vexilc` bin | Clean boundary, split into workspace crates later when complexity demands |
| First milestone | Frontend only (corpus passing) | Grammar correctness proven before building on top |
| Error reporting | Simple spans + ariadne (replaceable) | `Span { offset, len }` internally, ariadne for rendering. Hand-rollable later |
| Parser strategy | Hand-written recursive descent | 1:1 with PEG grammar, full control over error recovery, no external parser dep |

## Crate Layout

```
vexil-lang/
├── spec/                        # existing: spec + grammar
├── corpus/                      # existing: 18 valid, 56 invalid
├── crates/
│   ├── vexil-lang/              # library crate
│   │   └── src/
│   │       ├── lib.rs           # public API: parse(), Diagnostic
│   │       ├── span.rs          # Span, Spanned<T>
│   │       ├── diagnostic.rs    # Diagnostic, ErrorClass, Severity
│   │       ├── lexer/
│   │       │   ├── mod.rs       # Lexer struct, lex()
│   │       │   └── token.rs     # Token, TokenKind
│   │       ├── parser/
│   │       │   ├── mod.rs       # Parser struct, parse()
│   │       │   ├── decl.rs      # message, enum, flags, union, newtype, config
│   │       │   ├── expr.rs      # type expressions, annotation args
│   │       │   └── import.rs    # import forms
│   │       └── ast/
│   │           ├── mod.rs       # AST node types
│   │           └── visit.rs     # Visitor trait
│   └── vexilc/                  # binary crate
│       └── src/
│           └── main.rs          # CLI: parse file, report diagnostics
├── docs/
└── Cargo.toml                   # workspace root
```

## Lexer

Single-pass, zero-copy scanner over `&str`. Produces `Vec<Token>` eagerly.

**Token:**
```rust
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}
```

**TokenKind** — flat enum:
- **Punctuation:** `LBrace`, `RBrace`, `LParen`, `RParen`, `LAngle`, `RAngle`,
  `Colon`, `Comma`, `Dot`, `Eq`, `Hash`
- **Literals:** `Ident(SmolStr)`, `UpperIdent(SmolStr)`, `StringLit(String)`,
  `DecInt(u64)`, `HexInt(u64)`
- **Ordinal:** `Ordinal(u32)` — `@` followed by digits, lexed as single token
- **Annotation sigil:** `At` — bare `@` followed by letter
- **Keywords:** `KwNamespace`, `KwImport`, `KwFrom`, `KwAs`, `KwMessage`,
  `KwEnum`, `KwFlags`, `KwUnion`, `KwNewtype`, `KwConfig`, `KwOptional`,
  `KwArray`, `KwMap`, `KwResult`, `KwTrue`, `KwFalse`, `KwNone`, `KwVoid`
- **Special:** `Newline`, `Eof`, `Error`

**Key rules:**
- `@123` → `Ordinal(123)`. `@name` → `At` + `Ident("name")`. Resolved at lex time.
- Keywords recognized during lexing. Parser handles keyword-as-field-name by
  accepting `Kw*` tokens in field-name position.
- String literals unescaped during lexing. Invalid escapes emit diagnostic + `Error` token.
- Comments consumed and discarded. No comment tokens.
- `SmolStr` for identifiers (small string optimization, cheap clone).

## Parser

Hand-written recursive descent. Each PEG rule maps to a `Parser` method.

**Parser struct:**
```rust
pub struct Parser<'src> {
    tokens: Vec<Token>,
    pos: usize,
    source: &'src str,
    diagnostics: Vec<Diagnostic>,
}
```

**Core methods:**
- `peek()` — look at current token
- `advance()` — consume and return
- `expect(kind)` — consume or emit diagnostic
- `at(kind)` — check without consuming
- `checkpoint()` / `backtrack(cp)` — save/restore for ordered choice

**Grammar rule → method mapping:**

| PEG Rule | Method | Notes |
|---|---|---|
| `Schema` | `parse_schema()` | Entry point |
| `namespace-decl` | `parse_namespace()` | Expects `KwNamespace` |
| `import-decl` | `parse_import()` | 6 forms via ordered choice |
| `type-decl` | `parse_type_decl()` | Dispatches on declaration keyword |
| `message-body` | `parse_message_body()` | Loop: try tombstone, then field |
| `field` | `parse_field()` | Keyword-as-field-name here |
| `type-expr` | `parse_type_expr()` | Primitives, parameterized, user types |
| `annotation` | `parse_annotation()` | `At` + `Ident` + optional args |
| `tombstone` | `parse_tombstone()` | `@removed(...)` |

**Keyword-as-field-name:** `parse_field_name()` accepts `Ident` OR any `Kw*`
token, converting to string. Only place keywords are treated as identifiers.

**Error recovery:**
- Inside declaration body: skip to `}` or next declaration keyword, continue.
- At top level: skip to next recognizable keyword.
- Parser never panics. Every error path emits diagnostic, returns partial AST node.

**Import parsing:** Check for `{` (named) vs plain namespace, then look for
`as` / `@` suffixes. Named + aliased combination rejected during parsing.

## AST

Source-faithful. Every node carries a `Span`. No resolution or normalization.

**Key types:**

- `Schema` — annotations, namespace, imports, declarations
- `Decl` — enum: `Message`, `Enum`, `Flags`, `Union`, `Newtype`, `Config`
- `Field` — three annotation positions (pre-name, post-ordinal, post-type),
  optional ordinal (None for config), optional default value (config only)
- `TypeExpr` — enum: `Primitive`, `SubByte`, `Semantic`, `Named`,
  `Optional(Box)`, `Array(Box)`, `Map(Box, Box)`, `Result(Box, Box)`
- `Annotation` — name + optional args
- `Tombstone` — ordinal + named args (reason, since)
- `Spanned<T>` — generic wrapper: `{ node: T, span: Span }`

Three annotation positions on `Field` match spec §5.2. AST preserves which
position was used; IR won't care.

Union variants are mini-messages (name, ordinal, fields). Config fields have
`default_value` instead of `ordinal`.

## Diagnostic Model

```rust
pub struct Diagnostic {
    pub severity: Severity,
    pub span: Span,
    pub class: ErrorClass,
    pub message: String,
}

pub enum Severity { Error, Warning }
```

**ErrorClass** — one variant per distinct rejection condition in the spec.
Maps 1:1 to invalid corpus files. Stable enum for programmatic matching.

Categories: lexer errors (3), structure errors (4), namespace errors (3),
declaration errors (2), field errors (4), type errors (4), config errors (4),
enum/flags/union errors (9), annotation errors (14), generic (2).

**Separation:** `vexil-lang` returns `Vec<Diagnostic>`. `vexilc` converts to
ariadne `Report`s. Library never touches ariadne — keeps it replaceable.

## Testing Strategy

**Corpus-driven** — the 74 corpus files are the test suite.

```rust
// crates/vexil-lang/tests/corpus.rs
#[test_case("001_minimal.vexil"; "minimal")]
fn valid_corpus(file: &str) { /* parse, assert no errors */ }

#[test_case("001_missing_namespace.vexil", ErrorClass::MissingNamespace; "missing ns")]
fn invalid_corpus(file: &str, expected: ErrorClass) { /* parse, assert expected error class */ }
```

- Valid tests: assert zero `Error`-severity diagnostics
- Invalid tests: assert specific `ErrorClass` variant present (not just "some error")
- `test-case` crate for parameterized tests

**Additional targeted tests (not duplicating corpus):**
- Lexer unit tests: `@123` vs `@name`, keyword tokens, string escapes, hex literals
- Span correctness: specific spans point to right source ranges
- Round-trip: parse → pretty-print → parse → AST equality (added later)

## Dependencies

| Crate | Purpose |
|---|---|
| `smol_str` | Small string optimization for identifiers |
| `ariadne` | Error rendering (vexilc only, replaceable) |
| `thiserror` | Error type derives |
| `test-case` | Parameterized tests (dev-dependency) |

## Future Milestones (not in scope)

- **Milestone C:** Lowering → IR → Type Checker → Validated IR
- **Milestone D:** Rust codegen backend
- **Milestone E:** Canonical form + BLAKE3 schema hash
- **Milestone F:** Multi-file import resolution
