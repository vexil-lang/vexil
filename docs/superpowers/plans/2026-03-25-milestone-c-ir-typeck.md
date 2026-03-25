# Milestone C — IR + Type Checker Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an IR layer and type checker to the Vexil compiler, producing a `CompiledSchema` with resolved types, computed encodings, and wire sizes from valid AST input.

**Architecture:** AST → `lower.rs` → IR (`CompiledSchema` with `TypeRegistry`) → `typeck.rs` → Validated IR with `WireSize`. Existing `parse()` + `validate.rs` unchanged. New `compile()` entry point drives the full pipeline.

**Tech Stack:** Rust (edition 2021), `smol_str` (already a dep), `std::collections::HashMap`

---

## File Structure

```
crates/vexil-lang/src/
  ir/
    mod.rs          — IR node types (CompiledSchema, TypeDef, all *Def structs)
    types.rs        — TypeId, TypeRegistry, ResolvedType, Encoding, WireSize, ResolvedAnnotations
  lower.rs          — AST → IR lowering pass
  typeck.rs         — Type checker: wire sizes, recursion detection, newtype chains
  lib.rs            — Add `compile()` public API, add `pub mod ir; pub mod lower; pub mod typeck;`
  diagnostic.rs     — Add 3 new ErrorClass variants
crates/vexil-lang/tests/
  compile.rs        — All new IR/compile tests
```

**Dependency order:** `ir/types.rs` → `ir/mod.rs` → `lower.rs` → `typeck.rs` → `lib.rs` (compile API) → `compile.rs` (tests)

---

## Reference: Key Existing Types

These are in `crates/vexil-lang/src/` — you'll reference them extensively:

- **`ast/mod.rs`** — `Schema`, `Decl` (6 variants), `MessageDecl`, `MessageBodyItem`, `MessageField` (3 annotation positions: `pre_annotations`, `post_ordinal_annotations`, `post_type_annotations`), `EnumDecl`/`EnumVariant`/`EnumBacking`, `FlagsDecl`/`FlagsBit`, `UnionDecl`/`UnionVariant` (fields are `Vec<MessageBodyItem>`), `NewtypeDecl`, `ConfigDecl`/`ConfigField`, `TypeExpr` (9 variants), `PrimitiveType`, `SubByteType`, `SemanticType`, `Annotation`/`AnnotationArg`/`AnnotationValue`, `DefaultValue`, `Tombstone`/`TombstoneArg`
- **`span.rs`** — `Span { offset: u32, len: u32 }`, `Spanned<T> { node: T, span: Span }`
- **`diagnostic.rs`** — `Diagnostic::error(span, class, message)`, `Severity::Error`, `ErrorClass` enum
- **`lib.rs`** — `parse(source) -> ParseResult { schema, diagnostics }`
- **`validate.rs`** — `validate(schema) -> Vec<Diagnostic>` — called by `parse()`, no changes needed

**AST quirks the lowering must handle:**
- `MessageField` has 3 annotation positions that must be merged
- `Tombstone.args` is `Vec<TombstoneArg>` — extract `reason` and `since` keys by name
- `EnumDecl.backing` is `Option<Spanned<EnumBacking>>` — default to `EnumBacking::U32`
- `ImportKind` has 3 variants: `Wildcard`, `Named { names }`, `Aliased { alias }`
- `UnionVariant.fields` is `Vec<MessageBodyItem>` (reuses message body structure)

---

### Task 1: ErrorClass Variants

**Files:**
- Modify: `crates/vexil-lang/src/diagnostic.rs`

- [ ] **Step 1: Add 3 new ErrorClass variants**

In `crates/vexil-lang/src/diagnostic.rs`, add these variants to the `ErrorClass` enum, in a new `// IR / Type checker` section before the `// Generic` section:

