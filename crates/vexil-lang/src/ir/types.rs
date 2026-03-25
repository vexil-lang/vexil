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
    pub fn new() -> Self {
        Self {
            types: Vec::new(),
            by_name: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: SmolStr, def: TypeDef) -> TypeId {
        let id = TypeId(self.types.len() as u32);
        self.types.push(Some(def));
        self.by_name.insert(name, id);
        id
    }

    pub fn register_stub(&mut self, name: SmolStr) -> TypeId {
        let id = TypeId(self.types.len() as u32);
        self.types.push(None);
        self.by_name.insert(name, id);
        id
    }

    pub fn lookup(&self, name: &str) -> Option<TypeId> {
        self.by_name.get(name).copied()
    }

    pub fn get(&self, id: TypeId) -> Option<&TypeDef> {
        self.types.get(id.0 as usize).and_then(|opt| opt.as_ref())
    }

    pub fn get_mut(&mut self, id: TypeId) -> Option<&mut TypeDef> {
        self.types
            .get_mut(id.0 as usize)
            .and_then(|opt| opt.as_mut())
    }

    pub fn is_stub(&self, id: TypeId) -> bool {
        self.types
            .get(id.0 as usize)
            .is_some_and(|opt| opt.is_none())
    }

    pub fn len(&self) -> usize {
        self.types.len()
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

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

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Encoding {
    Default,
    Varint,
    ZigZag,
    Delta(Box<Encoding>),
}

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
