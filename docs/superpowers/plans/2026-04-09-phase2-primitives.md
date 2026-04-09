# Vexil Phase 2 Primitives Implementation Plan

**Date**: 2026-04-09  
**Status**: APPROVED  
**Scope**: Four language features for v0.4.0

---

## Overview

This plan details the implementation of four new Vexil language features:

1. **Fixed-point types** (`fixed32`, `fixed64`) — Q16.16 and Q32.32 fixed-point primitives
2. **Named constants** (`const NAME: type = value`) — Compile-time constants for use in constraints
3. **Type aliases** (`type Name = existing_type`) — Transparent type aliases
4. **Where clauses** (`field where expr`) — Field-level declarative constraints

**Pipeline Stages Affected**: Lexer → Parser → AST → Validate → Lower → IR → TypeCheck → Codegen

---

## Feature 1: Fixed-Point Types (fixed32, fixed64)

### Summary
Two new primitive types for fixed-point arithmetic:
- `fixed32`: Q16.16 format (16 bits integer, 16 bits fraction), 32 bits wire
- `fixed64`: Q32.32 format (32 bits integer, 32 bits fraction), 64 bits wire

Wire encoding uses two's complement (raw i32/i64). Optional `@varint` applies LEB128 to raw bits. `@zigzag` is NOT valid on fixed types (semantically different from signed integers).

### Grammar Changes

The grammar addition is minimal — two new entries in the primitive-type rule and one keyword.

**spec/vexil-grammar.peg** (lines 261-268):
```peg
primitive-type
    = 'bool' !ident-continue / 'void' !ident-continue
    / 'u64'  !ident-continue / 'u32'  !ident-continue
    / 'u16'  !ident-continue / 'u8'   !ident-continue
    / 'i64'  !ident-continue / 'i32'  !ident-continue
    / 'i16'  !ident-continue / 'i8'   !ident-continue
    / 'f64'  !ident-continue / 'f32'  !ident-continue
    / 'fixed64' !ident-continue / 'fixed32' !ident-continue  # ADD
```

**keyword rule** (line 374-383):
Add to keyword list: `'fixed32' / 'fixed64'`

### Files to Modify

The following files require changes, spanning lexer through codegen.

#### 1. `crates/vexil-lang/src/lexer/token.rs`

**Add new token kinds** (after line 63):
```rust
KwFixed32,
KwFixed64,
```

**Update `is_keyword()`** (line 69-90):
Add `| TokenKind::KwFixed32 | TokenKind::KwFixed64` to the matches pattern.

**Update `as_field_name()`** (lines 93-115):
Add:
```rust
TokenKind::KwFixed32 => Some("fixed32".into()),
TokenKind::KwFixed64 => Some("fixed64".into()),
```

#### 2. `crates/vexil-lang/src/lexer/mod.rs`

**Add keyword recognition** (line 278-298):
```rust
"fixed32" => TokenKind::KwFixed32,
"fixed64" => TokenKind::KwFixed64,
```

#### 3. `crates/vexil-lang/src/ast/mod.rs`

**Add to `PrimitiveType` enum** (lines 223-237):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    // ... existing variants
    Fixed32,  // Q16.16
    Fixed64,  // Q32.32
}
```

#### 4. `crates/vexil-lang/src/parser/expr.rs`

**Add type parsing** (lines 87-119):
```rust
"fixed32" => TypeExpr::Primitive(PrimitiveType::Fixed32),
"fixed64" => TypeExpr::Primitive(PrimitiveType::Fixed64),
```

#### 5. `crates/vexil-lang/src/ir/types.rs`

No changes needed — `PrimitiveType` is re-exported from `ast`.

#### 6. `crates/vexil-lang/src/typeck.rs`

**Update `primitive_wire_size()`** (lines 156-166):
```rust
fn primitive_wire_size(p: &PrimitiveType) -> WireSize {
    let bits = match p {
        // ... existing variants
        PrimitiveType::Fixed32 => 32,
        PrimitiveType::Fixed64 => 64,
    };
    WireSize::Fixed(bits)
}
```

**Update `is_delta_eligible()`** (lines 587-605):
Add `Fixed32` and `Fixed64` to the eligible list.

**Update `optimal_delta_inner()`** (lines 618-631):
Fixed-point types should use `Encoding::Varint` for delta (like other integers).

#### 7. `crates/vexil-lang/src/validate.rs`

No validation changes needed — fixed-point types follow same rules as other primitives.

#### 8. `crates/vexil-codegen-rust/src/lib.rs`

The codegen layer maps fixed-point types to their underlying integer representations and generates helper methods for conversion.

Update code generation to:
- Map `Fixed32` → `i32` with helper methods for fixed-point math
- Map `Fixed64` → `i64` with helper methods for fixed-point math
- Generate pack/unpack as raw i32/i64

**Suggested struct generation**:
```rust
// For fixed32 fields
pub struct Fixed32(i32);

