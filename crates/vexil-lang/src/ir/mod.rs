//! # Stability: Tier 1
//!
//! Intermediate representation for compiled Vexil schemas.
//!
//! The IR is produced by the lowering pass and refined by the type checker.
//! All type references are resolved to [`TypeId`] handles into a [`TypeRegistry`],
//! and wire sizes are computed for fixed-layout types.

pub mod types;

pub use types::{
    DeprecatedInfo, Encoding, FieldEncoding, ResolvedAnnotations, ResolvedType, TombstoneDef,
    TypeId, TypeRegistry, WireSize,
};

use crate::ast::{DefaultValue, EnumBacking};
use crate::span::Span;
use smol_str::SmolStr;

/// A single-file compilation result.
///
/// Contains the type registry, the list of types declared in this file,
/// and the schema-level namespace and annotations. Imported types live in
/// the registry but are **not** listed in `declarations`.
#[derive(Debug, Clone)]
pub struct CompiledSchema {
    /// Namespace segments, e.g. `["net", "example", "types"]`.
    pub namespace: Vec<SmolStr>,
    /// Schema-level annotations (version, doc, etc.).
    pub annotations: ResolvedAnnotations,
    /// All type definitions reachable from this schema (declared + imported).
    pub registry: TypeRegistry,
    /// Type IDs of declarations **defined** in this file (excludes imports).
    pub declarations: Vec<TypeId>,
}

/// A type definition in the Vexil IR.
///
/// Each variant corresponds to one of the six declaration forms in the
/// Vexil language. Marked `#[non_exhaustive]` to allow future expansion.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum TypeDef {
    /// A message with ordered, typed fields.
    Message(MessageDef),
    /// A closed or open enumeration.
    Enum(EnumDef),
    /// A bitmask / flag set.
    Flags(FlagsDef),
    /// A tagged union (sum type).
    Union(UnionDef),
    /// A newtype wrapper around another type.
    Newtype(NewtypeDef),
    /// A compile-time configuration record (not encoded on the wire).
    Config(ConfigDef),
}

/// A message type definition with ordered, typed fields.
#[derive(Debug, Clone)]
pub struct MessageDef {
    pub name: SmolStr,
    pub span: Span,
    pub fields: Vec<FieldDef>,
    pub tombstones: Vec<TombstoneDef>,
    pub annotations: ResolvedAnnotations,
    pub wire_size: Option<WireSize>,
}

/// A single field within a message or union variant.
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

/// A single variant within an enum type.
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

/// A single bit definition within a flags type.
#[derive(Debug, Clone)]
pub struct FlagsBitDef {
    pub name: SmolStr,
    pub span: Span,
    pub bit: u32,
    pub annotations: ResolvedAnnotations,
}

/// A tagged union (sum type) definition.
#[derive(Debug, Clone)]
pub struct UnionDef {
    pub name: SmolStr,
    pub span: Span,
    pub variants: Vec<UnionVariantDef>,
    pub tombstones: Vec<TombstoneDef>,
    pub annotations: ResolvedAnnotations,
    pub wire_size: Option<WireSize>,
}

/// A single variant within a union type.
#[derive(Debug, Clone)]
pub struct UnionVariantDef {
    pub name: SmolStr,
    pub span: Span,
    pub ordinal: u32,
    pub fields: Vec<FieldDef>,
    pub tombstones: Vec<TombstoneDef>,
    pub annotations: ResolvedAnnotations,
}

/// A newtype wrapper around another type.
#[derive(Debug, Clone)]
pub struct NewtypeDef {
    pub name: SmolStr,
    pub span: Span,
    pub inner_type: ResolvedType,
    pub terminal_type: ResolvedType,
    pub annotations: ResolvedAnnotations,
}

/// A compile-time configuration record (not wire-encoded).
#[derive(Debug, Clone)]
pub struct ConfigDef {
    pub name: SmolStr,
    pub span: Span,
    pub fields: Vec<ConfigFieldDef>,
    pub annotations: ResolvedAnnotations,
}

/// A single field within a config record.
#[derive(Debug, Clone)]
pub struct ConfigFieldDef {
    pub name: SmolStr,
    pub span: Span,
    pub resolved_type: ResolvedType,
    pub default_value: DefaultValue,
    pub annotations: ResolvedAnnotations,
}