```rust
    // IR / Type checker
    RecursiveTypeInfinite,
    EncodingTypeMismatch,
    UnresolvedType,
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p vexil-lang`
Expected: compiles clean (variants are unused for now, that's fine)

- [ ] **Step 3: Commit**

```bash
git add crates/vexil-lang/src/diagnostic.rs
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-lang): add IR ErrorClass variants"
```

---

### Task 2: IR Foundation Types (`ir/types.rs`)

**Files:**
- Create: `crates/vexil-lang/src/ir/types.rs`

- [ ] **Step 1: Create `ir/types.rs` with TypeId, TypeRegistry, ResolvedType, Encoding, WireSize, ResolvedAnnotations, TombstoneDef**

```rust
use std::collections::HashMap;
use smol_str::SmolStr;
use crate::ast::{PrimitiveType, SubByteType, SemanticType, EnumBacking, DefaultValue};
use crate::span::Span;

// ---------------------------------------------------------------------------
// TypeId + TypeRegistry
// ---------------------------------------------------------------------------

/// Opaque handle to a type definition in the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(pub(crate) u32);

/// Sentinel for unresolvable types (poison value).
pub(crate) const POISON_TYPE_ID: TypeId = TypeId(u32::MAX);

/// Central type store. All cross-references use TypeId.
#[derive(Debug, Clone)]
pub struct TypeRegistry {
    types: Vec<Option<TypeDef>>,        // None = stub (imported type)
    by_name: HashMap<SmolStr, TypeId>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self {
            types: Vec::new(),
            by_name: HashMap::new(),
        }
    }

    /// Register a fully-defined type. Returns its TypeId.
    pub fn register(&mut self, name: SmolStr, def: TypeDef) -> TypeId {
        let id = TypeId(self.types.len() as u32);
        self.types.push(Some(def));
        self.by_name.insert(name, id);
        id
    }

    /// Register a stub (imported type with no definition yet).
    pub fn register_stub(&mut self, name: SmolStr) -> TypeId {
        let id = TypeId(self.types.len() as u32);
        self.types.push(None);
        self.by_name.insert(name, id);
        id
    }

    /// Look up a type by name.
    pub fn lookup(&self, name: &str) -> Option<TypeId> {
        self.by_name.get(name).copied()
    }

    /// Get the definition for a TypeId. Returns None for stubs.
    pub fn get(&self, id: TypeId) -> Option<&TypeDef> {
        self.types.get(id.0 as usize).and_then(|opt| opt.as_ref())
    }

    /// Get a mutable reference to the definition.
    pub fn get_mut(&mut self, id: TypeId) -> Option<&mut TypeDef> {
        self.types.get_mut(id.0 as usize).and_then(|opt| opt.as_mut())
    }

    /// Returns true if this TypeId is a stub (imported, no definition).
    pub fn is_stub(&self, id: TypeId) -> bool {
        self.types.get(id.0 as usize).is_some_and(|opt| opt.is_none())
    }

    /// Number of registered types (including stubs).
    pub fn len(&self) -> usize {
        self.types.len()
    }

    /// Iterate all non-stub type definitions with their ids.
    pub fn iter(&self) -> impl Iterator<Item = (TypeId, &TypeDef)> {
        self.types.iter().enumerate().filter_map(|(i, opt)| {
            opt.as_ref().map(|def| (TypeId(i as u32), def))
        })
    }
}

// Forward-declare TypeDef here so TypeRegistry can reference it.
// The full enum is in ir/mod.rs — we re-export it.
use super::TypeDef;

// ---------------------------------------------------------------------------
// ResolvedType
// ---------------------------------------------------------------------------

/// Fully resolved type — no string references, only TypeIds.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedType {
    Primitive(PrimitiveType),
    SubByte(SubByteType),
    Semantic(SemanticType),
    Named(TypeId),
    Optional(Box<ResolvedType>),
    Array(Box<ResolvedType>),
    Map(Box<ResolvedType>, Box<ResolvedType>),
    Result(Box<ResolvedType>, Box<ResolvedType>),
}

// ---------------------------------------------------------------------------
// Encoding
// ---------------------------------------------------------------------------

/// Wire encoding strategy for a field.
#[derive(Debug, Clone, PartialEq)]
pub enum Encoding {
    /// Default for the logical type.
    Default,
    /// @varint — LEB128 variable-length encoding for u16/u32/u64.
    Varint,
    /// @zigzag — ZigZag + LEB128 for i16/i32/i64.
    ZigZag,
    /// @delta — delta encoding wrapping a base encoding.
    Delta(Box<Encoding>),
}

/// Full encoding metadata for a field.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldEncoding {
    pub encoding: Encoding,
    pub limit: Option<u64>,
}

impl FieldEncoding {
    pub fn default_encoding() -> Self {
        Self {
            encoding: Encoding::Default,
            limit: None,
        }
    }
}

// ---------------------------------------------------------------------------
// WireSize
// ---------------------------------------------------------------------------

/// Computed wire size for a type.
#[derive(Debug, Clone, PartialEq)]
pub enum WireSize {
    /// Exact known size in bits.
    Fixed(u64),
    /// Variable size with optional bounds.
    Variable { min_bits: u64, max_bits: Option<u64> },
}

// ---------------------------------------------------------------------------
// ResolvedAnnotations
// ---------------------------------------------------------------------------

/// Structured annotations — parsed from raw annotation bags during lowering.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResolvedAnnotations {
    pub deprecated: Option<SmolStr>,
    pub since: Option<SmolStr>,
    pub doc: Vec<SmolStr>,
    pub revision: Option<u64>,
    pub non_exhaustive: bool,
    pub version: Option<SmolStr>,
}

// ---------------------------------------------------------------------------
// TombstoneDef
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct TombstoneDef {
    pub span: Span,
    pub ordinal: u32,
    pub reason: SmolStr,
    pub since: Option<SmolStr>,
}
```

**WAIT:** This file references `super::TypeDef` from `ir/mod.rs`, which doesn't exist yet. That's OK — we'll create both files before compiling. Proceed to Task 3.

---

### Task 3: IR Node Types (`ir/mod.rs`)

**Files:**
- Create: `crates/vexil-lang/src/ir/mod.rs`

- [ ] **Step 1: Create `ir/mod.rs` with CompiledSchema, TypeDef, and all *Def structs**

```rust
pub mod types;

pub use types::{
    Encoding, FieldEncoding, ResolvedAnnotations, ResolvedType, TombstoneDef, TypeId,
    TypeRegistry, WireSize,
};

use crate::ast::{DefaultValue, EnumBacking};
use crate::span::Span;
use smol_str::SmolStr;

// ---------------------------------------------------------------------------
// CompiledSchema (IR root)
// ---------------------------------------------------------------------------

/// The root of the compiled IR — produced by `compile()`.
#[derive(Debug, Clone)]
pub struct CompiledSchema {
    pub namespace: Vec<SmolStr>,
    pub annotations: ResolvedAnnotations,
    pub registry: TypeRegistry,
    pub declarations: Vec<TypeId>,
}

// ---------------------------------------------------------------------------
// TypeDef
// ---------------------------------------------------------------------------

/// A resolved type definition in the IR.
#[derive(Debug, Clone)]
pub enum TypeDef {
    Message(MessageDef),
    Enum(EnumDef),
    Flags(FlagsDef),
    Union(UnionDef),
    Newtype(NewtypeDef),
    Config(ConfigDef),
}

// ---------------------------------------------------------------------------
// MessageDef
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MessageDef {
    pub name: SmolStr,
    pub span: Span,
    pub fields: Vec<FieldDef>,
    pub tombstones: Vec<TombstoneDef>,
    pub annotations: ResolvedAnnotations,
    pub wire_size: Option<WireSize>,
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: SmolStr,
    pub span: Span,
    pub ordinal: u32,
    pub resolved_type: ResolvedType,
    pub encoding: FieldEncoding,
    pub annotations: ResolvedAnnotations,
}

// ---------------------------------------------------------------------------
// EnumDef
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: SmolStr,
    pub span: Span,
    pub backing: EnumBacking,
    pub variants: Vec<EnumVariantDef>,
    pub tombstones: Vec<TombstoneDef>,
    pub annotations: ResolvedAnnotations,
}

#[derive(Debug, Clone)]
pub struct EnumVariantDef {
    pub name: SmolStr,
    pub span: Span,
    pub ordinal: u32,
    pub annotations: ResolvedAnnotations,
}

// ---------------------------------------------------------------------------
// FlagsDef
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FlagsDef {
    pub name: SmolStr,
    pub span: Span,
    pub bits: Vec<FlagsBitDef>,
    pub tombstones: Vec<TombstoneDef>,
    pub annotations: ResolvedAnnotations,
}

#[derive(Debug, Clone)]
pub struct FlagsBitDef {
    pub name: SmolStr,
    pub span: Span,
    pub bit: u32,
    pub annotations: ResolvedAnnotations,
}

// ---------------------------------------------------------------------------
// UnionDef
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct UnionDef {
    pub name: SmolStr,
    pub span: Span,
    pub variants: Vec<UnionVariantDef>,
    pub tombstones: Vec<TombstoneDef>,
    pub annotations: ResolvedAnnotations,
    pub wire_size: Option<WireSize>,
}

#[derive(Debug, Clone)]
pub struct UnionVariantDef {
    pub name: SmolStr,
    pub span: Span,
    pub ordinal: u32,
    pub fields: Vec<FieldDef>,
    pub tombstones: Vec<TombstoneDef>,
    pub annotations: ResolvedAnnotations,
}

// ---------------------------------------------------------------------------
// NewtypeDef
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct NewtypeDef {
    pub name: SmolStr,
    pub span: Span,
    pub inner_type: ResolvedType,
    pub terminal_type: ResolvedType,
    pub annotations: ResolvedAnnotations,
}

// ---------------------------------------------------------------------------
// ConfigDef
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ConfigDef {
    pub name: SmolStr,
    pub span: Span,
    pub fields: Vec<ConfigFieldDef>,
    pub annotations: ResolvedAnnotations,
}

#[derive(Debug, Clone)]
pub struct ConfigFieldDef {
    pub name: SmolStr,
    pub span: Span,
    pub resolved_type: ResolvedType,
    pub default_value: DefaultValue,
    pub annotations: ResolvedAnnotations,
}
```

- [ ] **Step 2: Wire up modules in `lib.rs`**

In `crates/vexil-lang/src/lib.rs`, add after the existing module declarations:

```rust
pub mod ir;
pub mod lower;
pub mod typeck;
```

Also create placeholder files so it compiles:

`crates/vexil-lang/src/lower.rs`:
```rust
use crate::ast::Schema;
use crate::diagnostic::Diagnostic;
use crate::ir::CompiledSchema;

/// Lower an AST Schema to the compiled IR.
pub fn lower(_schema: &Schema) -> (Option<CompiledSchema>, Vec<Diagnostic>) {
    // TODO: implement in Task 4
    (None, Vec::new())
}
```

`crates/vexil-lang/src/typeck.rs`:
```rust
use crate::diagnostic::Diagnostic;
use crate::ir::CompiledSchema;

/// Type-check and compute wire sizes.
pub fn check(_compiled: &mut CompiledSchema) -> Vec<Diagnostic> {
    // TODO: implement in Task 7
    Vec::new()
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p vexil-lang`
Expected: compiles clean. The `ir/types.rs` uses `super::TypeDef` which resolves to `ir::TypeDef` from `ir/mod.rs`.

- [ ] **Step 4: Commit**

```bash
git add crates/vexil-lang/src/ir/ crates/vexil-lang/src/lower.rs crates/vexil-lang/src/typeck.rs crates/vexil-lang/src/lib.rs
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-lang): IR type definitions and module skeleton"
```

---

### Task 4: Lowering Pass — Core (`lower.rs`)

**Files:**
- Modify: `crates/vexil-lang/src/lower.rs`

This is the largest task. The lowering pass converts AST → IR in 5 steps.

- [ ] **Step 1: Write the failing test**

Create `crates/vexil-lang/tests/compile.rs`:

```rust
use vexil_lang::diagnostic::Severity;

fn read_corpus(dir: &str, file: &str) -> String {
    let path = format!("{}/../../corpus/{dir}/{file}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

/// All valid corpus files must produce a CompiledSchema with zero errors.
#[test]
fn valid_corpus_compiles() {
    let valid_files = [
        "001_minimal.vexil",
        "002_primitives.vexil",
        "003_sub_byte.vexil",
        "004_semantic_types.vexil",
        "005_parameterized.vexil",
        "006_message.vexil",
        "007_enum.vexil",
        "008_flags.vexil",
        "009_union.vexil",
        "010_newtype.vexil",
        "011_config.vexil",
        "012_imports.vexil",
        "013_annotations.vexil",
        "014_keywords_as_fields.vexil",
        "015_forward_refs.vexil",
        "016_recursive.vexil",
        "017_escapes.vexil",
        "018_comments.vexil",
    ];
    for file in &valid_files {
        let source = read_corpus("valid", file);
        let result = vexil_lang::compile(&source);
        let errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "expected no errors in {file}, got: {errors:#?}"
        );
        assert!(
            result.compiled.is_some(),
            "expected CompiledSchema for valid {file}"
        );
    }
}

/// A simple message with two fields produces a CompiledSchema with 1 type.
#[test]
fn compile_simple_message() {
    let source = "namespace test.simple\nmessage Foo { a @0 : u32  b @1 : bool }";
    let result = vexil_lang::compile(source);
    let errors: Vec<_> = result.diagnostics.iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "errors: {errors:#?}");
    let compiled = result.compiled.as_ref().unwrap();
    assert_eq!(compiled.declarations.len(), 1);
    assert_eq!(compiled.registry.len(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vexil-lang --test compile compile_simple_message -- --nocapture`
Expected: FAIL — `compile` function doesn't exist yet, or lowering returns `None`

- [ ] **Step 3: Add `compile()` to `lib.rs`**

Replace the full contents of `crates/vexil-lang/src/lib.rs`:

```rust
pub mod ast;
pub mod diagnostic;
pub mod ir;
pub mod lexer;
pub mod lower;
pub mod parser;
pub mod span;
pub mod typeck;
pub mod validate;

use ast::Schema;
use diagnostic::{Diagnostic, Severity};
use ir::CompiledSchema;

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
    ParseResult {
        schema,
        diagnostics,
    }
}

pub struct CompileResult {
    pub schema: Option<Schema>,
    pub compiled: Option<CompiledSchema>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Full pipeline: parse -> validate -> lower -> type-check.
pub fn compile(source: &str) -> CompileResult {
    let parse_result = parse(source);
    if parse_result.diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return CompileResult {
            schema: parse_result.schema,
            compiled: None,
            diagnostics: parse_result.diagnostics,
        };
    }
    let schema = parse_result.schema.unwrap(); // safe: no errors means Some
    let (compiled, lower_diags) = lower::lower(&schema);
    let mut diagnostics = parse_result.diagnostics;
    diagnostics.extend(lower_diags);
    if let Some(ref mut compiled) = compiled {
        let check_diags = typeck::check(compiled);
        diagnostics.extend(check_diags);
    }
    CompileResult {
        schema: Some(schema),
        compiled,
        diagnostics,
    }
}
```

- [ ] **Step 4: Implement `lower.rs`**

Replace the full contents of `crates/vexil-lang/src/lower.rs`. This is the core lowering pass.

**Structure:** A `LowerCtx` struct holds the `TypeRegistry`, accumulated diagnostics, and import context. Five methods implement the lowering steps from the spec.

```rust
use std::collections::HashSet;

use smol_str::SmolStr;

use crate::ast::{
    self, Annotation, AnnotationValue, ConfigDecl, ConfigField, Decl, EnumBacking, EnumBodyItem,
    EnumDecl, FlagsBodyItem, FlagsDecl, ImportKind, MessageBodyItem, MessageDecl, MessageField,
    NewtypeDecl, PrimitiveType, Schema, SemanticType, TypeExpr, UnionBodyItem, UnionDecl,
};
use crate::diagnostic::{Diagnostic, ErrorClass};
use crate::ir::{
    self, CompiledSchema, ConfigDef, ConfigFieldDef, EnumDef, EnumVariantDef, Encoding, FieldDef,
    FieldEncoding, FlagsBitDef, FlagsDef, MessageDef, NewtypeDef, ResolvedAnnotations,
    ResolvedType, TombstoneDef, TypeDef, TypeId, TypeRegistry, UnionDef, UnionVariantDef,
};
use crate::span::Span;

// ---------------------------------------------------------------------------
// Lowering context
// ---------------------------------------------------------------------------

struct LowerCtx {
    registry: TypeRegistry,
    diagnostics: Vec<Diagnostic>,
    wildcard_imports: HashSet<SmolStr>,
}

impl LowerCtx {
    fn new() -> Self {
        Self {
            registry: TypeRegistry::new(),
            diagnostics: Vec::new(),
            wildcard_imports: HashSet::new(),
        }
    }

    fn emit(&mut self, span: Span, class: ErrorClass, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error(span, class, message));
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Lower an AST Schema to the compiled IR.
pub fn lower(schema: &Schema) -> (Option<CompiledSchema>, Vec<Diagnostic>) {
    let mut ctx = LowerCtx::new();

    // Step 0: Register import stubs.
    register_import_stubs(schema, &mut ctx);

    // Step 1: Register all declarations (forward pass).
    let decl_ids = register_declarations(schema, &mut ctx);

    // Step 2-5: Lower each declaration.
    for (decl_spanned, &id) in schema.declarations.iter().zip(decl_ids.iter()) {
        let def = lower_decl(&decl_spanned.node, decl_spanned.span, &mut ctx);
        // Replace the placeholder in the registry.
        if let Some(slot) = ctx.registry.get_mut(id) {
            *slot = def;
        }
    }

    // Build namespace.
    let namespace: Vec<SmolStr> = schema
        .namespace
        .as_ref()
        .map(|ns| ns.node.path.iter().map(|s| s.node.clone()).collect())
        .unwrap_or_default();

    // Resolve schema-level annotations.
    let annotations = resolve_annotations(&schema.annotations);

    let compiled = CompiledSchema {
        namespace,
        annotations,
        registry: ctx.registry,
        declarations: decl_ids,
    };

    (Some(compiled), ctx.diagnostics)
}

// ---------------------------------------------------------------------------
// Step 0: Import stubs
// ---------------------------------------------------------------------------

fn register_import_stubs(schema: &Schema, ctx: &mut LowerCtx) {
    for imp in &schema.imports {
        match &imp.node.kind {
            ImportKind::Wildcard => {
                // Wildcard import: mark the namespace as imported.
                let ns_path: SmolStr = imp
                    .node
                    .path
                    .iter()
                    .map(|s| s.node.as_str())
                    .collect::<Vec<_>>()
                    .join(".")
                    .into();
                ctx.wildcard_imports.insert(ns_path);
            }
            ImportKind::Named { names } => {
                for name in names {
                    ctx.registry.register_stub(name.node.clone());
                }
            }
            ImportKind::Aliased { alias } => {
                // Register the alias as a stub namespace marker.
                ctx.registry.register_stub(alias.node.clone());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Step 1: Register all declarations
// ---------------------------------------------------------------------------

fn register_declarations(schema: &Schema, ctx: &mut LowerCtx) -> Vec<TypeId> {
    let mut ids = Vec::new();

    for decl_spanned in &schema.declarations {
        let name = match &decl_spanned.node {
            Decl::Message(d) => d.name.node.clone(),
            Decl::Enum(d) => d.name.node.clone(),
            Decl::Flags(d) => d.name.node.clone(),
            Decl::Union(d) => d.name.node.clone(),
            Decl::Newtype(d) => d.name.node.clone(),
            Decl::Config(d) => d.name.node.clone(),
        };

        // Register with a placeholder TypeDef. We'll fill it in during lowering.
        // Use Message as a dummy — it will be overwritten.
        let placeholder = TypeDef::Message(MessageDef {
            name: name.clone(),
            span: decl_spanned.span,
            fields: Vec::new(),
            tombstones: Vec::new(),
            annotations: ResolvedAnnotations::default(),
            wire_size: None,
        });
        let id = ctx.registry.register(name, placeholder);
        ids.push(id);
    }

    ids
}

// ---------------------------------------------------------------------------
// Step 2-5: Lower individual declarations
// ---------------------------------------------------------------------------

fn lower_decl(decl: &Decl, span: Span, ctx: &mut LowerCtx) -> TypeDef {
    match decl {
        Decl::Message(d) => TypeDef::Message(lower_message(d, span, ctx)),
        Decl::Enum(d) => TypeDef::Enum(lower_enum(d, span, ctx)),
        Decl::Flags(d) => TypeDef::Flags(lower_flags(d, span, ctx)),
        Decl::Union(d) => TypeDef::Union(lower_union(d, span, ctx)),
        Decl::Newtype(d) => TypeDef::Newtype(lower_newtype(d, span, ctx)),
        Decl::Config(d) => TypeDef::Config(lower_config(d, span, ctx)),
    }
}

fn lower_message(msg: &MessageDecl, span: Span, ctx: &mut LowerCtx) -> MessageDef {
    let mut fields = Vec::new();
    let mut tombstones = Vec::new();

    for item in &msg.body {
        match item {
            MessageBodyItem::Field(f) => {
                fields.push(lower_field(&f.node, f.span, ctx));
            }
            MessageBodyItem::Tombstone(t) => {
                tombstones.push(lower_tombstone(&t.node, t.span));
            }
        }
    }

    let annotations = resolve_annotations(&msg.annotations);

    MessageDef {
        name: msg.name.node.clone(),
        span,
        fields,
        tombstones,
        annotations,
        wire_size: None,
    }
}

fn lower_field(field: &MessageField, span: Span, ctx: &mut LowerCtx) -> FieldDef {
    let resolved_type = resolve_type_expr(&field.ty.node, field.ty.span, ctx);

    // Merge all annotation positions.
    let all_annotations: Vec<&Annotation> = field
        .pre_annotations
        .iter()
        .chain(field.post_ordinal_annotations.iter())
        .chain(field.post_type_annotations.iter())
        .collect();

    let encoding = compute_field_encoding(&all_annotations);
    let annotations = resolve_annotations_refs(&all_annotations);

    FieldDef {
        name: field.name.node.clone(),
        span,
        ordinal: field.ordinal.node,
        resolved_type,
        encoding,
        annotations,
    }
}

fn lower_enum(en: &EnumDecl, span: Span, _ctx: &mut LowerCtx) -> EnumDef {
    let backing = en
        .backing
        .as_ref()
        .map(|b| b.node.clone())
        .unwrap_or(EnumBacking::U32);

    let mut variants = Vec::new();
    let mut tombstones = Vec::new();

    for item in &en.body {
        match item {
            EnumBodyItem::Variant(v) => {
                variants.push(EnumVariantDef {
                    name: v.node.name.node.clone(),
                    span: v.span,
                    ordinal: v.node.ordinal.node,
                    annotations: resolve_annotations(&v.node.annotations),
                });
            }
            EnumBodyItem::Tombstone(t) => {
                tombstones.push(lower_tombstone(&t.node, t.span));
            }
        }
    }

    EnumDef {
        name: en.name.node.clone(),
        span,
        backing,
        variants,
        tombstones,
        annotations: resolve_annotations(&en.annotations),
    }
}

fn lower_flags(flags: &FlagsDecl, span: Span, _ctx: &mut LowerCtx) -> FlagsDef {
    let mut bits = Vec::new();
    let mut tombstones = Vec::new();

    for item in &flags.body {
        match item {
            FlagsBodyItem::Bit(b) => {
                bits.push(FlagsBitDef {
                    name: b.node.name.node.clone(),
                    span: b.span,
                    bit: b.node.ordinal.node,
                    annotations: resolve_annotations(&b.node.annotations),
                });
            }
            FlagsBodyItem::Tombstone(t) => {
                tombstones.push(lower_tombstone(&t.node, t.span));
            }
        }
    }

    FlagsDef {
        name: flags.name.node.clone(),
        span,
        bits,
        tombstones,
        annotations: resolve_annotations(&flags.annotations),
    }
}

fn lower_union(un: &UnionDecl, span: Span, ctx: &mut LowerCtx) -> UnionDef {
    let mut variants = Vec::new();
    let mut top_tombstones = Vec::new();

    for item in &un.body {
        match item {
            UnionBodyItem::Variant(v) => {
                let mut fields = Vec::new();
                let mut tombstones = Vec::new();

                for body_item in &v.node.fields {
                    match body_item {
                        MessageBodyItem::Field(f) => {
                            fields.push(lower_field(&f.node, f.span, ctx));
                        }
                        MessageBodyItem::Tombstone(t) => {
                            tombstones.push(lower_tombstone(&t.node, t.span));
                        }
                    }
                }

                variants.push(UnionVariantDef {
                    name: v.node.name.node.clone(),
                    span: v.span,
                    ordinal: v.node.ordinal.node,
                    fields,
                    tombstones,
                    annotations: resolve_annotations(&v.node.annotations),
                });
            }
            UnionBodyItem::Tombstone(t) => {
                top_tombstones.push(lower_tombstone(&t.node, t.span));
            }
        }
    }

    UnionDef {
        name: un.name.node.clone(),
        span,
        variants,
        tombstones: top_tombstones,
        annotations: resolve_annotations(&un.annotations),
        wire_size: None,
    }
}

fn lower_newtype(nt: &NewtypeDecl, span: Span, ctx: &mut LowerCtx) -> NewtypeDef {
    let inner_type = resolve_type_expr(&nt.inner_type.node, nt.inner_type.span, ctx);
    // terminal_type is the same as inner_type for now (chains are length 1).
    // typeck will resolve chains if the rule relaxes.
    let terminal_type = inner_type.clone();

    NewtypeDef {
        name: nt.name.node.clone(),
        span,
        inner_type,
        terminal_type,
        annotations: resolve_annotations(&nt.annotations),
    }
}

fn lower_config(cfg: &ConfigDecl, span: Span, ctx: &mut LowerCtx) -> ConfigDef {
    let fields = cfg
        .fields
        .iter()
        .map(|f| {
            let resolved_type = resolve_type_expr(&f.node.ty.node, f.node.ty.span, ctx);
            let annotations = resolve_annotations(&f.node.annotations);
            ConfigFieldDef {
                name: f.node.name.node.clone(),
                span: f.span,
                resolved_type,
                default_value: f.node.default_value.node.clone(),
                annotations,
            }
        })
        .collect();

    ConfigDef {
        name: cfg.name.node.clone(),
        span,
        fields,
        annotations: resolve_annotations(&cfg.annotations),
    }
}

// ---------------------------------------------------------------------------
// Type resolution (Step 2)
// ---------------------------------------------------------------------------

fn resolve_type_expr(expr: &TypeExpr, span: Span, ctx: &mut LowerCtx) -> ResolvedType {
    match expr {
        TypeExpr::Primitive(p) => ResolvedType::Primitive(*p),
        TypeExpr::SubByte(s) => ResolvedType::SubByte(*s),
        TypeExpr::Semantic(s) => ResolvedType::Semantic(*s),
        TypeExpr::Named(name) => {
            if let Some(id) = ctx.registry.lookup(name.as_str()) {
                ResolvedType::Named(id)
            } else if !ctx.wildcard_imports.is_empty() {
                // Wildcard import — assume it's valid (Milestone F resolves).
                let id = ctx.registry.register_stub(name.clone());
                ResolvedType::Named(id)
            } else {
                ctx.emit(span, ErrorClass::UnresolvedType, format!("unresolved type `{name}`"));
                // Poison: use a sentinel.
                ResolvedType::Named(ir::types::POISON_TYPE_ID)
            }
        }
        TypeExpr::Qualified(ns, name) => {
            // Check if the namespace/alias is known.
            let qualified_name: SmolStr = format!("{ns}.{name}").into();
            if let Some(id) = ctx.registry.lookup(qualified_name.as_str()) {
                ResolvedType::Named(id)
            } else if ctx.registry.lookup(ns.as_str()).is_some() {
                // The alias/namespace is registered — register a stub for the qualified name.
                let id = ctx.registry.register_stub(qualified_name);
                ResolvedType::Named(id)
            } else {
                ctx.emit(span, ErrorClass::UnresolvedType, format!("unresolved qualified type `{ns}.{name}`"));
                ResolvedType::Named(ir::types::POISON_TYPE_ID)
            }
        }
        TypeExpr::Optional(inner) => {
            let resolved = resolve_type_expr(&inner.node, inner.span, ctx);
            ResolvedType::Optional(Box::new(resolved))
        }
        TypeExpr::Array(inner) => {
            let resolved = resolve_type_expr(&inner.node, inner.span, ctx);
            ResolvedType::Array(Box::new(resolved))
        }
        TypeExpr::Map(key, value) => {
            let rk = resolve_type_expr(&key.node, key.span, ctx);
            let rv = resolve_type_expr(&value.node, value.span, ctx);
            ResolvedType::Map(Box::new(rk), Box::new(rv))
        }
        TypeExpr::Result(ok, err) => {
            let ro = resolve_type_expr(&ok.node, ok.span, ctx);
            let re = resolve_type_expr(&err.node, err.span, ctx);
            ResolvedType::Result(Box::new(ro), Box::new(re))
        }
    }
}

// ---------------------------------------------------------------------------
// Field encoding computation (Step 3)
// ---------------------------------------------------------------------------

fn compute_field_encoding(annotations: &[&Annotation]) -> FieldEncoding {
    let has_varint = annotations.iter().any(|a| a.name.node == "varint");
    let has_zigzag = annotations.iter().any(|a| a.name.node == "zigzag");
    let has_delta = annotations.iter().any(|a| a.name.node == "delta");

    // Resolve base encoding first.
    let base = if has_varint {
        Encoding::Varint
    } else if has_zigzag {
        Encoding::ZigZag
    } else {
        Encoding::Default
    };

    // Wrap with delta if present.
    let encoding = if has_delta {
        Encoding::Delta(Box::new(base))
    } else {
        base
    };

    // Extract @limit value.
    let limit = annotations.iter().find_map(|a| {
        if a.name.node == "limit" {
            a.args.as_ref().and_then(|args| {
                args.iter().find_map(|arg| {
                    if arg.key.is_none() {
                        match &arg.value.node {
                            AnnotationValue::Int(v) => Some(*v),
                            AnnotationValue::Hex(v) => Some(*v),
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
            })
        } else {
            None
        }
    });

    FieldEncoding { encoding, limit }
}

// ---------------------------------------------------------------------------
// Annotation resolution (Step 4)
// ---------------------------------------------------------------------------

fn resolve_annotations(annotations: &[Annotation]) -> ResolvedAnnotations {
    let refs: Vec<&Annotation> = annotations.iter().collect();
    resolve_annotations_refs(&refs)
}

fn resolve_annotations_refs(annotations: &[&Annotation]) -> ResolvedAnnotations {
    let mut result = ResolvedAnnotations::default();

    for ann in annotations {
        match ann.name.node.as_str() {
            "deprecated" => {
                result.deprecated = extract_string_arg(ann, "reason");
            }
            "since" => {
                result.since = extract_first_string_arg(ann);
            }
            "doc" => {
                if let Some(s) = extract_first_string_arg(ann) {
                    result.doc.push(s);
                }
            }
            "revision" => {
                result.revision = extract_first_int_arg(ann);
            }
            "non_exhaustive" => {
                result.non_exhaustive = true;
            }
            "version" => {
                result.version = extract_first_string_arg(ann);
            }
            _ => {
                // Unknown annotations are ignored at the IR level.
                // validate.rs already handles unknown annotation errors.
            }
        }
    }

    result
}

fn extract_string_arg(ann: &Annotation, key: &str) -> Option<SmolStr> {
    ann.args.as_ref().and_then(|args| {
        args.iter().find_map(|arg| {
            if arg.key.as_ref().is_some_and(|k| k.node == key) {
                match &arg.value.node {
                    AnnotationValue::Str(s) => Some(SmolStr::new(s)),
                    _ => None,
                }
            } else {
                None
            }
        })
    })
}

fn extract_first_string_arg(ann: &Annotation) -> Option<SmolStr> {
    ann.args.as_ref().and_then(|args| {
        args.first().and_then(|arg| match &arg.value.node {
            AnnotationValue::Str(s) => Some(SmolStr::new(s)),
            _ => None,
        })
    })
}

fn extract_first_int_arg(ann: &Annotation) -> Option<u64> {
    ann.args.as_ref().and_then(|args| {
        args.first().and_then(|arg| match &arg.value.node {
            AnnotationValue::Int(v) => Some(*v),
            AnnotationValue::Hex(v) => Some(*v),
            _ => None,
        })
    })
}

// ---------------------------------------------------------------------------
// Tombstone lowering (Step 5)
// ---------------------------------------------------------------------------

fn lower_tombstone(tombstone: &ast::Tombstone, span: Span) -> TombstoneDef {
    let reason = tombstone
        .args
        .iter()
        .find_map(|arg| {
            if arg.key.node == "reason" {
                match &arg.value.node {
                    AnnotationValue::Str(s) => Some(SmolStr::new(s)),
                    _ => None,
                }
            } else {
                None
            }
        })
        .unwrap_or_else(|| SmolStr::new("(no reason)"));

    let since = tombstone.args.iter().find_map(|arg| {
        if arg.key.node == "since" {
            match &arg.value.node {
                AnnotationValue::Str(s) => Some(SmolStr::new(s)),
                _ => None,
            }
        } else {
            None
        }
    });

    TombstoneDef {
        span,
        ordinal: tombstone.ordinal.node,
        reason,
        since,
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p vexil-lang --test compile -- --nocapture`
Expected: `compile_simple_message` PASSES, `valid_corpus_compiles` PASSES

- [ ] **Step 6: Run quality gates**

Run: `cargo fmt --all -- --check && cargo clippy -p vexil-lang --all-targets -- -D warnings`
Expected: clean

- [ ] **Step 7: Commit**

```bash
git add crates/vexil-lang/src/lower.rs crates/vexil-lang/src/lib.rs crates/vexil-lang/tests/compile.rs
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-lang): lowering pass — AST to IR"
```

---

### Task 5: Lowering Tests — Type Resolution + Encoding

**Files:**
- Modify: `crates/vexil-lang/tests/compile.rs`

- [ ] **Step 1: Add type resolution tests**

Append to `compile.rs`:

```rust
use vexil_lang::ir::{TypeDef, ResolvedType, Encoding};

/// Forward references resolve to correct TypeIds.
#[test]
fn type_resolution_forward_ref() {
    let source = r#"
namespace test.resolve
message Container { item @0 : Item }
message Item { value @0 : u32 }
"#;
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();

    // Container's field should reference Item's TypeId.
    let container_id = compiled.declarations[0];
    let item_id = compiled.declarations[1];

    let container = compiled.registry.get(container_id).unwrap();
    if let TypeDef::Message(msg) = container {
        assert_eq!(msg.fields.len(), 1);
        assert_eq!(msg.fields[0].resolved_type, ResolvedType::Named(item_id));
    } else {
        panic!("expected Message");
    }
}

/// Newtype inner type resolves to primitive.
#[test]
fn newtype_resolution() {
    let source = "namespace test.nt\nnewtype SessionId : u64";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let id = compiled.declarations[0];
    if let TypeDef::Newtype(nt) = compiled.registry.get(id).unwrap() {
        assert_eq!(
            nt.inner_type,
            ResolvedType::Primitive(vexil_lang::ast::PrimitiveType::U64)
        );
    } else {
        panic!("expected Newtype");
    }
}

/// Enum backing defaults to U32 when unspecified.
#[test]
fn enum_default_backing() {
    let source = "namespace test.en\nenum Dir { North @0  South @1 }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let id = compiled.declarations[0];
    if let TypeDef::Enum(en) = compiled.registry.get(id).unwrap() {
        assert_eq!(en.backing, vexil_lang::ast::EnumBacking::U32);
        assert_eq!(en.variants.len(), 2);
    } else {
        panic!("expected Enum");
    }
}

/// Encoding annotations are correctly computed.
#[test]
fn encoding_varint() {
    let source = "namespace test.enc\nmessage M { v @0 @varint : u32 }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let id = compiled.declarations[0];
    if let TypeDef::Message(msg) = compiled.registry.get(id).unwrap() {
        assert_eq!(msg.fields[0].encoding.encoding, Encoding::Varint);
        assert_eq!(msg.fields[0].encoding.limit, None);
    } else {
        panic!("expected Message");
    }
}

/// @delta wraps the base encoding.
#[test]
fn encoding_delta_varint() {
    let source = "namespace test.dv\nmessage M { v @0 @delta @varint : u32 }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let id = compiled.declarations[0];
    if let TypeDef::Message(msg) = compiled.registry.get(id).unwrap() {
        assert_eq!(
            msg.fields[0].encoding.encoding,
            Encoding::Delta(Box::new(Encoding::Varint))
        );
    } else {
        panic!("expected Message");
    }
}

/// @limit is extracted correctly.
#[test]
fn encoding_limit() {
    let source = "namespace test.lim\nmessage M { s @0 : string @limit(1024) }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let id = compiled.declarations[0];
    if let TypeDef::Message(msg) = compiled.registry.get(id).unwrap() {
        assert_eq!(msg.fields[0].encoding.encoding, Encoding::Default);
        assert_eq!(msg.fields[0].encoding.limit, Some(1024));
    } else {
        panic!("expected Message");
    }
}

/// Annotations are resolved into structured form.
#[test]
fn resolved_annotations() {
    let source = r#"
namespace test.ann
@doc("A test") @deprecated(since: "1.0", reason: "use B")
message A { v @0 : u32 }
"#;
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let id = compiled.declarations[0];
    if let TypeDef::Message(msg) = compiled.registry.get(id).unwrap() {
        assert_eq!(msg.annotations.doc, vec!["A test"]);
        assert_eq!(msg.annotations.deprecated.as_deref(), Some("use B"));
    } else {
        panic!("expected Message");
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p vexil-lang --test compile -- --nocapture`
Expected: all PASS

- [ ] **Step 3: Commit**

```bash
git add crates/vexil-lang/tests/compile.rs
VEXIL_COMMIT_TASK=1 git commit -m "test(vexil-lang): type resolution and encoding lowering tests"
```

---

### Task 6: Import Stub Tests

**Files:**
- Modify: `crates/vexil-lang/tests/compile.rs`

- [ ] **Step 1: Add import stub tests**

Append to `compile.rs`:

```rust
/// Import stubs are created in the registry.
#[test]
fn import_named_creates_stubs() {
    let source = r#"
namespace test.imp
import { Shape, Color } from test.unions
message M { s @0 : Shape }
"#;
    let result = vexil_lang::compile(source);
    let errors: Vec<_> = result.diagnostics.iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "errors: {errors:#?}");
    let compiled = result.compiled.as_ref().unwrap();
    // Shape and Color should be stubs + M is a real def.
    assert!(compiled.registry.lookup("Shape").is_some());
    assert!(compiled.registry.lookup("Color").is_some());
}

/// Qualified type reference through alias.
#[test]
fn import_aliased_qualified_ref() {
    let source = r#"
namespace test.alias
import test.enums as E
message M { kind @0 : E.ClientKind }
"#;
    let result = vexil_lang::compile(source);
    let errors: Vec<_> = result.diagnostics.iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "errors: {errors:#?}");
    assert!(result.compiled.is_some());
}

/// Wildcard imports suppress unknown type errors.
#[test]
fn import_wildcard_suppresses_unknown() {
    let source = r#"
namespace test.wild
import test.newtypes
message M { id @0 : SessionId }
"#;
    let result = vexil_lang::compile(source);
    let errors: Vec<_> = result.diagnostics.iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "errors: {errors:#?}");
    assert!(result.compiled.is_some());
}

/// Invalid corpus files produce error diagnostics (compile returns None or errors).
#[test]
fn invalid_corpus_produces_errors() {
    let invalid_dir = format!("{}/../../corpus/invalid", env!("CARGO_MANIFEST_DIR"));
    let entries = std::fs::read_dir(&invalid_dir).unwrap();
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("vexil") {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        let result = vexil_lang::compile(&source);
        let has_error = result.diagnostics.iter().any(|d| d.severity == Severity::Error);
        assert!(
            has_error,
            "expected errors for invalid file {}, got none",
            path.display()
        );
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p vexil-lang --test compile -- --nocapture`
Expected: all PASS

- [ ] **Step 3: Commit**

```bash
git add crates/vexil-lang/tests/compile.rs
VEXIL_COMMIT_TASK=1 git commit -m "test(vexil-lang): import stubs and invalid corpus through compile()"
```

---

### Task 7: Type Checker — Wire Size Computation (`typeck.rs`)

**Files:**
- Modify: `crates/vexil-lang/src/typeck.rs`

- [ ] **Step 1: Write the failing test**

Append to `crates/vexil-lang/tests/compile.rs`:

```rust
use vexil_lang::ir::WireSize;

/// Wire size for a message with u32 + bool = Fixed(33 bits).
#[test]
fn wire_size_fixed_message() {
    let source = "namespace test.ws\nmessage M { a @0 : u32  b @1 : bool }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let id = compiled.declarations[0];
    if let TypeDef::Message(msg) = compiled.registry.get(id).unwrap() {
        assert_eq!(msg.wire_size, Some(WireSize::Fixed(33)));
    } else {
        panic!("expected Message");
    }
}

/// Wire size for a message with string = Variable.
#[test]
fn wire_size_variable_string() {
    let source = "namespace test.vs\nmessage M { s @0 : string }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let id = compiled.declarations[0];
    if let TypeDef::Message(msg) = compiled.registry.get(id).unwrap() {
        assert!(matches!(msg.wire_size, Some(WireSize::Variable { .. })));
    } else {
        panic!("expected Message");
    }
}

/// Wire size for optional<u8> = Variable(min=1, max=9).
#[test]
fn wire_size_optional() {
    let source = "namespace test.opt\nmessage M { v @0 : optional<u8> }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let id = compiled.declarations[0];
    if let TypeDef::Message(msg) = compiled.registry.get(id).unwrap() {
        // The message wire size is Variable because the field is variable.
        assert!(matches!(msg.wire_size, Some(WireSize::Variable { min_bits: 1, max_bits: Some(9) })));
    } else {
        panic!("expected Message");
    }
}

/// @varint on u32 makes it Variable(min=8, max=40).
#[test]
fn wire_size_varint() {
    let source = "namespace test.vw\nmessage M { v @0 @varint : u32 }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let id = compiled.declarations[0];
    if let TypeDef::Message(msg) = compiled.registry.get(id).unwrap() {
        assert!(matches!(msg.wire_size, Some(WireSize::Variable { min_bits: 8, max_bits: Some(40) })));
    } else {
        panic!("expected Message");
    }
}

/// Enum wire size = Fixed(backing size).
#[test]
fn wire_size_enum() {
    let source = "namespace test.ew\nenum Dir { N @0  S @1 }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let id = compiled.declarations[0];
    if let TypeDef::Enum(en) = compiled.registry.get(id).unwrap() {
        // U32 backing = 32 bits. Enum wire size isn't stored on EnumDef directly,
        // but the typeck should compute it. We check via message embedding.
    } else {
        panic!("expected Enum");
    }
    // Test via a message that embeds the enum.
    let source2 = "namespace test.ew2\nenum Dir { N @0 }\nmessage M { d @0 : Dir }";
    let result2 = vexil_lang::compile(source2);
    let compiled2 = result2.compiled.as_ref().unwrap();
    let msg_id = compiled2.declarations[1];
    if let TypeDef::Message(msg) = compiled2.registry.get(msg_id).unwrap() {
        assert_eq!(msg.wire_size, Some(WireSize::Fixed(32)));
    } else {
        panic!("expected Message");
    }
}

/// Flags wire size = Fixed(64 bits).
#[test]
fn wire_size_flags() {
    let source = "namespace test.fw\nflags F { R @0 }\nmessage M { f @0 : F }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let msg_id = compiled.declarations[1];
    if let TypeDef::Message(msg) = compiled.registry.get(msg_id).unwrap() {
        assert_eq!(msg.wire_size, Some(WireSize::Fixed(64)));
    } else {
        panic!("expected Message");
    }
}

/// Newtype wire size = same as inner type.
#[test]
fn wire_size_newtype() {
    let source = "namespace test.nw\nnewtype Id : u64\nmessage M { id @0 : Id }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let msg_id = compiled.declarations[1];
    if let TypeDef::Message(msg) = compiled.registry.get(msg_id).unwrap() {
        assert_eq!(msg.wire_size, Some(WireSize::Fixed(64)));
    } else {
        panic!("expected Message");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p vexil-lang --test compile wire_size -- --nocapture`
Expected: FAIL — typeck is a no-op, wire_size is None

- [ ] **Step 3: Implement `typeck.rs`**

Replace the full contents of `crates/vexil-lang/src/typeck.rs`:

```rust
use std::collections::HashSet;

use crate::ast::{EnumBacking, PrimitiveType, SemanticType, SubByteType};
use crate::diagnostic::{Diagnostic, ErrorClass};
use crate::ir::{
    CompiledSchema, Encoding, FieldDef, FieldEncoding, ResolvedType, TypeDef, TypeId, WireSize,
};

/// Type-check and compute wire sizes. Mutates the schema to fill in wire_size fields.
pub fn check(compiled: &mut CompiledSchema) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    // Check recursive types.
    check_recursion(compiled, &mut diags);

    // Compute wire sizes for messages and unions.
    let decl_ids: Vec<TypeId> = compiled.declarations.clone();
    for &id in &decl_ids {
        if let Some(def) = compiled.registry.get(id) {
            match def {
                TypeDef::Message(_) => {
                    let ws = compute_message_wire_size(id, compiled);
                    if let Some(TypeDef::Message(msg)) = compiled.registry.get_mut(id) {
                        msg.wire_size = Some(ws);
                    }
                }
                TypeDef::Union(_) => {
                    let ws = compute_union_wire_size(id, compiled);
                    if let Some(TypeDef::Union(un)) = compiled.registry.get_mut(id) {
                        un.wire_size = Some(ws);
                    }
                }
                _ => {}
            }
        }
    }

    diags
}

// ---------------------------------------------------------------------------
// Wire size computation
// ---------------------------------------------------------------------------

fn compute_type_wire_size(ty: &ResolvedType, enc: &FieldEncoding, compiled: &CompiledSchema) -> WireSize {
    // If encoding overrides (varint/zigzag), the type becomes variable.
    let base_ws = compute_resolved_type_wire_size(ty, compiled);

    match &enc.encoding {
        Encoding::Varint => varint_wire_size(ty),
        Encoding::ZigZag => zigzag_wire_size(ty),
        Encoding::Delta(inner) => {
            // Delta wraps the base encoding.
            let inner_enc = FieldEncoding {
                encoding: *inner.clone(),
                limit: enc.limit,
            };
            compute_type_wire_size(ty, &inner_enc, compiled)
        }
        Encoding::Default => base_ws,
    }
}

fn compute_resolved_type_wire_size(ty: &ResolvedType, compiled: &CompiledSchema) -> WireSize {
    match ty {
        ResolvedType::Primitive(p) => primitive_wire_size(p),
        ResolvedType::SubByte(s) => WireSize::Fixed(s.bits as u64),
        ResolvedType::Semantic(s) => semantic_wire_size(s),
        ResolvedType::Named(id) => named_type_wire_size(*id, compiled),
        ResolvedType::Optional(inner) => {
            let inner_ws = compute_resolved_type_wire_size(inner, compiled);
            match inner_ws {
                WireSize::Fixed(bits) => WireSize::Variable {
                    min_bits: 1,
                    max_bits: Some(1 + bits),
                },
                WireSize::Variable { max_bits, .. } => WireSize::Variable {
                    min_bits: 1,
                    max_bits: max_bits.map(|m| 1 + m),
                },
            }
        }
        ResolvedType::Array(_) => WireSize::Variable {
            min_bits: 8, // LEB128 header minimum
            max_bits: None,
        },
        ResolvedType::Map(_, _) => WireSize::Variable {
            min_bits: 8,
            max_bits: None,
        },
        ResolvedType::Result(ok, err) => {
            let ok_ws = compute_resolved_type_wire_size(ok, compiled);
            let err_ws = compute_resolved_type_wire_size(err, compiled);
            let min_ok = wire_size_min_bits(&ok_ws);
            let min_err = wire_size_min_bits(&err_ws);
            let min = 1 + std::cmp::min(min_ok, min_err);
            let max = match (wire_size_max_bits(&ok_ws), wire_size_max_bits(&err_ws)) {
                (Some(a), Some(b)) => Some(1 + std::cmp::max(a, b)),
                _ => None,
            };
            WireSize::Variable {
                min_bits: min,
                max_bits: max,
            }
        }
    }
}

fn primitive_wire_size(p: &PrimitiveType) -> WireSize {
    let bits = match p {
        PrimitiveType::Bool => 1,
        PrimitiveType::U8 | PrimitiveType::I8 => 8,
        PrimitiveType::U16 | PrimitiveType::I16 => 16,
        PrimitiveType::U32 | PrimitiveType::I32 | PrimitiveType::F32 => 32,
        PrimitiveType::U64 | PrimitiveType::I64 | PrimitiveType::F64 => 64,
        PrimitiveType::Void => 0,
    };
    WireSize::Fixed(bits)
}

fn semantic_wire_size(s: &SemanticType) -> WireSize {
    match s {
        SemanticType::String | SemanticType::Bytes => WireSize::Variable {
            min_bits: 0,
            max_bits: None,
        },
        SemanticType::Rgb => WireSize::Fixed(24),
        SemanticType::Uuid => WireSize::Fixed(128),
        SemanticType::Timestamp => WireSize::Fixed(64),
        SemanticType::Hash => WireSize::Fixed(256),
    }
}

fn varint_wire_size(ty: &ResolvedType) -> WireSize {
    // Varint: LEB128. Min 1 byte (8 bits), max depends on type width.
    let max_bits = match ty {
        ResolvedType::Primitive(PrimitiveType::U16) => 24,  // ceil(16/7)*8
        ResolvedType::Primitive(PrimitiveType::U32) => 40,  // ceil(32/7)*8
        ResolvedType::Primitive(PrimitiveType::U64) => 80,  // ceil(64/7)*8
        _ => 80, // fallback
    };
    WireSize::Variable {
        min_bits: 8,
        max_bits: Some(max_bits),
    }
}

fn zigzag_wire_size(ty: &ResolvedType) -> WireSize {
    // ZigZag: same byte count as varint of unsigned equivalent.
    let max_bits = match ty {
        ResolvedType::Primitive(PrimitiveType::I16) => 24,
        ResolvedType::Primitive(PrimitiveType::I32) => 40,
        ResolvedType::Primitive(PrimitiveType::I64) => 80,
        _ => 80,
    };
    WireSize::Variable {
        min_bits: 8,
        max_bits: Some(max_bits),
    }
}

fn named_type_wire_size(id: TypeId, compiled: &CompiledSchema) -> WireSize {
    match compiled.registry.get(id) {
        Some(TypeDef::Enum(en)) => {
            let bits = match en.backing {
                EnumBacking::U8 => 8,
                EnumBacking::U16 => 16,
                EnumBacking::U32 => 32,
                EnumBacking::U64 => 64,
            };
            WireSize::Fixed(bits)
        }
        Some(TypeDef::Flags(_)) => WireSize::Fixed(64),
        Some(TypeDef::Newtype(nt)) => {
            compute_resolved_type_wire_size(&nt.terminal_type, compiled)
        }
        Some(TypeDef::Message(msg)) => {
            // If wire_size already computed, use it. Otherwise compute now.
            msg.wire_size.clone().unwrap_or_else(|| compute_message_wire_size(id, compiled))
        }
        Some(TypeDef::Union(un)) => {
            un.wire_size.clone().unwrap_or_else(|| compute_union_wire_size(id, compiled))
        }
        Some(TypeDef::Config(_)) => {
            // Config types are not wire-encoded.
            WireSize::Fixed(0)
        }
        None => {
            // Stub type — unknown size.
            WireSize::Variable {
                min_bits: 0,
                max_bits: None,
            }
        }
    }
}

fn compute_message_wire_size(id: TypeId, compiled: &CompiledSchema) -> WireSize {
    let msg = match compiled.registry.get(id) {
        Some(TypeDef::Message(m)) => m,
        _ => return WireSize::Fixed(0),
    };

    if msg.fields.is_empty() {
        return WireSize::Fixed(0);
    }

    let mut total_min: u64 = 0;
    let mut total_max: Option<u64> = Some(0);
    let mut is_variable = false;

    for field in &msg.fields {
        let ws = compute_type_wire_size(&field.resolved_type, &field.encoding, compiled);
        match ws {
            WireSize::Fixed(bits) => {
                total_min += bits;
                if let Some(ref mut max) = total_max {
                    *max += bits;
                }
            }
            WireSize::Variable { min_bits, max_bits } => {
                is_variable = true;
                total_min += min_bits;
                match (total_max, max_bits) {
                    (Some(cur), Some(field_max)) => total_max = Some(cur + field_max),
                    _ => total_max = None,
                }
            }
        }
    }

    if is_variable {
        WireSize::Variable {
            min_bits: total_min,
            max_bits: total_max,
        }
    } else {
        WireSize::Fixed(total_min)
    }
}

fn compute_union_wire_size(_id: TypeId, compiled: &CompiledSchema) -> WireSize {
    let un = match compiled.registry.get(_id) {
        Some(TypeDef::Union(u)) => u,
        _ => return WireSize::Fixed(0),
    };

    if un.variants.is_empty() {
        // Tag only (varint).
        return WireSize::Variable {
            min_bits: 8,
            max_bits: Some(8),
        };
    }

    // Union = tag (varint) + largest variant.
    let tag_min: u64 = 8; // LEB128 min
    let mut max_variant_bits: Option<u64> = Some(0);
    let mut min_variant_bits: u64 = u64::MAX;

    for variant in &un.variants {
        let mut var_min: u64 = 0;
        let mut var_max: Option<u64> = Some(0);

        for field in &variant.fields {
            let ws = compute_type_wire_size(&field.resolved_type, &field.encoding, compiled);
            match ws {
                WireSize::Fixed(bits) => {
                    var_min += bits;
                    if let Some(ref mut max) = var_max {
                        *max += bits;
                    }
                }
                WireSize::Variable { min_bits, max_bits } => {
                    var_min += min_bits;
                    match (var_max, max_bits) {
                        (Some(cur), Some(field_max)) => var_max = Some(cur + field_max),
                        _ => var_max = None,
                    }
                }
            }
        }

        min_variant_bits = std::cmp::min(min_variant_bits, var_min);
        match (max_variant_bits, var_max) {
            (Some(cur), Some(v)) => max_variant_bits = Some(std::cmp::max(cur, v)),
            _ => max_variant_bits = None,
        }
    }

    if min_variant_bits == u64::MAX {
        min_variant_bits = 0;
    }

    WireSize::Variable {
        min_bits: tag_min + min_variant_bits,
        max_bits: max_variant_bits.map(|m| tag_min + m),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn wire_size_min_bits(ws: &WireSize) -> u64 {
    match ws {
        WireSize::Fixed(bits) => *bits,
        WireSize::Variable { min_bits, .. } => *min_bits,
    }
}

fn wire_size_max_bits(ws: &WireSize) -> Option<u64> {
    match ws {
        WireSize::Fixed(bits) => Some(*bits),
        WireSize::Variable { max_bits, .. } => *max_bits,
    }
}

// ---------------------------------------------------------------------------
// Recursive type detection
// ---------------------------------------------------------------------------

/// For each message type, DFS through field types. Maintain a "direct" path
/// set — TypeIds on the current path that were reached without passing through
/// an indirection point (Optional, Array, Map, Result, Union). If we revisit
/// any TypeId already in `direct_path`, it's infinite recursion.
fn check_recursion(compiled: &CompiledSchema, diags: &mut Vec<Diagnostic>) {
    for &id in &compiled.declarations {
        if let Some(TypeDef::Message(msg)) = compiled.registry.get(id) {
            let mut direct_path = HashSet::new();
            direct_path.insert(id);
            walk_message_fields(id, &msg.fields.iter().collect::<Vec<_>>(), true, &mut direct_path, compiled, msg.span, diags);
        }
    }
}

fn walk_type_for_recursion(
    ty: &ResolvedType,
    direct: bool,
    direct_path: &mut HashSet<TypeId>,
    compiled: &CompiledSchema,
    origin_span: crate::span::Span,
    diags: &mut Vec<Diagnostic>,
) {
    match ty {
        ResolvedType::Named(id) => {
            // If this TypeId is already on the direct path, we have a cycle.
            if direct && direct_path.contains(id) {
                diags.push(Diagnostic::error(
                    origin_span,
                    ErrorClass::RecursiveTypeInfinite,
                    "type contains infinite direct recursion",
                ));
                return;
            }
            // If indirect (through Optional/Array/Map/Result/Union), cycle is OK.
            if !direct && direct_path.contains(id) {
                return;
            }

            match compiled.registry.get(*id) {
                Some(TypeDef::Message(msg)) => {
                    let was_new = if direct { direct_path.insert(*id) } else { false };
                    let fields: Vec<&FieldDef> = msg.fields.iter().collect();
                    walk_message_fields(*id, &fields, direct, direct_path, compiled, origin_span, diags);
                    if was_new { direct_path.remove(id); }
                }
                Some(TypeDef::Union(un)) => {
                    // Union dispatch = indirection point. Walk variant fields
                    // with direct=false.
                    for variant in &un.variants {
                        for field in &variant.fields {
                            walk_type_for_recursion(
                                &field.resolved_type, false, direct_path, compiled, origin_span, diags,
                            );
                        }
                    }
                }
                Some(TypeDef::Newtype(nt)) => {
                    walk_type_for_recursion(
                        &nt.inner_type, direct, direct_path, compiled, origin_span, diags,
                    );
                }
                _ => {} // Enum, Flags, Config, stub — terminal
            }
        }
        ResolvedType::Optional(inner) | ResolvedType::Array(inner) => {
            walk_type_for_recursion(inner, false, direct_path, compiled, origin_span, diags);
        }
        ResolvedType::Map(k, v) => {
            walk_type_for_recursion(k, false, direct_path, compiled, origin_span, diags);
            walk_type_for_recursion(v, false, direct_path, compiled, origin_span, diags);
        }
        ResolvedType::Result(ok, err) => {
            walk_type_for_recursion(ok, false, direct_path, compiled, origin_span, diags);
            walk_type_for_recursion(err, false, direct_path, compiled, origin_span, diags);
        }
        _ => {} // Primitive, SubByte, Semantic — terminal
    }
}

fn walk_message_fields(
    _msg_id: TypeId,
    fields: &[&FieldDef],
    direct: bool,
    direct_path: &mut HashSet<TypeId>,
    compiled: &CompiledSchema,
    origin_span: crate::span::Span,
    diags: &mut Vec<Diagnostic>,
) {
    for field in fields {
        walk_type_for_recursion(&field.resolved_type, direct, direct_path, compiled, origin_span, diags);
    }
}
```

- [ ] **Step 4: Verify `compile()` in `lib.rs` already uses `&mut`**

The `compile()` function written in Task 4 already calls `typeck::check(compiled)` with
`if let Some(ref mut compiled) = compiled`. Since `check()` now takes `&mut CompiledSchema`,
this should compile. The `compiled` binding from `lower::lower()` must be `let mut compiled`.
Verify this compiles: `cargo check -p vexil-lang`

- [ ] **Step 5: Run tests**

Run: `cargo test -p vexil-lang --test compile -- --nocapture`
Expected: all PASS including wire size tests

- [ ] **Step 6: Run quality gates**

Run: `cargo fmt --all -- --check && cargo clippy -p vexil-lang --all-targets -- -D warnings`
Expected: clean (fix any warnings)

- [ ] **Step 7: Commit**

```bash
git add crates/vexil-lang/src/typeck.rs crates/vexil-lang/src/lib.rs crates/vexil-lang/tests/compile.rs
VEXIL_COMMIT_TASK=1 git commit -m "feat(vexil-lang): type checker — wire size computation and recursive type detection"
```

---

### Task 8: Type Checker Tests — Recursion + Newtype Chains

**Files:**
- Modify: `crates/vexil-lang/tests/compile.rs`

- [ ] **Step 1: Add recursion and newtype tests**

Append to `compile.rs`:

```rust
use vexil_lang::diagnostic::ErrorClass;

/// Valid recursion through optional — no error.
#[test]
fn recursion_through_optional_valid() {
    let source = r#"
namespace test.rec
message Node {
    value @0 : i32
    next  @1 : optional<Node>
}
"#;
    let result = vexil_lang::compile(source);
    let errors: Vec<_> = result.diagnostics.iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "should allow recursion through optional: {errors:#?}");
}

/// Valid recursion through array — no error.
#[test]
fn recursion_through_array_valid() {
    let source = r#"
namespace test.rec2
message Tree {
    value    @0 : i32
    children @1 : array<Tree>
}
"#;
    let result = vexil_lang::compile(source);
    let errors: Vec<_> = result.diagnostics.iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "should allow recursion through array: {errors:#?}");
}

/// Valid mutual recursion through union — no error (corpus 016).
#[test]
fn recursion_through_union_valid() {
    let source = r#"
namespace test.rec3
message Expr {
    kind @0 : ExprKind
}
union ExprKind {
    Literal @0 { value @0 : i64 }
    Binary  @1 { left @0 : Expr  op @1 : u8  right @2 : Expr }
}
"#;
    let result = vexil_lang::compile(source);
    let errors: Vec<_> = result.diagnostics.iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "should allow mutual recursion through union: {errors:#?}");
}

/// Invalid direct self-recursion — error.
#[test]
fn recursion_direct_invalid() {
    let source = r#"
namespace test.rec4
message Bad {
    self_ref @0 : Bad
}
"#;
    let result = vexil_lang::compile(source);
    let has_recursive_error = result
        .diagnostics
        .iter()
        .any(|d| d.class == ErrorClass::RecursiveTypeInfinite);
    assert!(has_recursive_error, "should detect direct infinite recursion");
}

/// Invalid mutual direct recursion (A -> B -> A) — error.
#[test]
fn recursion_mutual_direct_invalid() {
    let source = r#"
namespace test.rec5
message A { b @0 : B }
message B { a @0 : A }
"#;
    let result = vexil_lang::compile(source);
    let has_recursive_error = result
        .diagnostics
        .iter()
        .any(|d| d.class == ErrorClass::RecursiveTypeInfinite);
    assert!(has_recursive_error, "should detect mutual direct infinite recursion");
}

/// Newtype terminal type resolves to primitive.
#[test]
fn newtype_terminal_type() {
    let source = "namespace test.ntterm\nnewtype Id : u64";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    let id = compiled.declarations[0];
    if let TypeDef::Newtype(nt) = compiled.registry.get(id).unwrap() {
        assert_eq!(
            nt.terminal_type,
            ResolvedType::Primitive(vexil_lang::ast::PrimitiveType::U64)
        );
    } else {
        panic!("expected Newtype");
    }
}

/// Schema-level @version annotation is preserved.
#[test]
fn schema_annotations_preserved() {
    let source = "@version(\"1.2.0\")\nnamespace test.sa\nmessage M { v @0 : u32 }";
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.as_ref().unwrap();
    assert_eq!(compiled.annotations.version.as_deref(), Some("1.2.0"));
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p vexil-lang --test compile -- --nocapture`
Expected: all PASS

- [ ] **Step 3: Commit**

```bash
git add crates/vexil-lang/tests/compile.rs
VEXIL_COMMIT_TASK=1 git commit -m "test(vexil-lang): recursion detection, newtype chains, and schema annotations"
```

---

### Task 9: Quality Gate + Full Corpus Validation

**Files:**
- Possibly: minor fixes to any file if tests fail

- [ ] **Step 1: Run all tests**

Run: `cargo test --workspace`
Expected: all existing 89 tests pass + all new compile tests pass

- [ ] **Step 2: Run clippy**

Run: `cargo clippy -p vexil-lang --all-targets -- -D warnings`
Expected: clean. Fix any warnings.

- [ ] **Step 3: Run fmt**

Run: `cargo fmt --all -- --check`
Expected: clean.

- [ ] **Step 4: Verify exit criteria**

Run the test that checks all 18 valid corpus files compile:
`cargo test -p vexil-lang --test compile valid_corpus_compiles -- --nocapture`

Run the test that checks all 56 invalid corpus files produce errors:
`cargo test -p vexil-lang --test compile invalid_corpus_produces_errors -- --nocapture`

Expected: both PASS

- [ ] **Step 5: Final commit if any fixes were needed**

```bash
VEXIL_COMMIT_TASK=1 git commit -m "chore(vexil-lang): milestone C quality gate clean"
```

---

## Exit Criteria Checklist

1. `cargo test --workspace` — all tests pass (existing 89 + new compile tests)
2. `cargo clippy --workspace --all-targets -- -D warnings` — clean
3. `cargo fmt --all -- --check` — clean
4. All 18 valid corpus files produce `CompiledSchema` with correct type counts
5. All 56 invalid corpus files produce error diagnostics
6. Wire size computation correct for primitive, composite, and variable types
7. Recursive type detection catches direct cycles, allows indirect cycles (optional, array, union)
8. `compile()` is a documented public API in `lib.rs`