impl Fixed32 {
    pub const SCALE: i32 = 65536; // 2^16
    
    pub fn from_f64(v: f64) -> Self {
        Self((v * Self::SCALE as f64) as i32)
    }
    
    pub fn to_f64(self) -> f64 {
        self.0 as f64 / Self::SCALE as f64
    }
}
```

#### 9. `crates/vexil-runtime/src/`

The runtime crate needs corresponding wrapper types for pack/unpack support.

Add `Fixed32`/`Fixed64` wrapper types in `src/fixed.rs`:

```rust
pub struct Fixed32(pub i32);
pub struct Fixed64(pub i64);

impl Pack for Fixed32 { /* write i32 raw */ }
impl Unpack for Fixed32 { /* read i32 raw */ }
// ... same for Fixed64
```

### Test Files

**corpus/valid/033_fixed_point.vexil**:
```vexil
namespace test.fixed

message SensorReading {
    temperature @0 : fixed32  # Q16.16 degrees Celsius
    pressure    @1 : fixed64  # Q32.32 Pascals
    delta_temp  @2 : fixed32 @varint
}
```

**corpus/invalid/058_fixed_zigzag_invalid.vexil** (negative test for @zigzag on fixed):
```vexil
namespace test.fixed

message Invalid {
    bad @0 : fixed32 @zigzag  # Error: @zigzag not allowed on fixed types
}
```

---

## Feature 2: Named Constants (`const NAME: type = value`)

### Summary
Compile-time named constants usable in constraints, array sizes, and other const contexts. Constants form a DAG; topological sort resolves dependencies.

### Grammar Changes

**spec/vexil-grammar.peg**:

Add new declaration form:
```peg
# §4  Type declarations — add const-decl

type-decl
    = message-decl
    / enum-decl
    / flags-decl
    / union-decl
    / newtype-decl
    / config-decl
    / const-decl           # ADD

# ── const ────────────────────────────────────────────────────────────

const-decl
    = annotation* KW_CONST __ upper-ident _ ':' _ type-expr _ '=' _ const-expr

const-expr
    = dec-int
    / hex-int
    / float-lit              # ADD: fixed-point literal support (e.g., 100.0)
    / const-ref
    / const-expr _ ('+' / '-' / '*' / '/') _ const-expr

const-ref
    = upper-ident
```

Add `KW_CONST = 'const' !ident-continue` to keyword tokens.

### Files to Modify

The following files require changes to support const declarations, ordered by compilation pipeline stage.

#### 1. `crates/vexil-lang/src/lexer/token.rs`

Add `KwConst` token kind.

#### 2. `crates/vexil-lang/src/lexer/mod.rs`

Add `"const" => TokenKind::KwConst` to keyword map.

#### 3. `crates/vexil-lang/src/ast/mod.rs`

**Add `ConstDecl` to `Decl` enum** (line 56-64):
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    // ... existing variants
    Const(ConstDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl {
    pub annotations: Vec<Annotation>,
    pub name: Spanned<SmolStr>,
    pub ty: Spanned<TypeExpr>,
    pub value: Spanned<ConstExpr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConstExpr {
    Int(i64),
    UInt(u64),
    Hex(u64),
    Ref(SmolStr),              // Named constant reference
    Binary(Box<Spanned<ConstExpr>>, BinOp, Box<Spanned<ConstExpr>>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add, Sub, Mul, Div,
}
```

#### 4. `crates/vexil-lang/src/parser/decl.rs`

**Update `parse_type_decl()`** (lines 18-76):
Add `TokenKind::KwConst` case.

