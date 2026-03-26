//! # Stability: Tier 1
//!
pub mod types;

pub use types::{
    DeprecatedInfo, Encoding, FieldEncoding, ResolvedAnnotations, ResolvedType, TombstoneDef,
    TypeId, TypeRegistry, WireSize,
};

use crate::ast::{DefaultValue, EnumBacking};
use crate::span::Span;
use smol_str::SmolStr;

#[derive(Debug, Clone)]
pub struct CompiledSchema {
    pub namespace: Vec<SmolStr>,
    pub annotations: ResolvedAnnotations,
    pub registry: TypeRegistry,
    pub declarations: Vec<TypeId>,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum TypeDef {
    Message(MessageDef),
    Enum(EnumDef),
    Flags(FlagsDef),
    Union(UnionDef),
    Newtype(NewtypeDef),
    Config(ConfigDef),
}

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

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: SmolStr,
    pub span: Span,
    /// Explicit backing type specified by the user (`: u8`, `: u16`, etc.).
    /// `None` means no explicit backing — `wire_bits` is auto-computed by typeck.
    pub backing: Option<EnumBacking>,
    pub variants: Vec<EnumVariantDef>,
    pub tombstones: Vec<TombstoneDef>,
    pub annotations: ResolvedAnnotations,
    /// Computed by typeck: number of bits used on the wire.
    /// For explicit backing this equals the backing type width;
    /// for auto-sized enums this is the minimal bit width for the variant count.
    pub wire_bits: u8,
}

#[derive(Debug, Clone)]
pub struct EnumVariantDef {
    pub name: SmolStr,
    pub span: Span,
    pub ordinal: u32,
    pub annotations: ResolvedAnnotations,
}

#[derive(Debug, Clone)]
pub struct FlagsDef {
    pub name: SmolStr,
    pub span: Span,
    pub bits: Vec<FlagsBitDef>,
    pub tombstones: Vec<TombstoneDef>,
    pub annotations: ResolvedAnnotations,
    /// Computed by typeck: number of bytes used on the wire (1, 2, 4, or 8).
    pub wire_bytes: u8,
}

#[derive(Debug, Clone)]
pub struct FlagsBitDef {
    pub name: SmolStr,
    pub span: Span,
    pub bit: u32,
    pub annotations: ResolvedAnnotations,
}

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

#[derive(Debug, Clone)]
pub struct NewtypeDef {
    pub name: SmolStr,
    pub span: Span,
    pub inner_type: ResolvedType,
    pub terminal_type: ResolvedType,
    pub annotations: ResolvedAnnotations,
}

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
