//! # Stability: Tier 2
//!
//! Type checker: validates the IR and computes wire sizes.
//!
//! Detects infinite recursion, verifies encoding annotations match their
//! target types, and fills in `wire_size` / `wire_bits` / `wire_bytes`
//! fields on messages, enums, flags, and unions.

use std::collections::HashSet;

use crate::ast::{EnumBacking, PrimitiveType, SemanticType};
use crate::diagnostic::{Diagnostic, ErrorClass};
use crate::ir::{CompiledSchema, Encoding, FieldEncoding, ResolvedType, TypeDef, TypeId, WireSize};

/// Type-check and compute wire sizes. Mutates the schema to fill in wire_size fields.
pub fn check(compiled: &mut CompiledSchema) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    // Check recursive types.
    check_recursion(compiled, &mut diags);

    let decl_ids: Vec<TypeId> = compiled.declarations.clone();

    // Pass 1: compute wire_bits for enums and wire_bytes for flags.
    // These must be set before message/union wire sizes are computed so that
    // named_type_wire_size can read the correct values for enum/flags fields.
    for &id in &decl_ids {
        match compiled.registry.get(id) {
            Some(TypeDef::Enum(en)) => {
                let wire_bits = compute_enum_wire_bits(en);
                if let Some(TypeDef::Enum(en)) = compiled.registry.get_mut(id) {
                    en.wire_bits = wire_bits;
                }
            }
            Some(TypeDef::Flags(fl)) => {
                let wire_bytes = compute_flags_wire_bytes(fl);
                if let Some(TypeDef::Flags(fl)) = compiled.registry.get_mut(id) {
                    fl.wire_bytes = wire_bytes;
                }
            }
            _ => {}
        }
    }

    // Pass 2: compute wire sizes for messages and unions.
    for &id in &decl_ids {
        if let Some(def) = compiled.registry.get(id) {
            match def {
                TypeDef::Message(_) => {
                    let mut computing = HashSet::new();
                    let ws = compute_message_wire_size(id, compiled, &mut computing);
                    if let Some(TypeDef::Message(msg)) = compiled.registry.get_mut(id) {
                        msg.wire_size = Some(ws);
                    }
                }
                TypeDef::Union(_) => {
                    let mut computing = HashSet::new();
                    let ws = compute_union_wire_size(id, compiled, &mut computing);
                    if let Some(TypeDef::Union(un)) = compiled.registry.get_mut(id) {
                        un.wire_size = Some(ws);
                    }
                }
                _ => {}
            }
        }
    }

    // Check trait conformance and resolve trait method calls
    check_impl_conformance(compiled, &mut diags);
    resolve_trait_calls(compiled, &mut diags);

    diags
}

// ---------------------------------------------------------------------------
// Wire size computation
// ---------------------------------------------------------------------------

/// Sentinel returned when we detect a cycle mid-computation (valid recursive
/// types with indirection are always variable-length and unbounded).
fn cycle_wire_size() -> WireSize {
    WireSize::Variable {
        min_bits: 0,
        max_bits: None,
    }
}

fn compute_type_wire_size(
    ty: &ResolvedType,
    enc: &FieldEncoding,
    compiled: &CompiledSchema,
    computing: &mut HashSet<TypeId>,
) -> WireSize {
    match &enc.encoding {
        Encoding::Varint => varint_wire_size(ty),
        Encoding::ZigZag => zigzag_wire_size(ty),
        Encoding::Delta(inner) => {
            let inner_enc = FieldEncoding {
                encoding: *inner.clone(),
                limit: enc.limit,
            };
            compute_type_wire_size(ty, &inner_enc, compiled, computing)
        }
        Encoding::Default => compute_resolved_type_wire_size(ty, compiled, computing),
    }
}