**Add `parse_const_decl()`**:
```rust
fn parse_const_decl(annotations: Vec<Annotation>, p: &mut Parser<'_>) -> ConstDecl {
    p.advance(); // consume KwConst
    let name = parse_decl_name(p).unwrap_or_else(|| /* error handling */);
    p.expect(&TokenKind::Colon);
    let ty = parse_type_expr(p);
    p.expect(&TokenKind::Eq);
    let value = parse_const_expr(p);
    ConstDecl { annotations, name, ty, value }
}
```

**Add `parse_const_expr()`** (new function):
Parse arithmetic expressions with operator precedence (+, -, *, /) and constant references.

#### 5. `crates/vexil-lang/src/validate.rs`

**Add validation for const declarations**:

- Type must be integral (u8-u64, i8-i64, or sub-byte) or fixed-point (fixed32, fixed64)
- Fixed-point literals (e.g., `100.0`) supported via new ConstExpr::Float variant
- No cycles in const dependencies (requires building a dependency graph)
- Division by zero check for evaluated expressions
- Expression overflow detection
- **Division semantics**: Integer truncation for integral types; fixed-point division for fixed-point types

**New error classes in `diagnostic.rs`**:
```rust
ConstTypeInvalid,          // Non-integral type in const
ConstCycleDetected,        // Circular dependency
ConstDivByZero,          // Division by zero in expr
ConstOverflow,           // Arithmetic overflow
ConstRefNotFound,        // Unknown constant reference
```

**Add topological sort for const evaluation**:
```rust
fn evaluate_const_values(
    consts: &[&ConstDecl],
    diags: &mut Vec<Diagnostic>
) -> HashMap<SmolStr, i64> {
    // 1. Build dependency graph
    // 2. Topological sort (Kahn's algorithm)
    // 3. Evaluate in dependency order
}
```

#### 6. `crates/vexil-lang/src/lower.rs`

**Add `ConstDef` to IR** in `ir/mod.rs`:
```rust
#[derive(Debug, Clone)]
pub struct ConstDef {
    pub name: SmolStr,
    pub span: Span,
    pub resolved_type: ResolvedType,
    pub evaluated_value: i64,  // After expression evaluation
    pub annotations: ResolvedAnnotations,
}
```

**Update `TypeDef` enum**:
```rust
pub enum TypeDef {
    // ... existing variants
    Const(ConstDef),
}
```

**Add `lower_const()` function**:
```rust
fn lower_const(c: &ConstDecl, span: Span, ctx: &mut LowerCtx) -> ConstDef {
    let resolved_type = resolve_type_expr(&c.ty.node, c.ty.span, ctx);
    // Value is already evaluated by validate phase
    let evaluated_value = c.evaluated_value;
    ConstDef { /* ... */ }
}
```

#### 7. `crates/vexil-lang/src/typeck.rs`

No changes — constants are compile-time only.

### Test Files

**corpus/valid/034_constants.vexil**:
```vexil
namespace test.consts

const MAX_BUFFER_SIZE: u32 = 1024
const HEADER_SIZE: u32 = 16
const PAYLOAD_SIZE: u32 = MAX_BUFFER_SIZE - HEADER_SIZE

message Packet {
    header @0 : [u8; HEADER_SIZE]   # Array size from constant
    data   @1 : [u8; PAYLOAD_SIZE]
}
```

**corpus/invalid/059_const_cycle.vexil**:
```vexil
namespace test.consts

const A: u32 = B + 1
const B: u32 = A + 1  # Error: circular dependency
```

**corpus/invalid/060_const_div_by_zero.vexil**:
```vexil
namespace test.consts

const BAD: u32 = 100 / 0  # Error: division by zero
```

**corpus/invalid/061_const_type_invalid.vexil**:
```vexil
namespace test.consts

const STR: string = "hello"  # Error: const must be integral type
```

---

## Feature 3: Type Aliases (`type Name = existing_type`)

### Summary
Transparent type aliases that resolve to their terminal type during lowering. Aliases are semantically invisible — they don't create new types, just new names for existing ones.

### Grammar Changes

A single new declaration form is added to the grammar.

**spec/vexil-grammar.peg**:

```peg
# §4  Type declarations

type-decl
    = message-decl
    / enum-decl
    / flags-decl
    / union-decl
    / newtype-decl
    / config-decl
    / const-decl
    / alias-decl           # ADD

# ── type alias ───────────────────────────────────────────────────────

alias-decl
    = annotation* KW_TYPE __ upper-ident _ '=' _ type-expr

# NOTE: Aliases must resolve to a terminal (non-alias) type.
# type A = B where B is `type B = u32` is REJECTED.
# Only terminal types (primitives, messages, enums, etc.) allowed as targets.
```

