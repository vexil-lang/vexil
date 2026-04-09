//! # Stability: Tier 2
//!
//! Abstract syntax tree types produced by the parser.
//!
//! The AST is source-faithful: it preserves all syntactic structure
//! from the original `.vexil` file. It is consumed by the validator
//! and the lowering pass, and can also serve as the basis for LSP
//! features and source-level tooling.

pub mod visit;

use crate::span::{Span, Spanned};
use smol_str::SmolStr;

// ---------------------------------------------------------------------------
// Top-level
// ---------------------------------------------------------------------------

/// Top-level schema document.
#[derive(Debug, Clone, PartialEq)]
pub struct Schema {
    pub span: Span,
    pub annotations: Vec<Annotation>,
    pub namespace: Option<Spanned<NamespaceDecl>>,
    pub imports: Vec<Spanned<ImportDecl>>,
    pub declarations: Vec<Spanned<Decl>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamespaceDecl {
    pub path: Vec<Spanned<SmolStr>>,
}

// ---------------------------------------------------------------------------
// Imports
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl {
    pub kind: ImportKind,
    pub path: Vec<Spanned<SmolStr>>,
    pub version: Option<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportKind {
    Wildcard,
    Named { names: Vec<Spanned<SmolStr>> },
    Aliased { alias: Spanned<SmolStr> },
}

// ---------------------------------------------------------------------------
// Declarations
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    Message(MessageDecl),
    Enum(EnumDecl),
    Flags(FlagsDecl),
    Union(UnionDecl),
    Newtype(NewtypeDecl),
    Config(ConfigDecl),
    Alias(AliasDecl),
    Const(ConstDecl),
}

// ---------------------------------------------------------------------------
// Message
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct MessageDecl {
    pub annotations: Vec<Annotation>,
    pub name: Spanned<SmolStr>,
    pub body: Vec<MessageBodyItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageBodyItem {
    Field(Spanned<MessageField>),
    Tombstone(Spanned<Tombstone>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MessageField {
    pub pre_annotations: Vec<Annotation>,
    pub name: Spanned<SmolStr>,
    pub ordinal: Spanned<u32>,
    pub post_ordinal_annotations: Vec<Annotation>,
    pub ty: Spanned<TypeExpr>,
    pub post_type_annotations: Vec<Annotation>,
    pub where_clause: Option<Spanned<WhereExpr>>,
}

// ---------------------------------------------------------------------------
// Enum
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub annotations: Vec<Annotation>,
    pub name: Spanned<SmolStr>,
    pub backing: Option<Spanned<EnumBacking>>,
    pub body: Vec<EnumBodyItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnumBodyItem {
    Variant(Spanned<EnumVariant>),
    Tombstone(Spanned<Tombstone>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub annotations: Vec<Annotation>,
    pub name: Spanned<SmolStr>,
    pub ordinal: Spanned<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnumBacking {
    U8,
    U16,
    U32,
    U64,
}

// ---------------------------------------------------------------------------
// Flags
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct FlagsDecl {
    pub annotations: Vec<Annotation>,
    pub name: Spanned<SmolStr>,
    pub body: Vec<FlagsBodyItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FlagsBodyItem {
    Bit(Spanned<FlagsBit>),
    Tombstone(Spanned<Tombstone>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlagsBit {
    pub annotations: Vec<Annotation>,
    pub name: Spanned<SmolStr>,
    pub ordinal: Spanned<u32>,
}

// ---------------------------------------------------------------------------
// Union
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct UnionDecl {
    pub annotations: Vec<Annotation>,
    pub name: Spanned<SmolStr>,
    pub body: Vec<UnionBodyItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnionBodyItem {
    Variant(Spanned<UnionVariant>),
    Tombstone(Spanned<Tombstone>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnionVariant {
    pub annotations: Vec<Annotation>,
    pub name: Spanned<SmolStr>,
    pub ordinal: Spanned<u32>,
    pub fields: Vec<MessageBodyItem>,
}

// ---------------------------------------------------------------------------
// Newtype
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct NewtypeDecl {
    pub annotations: Vec<Annotation>,
    pub name: Spanned<SmolStr>,
    pub inner_type: Spanned<TypeExpr>,
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigDecl {
    pub annotations: Vec<Annotation>,
    pub name: Spanned<SmolStr>,
    pub fields: Vec<Spanned<ConfigField>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigField {
    pub annotations: Vec<Annotation>,
    pub name: Spanned<SmolStr>,
    pub ty: Spanned<TypeExpr>,
    pub default_value: Spanned<DefaultValue>,
}

// ---------------------------------------------------------------------------
// Type Alias
// ---------------------------------------------------------------------------

/// A type parameter for generic type aliases (e.g., `T` in `type Vec3<T> = ...`).
#[derive(Debug, Clone, PartialEq)]
pub struct TypeParam {
    pub name: Spanned<SmolStr>,
    /// Optional bounds on the type parameter (e.g., `T: Numeric`).
    /// Currently unused but reserved for future use.
    pub bounds: Vec<Spanned<SmolStr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AliasDecl {
    pub annotations: Vec<Annotation>,
    pub name: Spanned<SmolStr>,
    /// Type parameters for generic aliases (e.g., `<T>` in `type Vec3<T> = ...`).
    /// Empty for non-generic aliases.
    pub type_params: Vec<TypeParam>,
    pub target: Spanned<TypeExpr>,
}

// ---------------------------------------------------------------------------
// Const Declaration
// ---------------------------------------------------------------------------

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
    Float(f64),
    Hex(u64),
    Bool(bool),
    ConstRef(SmolStr),
    BinOp {
        op: BinOpKind,
        left: Box<ConstExpr>,
        right: Box<ConstExpr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOpKind {
    Add,
    Sub,
    Mul,
    Div,
}

// ---------------------------------------------------------------------------
// Type expressions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    Primitive(PrimitiveType),
    SubByte(SubByteType),
    Semantic(SemanticType),
    Named(SmolStr),
    /// namespace.Type
    Qualified(SmolStr, SmolStr),
    /// Generic type instantiation: Name<TypeArg>
    Generic(SmolStr, Box<Spanned<TypeExpr>>),
    Optional(Box<Spanned<TypeExpr>>),
    Array(Box<Spanned<TypeExpr>>),
    /// Fixed-size array with compile-time known length: `array<T, N>`
    FixedArray(Box<Spanned<TypeExpr>>, u64),
    Set(Box<Spanned<TypeExpr>>),
    Map(Box<Spanned<TypeExpr>>, Box<Spanned<TypeExpr>>),
    Result(Box<Spanned<TypeExpr>>, Box<Spanned<TypeExpr>>),
    /// Geometric types parameterized by element type
    Vec2(Box<Spanned<TypeExpr>>),
    Vec3(Box<Spanned<TypeExpr>>),
    Vec4(Box<Spanned<TypeExpr>>),
    Quat(Box<Spanned<TypeExpr>>),
    Mat3(Box<Spanned<TypeExpr>>),
    Mat4(Box<Spanned<TypeExpr>>),
    /// Inline bitfield: bits { name1, name2, ... }
    BitsInline(Vec<SmolStr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    Bool,
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    Fixed32,
    Fixed64,
    Void,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubByteType {
    pub signed: bool,
    pub bits: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticType {
    String,
    Bytes,
    Rgb,
    Uuid,
    Timestamp,
    Hash,
}

// ---------------------------------------------------------------------------
// Annotations
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    pub span: Span,
    pub name: Spanned<SmolStr>,
    pub args: Option<Vec<AnnotationArg>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationArg {
    pub span: Span,
    pub key: Option<Spanned<SmolStr>>,
    pub value: Spanned<AnnotationValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnnotationValue {
    Int(u64),
    Hex(u64),
    Str(String),
    Bool(bool),
    Ident(SmolStr),
    UpperIdent(SmolStr),
}

// ---------------------------------------------------------------------------
// Where Clause Constraints
// ---------------------------------------------------------------------------

/// A constraint expression attached to a field via `where`.
#[derive(Debug, Clone, PartialEq)]
pub enum WhereExpr {
    /// Binary logical AND: `a && b`
    And(Box<Spanned<WhereExpr>>, Box<Spanned<WhereExpr>>),
    /// Binary logical OR: `a || b`
    Or(Box<Spanned<WhereExpr>>, Box<Spanned<WhereExpr>>),
    /// Logical NOT: `!a`
    Not(Box<Spanned<WhereExpr>>),
    /// Comparison: `value == expr`, `value != expr`, etc.
    Cmp {
        op: CmpOp,
        operand: Box<Spanned<WhereOperand>>,
    },
    /// Range check: `value in low..high` or `value in low..<high`
    Range {
        low: Box<Spanned<WhereOperand>>,
        high: Box<Spanned<WhereOperand>>,
        exclusive_high: bool, // true for `..<`, false for `..`
    },
    /// Length check: `len(value)` compared to something
    LenCmp {
        op: CmpOp,
        operand: Box<Spanned<WhereOperand>>,
    },
    /// Length in range: `len(value) in low..high`
    LenRange {
        low: Box<Spanned<WhereOperand>>,
        high: Box<Spanned<WhereOperand>>,
        exclusive_high: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq, // ==
    Ne, // !=
    Lt, // <
    Gt, // >
    Le, // <=
    Ge, // >=
}

/// Operands in where expressions (the `value` keyword is implicit).
#[derive(Debug, Clone, PartialEq)]
pub enum WhereOperand {
    /// Integer literal
    Int(i64),
    /// Float literal
    Float(f64),
    /// String literal
    String(String),
    /// Boolean literal
    Bool(bool),
    /// The `value` keyword - refers to the field's value
    Value,
    /// Reference to a const: `MAX_SIZE`, etc.
    ConstRef(SmolStr),
}

// ---------------------------------------------------------------------------
// Default values (for config fields)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum DefaultValue {
    None,
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    Str(String),
    Ident(SmolStr),
    UpperIdent(SmolStr),
    Array(Vec<Spanned<DefaultValue>>),
}

// ---------------------------------------------------------------------------
// Tombstone
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct Tombstone {
    pub ordinal: Spanned<u32>,
    pub args: Vec<TombstoneArg>,
    pub original_type: Option<Spanned<TypeExpr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TombstoneArg {
    pub span: Span,
    pub key: Spanned<SmolStr>,
    pub value: Spanned<AnnotationValue>,
}
