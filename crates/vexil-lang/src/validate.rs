//! # Stability: Tier 2
//!
use std::collections::{HashMap, HashSet};

use smol_str::SmolStr;

use crate::ast::{
    Annotation, AnnotationValue, ConfigDecl, Decl, EnumBacking, EnumBodyItem, EnumDecl,
    FlagsBodyItem, FlagsDecl, ImportKind, MessageBodyItem, MessageDecl, MessageField, NewtypeDecl,
    PrimitiveType, Schema, SemanticType, TypeExpr, UnionBodyItem, UnionDecl,
};
use crate::diagnostic::{Diagnostic, ErrorClass};
use crate::span::{Span, Spanned};

// ---------------------------------------------------------------------------
// Declaration kind map (for type-level checks)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeclKind {
    Message,
    Enum,
    Flags,
    Union,
    Newtype,
    Config,
}

/// Context passed to all validation functions.
struct ValidationContext<'a> {
    decl_map: &'a HashMap<&'a SmolStr, (DeclKind, Span)>,
    imported_names: &'a HashSet<&'a SmolStr>,
    has_wildcard_import: bool,
}

impl ValidationContext<'_> {
    /// Returns true if a Named type reference is known (local decl or import).
    fn is_known_type(&self, name: &SmolStr) -> bool {
        self.decl_map.contains_key(name)
            || self.imported_names.contains(name)
            || self.has_wildcard_import
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Validate a parsed Schema, returning any semantic diagnostics.
pub fn validate(schema: &Schema) -> Vec<Diagnostic> {
    validate_impl(schema, false)
}

/// Validate a parsed Schema, skipping the reserved-namespace check.
///
/// This is intended for internal/meta schemas that are part of the
/// implementation itself (e.g. `vexil.schema`, `vexil.pack`), not for
/// user-authored schemas.
#[doc(hidden)]
pub fn validate_allow_reserved(schema: &Schema) -> Vec<Diagnostic> {
    validate_impl(schema, true)
}

fn validate_impl(schema: &Schema, allow_reserved: bool) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    // Build declaration map: name -> (kind, span)
    let mut decl_map: HashMap<&SmolStr, (DeclKind, Span)> = HashMap::new();

    for decl_spanned in &schema.declarations {
        let (name, kind) = match &decl_spanned.node {
            Decl::Message(d) => (&d.name, DeclKind::Message),
            Decl::Enum(d) => (&d.name, DeclKind::Enum),
            Decl::Flags(d) => (&d.name, DeclKind::Flags),
            Decl::Union(d) => (&d.name, DeclKind::Union),
            Decl::Newtype(d) => (&d.name, DeclKind::Newtype),
            Decl::Config(d) => (&d.name, DeclKind::Config),
        };
        decl_map.insert(&name.node, (kind, name.span));
    }

    // Build imported names set + wildcard flag
    let mut imported_names: HashSet<&SmolStr> = HashSet::new();
    let mut has_wildcard_import = false;
    for imp in &schema.imports {
        match &imp.node.kind {
            ImportKind::Wildcard => {
                has_wildcard_import = true;
            }
            ImportKind::Named { names } => {
                for n in names {
                    imported_names.insert(&n.node);
                }
            }
            ImportKind::Aliased { .. } => {
                // Aliased imports use Qualified types (Alias.Type), not Named.
            }
        }
    }

    let ctx = ValidationContext {
        decl_map: &decl_map,
        imported_names: &imported_names,
        has_wildcard_import,
    };

    if !allow_reserved {
        check_namespace_reserved(schema, &mut diags);
    }
    check_decl_name_duplicate(schema, &mut diags);
    check_schema_annotations(schema, &mut diags);

    for decl_spanned in &schema.declarations {
        match &decl_spanned.node {
            Decl::Message(d) => check_message(d, &ctx, &mut diags),
            Decl::Enum(d) => check_enum(d, &mut diags),
            Decl::Flags(d) => check_flags(d, &mut diags),
            Decl::Union(d) => check_union(d, &ctx, &mut diags),
            Decl::Newtype(d) => check_newtype(d, &ctx, &mut diags),
            Decl::Config(d) => check_config(d, &ctx, &mut diags),
        }
    }

    diags
}

// ---------------------------------------------------------------------------
// Namespace checks
// ---------------------------------------------------------------------------