Add `KW_TYPE = 'type' !ident-continue`.

### Files to Modify

The following files require changes, spanning lexer through lowering.

#### 1. `crates/vexil-lang/src/lexer/token.rs`

Add `KwType` token.

#### 2. `crates/vexil-lang/src/lexer/mod.rs`

Add `"type" => TokenKind::KwType`.

#### 3. `crates/vexil-lang/src/ast/mod.rs`

**Add `AliasDecl` to `Decl` enum**:
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    // ... existing
    Alias(AliasDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AliasDecl {
    pub annotations: Vec<Annotation>,
    pub name: Spanned<SmolStr>,
    pub target: Spanned<TypeExpr>,
}
```

#### 4. `crates/vexil-lang/src/parser/decl.rs`

**Add `parse_alias_decl()`**:
```rust
fn parse_alias_decl(annotations: Vec<Annotation>, p: &mut Parser<'_>) -> AliasDecl {
    p.advance(); // consume KwType
    let name = parse_decl_name(p).unwrap_or_else(|| /* error */);
    p.expect(&TokenKind::Eq);
    let target = parse_type_expr(p);
    AliasDecl { annotations, name, target }
}
```

#### 5. `crates/vexil-lang/src/validate.rs`

**Validation rules**:
- Target type must exist (not a forward reference to undefined type)
- **CRITICAL**: Target must be a terminal type — aliases cannot reference other aliases
- No alias cycles (A = B, B = A)

**Alias-to-alias rejection example**:
```vexil
type B = u32
type A = B   # Error: A references alias B; must reference u32 directly
```

**New error classes**:
```rust
AliasTargetIsAlias,      // Alias references another alias (not terminal)
AliasCycleDetected,      // Circular type alias
AliasTargetNotFound,     // Target type doesn't exist
```

#### 6. `crates/vexil-lang/src/lower.rs`

**Critical**: Aliases are transparent — they don't create `TypeDef` entries. Instead, they add name→TypeId mappings in the registry that point to the resolved target type.

```rust
fn lower_alias(
    alias: &AliasDecl,
    ctx: &mut LowerCtx
) -> (SmolStr, TypeId) {
    let target_type = resolve_type_expr(&alias.target.node, alias.target.span, ctx);
    
    // Get or create TypeId for target
    let target_id = match target_type {
        ResolvedType::Named(id) => id,
        ResolvedType::Primitive(p) => {
            // Create anonymous primitive entry or use sentinel
            ctx.registry.lookup_or_create_primitive(p)
        }
        // ... other cases
    };
    
    // Return (alias_name, target_id) — registry will add secondary mapping
    (alias.name.node.clone(), target_id)
}
```

**Update `TypeRegistry`** in `ir/types.rs`:
```rust
/// Secondary name mappings for aliases (alias name -> target TypeId)
pub alias_map: HashMap<SmolStr, TypeId>,

pub fn register_alias(&mut self, alias: SmolStr, target: TypeId) {
    self.alias_map.insert(alias, target);
}

pub fn lookup(&self, name: &str) -> Option<TypeId> {
    self.by_name.get(name).copied()
        .or_else(|| self.alias_map.get(name).copied())
}
```

**Update `lower()` function**:
Aliases must be processed AFTER all regular declarations to ensure target types exist.

### Test Files

**corpus/valid/035_type_alias.vexil**:
```vexil
namespace test.aliases

type UserId = u64
type Score = fixed32

message GameState {
    player_id @0 : UserId
    score     @1 : Score
}
```

**corpus/invalid/062_alias_to_alias.vexil**:
```vexil
namespace test.aliases

type B = u32
type A = B  # Error: alias A references alias B; must reference terminal type
```

**corpus/invalid/063_alias_cycle.vexil**:
```vexil
namespace test.aliases

type A = B
type B = A  # Error: circular alias
```

---

## Feature 4: Where Clauses (`field where expr`)

### Summary

Declarative field-level constraints using the `where` keyword. Validation code is generated into pack/unpack, rejecting invalid data at encode/decode time. Supports:
- Equality: `==`, `!=`
- Comparison: `<`, `<=`, `>`, `>=`
- Logical: `&&`, `||`, `!`
- Range: `value in low..high` (inclusive), `value in low..<high` (exclusive upper bound)
- Built-in: `len(field)` for arrays, maps, strings, bytes

### Grammar Changes

Where clauses extend the field production with an optional constraint expression.

**spec/vexil-grammar.peg**:

```peg
# §5  Fields

field
    = field-annotations field-name _ ordinal field-annotations ':' _ type-expr 
      field-annotations where-clause?   # ADD where-clause

where-clause
    = _ KW_WHERE _ where-expr

where-expr
    = or-expr

or-expr
    = and-expr ( _ '||' _ and-expr )*

and-expr
    = not-expr ( _ '&&' _ not-expr )*

not-expr
    = '!' _ not-expr / primary-expr

primary-expr
    = comparison-expr
    / range-expr
    / '(' _ where-expr _ ')'

comparison-expr
    = operand _ ('==' / '!=' / '<' / '<=' / '>' / '>=') _ operand

range-expr
    = 'value' _ KW_IN _ operand _ '..' _ operand     # inclusive..inclusive
    / 'value' _ KW_IN _ operand _ '..<' _ operand    # inclusive..exclusive (ADD)

operand
    = dec-int
    / hex-int
    / float-lit           # ADD: float literal (e.g., 0.0, 1.0)
    / 'len' _ '(' _ field-ref _ ')'
    / field-ref
    / upper-ident         # const reference

field-ref
    = 'value'               # The field being constrained
    / ident                 # Another field in same message
```

Add `KW_WHERE = 'where' !ident-continue` and `KW_IN = 'in' !ident-continue`.

### Files to Modify

Where clauses touch the most files — lexer, parser, AST, validate, lower, type checker, and both codegen backends.

#### 1. `crates/vexil-lang/src/lexer/token.rs`

Add:
```rust
KwWhere,
KwIn,
// Operators
PipePipe,      // ||
AmpersandAmpersand, // &&
Bang,          // ! (already have Minus, need dedicated)
EqEq,          // ==
NotEq,         // !=
Lt,            // < (already have LAngle, disambiguate)
Le,            // <=
Gt,            // > (already have RAngle, disambiguate)
Ge,            // >=
DotDot,        // ..
```

**M1**: Verify no token name conflicts in `lexer/token.rs`. Proposed names: `KwWhere`, `KwIn`, `PipePipe`, `AmpersandAmpersand`, `Bang`, `EqEq`, `NotEq`, `Le`, `Ge`, `DotDot`, `DotDotLt` (for `..<`).

Note: Some tokens exist (`LAngle`, `RAngle`, `Minus`). Need to handle `<=`, `>=`, `!=`, `==` as multi-char tokens.

#### 2. `crates/vexil-lang/src/lexer/mod.rs`

**Update `next_token()`** to handle multi-char operators:
```rust
b'!' => {
    if self.peek() == Some(b'=') {
        self.pos += 1;
        self.make_token(TokenKind::NotEq, start)
    } else {
        self.make_token(TokenKind::Bang, start)
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
b'|' => {
    if self.peek() == Some(b'|') {
        self.pos += 1;
        self.make_token(TokenKind::PipePipe, start)
    } else {
        // Error: single | not valid
    }
}
b'&' => {
    if self.peek() == Some(b'&') {
        self.pos += 1;
        self.make_token(TokenKind::AmpersandAmpersand, start)
    } else {
        // Error: single & not valid
    }
}
```

Add keywords: `"where"`, `"in"`.

#### 3. `crates/vexil-lang/src/ast/mod.rs`

**Add to `MessageField`** (lines 83-91):
```rust
pub struct MessageField {
    // ... existing fields
    pub where_clause: Option<Spanned<WhereExpr>>,
}
```

**Add WhereExpr types**:
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct WhereExpr {
    pub kind: WhereExprKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WhereExprKind {
    Binary(Box<Spanned<WhereExpr>>, WhereOp, Box<Spanned<WhereExpr>>),
    Not(Box<Spanned<WhereExpr>>),
    Comparison(Operand, CmpOp, Operand),
    Range { 
        low: Operand, 
        high: Operand, 
        inclusive_high: bool  // true for `..`, false for `..<`
    }, // value in low..high or value in low..<high
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhereOp {
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq, Ne, Lt, Le, Gt, Ge,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    Int(i64),
    UInt(u64),
    FieldRef(SmolStr),      // "value" or field name
    ConstRef(SmolStr),      // Named constant
    Len(SmolStr),           // len(field_name)
}
```

#### 4. `crates/vexil-lang/src/parser/decl.rs`

**Update `parse_field()`** (lines 173-243):
After parsing post_type_annotations, check for `KwWhere`:
```rust
let where_clause = if p.at(&TokenKind::KwWhere) {
    p.advance();
    Some(parse_where_expr(p))
} else {
    None
};
```

**Add `parse_where_expr()`** (new module or in expr.rs):
Implement recursive descent for expression grammar with precedence climbing.

#### 5. `crates/vexil-lang/src/validate.rs`

**Validation rules**:
- Field references in `where` clause must exist in the same message
- Constant references must resolve to declared constants
- Type-compatible comparisons (can't compare string to number)
- `len()` on arrays, maps, strings, bytes
- Range bounds must be ordered (low < high for exclusive, low <= high for inclusive)

**Auto-validation on encode/decode**: 
Where clause validation is generated into pack/unpack implementations. Invalid data is rejected at encode/decode time, not via explicit validate() methods.

**New error classes**:
```rust
WhereFieldNotFound,      // Referenced field doesn't exist
WhereConstNotFound,      // Referenced const doesn't exist
WhereTypeMismatch,       // Comparing incompatible types
WhereLenOnInvalidType,   // len() on non-collection
WhereRangeInvalid,       // low >= high in range
```

#### 6. `crates/vexil-lang/src/lower.rs`

**Add to IR `FieldDef`** in `ir/mod.rs`:
```rust
pub struct FieldDef {
    // ... existing fields
    pub where_clause: Option<WhereClauseDef>,
}

#[derive(Debug, Clone)]
pub struct WhereClauseDef {
    pub expr: WhereExprDef,
    // Pre-evaluated for codegen
    pub referenced_fields: Vec<SmolStr>,
    pub referenced_consts: Vec<SmolStr>,
}

#[derive(Debug, Clone)]
pub enum WhereExprDef {
    And(Box<WhereExprDef>, Box<WhereExprDef>),
    Or(Box<WhereExprDef>, Box<WhereExprDef>),
    Not(Box<WhereExprDef>),
    Comparison(WhereOperandDef, CmpOp, WhereOperandDef),
    Range { 
        low: WhereOperandDef, 
        high: WhereOperandDef,
        inclusive_high: bool,
    },
}

#[derive(Debug, Clone)]
pub enum WhereOperandDef {
    I64(i64),
    U64(u64),
    F64(f64),              // ADD: float literal
    FieldRef(SmolStr),
    ConstRef(SmolStr),
    Len(SmolStr),
}
```

**Add `lower_where_expr()`**:
Resolve all references to concrete IR forms.

#### 7. `crates/vexil-lang/src/typeck.rs`

Type-check where clause operands against field types:
```rust
fn check_where_clause(
    expr: &WhereExprDef,
    field_types: &HashMap<SmolStr, ResolvedType>,
    diags: &mut Vec<Diagnostic>
)
```

#### 8. Codegen Backends

Both Rust and TypeScript backends generate validation code in pack/unpack methods:

**Rust example**:
```rust
impl Pack for MessageName {
    fn pack<W: BitWriter>(&self, w: &mut W) -> Result<(), EncodeError> {
        // where clause validation on ENCODE
        // where clause: value > 0 && value < 100
        if !(self.field_name > 0 && self.field_name < 100) {
            return Err(EncodeError::WhereConstraintFailed {
                field: "field_name",
                constraint: "value > 0 && value < 100",
            });
        }
        
        // where clause: len(data) <= MAX_SIZE
        if !(self.data.len() <= MAX_SIZE as usize) {
            return Err(EncodeError::WhereConstraintFailed { /* ... */ });
        }
        
        // ... actual packing
        Ok(())
    }
}

impl Unpack for MessageName {
    fn unpack<R: BitReader>(r: &mut R) -> Result<Self, DecodeError> {
        // ... unpacking fields ...
        
        // where clause validation on DECODE
        if !(field_name > 0 && field_name < 100) {
            return Err(DecodeError::WhereConstraintFailed { /* ... */ });
        }
        
        Ok(Self { field_name, /* ... */ })
    }
}
```

**TypeScript example**:
```typescript
// Generated encode function
export function encodeMessageName(msg: MessageName, w: BitWriter): void {
    // where clause validation on ENCODE
    if (!(msg.smallPositive > 0 && msg.smallPositive < 100)) {
        throw new EncodeError("Where constraint failed: small_positive");
    }
    
    if (!(msg.percentage >= 0.0 && msg.percentage <= 1.0)) {
        throw new EncodeError("Where constraint failed: percentage");
    }
    
    if (!(msg.items.length <= MAX_ITEMS)) {
        throw new EncodeError("Where constraint failed: items length");
    }
    
    // ... actual encoding
}

// Generated decode function
export function decodeMessageName(r: BitReader): MessageName {
    // ... decoding fields ...
    
    // where clause validation on DECODE
    if (!(smallPositive > 0 && smallPositive < 100)) {
        throw new DecodeError("Where constraint failed: small_positive");
    }
    
    return { smallPositive, /* ... */ };
}
```

### Test Files

**corpus/valid/036_where_clauses.vexil**:
```vexil
namespace test.where

const MAX_ITEMS: u32 = 100
const MIN_PERCENT: fixed32 = 0.0
const MAX_PERCENT: fixed32 = 1.0

message ConstrainedMessage {
    small_positive @0 : u32 where value > 0 && value < 100
    percentage     @1 : fixed32 where value >= MIN_PERCENT && value <= MAX_PERCENT
    items          @2 : array<Item> where len(items) <= MAX_ITEMS
    code           @3 : u8 where value in 0..<256   # exclusive: 0-255
    status         @4 : u8 where value in 1..5     # inclusive: 1-5
}

message Item {
    data @0 : u32
}
```

**corpus/invalid/064_where_field_not_found.vexil**:
```vexil
namespace test.where

message Bad {
    x @0 : u32 where y > 10  # Error: no field named 'y'
}
```

**corpus/invalid/065_where_type_mismatch.vexil**:
```vexil
namespace test.where

message Bad {
    name @0 : string where name > 10  # Error: can't compare string to int
}
```

**corpus/invalid/066_where_range_invalid.vexil**:
```vexil
namespace test.where

message Bad {
    x @0 : u32 where value in 100..10   # Error: low >= high
    y @1 : u32 where value in 5..<5    # Error: low == high with exclusive
}
```

---

## Implementation Order

With all four features specified above, the following ordering minimizes integration issues during development.

The features have minimal dependencies and can be implemented in parallel, but this sequence is recommended:

1. **Fixed-point types** — Self-contained, extends primitive type system
2. **Type aliases** — Requires registry changes, simpler than constants
3. **Named constants** — Builds on expression evaluation, needed for where clauses
4. **Where clauses** — Uses constants and requires most new grammar

### Milestone Checkpoints

| Checkpoint | Deliverables | Tests |
|------------|-------------|-------|
| M1 | Fixed32/64 lexing, parsing, IR, wire sizes | corpus/valid/033, corpus/invalid/058, codegen-rust golden tests |
| M2 | Type alias lexing, parsing, lowering, resolution | corpus/valid/035, corpus/invalid/062-063 |
| M3 | Const lexing, parsing, evaluation, topo sort | corpus/valid/034, corpus/invalid/059-061 |
| M4 | Where clause lexing, parsing, validation, inclusive/exclusive range | corpus/valid/036, corpus/invalid/064-066 |
| M5 | Codegen integration for all features | All compliance tests pass |

---

## Registry Changes Summary

The `TypeRegistry` requires these enhancements:

```rust
pub struct TypeRegistry {
    types: Vec<Option<TypeDef>>,
    by_name: HashMap<SmolStr, TypeId>,
    // NEW: Secondary mappings for aliases (alias name -> target TypeId)
    alias_map: HashMap<SmolStr, TypeId>,
    // NEW: Constant values (const name -> evaluated value)
    constants: HashMap<SmolStr, ConstValue>,
}

pub struct ConstValue {
    pub ty: ResolvedType,
    pub value: i64,
    pub span: Span,
}
```

---

## Edge Cases & Design Decisions

### Fixed-Point Types
- **Q**: Should `@zigzag` be allowed on fixed types?  
  **A**: No — fixed types are conceptually "decimal" numbers, not signed integers. Use `@varint` only.
- **Q**: What's the wire format for fixed32 with @varint?  
  **A**: Raw i32 bits encoded as LEB128 (same as any 32-bit signed varint).

### Named Constants
- **Q**: Can constants reference imported constants?  
  **A**: Yes, if the import is available at compile time.
- **Q**: What about floating-point constants?  
  **A**: NOW SUPPORTED — fixed32/fixed64 const types with float literal syntax (e.g., `const MAX: fixed32 = 100.0`).
- **Q**: What are the division semantics?  
  **A**: Integer truncation for integral types; fixed-point division for fixed types. Example: `const TICK_DURATION: fixed64 = 1.0 / TICKS_PER_SEC` performs fixed-point division.

### Type Aliases
- **Q**: Can aliases be exported/imported?  
  **A**: Yes, they resolve to the target type on import.
- **Q**: Alias vs Newtype distinction?  
  **A**: Alias = transparent (same type), Newtype = distinct wrapper type.
- **Q**: Can aliases reference other aliases (chains)?  
  **A**: **NO** — aliases must resolve directly to a terminal type. `type A = B` where B is `type B = u32` is rejected.

### Where Clauses
- **Q**: Are where clauses checked at decode time?  
  **A**: **YES** — validation code is generated into pack/unpack; invalid data is rejected at encode/decode time.
- **Q**: Can where clauses reference optional fields?  
  **A**: Yes, but the constraint is only checked when the field is present.
- **Q**: What types support `len()`?  
  **A**: Arrays, maps, strings, and bytes.
- **Q**: Range syntax clarification?  
  **A**: `value in 0..100` means 0 to 100 inclusive. `value in 0..<100` means 0 to 99 (exclusive upper bound).

---

## Compliance Vectors

New compliance vectors needed in `compliance/vectors/`:

| Vector | Feature | Description |
|--------|---------|-------------|
| fixed32_basic | Fixed-point | fixed32 value 1.5 — wire bytes `00 80 01 00` (LE) |
| fixed64_basic | Fixed-point | fixed64 value 1.5 — wire bytes `00 00 00 00 00 80 01 00` (LE) |
| fixed32_varint | Fixed-point | fixed32 @varint encoding — wire bytes LEB128 of 0x00018000 |

---

## Appendix: File Change Summary

| File | Lines Changed | Nature |
|------|---------------|--------|
| `spec/vexil-grammar.peg` | +40 | Grammar additions |
| `crates/vexil-lang/src/lexer/token.rs` | +25 | New token kinds |
| `crates/vexil-lang/src/lexer/mod.rs` | +35 | Keyword recognition, multi-char ops |
| `crates/vexil-lang/src/ast/mod.rs` | +80 | New AST node types |
| `crates/vexil-lang/src/parser/decl.rs` | +120 | New declaration parsers |
| `crates/vexil-lang/src/parser/expr.rs` | +150 | Const expr, where expr parsing |
| `crates/vexil-lang/src/validate.rs` | +200 | Semantic validation |
| `crates/vexil-lang/src/lower.rs` | +100 | Lowering new constructs |
| `crates/vexil-lang/src/ir/mod.rs` | +60 | IR extensions |
| `crates/vexil-lang/src/ir/types.rs` | +20 | Registry enhancements |
| `crates/vexil-lang/src/typeck.rs` | +50 | Where clause type checking |
| `crates/vexil-lang/src/diagnostic.rs` | +15 | New error classes |
| `crates/vexil-runtime/src/fixed.rs` | +80 | Fixed-point runtime types |
| `crates/vexil-codegen-rust/src/lib.rs` | +100 | Rust codegen updates |
| `crates/vexil-codegen-ts/src/lib.rs` | +100 | TypeScript codegen updates |
| `corpus/valid/*.vexil` | +4 files | New test cases |
| `corpus/invalid/*.vexil` | +7 files | Error test cases (058, 059-061, 062-063, 064-066) |
| `compliance/vectors/*.json` | +3 files | Cross-impl tests |
| `corpus/MANIFEST.md` | +11 entries | **M2**: Must update with spec references per CLAUDE.md |

**Estimated Total**: ~1,175 lines across 19 files
</content>
