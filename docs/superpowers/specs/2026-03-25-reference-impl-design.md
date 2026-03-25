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
  `DecInt(u64)`, `HexInt(u64)`, `FloatLit(f64)`
- **Ordinal:** `Ordinal(u32)` — `@` followed by digits, lexed as single token
- **Annotation sigil:** `At` — bare `@` followed by letter
- **Keywords:** `KwNamespace`, `KwImport`, `KwFrom`, `KwAs`, `KwMessage`,
  `KwEnum`, `KwFlags`, `KwUnion`, `KwNewtype`, `KwConfig`, `KwOptional`,
  `KwArray`, `KwMap`, `KwResult`, `KwTrue`, `KwFalse`, `KwNone`, `KwVoid`
- **Special:** `Eof`, `Error`

**Key rules:**
- `@123` → `Ordinal(123)`. `@name` → `At` + `Ident("name")`. Resolved at lex time.
- Keywords recognized during lexing. Parser handles keyword-as-field-name by
  accepting `Kw*` tokens in field-name position.
- String literals unescaped during lexing. Invalid escapes emit diagnostic + `Error` token.
- Comments consumed and discarded. No comment tokens.
- Whitespace (spaces, tabs, newlines) consumed and discarded. No whitespace tokens.
- `SmolStr` for identifiers (small string optimization, cheap clone).

**Type name tokenization:** Primitive type names (`bool`, `u8`, `u16`, `u32`,
`u64`, `i8`, `i16`, `i32`, `i64`, `f32`, `f64`, `void`) and semantic type names
(`string`, `bytes`, `rgb`, `uuid`, `timestamp`, `hash`) are emitted as `Ident`
tokens, not keywords. The parser's `parse_type_expr()` matches on the string
value to distinguish them from user-defined type names.

Sub-byte types (`u3`, `i7`, etc.) are also emitted as `Ident` tokens. The parser
recognizes the `u`/`i` prefix + digit suffix pattern and validates the bit width
(u: 1–64 excluding 8/16/32/64; i: 2–64 excluding 8/16/32/64). This avoids
polluting the keyword list with all valid bit widths.

This means `string`, `hash`, `void`, etc. are `Ident` tokens everywhere. In
field-name position they're accepted as field names. In type-expression position
they're recognized as built-in types. The disambiguation is purely positional in
the parser, not lexer-level.

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
| `Schema` | `parse_schema()` | Collects `annotation*` first, then expects `namespace-decl`. Annotations after namespace are a parse error (`VersionAfterNamespace`). |
| `namespace-decl` | `parse_namespace()` | Expects `KwNamespace` |
| `import-decl` | `parse_import()` | Left-factored: parse namespace path + optional version, then peek for `KwAs` → aliased, else → wildcard. Named form branches on leading `{`. |
| `type-decl` | `parse_type_decl()` | Dispatches on declaration keyword |
| `message-body` | `parse_message_body()` | Loop: try tombstone, then field |
| `field` | `parse_field()` | Keyword-as-field-name here |
| `type-expr` | `parse_type_expr()` | Matches `Ident` value against known type names, sub-byte pattern, or treats as user-defined |
| `annotation` | `parse_annotation()` | `At` + `Ident` + optional args |
| `tombstone` | `parse_tombstone()` | See tombstone section below |

**Keyword-as-field-name:** `parse_field_name()` accepts `Ident` OR any `Kw*`
token, converting to string. Only place keywords are treated as identifiers.

**Error recovery:**
- Inside declaration body: skip to `}` or next declaration keyword, continue.
- At top level: skip to next recognizable keyword.
- Parser never panics. Every error path emits diagnostic, returns partial AST node.

**Import parsing — left-factored dispatch:**
1. If next token is `LBrace` → named import path (`{ A, B } from ns`).
2. Else parse `namespace-path`, then optional `@ version-constraint`.
3. After namespace + optional version: peek for `KwAs` → aliased form. Else → wildcard.
4. Named + aliased combination: if named path was taken AND `KwAs` follows, emit
   `ImportNamedAliasedCombined` error.

This avoids ordered-choice backtracking entirely. The parser commits after parsing
the namespace path and uses a single-token peek (`KwAs`) to choose the suffix.

**Tombstone parsing:**
`@removed` is lexed as `At` + `Ident("removed")` — no special keyword token.
The parser recognizes tombstones by checking: in a declaration body, if the current
token is `At` and the next token is `Ident("removed")`, enter `parse_tombstone()`.
Otherwise, `At` + other `Ident` is a field annotation, and the parser falls through
to `parse_field()`.