fn check_namespace_reserved(schema: &Schema, diags: &mut Vec<Diagnostic>) {
    if let Some(ref ns) = schema.namespace {
        if let Some(first) = ns.node.path.first() {
            if first.node == "vexil" {
                diags.push(Diagnostic::error(
                    ns.span,
                    ErrorClass::NamespaceReserved,
                    "namespace `vexil` is reserved",
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Declaration-level checks
// ---------------------------------------------------------------------------

fn check_decl_name_duplicate(schema: &Schema, diags: &mut Vec<Diagnostic>) {
    let mut seen: HashMap<&SmolStr, Span> = HashMap::new();
    for decl_spanned in &schema.declarations {
        let name = match &decl_spanned.node {
            Decl::Message(d) => &d.name,
            Decl::Enum(d) => &d.name,
            Decl::Flags(d) => &d.name,
            Decl::Union(d) => &d.name,
            Decl::Newtype(d) => &d.name,
            Decl::Config(d) => &d.name,
        };
        if seen.contains_key(&name.node) {
            diags.push(Diagnostic::error(
                name.span,
                ErrorClass::DeclNameDuplicate,
                format!("duplicate declaration name `{}`", name.node),
            ));
        } else {
            seen.insert(&name.node, name.span);
        }
    }
}

// ---------------------------------------------------------------------------
// Schema-level annotation checks
// ---------------------------------------------------------------------------

fn check_schema_annotations(schema: &Schema, diags: &mut Vec<Diagnostic>) {
    // @version duplicate
    let mut version_count = 0u32;
    for ann in &schema.annotations {
        if ann.name.node == "version" {
            version_count += 1;
            if version_count > 1 {
                diags.push(Diagnostic::error(
                    ann.span,
                    ErrorClass::VersionDuplicate,
                    "@version must not appear more than once",
                ));
            }
        }
    }

    // Per-declaration annotation checks
    for decl_spanned in &schema.declarations {
        let annotations = match &decl_spanned.node {
            Decl::Message(d) => &d.annotations,
            Decl::Enum(d) => &d.annotations,
            Decl::Flags(d) => &d.annotations,
            Decl::Union(d) => &d.annotations,
            Decl::Newtype(d) => &d.annotations,
            Decl::Config(d) => &d.annotations,
        };

        check_duplicate_annotations(annotations, diags);

        let decl_kind = match &decl_spanned.node {
            Decl::Message(_) => DeclKind::Message,
            Decl::Enum(_) => DeclKind::Enum,
            Decl::Flags(_) => DeclKind::Flags,
            Decl::Union(_) => DeclKind::Union,
            Decl::Newtype(_) => DeclKind::Newtype,
            Decl::Config(_) => DeclKind::Config,
        };

        for ann in annotations {
            if ann.name.node == "non_exhaustive"
                && decl_kind != DeclKind::Enum
                && decl_kind != DeclKind::Union
            {
                diags.push(Diagnostic::error(
                    ann.span,
                    ErrorClass::NonExhaustiveInvalidTarget,
                    "@non_exhaustive is only valid on enum or union declarations",
                ));
            }

            if ann.name.node == "deprecated" {
                check_deprecated_has_reason(ann, diags);
            }

            if ann.name.node == "type" {
                check_type_annotation_value(ann, diags);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Duplicate annotation check
// ---------------------------------------------------------------------------

fn check_duplicate_annotations(annotations: &[Annotation], diags: &mut Vec<Diagnostic>) {
    let mut seen: HashSet<&SmolStr> = HashSet::new();
    for ann in annotations {
        if ann.name.node == "doc" {
            continue;
        }
        if !seen.insert(&ann.name.node) {
            diags.push(Diagnostic::error(
                ann.span,
                ErrorClass::DuplicateAnnotation,
                format!("duplicate annotation @{}", ann.name.node),
            ));
        }
    }
}

fn check_duplicate_annotations_refs(annotations: &[&Annotation], diags: &mut Vec<Diagnostic>) {
    let mut seen: HashSet<&SmolStr> = HashSet::new();
    for ann in annotations {
        if ann.name.node == "doc" {
            continue;
        }
        if !seen.insert(&ann.name.node) {
            diags.push(Diagnostic::error(
                ann.span,
                ErrorClass::DuplicateAnnotation,
                format!("duplicate annotation @{}", ann.name.node),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// @deprecated reason check
// ---------------------------------------------------------------------------

fn check_deprecated_has_reason(ann: &Annotation, diags: &mut Vec<Diagnostic>) {
    let has_reason = ann.args.as_ref().is_some_and(|args| {
        args.iter()
            .any(|arg| arg.key.as_ref().is_some_and(|k| k.node == "reason"))
    });
    if !has_reason {
        diags.push(Diagnostic::error(
            ann.span,
            ErrorClass::DeprecatedMissingReason,
            "@deprecated must include a `reason` argument",
        ));
    }
}

// ---------------------------------------------------------------------------
// @type value overflow check
// ---------------------------------------------------------------------------

fn check_type_annotation_value(ann: &Annotation, diags: &mut Vec<Diagnostic>) {
    if let Some(ref args) = ann.args {
        for arg in args {
            let val = match &arg.value.node {
                AnnotationValue::Int(v) => Some(*v),
                AnnotationValue::Hex(v) => Some(*v),
                _ => None,
            };
            if let Some(v) = val {
                if v > 255 {
                    diags.push(Diagnostic::error(
                        ann.span,
                        ErrorClass::TypeValueOverflow,
                        format!("@type value {v} exceeds maximum of 255"),
                    ));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Message
// ---------------------------------------------------------------------------

fn check_message(msg: &MessageDecl, ctx: &ValidationContext<'_>, diags: &mut Vec<Diagnostic>) {
    check_message_body(&msg.body, ctx, diags);
}

fn check_message_body(
    body: &[MessageBodyItem],
    ctx: &ValidationContext<'_>,
    diags: &mut Vec<Diagnostic>,
) {
    let mut ordinal_set: HashSet<u32> = HashSet::new();
    let mut tombstone_ordinals: HashSet<u32> = HashSet::new();
    let mut name_set: HashSet<&SmolStr> = HashSet::new();

    // First pass: collect tombstone ordinals
    for item in body {
        if let MessageBodyItem::Tombstone(ts) = item {
            tombstone_ordinals.insert(ts.node.ordinal.node);
            ordinal_set.insert(ts.node.ordinal.node);
        }
    }

    // Second pass: check fields
    for item in body {
        if let MessageBodyItem::Field(field) = item {
            let f = &field.node;

            // Ordinal too large
            if f.ordinal.node > 65535 {
                diags.push(Diagnostic::error(
                    f.ordinal.span,
                    ErrorClass::OrdinalTooLarge,
                    format!("ordinal {} exceeds maximum of 65535", f.ordinal.node),
                ));
            }

            // Ordinal reused after removed
            if tombstone_ordinals.contains(&f.ordinal.node) {
                diags.push(Diagnostic::error(
                    f.ordinal.span,
                    ErrorClass::OrdinalReusedAfterRemoved,
                    format!(
                        "ordinal {} was tombstoned by @removed and cannot be reused",
                        f.ordinal.node
                    ),
                ));
            }

            // Ordinal duplicate (field vs field)
            if !ordinal_set.insert(f.ordinal.node) && !tombstone_ordinals.contains(&f.ordinal.node)
            {
                diags.push(Diagnostic::error(
                    f.ordinal.span,
                    ErrorClass::OrdinalDuplicate,
                    format!("duplicate ordinal {}", f.ordinal.node),
                ));
            }

            // Field name duplicate
            if !name_set.insert(&f.name.node) {
                diags.push(Diagnostic::error(
                    f.name.span,
                    ErrorClass::FieldNameDuplicate,
                    format!("duplicate field name `{}`", f.name.node),
                ));
            }

            // Type-level checks
            check_field_type(&f.ty, ctx, diags);

            // Annotation checks on field
            let all_annotations = collect_field_annotations(f);
            check_field_annotations(f, &all_annotations, ctx, diags);
        }
    }
}

fn collect_field_annotations(f: &MessageField) -> Vec<&Annotation> {
    let mut all = Vec::new();
    all.extend(f.pre_annotations.iter());
    all.extend(f.post_ordinal_annotations.iter());
    all.extend(f.post_type_annotations.iter());
    all
}

// ---------------------------------------------------------------------------
// Field type checks
// ---------------------------------------------------------------------------

fn check_field_type(
    ty: &Spanned<TypeExpr>,
    ctx: &ValidationContext<'_>,
    diags: &mut Vec<Diagnostic>,
) {
    match &ty.node {
        TypeExpr::Named(name) => {
            if let Some((kind, _)) = ctx.decl_map.get(name) {
                if *kind == DeclKind::Config {
                    diags.push(Diagnostic::error(
                        ty.span,
                        ErrorClass::ConfigTypeAsField,
                        format!("config type `{name}` cannot be used as a field type"),
                    ));
                }
            } else if !ctx.is_known_type(name) {
                diags.push(Diagnostic::error(
                    ty.span,
                    ErrorClass::UnknownType,
                    format!("unknown type `{name}`"),
                ));
            }
        }
        TypeExpr::Map(key, value) => {
            check_map_key_type(key, ctx, diags);
            check_field_type(value, ctx, diags);
        }
        TypeExpr::Optional(inner) => {
            check_field_type(inner, ctx, diags);
        }
        TypeExpr::Array(inner) => {
            check_field_type(inner, ctx, diags);
        }
        TypeExpr::Result(ok, err) => {
            check_field_type(ok, ctx, diags);
            check_field_type(err, ctx, diags);
        }
        _ => {}
    }
}

fn check_map_key_type(
    key: &Spanned<TypeExpr>,
    ctx: &ValidationContext<'_>,
    diags: &mut Vec<Diagnostic>,
) {
    let invalid = match &key.node {
        TypeExpr::Primitive(PrimitiveType::F32 | PrimitiveType::F64 | PrimitiveType::Void) => true,
        TypeExpr::Optional(_)
        | TypeExpr::Array(_)
        | TypeExpr::Map(_, _)
        | TypeExpr::Result(_, _) => true,
        TypeExpr::Named(name) => {
            if let Some((kind, _)) = ctx.decl_map.get(name) {
                matches!(
                    kind,
                    DeclKind::Message | DeclKind::Union | DeclKind::Newtype | DeclKind::Config
                )
            } else {
                false
            }
        }
        _ => false,
    };

    if invalid {
        diags.push(Diagnostic::error(
            key.span,
            ErrorClass::InvalidMapKey,
            "invalid map key type",
        ));
    }
}

// ---------------------------------------------------------------------------
// Field annotation checks
// ---------------------------------------------------------------------------

fn check_field_annotations(
    field: &MessageField,
    annotations: &[&Annotation],
    _ctx: &ValidationContext<'_>,
    diags: &mut Vec<Diagnostic>,
) {
    let has_varint = annotations.iter().any(|a| a.name.node == "varint");
    let has_zigzag = annotations.iter().any(|a| a.name.node == "zigzag");

    if has_varint && has_zigzag {
        if let Some(ann) = annotations.iter().find(|a| a.name.node == "zigzag") {
            diags.push(Diagnostic::error(
                ann.span,
                ErrorClass::VarintZigzagCombined,
                "@varint and @zigzag cannot be combined on the same field",
            ));
        }
    }

    let ty = &field.ty.node;

    for ann in annotations {
        match ann.name.node.as_str() {
            "varint" => {
                if !is_varint_valid_type(ty) {
                    diags.push(Diagnostic::error(
                        ann.span,
                        ErrorClass::VarintInvalidTarget,
                        "@varint is only valid on u16, u32, u64 fields",
                    ));
                }
            }
            "zigzag" => {
                if !is_zigzag_valid_type(ty) {
                    diags.push(Diagnostic::error(
                        ann.span,
                        ErrorClass::ZigzagInvalidTarget,
                        "@zigzag is only valid on i16, i32, i64 fields",
                    ));
                }
            }
            "delta" => {
                if !is_delta_valid_type(ty) {
                    diags.push(Diagnostic::error(
                        ann.span,
                        ErrorClass::DeltaInvalidTarget,
                        "@delta is only valid on numeric types",
                    ));
                }
            }
            "limit" => {
                if !is_limit_valid_type(ty) {
                    diags.push(Diagnostic::error(
                        ann.span,
                        ErrorClass::LimitInvalidTarget,
                        "@limit is only valid on string, bytes, array, or map types",
                    ));
                }
                check_limit_value(ann, ty, diags);
            }
            "deprecated" => {
                check_deprecated_has_reason(ann, diags);
            }
            _ => {}
        }
    }

    // Duplicate annotations on field
    let all_flat: Vec<&Annotation> = field
        .pre_annotations
        .iter()
        .chain(field.post_ordinal_annotations.iter())
        .chain(field.post_type_annotations.iter())
        .collect();
    check_duplicate_annotations_refs(&all_flat, diags);
}

// ---------------------------------------------------------------------------
// Type predicates for annotation validation
// ---------------------------------------------------------------------------

fn is_varint_valid_type(ty: &TypeExpr) -> bool {
    matches!(
        ty,
        TypeExpr::Primitive(PrimitiveType::U16 | PrimitiveType::U32 | PrimitiveType::U64)
    )
}

fn is_zigzag_valid_type(ty: &TypeExpr) -> bool {
    matches!(
        ty,
        TypeExpr::Primitive(PrimitiveType::I16 | PrimitiveType::I32 | PrimitiveType::I64)
    )
}

fn is_delta_valid_type(ty: &TypeExpr) -> bool {
    matches!(
        ty,
        TypeExpr::Primitive(
            PrimitiveType::U8
                | PrimitiveType::U16
                | PrimitiveType::U32
                | PrimitiveType::U64
                | PrimitiveType::I8
                | PrimitiveType::I16
                | PrimitiveType::I32
                | PrimitiveType::I64
                | PrimitiveType::F32
                | PrimitiveType::F64
        ) | TypeExpr::SubByte(_)
    )
}

fn is_limit_valid_type(ty: &TypeExpr) -> bool {
    matches!(
        ty,
        TypeExpr::Semantic(SemanticType::String | SemanticType::Bytes)
            | TypeExpr::Array(_)
            | TypeExpr::Map(_, _)
    )
}

fn check_limit_value(ann: &Annotation, ty: &TypeExpr, diags: &mut Vec<Diagnostic>) {
    if let Some(ref args) = ann.args {
        for arg in args {
            if arg.key.is_none() {
                let val = match &arg.value.node {
                    AnnotationValue::Int(v) => Some(*v),
                    AnnotationValue::Hex(v) => Some(*v),
                    _ => None,
                };
                if let Some(v) = val {
                    if v == 0 {
                        diags.push(Diagnostic::error(
                            ann.span,
                            ErrorClass::LimitZero,
                            "@limit value must be positive",
                        ));
                    }
                    let max = match ty {
                        TypeExpr::Semantic(SemanticType::String | SemanticType::Bytes) => {
                            16_777_216u64
                        }
                        TypeExpr::Array(_) | TypeExpr::Map(_, _) => 16_777_216,
                        _ => u64::MAX,
                    };
                    if v > max {
                        diags.push(Diagnostic::error(
                            ann.span,
                            ErrorClass::LimitExceedsGlobal,
                            format!("@limit({v}) exceeds global maximum of {max}"),
                        ));
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Enum
// ---------------------------------------------------------------------------

fn check_enum(en: &EnumDecl, diags: &mut Vec<Diagnostic>) {
    let mut ordinal_set: HashSet<u32> = HashSet::new();
    let mut max_ordinal: u32 = 0;

    for item in &en.body {
        if let EnumBodyItem::Tombstone(ts) = item {
            ordinal_set.insert(ts.node.ordinal.node);
        }
    }

    for item in &en.body {
        if let EnumBodyItem::Variant(v) = item {
            let ord = v.node.ordinal.node;

            if ord > 65535 {
                diags.push(Diagnostic::error(
                    v.node.ordinal.span,
                    ErrorClass::EnumOrdinalTooLarge,
                    format!("enum ordinal {ord} exceeds maximum of 65535"),
                ));
            }

            if !ordinal_set.insert(ord) {
                diags.push(Diagnostic::error(
                    v.node.ordinal.span,
                    ErrorClass::EnumOrdinalDuplicate,
                    format!("duplicate enum ordinal {ord}"),
                ));
            }

            if ord > max_ordinal {
                max_ordinal = ord;
            }
        }
    }

    if let Some(ref backing) = en.backing {
        let backing_max: u64 = match backing.node {
            EnumBacking::U8 => 255,
            EnumBacking::U16 => 65535,
            EnumBacking::U32 => u32::MAX as u64,
            EnumBacking::U64 => u64::MAX,
        };
        if (max_ordinal as u64) > backing_max {
            diags.push(Diagnostic::error(
                backing.span,
                ErrorClass::EnumBackingTooNarrow,
                format!(
                    "backing type {:?} cannot hold ordinal {max_ordinal}",
                    backing.node
                ),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Flags
// ---------------------------------------------------------------------------

fn check_flags(flags: &FlagsDecl, diags: &mut Vec<Diagnostic>) {
    for item in &flags.body {
        if let FlagsBodyItem::Bit(b) = item {
            if b.node.ordinal.node > 63 {
                diags.push(Diagnostic::error(
                    b.node.ordinal.span,
                    ErrorClass::FlagsBitTooHigh,
                    format!(
                        "flags bit position {} exceeds maximum of 63",
                        b.node.ordinal.node
                    ),
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Union
// ---------------------------------------------------------------------------

fn check_union(un: &UnionDecl, ctx: &ValidationContext<'_>, diags: &mut Vec<Diagnostic>) {
    let mut ordinal_set: HashSet<u32> = HashSet::new();

    for item in &un.body {
        if let UnionBodyItem::Tombstone(ts) = item {
            ordinal_set.insert(ts.node.ordinal.node);
        }
    }

    for item in &un.body {
        if let UnionBodyItem::Variant(v) = item {
            let ord = v.node.ordinal.node;

            if ord > 65535 {
                diags.push(Diagnostic::error(
                    v.node.ordinal.span,
                    ErrorClass::UnionOrdinalTooLarge,
                    format!("union variant ordinal {ord} exceeds maximum of 65535"),
                ));
            }

            if !ordinal_set.insert(ord) {
                diags.push(Diagnostic::error(
                    v.node.ordinal.span,
                    ErrorClass::UnionOrdinalDuplicate,
                    format!("duplicate union variant ordinal {ord}"),
                ));
            }

            check_message_body(&v.node.fields, ctx, diags);
        }
    }
}

// ---------------------------------------------------------------------------
// Newtype
// ---------------------------------------------------------------------------

fn check_newtype(nt: &NewtypeDecl, ctx: &ValidationContext<'_>, diags: &mut Vec<Diagnostic>) {
    if let TypeExpr::Named(ref name) = nt.inner_type.node {
        if let Some((kind, _)) = ctx.decl_map.get(name) {
            match kind {
                DeclKind::Newtype => {
                    diags.push(Diagnostic::error(
                        nt.inner_type.span,
                        ErrorClass::NewtypeOverNewtype,
                        format!(
                            "newtype `{}` cannot wrap another newtype `{name}`",
                            nt.name.node
                        ),
                    ));
                }
                DeclKind::Config => {
                    diags.push(Diagnostic::error(
                        nt.inner_type.span,
                        ErrorClass::NewtypeOverConfig,
                        format!(
                            "newtype `{}` cannot wrap config type `{name}`",
                            nt.name.node
                        ),
                    ));
                }
                _ => {}
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

fn check_config(cfg: &ConfigDecl, ctx: &ValidationContext<'_>, diags: &mut Vec<Diagnostic>) {
    for field in &cfg.fields {
        let f = &field.node;

        // Config field type cannot be map or result
        match &f.ty.node {
            TypeExpr::Map(_, _) => {
                diags.push(Diagnostic::error(
                    f.ty.span,
                    ErrorClass::ConfigInvalidType,
                    "config fields cannot use map type",
                ));
            }
            TypeExpr::Result(_, _) => {
                diags.push(Diagnostic::error(
                    f.ty.span,
                    ErrorClass::ConfigInvalidType,
                    "config fields cannot use result type",
                ));
            }
            _ => {}
        }

        // Config fields must not carry encoding annotations
        for ann in &f.annotations {
            if matches!(ann.name.node.as_str(), "varint" | "zigzag" | "delta") {
                diags.push(Diagnostic::error(
                    ann.span,
                    ErrorClass::ConfigEncodingAnnotation,
                    format!(
                        "config fields must not carry encoding annotation @{}",
                        ann.name.node
                    ),
                ));
            }
        }

        // Type-level checks for config fields
        check_field_type(&f.ty, ctx, diags);
    }
}
