use crate::ast::{PrimitiveType, SemanticType, SubByteType};
use crate::span::Span;
use smol_str::SmolStr;
use std::collections::HashMap;

// Forward-declare TypeDef so TypeRegistry can reference it.
use super::TypeDef;

// ---------------------------------------------------------------------------
// TypeId + TypeRegistry
// ---------------------------------------------------------------------------

/// Opaque handle to a type definition in the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(pub(crate) u32);

impl TypeId {
    /// Returns the underlying registry index.
    pub fn index(self) -> u32 {
        self.0
    }
}

/// Sentinel for unresolvable types (poison value).
pub(crate) const POISON_TYPE_ID: TypeId = TypeId(u32::MAX);

/// Central type store. All cross-references use TypeId.
#[derive(Debug, Clone)]
pub struct TypeRegistry {
    types: Vec<Option<TypeDef>>,
    by_name: HashMap<SmolStr, TypeId>,
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeRegistry {
    /// Create an empty type registry.
    pub fn new() -> Self {
        Self {
            types: Vec::new(),
            by_name: HashMap::new(),
        }
    }

    /// Register a complete type definition, returning its [`TypeId`].
    pub fn register(&mut self, name: SmolStr, def: TypeDef) -> TypeId {
        let id = TypeId(self.types.len() as u32);
        self.types.push(Some(def));
        self.by_name.insert(name, id);
        id
    }

    /// Register a stub (forward declaration) that will be filled later via [`fill_stub`](Self::fill_stub).
    pub fn register_stub(&mut self, name: SmolStr) -> TypeId {
        let id = TypeId(self.types.len() as u32);
        self.types.push(None);
        self.by_name.insert(name, id);
        id
    }

    /// Look up a type by name, returning its [`TypeId`] if registered.
    pub fn lookup(&self, name: &str) -> Option<TypeId> {
        self.by_name.get(name).copied()
    }

    /// Get a reference to the type definition for `id`, if it exists and is not a stub.
    pub fn get(&self, id: TypeId) -> Option<&TypeDef> {
        self.types.get(id.0 as usize).and_then(|opt| opt.as_ref())
    }

    /// Get a mutable reference to the type definition for `id`.
    pub fn get_mut(&mut self, id: TypeId) -> Option<&mut TypeDef> {
        self.types
            .get_mut(id.0 as usize)
            .and_then(|opt| opt.as_mut())
    }

    /// Returns `true` if `id` is a registered stub that has not yet been filled.
    pub fn is_stub(&self, id: TypeId) -> bool {
        self.types
            .get(id.0 as usize)
            .is_some_and(|opt| opt.is_none())
    }

    /// Returns the total number of slots (filled + stubs) in the registry.
    pub fn len(&self) -> usize {
        self.types.len()
    }

    /// Returns `true` if the registry contains no types.
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    /// Rename a type in the by-name index (used for aliased import qualification).
    pub fn rename(&mut self, id: TypeId, old_name: &str, new_name: SmolStr) {
        self.by_name.remove(old_name);
        self.by_name.insert(new_name, id);
    }

    /// Fill a stub slot with a real type definition.
    pub fn fill_stub(&mut self, id: TypeId, def: TypeDef) {
        let idx = id.0 as usize;
        if idx < self.types.len() {
            self.types[idx] = Some(def);
        }
    }

    /// Iterate over all filled type definitions and their IDs.
    pub fn iter(&self) -> impl Iterator<Item = (TypeId, &TypeDef)> {
        self.types
            .iter()
            .enumerate()
            .filter_map(|(i, opt)| opt.as_ref().map(|def| (TypeId(i as u32), def)))
    }
}

// ---------------------------------------------------------------------------
// ResolvedType
// ---------------------------------------------------------------------------

/// A fully resolved type reference in the IR.
///
/// All named types have been resolved to [`TypeId`] handles. Container types
/// (optional, array, map, result) wrap their inner types recursively.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
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

/// Wire encoding strategy for a field or type.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Encoding {
    Default,
    Varint,
    ZigZag,
    Delta(Box<Encoding>),
}

/// Per-field encoding configuration (encoding strategy and optional element limit).
#[derive(Debug, Clone, PartialEq)]
pub struct FieldEncoding {
    pub encoding: Encoding,
    pub limit: Option<u64>,
}

impl FieldEncoding {
    /// Create a default field encoding (no varint, no limit).
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

/// The computed wire size of a type, in bits.
///
/// Fixed-size types have a known bit count. Variable-size types (containing
/// arrays, optionals, or varints) have a minimum and optional maximum.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum WireSize {
    Fixed(u64),
    Variable {
        min_bits: u64,
        max_bits: Option<u64>,
    },
}

// ---------------------------------------------------------------------------
// ResolvedAnnotations
// ---------------------------------------------------------------------------

/// Information attached to a `@deprecated` annotation.
#[derive(Debug, Clone, PartialEq)]
pub struct DeprecatedInfo {
    pub reason: SmolStr,
    pub since: Option<SmolStr>,
}

/// Annotations resolved from source and available on any IR node.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResolvedAnnotations {
    pub deprecated: Option<DeprecatedInfo>,
    pub since: Option<SmolStr>,
    pub doc: Vec<SmolStr>,
    pub revision: Option<u64>,
    pub non_exhaustive: bool,
    pub version: Option<SmolStr>,
}

// ---------------------------------------------------------------------------
// TombstoneDef
// ---------------------------------------------------------------------------

/// A tombstoned (removed) field or variant ordinal.
#[derive(Debug, Clone, PartialEq)]
pub struct TombstoneDef {
    pub span: Span,
    pub ordinal: u32,
    pub reason: SmolStr,
    pub since: Option<SmolStr>,
}
