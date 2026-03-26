//! # Stability: Tier 2
//!
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
    Optional(Box<Spanned<TypeExpr>>),
    Array(Box<Spanned<TypeExpr>>),
    Map(Box<Spanned<TypeExpr>>, Box<Spanned<TypeExpr>>),
    Result(Box<Spanned<TypeExpr>>, Box<Spanned<TypeExpr>>),
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct TombstoneArg {
    pub span: Span,
    pub key: Spanned<SmolStr>,
    pub value: Spanned<AnnotationValue>,
}
