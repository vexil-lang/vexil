//! # Stability: Tier 1
//!
//! Intermediate representation for compiled Vexil schemas.
//!
//! The IR is produced by the lowering pass and refined by the type checker.
//! All type references are resolved to [`TypeId`] handles into a [`TypeRegistry`],
//! and wire sizes are computed for fixed-layout types.

pub mod types;

pub use types::{
    CustomAnnotation, CustomAnnotationArg, CustomAnnotationValue, DeprecatedInfo, Encoding,
    FieldEncoding, ResolvedAnnotations, ResolvedType, TombstoneDef, TypeId, TypeRegistry, WireSize,
};

use crate::ast::{DefaultValue, EnumBacking};
use crate::span::Span;
use smol_str::SmolStr;
use std::collections::HashMap;

/// Constraint expression for field validation.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldConstraint {
    /// Binary logical AND
    And(Box<FieldConstraint>, Box<FieldConstraint>),
    /// Binary logical OR
    Or(Box<FieldConstraint>, Box<FieldConstraint>),
    /// Logical NOT
    Not(Box<FieldConstraint>),
    /// Comparison: value op operand
    Cmp {
        op: CmpOp,
        operand: ConstraintOperand,
    },
    /// Range check: value in [low, high) or [low, high]
    Range {
        low: ConstraintOperand,
        high: ConstraintOperand,
        exclusive_high: bool,
    },
    /// Length comparison: len(value) op operand
    LenCmp {
        op: CmpOp,
        operand: ConstraintOperand,
    },
    /// Length range: len(value) in range
    LenRange {
        low: ConstraintOperand,
        high: ConstraintOperand,
        exclusive_high: bool,
    },
}

/// Comparison operators in IR constraint expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    /// `==` — equality.
    Eq,
    /// `!=` — inequality.
    Ne,
    /// `<` — less than.
    Lt,
    /// `>` — greater than.
    Gt,
    /// `<=` — less than or equal.
    Le,
    /// `>=` — greater than or equal.
    Ge,
}

/// Operands in constraint expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintOperand {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    ConstRef(SmolStr),
}

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
    /// Evaluated constant values (const name -> value).
    pub constants: HashMap<SmolStr, ConstValue>,
}

// Compile-time assertion: CompiledSchema must be Send + Sync for
// potential future parallel compilation and cross-thread sharing.
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<CompiledSchema>();
};

impl CompiledSchema {
    /// Returns all type names declared in this schema (not imports).
    pub fn type_names(&self) -> Vec<&str> {
        self.declarations
            .iter()
            .filter_map(|&id| self.registry.get(id))
            .map(|def| match def {
                TypeDef::Message(m) => m.name.as_str(),
                TypeDef::Enum(e) => e.name.as_str(),
                TypeDef::Flags(f) => f.name.as_str(),
                TypeDef::Union(u) => u.name.as_str(),
                TypeDef::Newtype(n) => n.name.as_str(),
                TypeDef::Config(c) => c.name.as_str(),
                TypeDef::GenericAlias(g) => g.name.as_str(),
            })
            .collect()
    }

    /// Look up a type by name. Returns the TypeId and TypeDef if found.
    pub fn find_type(&self, name: &str) -> Option<(TypeId, &TypeDef)> {
        for &id in &self.declarations {
            if let Some(def) = self.registry.get(id) {
                let def_name = match def {
                    TypeDef::Message(m) => m.name.as_str(),
                    TypeDef::Enum(e) => e.name.as_str(),
                    TypeDef::Flags(f) => f.name.as_str(),
                    TypeDef::Union(u) => u.name.as_str(),
                    TypeDef::Newtype(n) => n.name.as_str(),
                    TypeDef::Config(c) => c.name.as_str(),
                    TypeDef::GenericAlias(g) => g.name.as_str(),
                };
                if def_name == name {
                    return Some((id, def));
                }
            }
        }
        None
    }

    /// Returns the fully-qualified namespace as a dot-separated string.
    pub fn namespace_str(&self) -> String {
        self.namespace
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(".")
    }

    /// Returns the BLAKE3 schema hash as a hex string.
    pub fn hash_hex(&self) -> String {
        let hash = crate::canonical::schema_hash(self);
        hash.iter().map(|b| format!("{b:02x}")).collect()
    }
}

/// A compiled constant value.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstValue {
    /// The constant's resolved type.
    pub ty: ResolvedType,
    /// The evaluated value (stored as i64 for all integral types).
    pub value: i64,
    /// Source span for error reporting.
    pub span: Span,
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
    /// A generic type alias with type parameters.
    /// Stores the alias definition with type parameters and target type expression.
    GenericAlias(GenericAliasDef),
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
    pub constraint: Option<FieldConstraint>,
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

/// A generic type alias definition.
///
/// Generic aliases are stored with their type parameters and target type.
/// When a generic alias is used with type arguments (e.g., `Vec3<fixed64>`),
/// the type arguments are substituted into the target type to produce the
/// final resolved type.
#[derive(Debug, Clone)]
pub struct GenericAliasDef {
    pub name: SmolStr,
    pub span: Span,
    /// Type parameter names (e.g., `["T"]` for `type Vec3<T> = ...`).
    pub type_params: Vec<SmolStr>,
    /// The target type expression with unresolved type parameters.
    /// Type arguments are substituted into this to produce the resolved type.
    pub target_type: crate::ast::TypeExpr,
    pub annotations: ResolvedAnnotations,
}
