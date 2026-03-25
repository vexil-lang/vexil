pub mod types;

pub use types::{
    Encoding, FieldEncoding, ResolvedAnnotations, ResolvedType, TombstoneDef, TypeId,
    TypeRegistry, WireSize,
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