`parse_tombstone()` consumes: `At`, `Ident("removed")`, `LParen`, `DecInt` (the
ordinal as a non-negative integer), `Comma`, then one or more named `tombstone-arg`s
(key `:` value pairs), `RParen`. Arguments are mandatory — `reason` must be present
(semantic check emits `RemovedMissingReason` if absent). `since` is optional.

**Checkpoint/backtrack semantics:**
`checkpoint()` saves `(pos, diagnostics.len())`. `backtrack(cp)` restores both —
diagnostics emitted on a failed speculative branch are truncated. This prevents
false diagnostics from branches that don't match.

## AST

Source-faithful. Every node carries a `Span`. No resolution or normalization.

**Key types:**

- `Schema` — annotations, namespace, imports, declarations
- `Decl` — enum: `Message`, `Enum`, `Flags`, `Union`, `Newtype`, `Config`
- `MessageField` — three annotation positions (pre-name, post-ordinal, post-type),
  ordinal (required), type expression
- `ConfigField` — annotations, name, type expression, default value (required)
- `TypeExpr` — enum: `Primitive`, `SubByte`, `Semantic`, `Named`,
  `Optional(Box)`, `Array(Box)`, `Map(Box, Box)`, `Result(Box, Box)`
- `DefaultValue` — enum: `None`, `Bool(bool)`, `Int(i64)`, `UInt(u64)`,
  `Float(f64)`, `Str(String)`, `Ident(SmolStr)`, `Array(Vec<Spanned<DefaultValue>>)`
- `Annotation` — name + optional args
- `Tombstone` — ordinal (`u32`, non-negative) + named args (reason, since)
- `Spanned<T>` — generic wrapper: `{ node: T, span: Span }`

Three annotation positions on `MessageField` match spec §5.2. AST preserves
which position was used; IR won't care.

`MessageField` and `ConfigField` are separate types — not a unified `Field` with
optional ordinal/default. This makes invalid states unrepresentable: a message
field always has an ordinal, a config field always has a default.
`parse_message_body()` produces `MessageField`, `parse_config_body()` produces
`ConfigField`. Union variant fields reuse `MessageField`.

`DefaultValue` covers all literal forms from the grammar's `literal-value` rule:
`none` keyword, booleans, integers (signed via context), floats, strings,
identifiers (for enum variant references), and array literals (`[v1, v2]`).

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
Multiple corpus files may share an `ErrorClass` when they test the same rule with
different inputs (e.g., `029_map_invalid_key` and `030_map_void_key` both map to
`InvalidMapKey`). Stable enum for programmatic matching.

**Parser-detectable vs semantic-only errors (Milestone B boundary):**

The parser detects and emits errors that are inherent to syntax or structure:
`MissingNamespace`, `DuplicateNamespace`, `ImportAfterDecl`,
`ImportNamedAliasedCombined`, `NamespaceInvalidComponent`, `NamespaceEmpty`,
`DeclNameInvalid`, `FieldNameInvalid`, `ConfigMissingDefault`, `ConfigHasOrdinal`,
`InvalidEscape`, `UnterminatedString`, `InvalidCharacter`,
`EnumVariantNameInvalid`, `UnionVariantNameInvalid`, `VersionAfterNamespace`,
`VersionInvalidSemver`, `UnexpectedToken`, `UnexpectedEof`,
`RemovedMissingReason` (named arg check in parser).

The following require semantic analysis (a post-parse pass over the AST, still
within Milestone B scope since they're needed for corpus correctness):
`DeclNameDuplicate`, `OrdinalDuplicate`, `OrdinalTooLarge`,
`OrdinalReusedAfterRemoved`, `FieldNameDuplicate`, `EnumOrdinalDuplicate`,
`EnumOrdinalTooLarge`, `EnumBackingTooNarrow`, `EnumBackingInvalid`,
`FlagsBitTooHigh`, `UnionOrdinalDuplicate`, `UnionOrdinalTooLarge`,
`DuplicateAnnotation`, `VersionDuplicate`, `NamespaceReserved`,
`LimitZero`, `LimitExceedsGlobal`, `TypeValueOverflow`.

Type-level errors (`UnknownType`, `ConfigTypeAsField`, `NewtypeOverNewtype`,
`NewtypeOverConfig`, `InvalidMapKey`, `ConfigInvalidType`,
`NonExhaustiveInvalidTarget`, `LimitInvalidTarget`, `VarintInvalidTarget`,
`ZigzagInvalidTarget`, `VarintZigzagCombined`, `DeltaInvalidTarget`,
`DeprecatedMissingReason`, `ConfigEncodingAnnotation`) require type resolution.
These are also within Milestone B scope — implemented as a validation pass over
the parsed AST.

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