fn compute_resolved_type_wire_size(
    ty: &ResolvedType,
    compiled: &CompiledSchema,
    computing: &mut HashSet<TypeId>,
) -> WireSize {
    match ty {
        ResolvedType::Primitive(p) => primitive_wire_size(p),
        ResolvedType::SubByte(s) => WireSize::Fixed(s.bits as u64),
        ResolvedType::Semantic(s) => semantic_wire_size(s),
        ResolvedType::Named(id) => named_type_wire_size(*id, compiled, computing),
        ResolvedType::Optional(inner) => {
            // Optional is an indirection point — contents don't recurse directly.
            // We still need the inner size for the max bound, but if it cycles we
            // just use None (unbounded).
            let inner_ws = compute_resolved_type_wire_size(inner, compiled, computing);
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
            min_bits: 8,
            max_bits: None,
        },
        ResolvedType::FixedArray(inner, size) => {
            let inner_ws = compute_resolved_type_wire_size(inner, compiled, computing);
            match inner_ws {
                WireSize::Fixed(bits) => WireSize::Fixed(bits * size),
                WireSize::Variable { min_bits, .. } => WireSize::Variable {
                    min_bits: min_bits * size,
                    max_bits: None,
                },
            }
        }
        ResolvedType::Map(_, _) | ResolvedType::Set(_) => WireSize::Variable {
            min_bits: 8,
            max_bits: None,
        },
        ResolvedType::Result(ok, err) => {
            let ok_ws = compute_resolved_type_wire_size(ok, compiled, computing);
            let err_ws = compute_resolved_type_wire_size(err, compiled, computing);
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
        ResolvedType::Vec2(inner) => {
            let inner_ws = compute_resolved_type_wire_size(inner, compiled, computing);
            multiply_wire_size(&inner_ws, 2)
        }
        ResolvedType::Vec3(inner) => {
            let inner_ws = compute_resolved_type_wire_size(inner, compiled, computing);
            multiply_wire_size(&inner_ws, 3)
        }
        ResolvedType::Vec4(inner) => {
            let inner_ws = compute_resolved_type_wire_size(inner, compiled, computing);
            multiply_wire_size(&inner_ws, 4)
        }
        ResolvedType::Quat(inner) => {
            let inner_ws = compute_resolved_type_wire_size(inner, compiled, computing);
            multiply_wire_size(&inner_ws, 4)
        }
        ResolvedType::Mat3(inner) => {
            let inner_ws = compute_resolved_type_wire_size(inner, compiled, computing);
            multiply_wire_size(&inner_ws, 9)
        }
        ResolvedType::Mat4(inner) => {
            let inner_ws = compute_resolved_type_wire_size(inner, compiled, computing);
            multiply_wire_size(&inner_ws, 16)
        }
        ResolvedType::BitsInline(names) => WireSize::Fixed(names.len() as u64),
    }
}

fn primitive_wire_size(p: &PrimitiveType) -> WireSize {
    let bits = match p {
        PrimitiveType::Bool => 1,
        PrimitiveType::U8 | PrimitiveType::I8 => 8,
        PrimitiveType::U16 | PrimitiveType::I16 => 16,
        PrimitiveType::U32 | PrimitiveType::I32 | PrimitiveType::F32 | PrimitiveType::Fixed32 => 32,
        PrimitiveType::U64 | PrimitiveType::I64 | PrimitiveType::F64 | PrimitiveType::Fixed64 => 64,
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
    let max_bits = match ty {
        ResolvedType::Primitive(PrimitiveType::U16) => 24,
        ResolvedType::Primitive(PrimitiveType::U32) => 40,
        ResolvedType::Primitive(PrimitiveType::U64) => 80,
        _ => 80,
    };
    WireSize::Variable {
        min_bits: 8,
        max_bits: Some(max_bits),
    }
}

fn zigzag_wire_size(ty: &ResolvedType) -> WireSize {
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

fn named_type_wire_size(
    id: TypeId,
    compiled: &CompiledSchema,
    computing: &mut HashSet<TypeId>,
) -> WireSize {
    // If we're already computing this type's wire size, we've hit a cycle.
    // Return a sentinel — the type is recursive via indirection so it's unbounded.
    if computing.contains(&id) {
        return cycle_wire_size();
    }

    match compiled.registry.get(id) {
        Some(TypeDef::Enum(en)) => {
            // wire_bits is computed in pass 1 before message wire sizes (pass 2),
            // so it is always set by the time we reach here.
            WireSize::Fixed(u64::from(en.wire_bits))
        }
        Some(TypeDef::Flags(fl)) => {
            // wire_bytes is computed in pass 1 before message wire sizes (pass 2).
            WireSize::Fixed(u64::from(fl.wire_bytes) * 8)
        }
        Some(TypeDef::Newtype(nt)) => {
            let terminal = nt.terminal_type.clone();
            compute_resolved_type_wire_size(&terminal, compiled, computing)
        }
        Some(TypeDef::Message(msg)) => {
            if let Some(ws) = msg.wire_size.clone() {
                return ws;
            }
            compute_message_wire_size(id, compiled, computing)
        }
        Some(TypeDef::Union(un)) => {
            if let Some(ws) = un.wire_size.clone() {
                return ws;
            }
            compute_union_wire_size(id, compiled, computing)
        }
        Some(TypeDef::Config(_))
        | Some(TypeDef::GenericAlias(_))
        | Some(TypeDef::Trait(_))
        | Some(TypeDef::Impl(_))
        | None => WireSize::Variable {
            min_bits: 0,
            max_bits: None,
        },
    }
}

fn compute_message_wire_size(
    id: TypeId,
    compiled: &CompiledSchema,
    computing: &mut HashSet<TypeId>,
) -> WireSize {
    let msg = match compiled.registry.get(id) {
        Some(TypeDef::Message(m)) => m,
        _ => return WireSize::Fixed(0),
    };

    if msg.fields.is_empty() {
        return WireSize::Fixed(0);
    }

    // Clone fields to avoid borrow conflicts when we call back into compiled.
    let fields: Vec<(ResolvedType, FieldEncoding)> = msg
        .fields
        .iter()
        .map(|f| (f.resolved_type.clone(), f.encoding.clone()))
        .collect();

    computing.insert(id);

    let mut total_min: u64 = 0;
    let mut total_max: Option<u64> = Some(0);
    let mut is_variable = false;

    for (resolved_type, encoding) in &fields {
        let ws = compute_type_wire_size(resolved_type, encoding, compiled, computing);
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

    computing.remove(&id);

    if is_variable {
        WireSize::Variable {
            min_bits: total_min,
            max_bits: total_max,
        }
    } else {
        WireSize::Fixed(total_min)
    }
}

fn compute_union_wire_size(
    id: TypeId,
    compiled: &CompiledSchema,
    computing: &mut HashSet<TypeId>,
) -> WireSize {
    let un = match compiled.registry.get(id) {
        Some(TypeDef::Union(u)) => u,
        _ => return WireSize::Fixed(0),
    };

    if un.variants.is_empty() {
        return WireSize::Variable {
            min_bits: 8,
            max_bits: Some(8),
        };
    }

    let tag_min: u64 = 8;
    let mut max_variant_bits: Option<u64> = Some(0);
    let mut min_variant_bits: u64 = u64::MAX;

    // Clone variant fields to avoid borrow conflicts.
    let variants: Vec<Vec<(ResolvedType, FieldEncoding)>> = un
        .variants
        .iter()
        .map(|v| {
            v.fields
                .iter()
                .map(|f| (f.resolved_type.clone(), f.encoding.clone()))
                .collect()
        })
        .collect();

    computing.insert(id);

    for variant_fields in &variants {
        let mut var_min: u64 = 0;
        let mut var_max: Option<u64> = Some(0);

        for (resolved_type, encoding) in variant_fields {
            let ws = compute_type_wire_size(resolved_type, encoding, compiled, computing);
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

    computing.remove(&id);

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

/// Multiply a wire size by a constant factor (for geometric types).
fn multiply_wire_size(ws: &WireSize, multiplier: u64) -> WireSize {
    match ws {
        WireSize::Fixed(bits) => WireSize::Fixed(bits * multiplier),
        WireSize::Variable { min_bits, max_bits } => WireSize::Variable {
            min_bits: min_bits * multiplier,
            max_bits: max_bits.map(|m| m * multiplier),
        },
    }
}

// ---------------------------------------------------------------------------
// Recursive type detection
// ---------------------------------------------------------------------------

/// DFS state for the recursive type check.
struct RecursionState<'a> {
    /// TypeIds on the current path reached without passing through an
    /// indirection point (Optional, Array, Map, Result, Union).
    /// A cycle here is infinite recursion.
    direct_path: HashSet<TypeId>,
    /// TypeIds we have already fully explored — prevents re-entering
    /// already-finished subtrees and infinite loops through mutual recursion.
    visited: HashSet<TypeId>,
    compiled: &'a CompiledSchema,
    origin_span: crate::span::Span,
    diags: &'a mut Vec<Diagnostic>,
}

/// For each message type, DFS through field types to detect direct infinite cycles.
fn check_recursion(compiled: &CompiledSchema, diags: &mut Vec<Diagnostic>) {
    for &id in &compiled.declarations {
        if let Some(TypeDef::Message(msg)) = compiled.registry.get(id) {
            let mut state = RecursionState {
                direct_path: {
                    let mut s = HashSet::new();
                    s.insert(id);
                    s
                },
                visited: HashSet::new(),
                compiled,
                origin_span: msg.span,
                diags,
            };
            let fields: Vec<(ResolvedType, FieldEncoding)> = msg
                .fields
                .iter()
                .map(|f| (f.resolved_type.clone(), f.encoding.clone()))
                .collect();
            for (ty, _) in &fields {
                walk_type_for_recursion(ty, true, &mut state);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// wire_bits / wire_bytes helpers
// ---------------------------------------------------------------------------

fn compute_enum_wire_bits(en: &crate::ir::EnumDef) -> u8 {
    if let Some(backing) = &en.backing {
        return match backing {
            EnumBacking::U8 => 8,
            EnumBacking::U16 => 16,
            EnumBacking::U32 => 32,
            EnumBacking::U64 => 64,
        };
    }
    let max_ordinal = en.variants.iter().map(|v| v.ordinal).max().unwrap_or(0);
    let min_bits: u8 = if max_ordinal == 0 {
        1
    } else {
        let n = u64::from(max_ordinal) + 1;
        // Number of bits needed to represent n distinct values: ceil(log2(n))
        // = bit_width(n - 1) = 64 - leading_zeros(n - 1), clamped to at least 1.
        let leading = (n - 1).leading_zeros();
        let bits = 64u8.saturating_sub(leading as u8);
        std::cmp::max(bits, 1)
    };
    if en.annotations.non_exhaustive {
        std::cmp::max(min_bits, 8)
    } else {
        std::cmp::max(min_bits, 1)
    }
}

fn compute_flags_wire_bytes(fl: &crate::ir::FlagsDef) -> u8 {
    let max_bit = fl.bits.iter().map(|b| b.bit).max().unwrap_or(0);
    match max_bit {
        0..=7 => 1,
        8..=15 => 2,
        16..=31 => 4,
        _ => 8,
    }
}

fn walk_type_for_recursion(ty: &ResolvedType, direct: bool, state: &mut RecursionState<'_>) {
    match ty {
        ResolvedType::Named(id) => {
            // Direct cycle = infinite recursion.
            if direct && state.direct_path.contains(id) {
                state.diags.push(Diagnostic::error(
                    state.origin_span,
                    ErrorClass::RecursiveTypeInfinite,
                    "type contains infinite direct recursion",
                ));
                return;
            }

            // Indirect back-reference to a node on the direct path (e.g. via
            // array/optional) — this is valid recursion with indirection.
            if !direct && state.direct_path.contains(id) {
                return;
            }

            // Already fully explored this node — no need to descend again.
            if state.visited.contains(id) {
                return;
            }

            state.visited.insert(*id);

            match state.compiled.registry.get(*id) {
                Some(TypeDef::Message(msg)) => {
                    let was_new = if direct {
                        state.direct_path.insert(*id)
                    } else {
                        false
                    };
                    let fields: Vec<(ResolvedType, FieldEncoding)> = msg
                        .fields
                        .iter()
                        .map(|f| (f.resolved_type.clone(), f.encoding.clone()))
                        .collect();
                    for (field_ty, _) in &fields {
                        walk_type_for_recursion(field_ty, direct, state);
                    }
                    if was_new {
                        state.direct_path.remove(id);
                    }
                }
                Some(TypeDef::Union(un)) => {
                    // Union dispatch = indirection point.
                    let variant_fields: Vec<Vec<ResolvedType>> = un
                        .variants
                        .iter()
                        .map(|v| v.fields.iter().map(|f| f.resolved_type.clone()).collect())
                        .collect();
                    for fields in &variant_fields {
                        for field_ty in fields {
                            walk_type_for_recursion(field_ty, false, state);
                        }
                    }
                }
                Some(TypeDef::Newtype(nt)) => {
                    let inner = nt.inner_type.clone();
                    walk_type_for_recursion(&inner, direct, state);
                }
                _ => {} // Enum, Flags, Config, stub — terminal
            }
        }
        ResolvedType::Optional(inner) | ResolvedType::Array(inner) => {
            walk_type_for_recursion(inner, false, state);
        }
        ResolvedType::FixedArray(inner, _) => {
            walk_type_for_recursion(inner, false, state);
        }
        ResolvedType::Map(k, v) => {
            walk_type_for_recursion(k, false, state);
            walk_type_for_recursion(v, false, state);
        }
        ResolvedType::Set(inner) => {
            walk_type_for_recursion(inner, false, state);
        }
        ResolvedType::Result(ok, err) => {
            walk_type_for_recursion(ok, false, state);
            walk_type_for_recursion(err, false, state);
        }
        ResolvedType::Vec2(inner)
        | ResolvedType::Vec3(inner)
        | ResolvedType::Vec4(inner)
        | ResolvedType::Quat(inner)
        | ResolvedType::Mat3(inner)
        | ResolvedType::Mat4(inner) => {
            walk_type_for_recursion(inner, false, state);
        }
        _ => {} // Primitive, SubByte, Semantic — terminal
    }
}

// ---------------------------------------------------------------------------
// Trait Conformance Checking
// ---------------------------------------------------------------------------

use smol_str::SmolStr;
use std::collections::HashMap;

use crate::ir::{ImplDef, TraitDef};

/// Check that all impls in the schema conform to their traits.
fn check_impl_conformance(compiled: &CompiledSchema, diags: &mut Vec<Diagnostic>) {
    // Collect all traits and impls
    let mut traits: HashMap<SmolStr, &TraitDef> = HashMap::new();
    let mut impls: Vec<&ImplDef> = Vec::new();

    for &id in &compiled.declarations {
        if let Some(type_def) = compiled.registry.get(id) {
            match type_def {
                TypeDef::Trait(t) => {
                    traits.insert(t.name.clone(), t);
                }
                TypeDef::Impl(i) => {
                    impls.push(i);
                }
                _ => {}
            }
        }
    }

    // Also check impls that are not in declarations (collected separately)
    for (_id, type_def) in compiled.registry.iter() {
        if let TypeDef::Impl(i) = type_def {
            // Check if we already have this impl
            if !impls.iter().any(|existing| std::ptr::eq(*existing, i)) {
                impls.push(i);
            }
        }
    }

    // Check each impl
    for impl_def in impls {
        check_single_impl_conformance(impl_def, &traits, compiled, diags);
    }
}

fn check_single_impl_conformance(
    impl_def: &ImplDef,
    traits: &HashMap<SmolStr, &TraitDef>,
    compiled: &CompiledSchema,
    diags: &mut Vec<Diagnostic>,
) {
    let Some(trait_def) = traits.get(&impl_def.trait_name) else {
        diags.push(Diagnostic::error(
            impl_def.span,
            ErrorClass::UnresolvedType,
            format!("impl references unknown trait '{}'", impl_def.trait_name),
        ));
        return;
    };

    if impl_def.type_args.len() != trait_def.type_params.len() {
        diags.push(Diagnostic::error(
            impl_def.span,
            ErrorClass::UnresolvedType,
            format!(
                "trait '{}' has {} type parameters but impl provides {}",
                impl_def.trait_name,
                trait_def.type_params.len(),
                impl_def.type_args.len()
            ),
        ));
    }

    // Check target type has all required trait fields
    check_trait_fields(impl_def, trait_def, compiled, diags);

    // Check all trait functions are implemented
    check_trait_functions(impl_def, trait_def, diags);
}

fn check_trait_fields(
    impl_def: &ImplDef,
    trait_def: &TraitDef,
    compiled: &CompiledSchema,
    diags: &mut Vec<Diagnostic>,
) {
    // Get the target type definition
    let target_type_def = match &impl_def.target_type {
        ResolvedType::Named(id) => compiled.registry.get(*id),
        _ => None,
    };

    let Some(target_def) = target_type_def else {
        // Type validation happens elsewhere
        return;
    };

    let target_fields = match target_def {
        TypeDef::Message(m) => &m.fields,
        _ => {
            diags.push(Diagnostic::error(
                impl_def.span,
                ErrorClass::UnresolvedType,
                format!(
                    "impl target '{:?}' is not a message type",
                    impl_def.target_type
                ),
            ));
            return;
        }
    };

    // Check each required trait field exists on target
    for trait_field in &trait_def.fields {
        let found = target_fields.iter().any(|f| {
            f.name == trait_field.name && types_compatible(&f.resolved_type, &trait_field.ty)
        });

        if !found {
            diags.push(Diagnostic::error(
                impl_def.span,
                ErrorClass::UnresolvedType,
                format!(
                    "impl for '{:?}' missing required trait field '{}' of type '{:?}'",
                    impl_def.target_type, trait_field.name, trait_field.ty
                ),
            ));
        }
    }
}

fn check_trait_functions(impl_def: &ImplDef, trait_def: &TraitDef, diags: &mut Vec<Diagnostic>) {
    // Check all trait functions are implemented
    for trait_fn in &trait_def.functions {
        let found = impl_def.functions.iter().any(|f| {
            f.name == trait_fn.name
                && f.params.len() == trait_fn.params.len()
                && f.return_type == trait_fn.return_type
        });

        if !found {
            diags.push(Diagnostic::error(
                impl_def.span,
                ErrorClass::UnresolvedType,
                format!(
                    "impl for '{:?}' missing trait function '{}'",
                    impl_def.target_type, trait_fn.name
                ),
            ));
        }
    }

    for impl_fn in &impl_def.functions {
        let found = trait_def.functions.iter().any(|f| f.name == impl_fn.name);
        if !found {
            diags.push(Diagnostic::error(
                impl_def.span,
                ErrorClass::UnresolvedType,
                format!(
                    "impl for '{:?}' has extra function '{}' not in trait '{}'",
                    impl_def.target_type, impl_fn.name, trait_def.name
                ),
            ));
        }
    }
}

fn types_compatible(a: &ResolvedType, b: &ResolvedType) -> bool {
    if a == b {
        return true;
    }

    match (a, b) {
        (ResolvedType::Named(id_a), ResolvedType::Named(id_b)) => id_a == id_b,
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Static Trait Dispatch (Monomorphization)
// ---------------------------------------------------------------------------

use crate::ir::{Expr, FnBody, Statement};

/// Resolve trait method calls to specific impl functions.
/// This implements static dispatch by replacing TraitMethodCall with direct Call.
fn resolve_trait_calls(compiled: &CompiledSchema, diags: &mut Vec<Diagnostic>) {
    // Collect all traits and impls for lookup
    let mut traits: HashMap<SmolStr, &TraitDef> = HashMap::new();
    let mut impls: Vec<&ImplDef> = Vec::new();

    for &id in &compiled.declarations {
        if let Some(type_def) = compiled.registry.get(id) {
            match type_def {
                TypeDef::Trait(t) => {
                    traits.insert(t.name.clone(), t);
                }
                TypeDef::Impl(i) => {
                    impls.push(i);
                }
                _ => {}
            }
        }
    }

    // Also check impls that are not in declarations (collected separately)
    for (_id, type_def) in compiled.registry.iter() {
        if let TypeDef::Impl(i) = type_def {
            if !impls.iter().any(|existing| std::ptr::eq(*existing, i)) {
                impls.push(i);
            }
        }
    }

    // For each impl, verify that method calls in function bodies resolve correctly
    for impl_def in &impls {
        let Some(trait_def) = traits.get(&impl_def.trait_name) else {
            continue; // Error already reported during conformance checking
        };

        // Verify each function in the impl matches a trait function
        for impl_fn in &impl_def.functions {
            let trait_fn = trait_def.functions.iter().find(|f| f.name == impl_fn.name);

            if trait_fn.is_none() {
                diags.push(Diagnostic::error(
                    impl_def.span,
                    ErrorClass::UnresolvedType,
                    format!(
                        "impl function '{}' not found in trait '{}'",
                        impl_fn.name, impl_def.trait_name
                    ),
                ));
            }

            // Verify method calls in function body if present
            if let FnBody::Block(stmts) = &impl_fn.body {
                for stmt in stmts {
                    verify_trait_calls_in_statement(
                        stmt,
                        &impl_def.trait_name,
                        trait_def,
                        impl_def,
                        diags,
                    );
                }
            }
        }
    }
}

/// Verify that trait method calls in a statement are valid.
fn verify_trait_calls_in_statement(
    stmt: &Statement,
    trait_name: &SmolStr,
    trait_def: &crate::ir::TraitDef,
    impl_def: &crate::ir::ImplDef,
    diags: &mut Vec<Diagnostic>,
) {
    match stmt {
        Statement::Expr(expr)
        | Statement::Let { value: expr, .. }
        | Statement::Return(Some(expr)) => {
            verify_trait_calls_in_expr(expr, trait_name, trait_def, impl_def, diags);
        }
        Statement::Return(None) => {}
        Statement::Assign { target, value } => {
            verify_trait_calls_in_expr(target, trait_name, trait_def, impl_def, diags);
            verify_trait_calls_in_expr(value, trait_name, trait_def, impl_def, diags);
        }
    }
}

/// Verify that trait method calls in an expression are valid.
fn verify_trait_calls_in_expr(
    expr: &Expr,
    trait_name: &SmolStr,
    trait_def: &crate::ir::TraitDef,
    impl_def: &crate::ir::ImplDef,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::TraitMethodCall {
            trait_name: call_trait,
            method_name,
            receiver,
            args,
        } => {
            // Check if the trait name is resolved or if it matches the current impl's trait
            if call_trait.as_str() == "__unresolved" {
                // Try to resolve by looking for a matching trait function
                let found = trait_def.functions.iter().any(|f| &f.name == method_name);
                if !found {
                    diags.push(Diagnostic::error(
                        impl_def.span,
                        ErrorClass::UnresolvedType,
                        format!(
                            "method '{}' not found in trait '{}'",
                            method_name, trait_name
                        ),
                    ));
                }
            } else if call_trait != trait_name {
                // Call to a different trait - verify that trait exists
                // (this will be checked separately for each impl)
            }

            // Recursively check the receiver and arguments
            verify_trait_calls_in_expr(receiver, trait_name, trait_def, impl_def, diags);
            for arg in args {
                verify_trait_calls_in_expr(arg, trait_name, trait_def, impl_def, diags);
            }
        }
        Expr::Binary(_, lhs, rhs) => {
            verify_trait_calls_in_expr(lhs, trait_name, trait_def, impl_def, diags);
            verify_trait_calls_in_expr(rhs, trait_name, trait_def, impl_def, diags);
        }
        Expr::Unary(_, inner) => {
            verify_trait_calls_in_expr(inner, trait_name, trait_def, impl_def, diags);
        }
        Expr::FieldAccess(obj, _) => {
            verify_trait_calls_in_expr(obj, trait_name, trait_def, impl_def, diags);
        }
        Expr::Call(_, args) => {
            for arg in args {
                verify_trait_calls_in_expr(arg, trait_name, trait_def, impl_def, diags);
            }
        }
        _ => {} // Literals, Local, SelfRef - no nested expressions
    }
}
