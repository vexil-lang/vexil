//! # Stability: Tier 2
//!
//! Semantic validation of parsed schemas.
//!
//! Checks naming conventions, ordinal constraints, annotation validity,
//! and other structural rules that go beyond syntax. Runs on the AST
//! before lowering to IR.

use std::collections::{HashMap, HashSet};

use smol_str::SmolStr;

use crate::ast::{
    Annotation, AnnotationValue, BinOpKind, CmpOp, ConfigDecl, ConstDecl, ConstExpr, Decl,
    EnumBacking, EnumBodyItem, EnumDecl, FlagsBodyItem, FlagsDecl, ImplDecl, ImportKind,
    MessageBodyItem, MessageDecl, MessageField, NewtypeDecl, PrimitiveType, Schema, SemanticType,
    TypeExpr, UnionBodyItem, UnionDecl, WhereExpr, WhereOperand,
};
use crate::diagnostic::{find_closest_match, Diagnostic, ErrorClass, Note};
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
    Alias,
    Const,
    Trait,
    Impl,
}

/// Context passed to all validation functions.
struct ValidationContext<'a> {
    decl_map: &'a HashMap<&'a SmolStr, (DeclKind, Span)>,
    imported_names: &'a HashSet<&'a SmolStr>,
    has_wildcard_import: bool,
    newtype_inners: &'a HashMap<&'a SmolStr, &'a TypeExpr>,
    alias_targets: &'a HashMap<&'a SmolStr, &'a TypeExpr>,
    const_map: &'a HashMap<&'a SmolStr, &'a ConstDecl>,
}

impl ValidationContext<'_> {
    /// Returns true if a Named type reference is known (local decl or import).
    fn is_known_type(&self, name: &SmolStr) -> bool {
        self.decl_map.contains_key(name)
            || self.imported_names.contains(name)
            || self.has_wildcard_import
    }

    fn newtype_inner(&self, name: &SmolStr) -> Option<&TypeExpr> {
        self.newtype_inners.get(name).copied()
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
pub(crate) fn validate_allow_reserved(schema: &Schema) -> Vec<Diagnostic> {
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
            Decl::Alias(d) => (&d.name, DeclKind::Alias),
            Decl::Const(d) => (&d.name, DeclKind::Const),
            Decl::Trait(d) => (&d.name, DeclKind::Trait),
            Decl::Impl(_) => continue, // Impls don't register names
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

    let mut newtype_inners: HashMap<&SmolStr, &TypeExpr> = HashMap::new();
    for decl_spanned in &schema.declarations {
        if let Decl::Newtype(d) = &decl_spanned.node {
            newtype_inners.insert(&d.name.node, &d.inner_type.node);
        }
    }

    let mut alias_targets: HashMap<&SmolStr, &TypeExpr> = HashMap::new();
    for decl_spanned in &schema.declarations {
        if let Decl::Alias(d) = &decl_spanned.node {
            alias_targets.insert(&d.name.node, &d.target.node);
        }
    }

    let mut const_map: HashMap<&SmolStr, &ConstDecl> = HashMap::new();
    for decl_spanned in &schema.declarations {
        if let Decl::Const(c) = &decl_spanned.node {
            const_map.insert(&c.name.node, c);
        }
    }

    let ctx = ValidationContext {
        decl_map: &decl_map,
        imported_names: &imported_names,
        has_wildcard_import,
        newtype_inners: &newtype_inners,
        alias_targets: &alias_targets,
        const_map: &const_map,
    };

    if !allow_reserved {
        check_namespace_reserved(schema, &mut diags);
    }
    check_decl_name_duplicate(schema, &mut diags);
    check_schema_annotations(schema, &mut diags);

    // Evaluate const declarations (detects cycles and division by zero)
    let _const_values = evaluate_consts(schema, &mut diags);

    for decl_spanned in &schema.declarations {
        match &decl_spanned.node {
            Decl::Message(d) => check_message(d, &ctx, &mut diags),
            Decl::Enum(d) => check_enum(d, &mut diags),
            Decl::Flags(d) => check_flags(d, &mut diags),
            Decl::Union(d) => check_union(d, &ctx, &mut diags),
            Decl::Newtype(d) => check_newtype(d, &ctx, &mut diags),
            Decl::Config(d) => check_config(d, &ctx, &mut diags),
            Decl::Alias(d) => check_alias(d, &ctx, &mut diags),
            Decl::Const(d) => check_const(d, &ctx, &mut diags),
            Decl::Trait(_) => {} // trait validation during impl
            Decl::Impl(d) => check_impl(d, &ctx, &mut diags),
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
            Decl::Alias(d) => &d.name,
            Decl::Const(d) => &d.name,
            Decl::Trait(d) => &d.name,
            Decl::Impl(_) => continue, // Impls don't declare names, skip duplicate check
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
            Decl::Alias(d) => &d.annotations,
            Decl::Const(d) => &d.annotations,
            Decl::Trait(d) => &d.annotations,
            Decl::Impl(d) => &d.annotations,
        };

        check_duplicate_annotations(annotations, diags);

        let decl_kind = match &decl_spanned.node {
            Decl::Message(_) => DeclKind::Message,
            Decl::Enum(_) => DeclKind::Enum,
            Decl::Flags(_) => DeclKind::Flags,
            Decl::Union(_) => DeclKind::Union,
            Decl::Newtype(_) => DeclKind::Newtype,
            Decl::Config(_) => DeclKind::Config,
            Decl::Alias(_) => DeclKind::Alias,
            Decl::Const(_) => DeclKind::Const,
            Decl::Trait(_) => DeclKind::Trait,
            Decl::Impl(_) => DeclKind::Impl,
        };

        for ann in annotations {
            if ann.name.node == "non_exhaustive"
                && decl_kind != DeclKind::Enum
                && decl_kind != DeclKind::Union
            {
                let decl_type = match decl_kind {
                    DeclKind::Message => "message",
                    DeclKind::Enum => "enum",
                    DeclKind::Flags => "flags",
                    DeclKind::Union => "union",
                    DeclKind::Newtype => "newtype",
                    DeclKind::Config => "config",
                    DeclKind::Alias => "alias",
                    DeclKind::Const => "const",
                    DeclKind::Trait => "trait",
                    DeclKind::Impl => "impl",
                };
                diags.push(
                    Diagnostic::error(
                        ann.span,
                        ErrorClass::NonExhaustiveInvalidTarget,
                        format!("@non_exhaustive cannot be applied to {decl_type} declarations"),
                    )
                    .with_note(Note::ValidOptions(vec!["enum".to_string(), "union".to_string()]))
                    .with_help("@non_exhaustive allows adding variants/bits without breaking compatibility by reserving encoding space for future additions"),
                );
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

    // Track ordinal -> field name for conflict reporting
    let mut ordinal_names: HashMap<u32, SmolStr> = HashMap::new();

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
                if let Some(existing) = ordinal_names.get(&f.ordinal.node) {
                    diags.push(Diagnostic::error(
                        f.ordinal.span,
                        ErrorClass::OrdinalDuplicate,
                        format!(
                            "ordinal @{} conflicts with field `{}` (both use the same ordinal)",
                            f.ordinal.node, existing
                        ),
                    ));
                } else {
                    diags.push(Diagnostic::error(
                        f.ordinal.span,
                        ErrorClass::OrdinalDuplicate,
                        format!("duplicate ordinal @{}", f.ordinal.node),
                    ));
                }
            }
            ordinal_names.insert(f.ordinal.node, f.name.node.clone());

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
                // Collect all known type names for suggestions
                let all_types: Vec<&str> = ctx
                    .decl_map
                    .keys()
                    .map(|k| k.as_str())
                    .chain(ctx.imported_names.iter().map(|k| k.as_str()))
                    .collect();

                let mut diag = Diagnostic::error(
                    ty.span,
                    ErrorClass::UnknownType,
                    format!("unknown type `{name}`"),
                );

                // Add "did you mean" suggestion if we find a close match
                if let Some(suggestion) =
                    find_closest_match(name.as_str(), all_types.iter().map(|s| s.as_ref()))
                {
                    diag = diag.with_suggestion(suggestion);
                }

                // Add help text with available types
                let available = all_types.to_vec();
                if !available.is_empty() {
                    diag = diag.with_help(format!(
                        "available types in scope: {}",
                        available.join(", ")
                    ));
                }

                diags.push(diag);
            }
        }
        TypeExpr::Set(inner) => {
            check_map_key_type(inner, ctx, diags);
            check_field_type(inner, ctx, diags);
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
        TypeExpr::FixedArray(inner, size) => {
            if *size == 0 {
                diags.push(Diagnostic::error(
                    ty.span,
                    ErrorClass::LimitZero,
                    "fixed array size must be positive",
                ));
            }
            check_field_type(inner, ctx, diags);
        }
        TypeExpr::Result(ok, err) => {
            check_field_type(ok, ctx, diags);
            check_field_type(err, ctx, diags);
        }
        TypeExpr::Vec2(inner)
        | TypeExpr::Vec3(inner)
        | TypeExpr::Vec4(inner)
        | TypeExpr::Quat(inner)
        | TypeExpr::Mat3(inner)
        | TypeExpr::Mat4(inner) => {
            // Validate element type: must be fixed32, fixed64, f32, or f64
            if !is_valid_geometric_element_type(&inner.node) {
                diags.push(Diagnostic::error(
                    inner.span,
                    ErrorClass::GeometricInvalidElementType,
                    "geometric type element must be fixed32, fixed64, f32, or f64",
                ));
            }
            check_field_type(inner, ctx, diags);
        }
        TypeExpr::BitsInline(names) => {
            if names.is_empty() {
                diags.push(Diagnostic::error(
                    ty.span,
                    ErrorClass::BitsInlineEmpty,
                    "inline bits type must have at least one bit name",
                ));
            }
            for name in names {
                if !is_valid_field_name(name) {
                    diags.push(Diagnostic::error(
                        ty.span,
                        ErrorClass::InvalidBitName,
                        format!("invalid bit name `{name}` in inline bits"),
                    ));
                }
            }
        }
        _ => {}
    }
}

fn check_map_key_type(
    key: &Spanned<TypeExpr>,
    ctx: &ValidationContext<'_>,
    diags: &mut Vec<Diagnostic>,
) {
    if is_invalid_map_key(&key.node, ctx) {
        let key_type_str = format_type_expr(&key.node);

        let diag = Diagnostic::error(
            key.span,
            ErrorClass::InvalidMapKey,
            format!(
                "{key_type_str} is not a valid map key type; \
                 valid map key types: integers (u8-u64, i8-i64), string, bytes, uuid, enum, flags, fixed32, fixed64"
            ),
        )
        .with_help("map keys must be hashable and comparable; floating point, void, message, union, config, and container types are not allowed");

        diags.push(diag);
    }
}

/// Returns true if the given type expression is NOT a valid map key.
fn is_invalid_map_key(ty: &TypeExpr, ctx: &ValidationContext<'_>) -> bool {
    match ty {
        // Floating point and void are NOT valid map key types
        TypeExpr::Primitive(PrimitiveType::F32 | PrimitiveType::F64 | PrimitiveType::Void) => true,
        // Parameterized types are NOT valid map key types
        TypeExpr::Optional(_)
        | TypeExpr::Array(_)
        | TypeExpr::Set(_)
        | TypeExpr::Map(_, _)
        | TypeExpr::Result(_, _) => true,
        TypeExpr::Named(name) => {
            if let Some((kind, _)) = ctx.decl_map.get(name) {
                match kind {
                    DeclKind::Message | DeclKind::Union | DeclKind::Config => true,
                    DeclKind::Newtype => {
                        // Newtypes can't nest (NewtypeOverNewtype), so inner is never another newtype.
                        // Check if the inner type is a valid key type. For imported newtypes
                        // where we don't have the inner type, allow it — the type checker
                        // will catch actual errors later.
                        ctx.newtype_inner(name)
                            .is_some_and(|inner| is_invalid_map_key(inner, ctx))
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Field annotation checks
// ---------------------------------------------------------------------------

fn check_field_annotations(
    field: &MessageField,
    annotations: &[&Annotation],
    ctx: &ValidationContext<'_>,
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
                    let type_str = format_type_expr(ty);
                    diags.push(
                        Diagnostic::error(
                            ann.span,
                            ErrorClass::VarintInvalidTarget,
                            format!("@varint is not valid on {type_str}. Valid types for @varint: u16, u32, u64, fixed32, fixed64"),
                        )
                        .with_help("@varint optimizes unsigned integer encoding using LEB128 variable-length encoding"),
                    );
                }
            }
            "zigzag" => {
                if !is_zigzag_valid_type(ty) {
                    let type_str = format_type_expr(ty);
                    diags.push(
                        Diagnostic::error(
                            ann.span,
                            ErrorClass::ZigzagInvalidTarget,
                            format!("@zigzag is not valid on {type_str}. Valid types for @zigzag: i16, i32, i64"),
                        )
                        .with_help("@zigzag optimizes signed integer encoding using ZigZag + LEB128 for small absolute values"),
                    );
                }
            }
            "delta" => {
                if !is_delta_valid_type(ty) {
                    let type_str = format_type_expr(ty);
                    diags.push(
                        Diagnostic::error(
                            ann.span,
                            ErrorClass::DeltaInvalidTarget,
                            format!(
                                "@delta is not valid on {type_str}. Valid types for @delta: u8, u16, u32, u64, i8, i16, i32, i64, f32, f64, fixed32, fixed64"
                            ),
                        )
                        .with_help("@delta encodes differences between consecutive values, effective when values change slowly"),
                    );
                }
            }
            "limit" => {
                if !is_limit_valid_type(ty) {
                    let type_str = format_type_expr(ty);
                    diags.push(
                        Diagnostic::error(
                            ann.span,
                            ErrorClass::LimitInvalidTarget,
                            format!(
                                "@limit is not valid on {type_str}. Valid types for @limit: string, bytes, array, fixed_array, set, map"
                            ),
                        )
                        .with_help("@limit restricts the maximum size of collection types"),
                    );
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

    // Check where clause if present
    if let Some(ref where_clause) = field.where_clause {
        check_where_clause(where_clause, &field.ty.node, ctx, diags);
    }
}

// ---------------------------------------------------------------------------
// Type predicates for annotation validation
// ---------------------------------------------------------------------------

fn is_varint_valid_type(ty: &TypeExpr) -> bool {
    matches!(
        ty,
        TypeExpr::Primitive(
            PrimitiveType::U16
                | PrimitiveType::U32
                | PrimitiveType::U64
                | PrimitiveType::Fixed32
                | PrimitiveType::Fixed64
        )
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
                | PrimitiveType::Fixed32
                | PrimitiveType::Fixed64
        ) | TypeExpr::SubByte(_)
    )
}

fn is_limit_valid_type(ty: &TypeExpr) -> bool {
    matches!(
        ty,
        TypeExpr::Semantic(SemanticType::String | SemanticType::Bytes)
            | TypeExpr::Array(_)
            | TypeExpr::FixedArray(_, _)
            | TypeExpr::Set(_)
            | TypeExpr::Map(_, _)
    )
}

/// Check if a type is a valid element type for geometric types.
/// Valid: fixed32, fixed64, f32, f64
fn is_valid_geometric_element_type(ty: &TypeExpr) -> bool {
    matches!(
        ty,
        TypeExpr::Primitive(
            PrimitiveType::Fixed32
                | PrimitiveType::Fixed64
                | PrimitiveType::F32
                | PrimitiveType::F64
        )
    )
}

/// Returns the list of valid encoding annotations for a given type.
/// Used to provide helpful error messages when an annotation is applied to an incompatible type.
#[allow(dead_code)] // Will be used for improved error messages
fn valid_annotations_for_type(ty: &TypeExpr) -> Vec<&'static str> {
    let mut valid = Vec::new();

    if is_varint_valid_type(ty) {
        valid.push("@varint");
    }
    if is_zigzag_valid_type(ty) {
        valid.push("@zigzag");
    }
    if is_delta_valid_type(ty) {
        valid.push("@delta");
    }
    if is_limit_valid_type(ty) {
        valid.push("@limit");
    }

    // If no encoding annotations are valid, provide a helpful message
    if valid.is_empty() {
        valid.push("(none for this type)");
    }

    valid
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
                        TypeExpr::Array(_) | TypeExpr::Set(_) | TypeExpr::Map(_, _) => 16_777_216,
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
    let mut ordinal_names: HashMap<u32, SmolStr> = HashMap::new();
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
                if let Some(existing) = ordinal_names.get(&ord) {
                    diags.push(Diagnostic::error(
                        v.node.ordinal.span,
                        ErrorClass::EnumOrdinalDuplicate,
                        format!(
                            "enum ordinal @{} conflicts with variant `{}` (both use the same ordinal)",
                            ord, existing
                        ),
                    ));
                } else {
                    diags.push(Diagnostic::error(
                        v.node.ordinal.span,
                        ErrorClass::EnumOrdinalDuplicate,
                        format!("duplicate enum ordinal @{}", ord),
                    ));
                }
            }
            ordinal_names.insert(ord, v.node.name.node.clone());

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
    let mut ordinal_names: HashMap<u32, SmolStr> = HashMap::new();

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
                if let Some(existing) = ordinal_names.get(&ord) {
                    diags.push(Diagnostic::error(
                        v.node.ordinal.span,
                        ErrorClass::UnionOrdinalDuplicate,
                        format!(
                            "union variant ordinal @{} conflicts with variant `{}` (both use the same ordinal)",
                            ord, existing
                        ),
                    ));
                } else {
                    diags.push(Diagnostic::error(
                        v.node.ordinal.span,
                        ErrorClass::UnionOrdinalDuplicate,
                        format!("duplicate union variant ordinal @{}", ord),
                    ));
                }
            }
            ordinal_names.insert(ord, v.node.name.node.clone());

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

// ---------------------------------------------------------------------------
// Type Alias
// ---------------------------------------------------------------------------

fn check_alias(
    alias: &crate::ast::AliasDecl,
    ctx: &ValidationContext<'_>,
    diags: &mut Vec<Diagnostic>,
) {
    // Check 1: For generic aliases, type param names must not shadow existing types
    for param in &alias.type_params {
        let param_name = &param.name.node;
        if ctx.is_known_type(param_name) {
            diags.push(Diagnostic::error(
                param.name.span,
                ErrorClass::AliasTargetNotFound,
                format!(
                    "type parameter `{param_name}` shadows an existing type name; \
                     type parameters must be unique and not conflict with declared types"
                ),
            ));
        }
    }

    // Check 2: Target type must exist (cannot be a forward reference to undefined type)
    // For a type alias, the target must be resolvable at declaration time.
    match &alias.target.node {
        TypeExpr::Named(name) => {
            // Check if the name refers to another alias (alias chains are forbidden)
            if let Some((kind, _)) = ctx.decl_map.get(name) {
                if *kind == DeclKind::Alias {
                    diags.push(Diagnostic::error(
                        alias.target.span,
                        ErrorClass::AliasTargetIsAlias,
                        format!(
                            "alias `{}` references another alias `{name}`; \
                             must reference a terminal type directly",
                            alias.name.node
                        ),
                    ));
                }
            } else if !ctx.is_known_type(name) {
                diags.push(Diagnostic::error(
                    alias.target.span,
                    ErrorClass::AliasTargetNotFound,
                    format!("alias target type `{name}` not found",),
                ));
            }
        }
        _ => {
            // Non-named types (primitives, containers, etc.) are always valid targets
        }
    }

    // Check 3: Alias cycles (A = B, B = A)
    // We need to detect cycles where aliases reference each other.
    // Since we already forbid alias-to-alias references above, a cycle can only
    // happen through indirect means, but let's be thorough.
    check_alias_cycles(alias, ctx, diags);
}

/// Detect alias cycles using DFS.
/// Since alias-to-alias references are already forbidden, this is mainly
/// for completeness and to catch any edge cases.
fn check_alias_cycles(
    alias: &crate::ast::AliasDecl,
    ctx: &ValidationContext<'_>,
    diags: &mut Vec<Diagnostic>,
) {
    let mut path: HashSet<&SmolStr> = HashSet::new();
    path.insert(&alias.name.node);

    // Follow the target chain
    let mut current: &TypeExpr = &alias.target.node;
    while let TypeExpr::Named(name) = current {
        // Check if we've seen this name before (cycle detected)
        if path.contains(name) {
            if name.as_str() == alias.name.node.as_str() {
                diags.push(Diagnostic::error(
                    alias.target.span,
                    ErrorClass::AliasCycleDetected,
                    format!("alias `{}` forms a cycle", alias.name.node),
                ));
            }
            break;
        }

        // If the target is another alias, continue following
        if let Some(target_expr) = ctx.alias_targets.get(name) {
            path.insert(name);
            current = target_expr;
        } else {
            // Not an alias, chain ends here
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// Const
// ---------------------------------------------------------------------------

fn check_const(const_decl: &ConstDecl, _ctx: &ValidationContext<'_>, diags: &mut Vec<Diagnostic>) {
    // Check 1: Type must be integral (u8-u64, i8-i64, fixed32, fixed64) or bool
    let is_valid_const_type = match &const_decl.ty.node {
        TypeExpr::Primitive(p) => matches!(
            p,
            PrimitiveType::U8
                | PrimitiveType::U16
                | PrimitiveType::U32
                | PrimitiveType::U64
                | PrimitiveType::I8
                | PrimitiveType::I16
                | PrimitiveType::I32
                | PrimitiveType::I64
                | PrimitiveType::Fixed32
                | PrimitiveType::Fixed64
                | PrimitiveType::Bool
        ),
        TypeExpr::SubByte(_) => true, // Sub-byte types are valid
        _ => false,
    };

    if !is_valid_const_type {
        diags.push(Diagnostic::error(
            const_decl.ty.span,
            ErrorClass::ConstTypeInvalid,
            format!(
                "const `{}` has invalid type; must be integral or bool",
                const_decl.name.node
            ),
        ));
    }

    // Check 2: Collect all const declarations for dependency analysis
    // This is done at the schema level, so we just validate this one const's expression
    // The full cycle detection is done at the schema level
}

/// Evaluate const expressions and detect cycles.
/// Returns a map of const name -> evaluated value.
pub fn evaluate_consts(schema: &Schema, diags: &mut Vec<Diagnostic>) -> HashMap<SmolStr, i64> {
    let mut result = HashMap::new();

    // Collect all const declarations
    let mut const_map: HashMap<&SmolStr, &ConstDecl> = HashMap::new();
    for decl_spanned in &schema.declarations {
        if let Decl::Const(c) = &decl_spanned.node {
            const_map.insert(&c.name.node, c);
        }
    }

    if const_map.is_empty() {
        return result;
    }

    // Build dependency graph
    let mut deps: HashMap<&SmolStr, Vec<&SmolStr>> = HashMap::new();
    for (name, c) in &const_map {
        let mut refs = Vec::new();
        collect_const_refs(&c.value.node, &mut refs);
        deps.insert(name, refs);
    }

    // Topological sort using Kahn's algorithm
    let mut in_degree: HashMap<&SmolStr, usize> = HashMap::new();
    for name in const_map.keys() {
        in_degree.insert(name, 0);
    }

    for (name, refs) in &deps {
        for ref_name in refs {
            if const_map.contains_key(ref_name) {
                *in_degree.entry(name).or_insert(0) += 1;
            }
        }
    }

    let mut queue: Vec<&SmolStr> = in_degree
        .iter()
        .filter(|(_, &count)| count == 0)
        .map(|(name, _)| *name)
        .collect();

    let mut eval_order: Vec<&SmolStr> = Vec::new();

    while let Some(name) = queue.pop() {
        eval_order.push(name);

        // Find all consts that depend on this one
        for (other_name, other_deps) in &deps {
            if other_deps.contains(&name) {
                let count = in_degree.get_mut(other_name).unwrap();
                *count -= 1;
                if *count == 0 {
                    queue.push(other_name);
                }
            }
        }
    }

    // Check for cycles (nodes not in eval_order have circular dependencies)
    if eval_order.len() != const_map.len() {
        for name in const_map.keys() {
            if !eval_order.contains(name) {
                // Find the const declaration to get its span
                if let Some(c) = const_map.get(name) {
                    diags.push(Diagnostic::error(
                        c.name.span,
                        ErrorClass::ConstCycleDetected,
                        format!("const `{}` has circular dependency", name),
                    ));
                }
            }
        }
        return result;
    }

    // Evaluate in dependency order
    for name in eval_order {
        if let Some(c) = const_map.get(name) {
            match eval_const_expr(&c.value.node, &result, diags) {
                Some(value) => {
                    result.insert((*name).clone(), value);
                }
                None => {
                    // Error already reported
                }
            }
        }
    }

    result
}

/// Collect all const references from an expression.
fn collect_const_refs<'a>(expr: &'a ConstExpr, refs: &mut Vec<&'a SmolStr>) {
    match expr {
        ConstExpr::ConstRef(name) => refs.push(name),
        ConstExpr::BinOp { left, right, .. } => {
            collect_const_refs(left, refs);
            collect_const_refs(right, refs);
        }
        _ => {}
    }
}

/// Evaluate a constant expression.
fn eval_const_expr(
    expr: &ConstExpr,
    values: &HashMap<SmolStr, i64>,
    diags: &mut Vec<Diagnostic>,
) -> Option<i64> {
    match expr {
        ConstExpr::Int(v) => Some(*v),
        ConstExpr::UInt(v) => Some(*v as i64),
        ConstExpr::Hex(v) => Some(*v as i64),
        ConstExpr::ConstRef(name) => {
            if let Some(&val) = values.get(name) {
                Some(val)
            } else {
                // Reference not found - will be caught as cycle or undefined ref
                None
            }
        }
        ConstExpr::BinOp { op, left, right } => {
            let left_val = eval_const_expr(left, values, diags)?;
            let right_val = eval_const_expr(right, values, diags)?;

            match op {
                BinOpKind::Add => Some(left_val + right_val),
                BinOpKind::Sub => Some(left_val - right_val),
                BinOpKind::Mul => Some(left_val * right_val),
                BinOpKind::Div => {
                    if right_val == 0 {
                        diags.push(Diagnostic::error(
                            Span::empty(0), // TODO: get proper span
                            ErrorClass::ConstDivByZero,
                            "division by zero in constant expression",
                        ));
                        None
                    } else {
                        Some(left_val / right_val)
                    }
                }
            }
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// Impl validation
// ---------------------------------------------------------------------------

fn check_impl(impl_decl: &ImplDecl, ctx: &ValidationContext<'_>, diags: &mut Vec<Diagnostic>) {
    let target_name = &impl_decl.target_type.node;
    let trait_name = &impl_decl.trait_name.node;

    // Check: target type must exist
    if !ctx.decl_map.contains_key(target_name) && !ctx.imported_names.contains(target_name) {
        diags.push(Diagnostic::error(
            impl_decl.target_type.span,
            ErrorClass::UnknownType,
            format!("unknown type '{target_name}' in impl"),
        ));
    }

    // Check: trait must exist and be a trait
    match ctx.decl_map.get(trait_name) {
        Some((DeclKind::Trait, _)) => {} // OK
        Some((kind, _)) => {
            diags.push(Diagnostic::error(
                impl_decl.trait_name.span,
                ErrorClass::UnknownType,
                format!("'{trait_name}' is a {kind:?}, not a trait"),
            ));
        }
        None => {
            if !ctx.imported_names.contains(trait_name) {
                diags.push(Diagnostic::error(
                    impl_decl.trait_name.span,
                    ErrorClass::UnknownType,
                    format!("unknown trait '{trait_name}'"),
                ));
            }
        }
    }
}

// Where clause validation
// ---------------------------------------------------------------------------

fn check_where_clause(
    where_expr: &crate::span::Spanned<WhereExpr>,
    field_type: &TypeExpr,
    ctx: &ValidationContext<'_>,
    diags: &mut Vec<Diagnostic>,
) {
    validate_where_expr(where_expr, field_type, ctx, diags);
}

fn validate_where_expr(
    expr: &crate::span::Spanned<WhereExpr>,
    field_type: &TypeExpr,
    ctx: &ValidationContext<'_>,
    diags: &mut Vec<Diagnostic>,
) {
    match &expr.node {
        WhereExpr::And(left, right) | WhereExpr::Or(left, right) => {
            validate_where_expr(left, field_type, ctx, diags);
            validate_where_expr(right, field_type, ctx, diags);
        }
        WhereExpr::Not(inner) => {
            validate_where_expr(inner, field_type, ctx, diags);
        }
        WhereExpr::Cmp { op, operand } => {
            validate_where_operand(operand, field_type, ctx, diags, "comparison");
            // Check operator is valid for the type
            validate_cmp_operator(*op, field_type, expr.span, diags);
        }
        WhereExpr::Range {
            low,
            high,
            exclusive_high: _,
        } => {
            validate_where_operand(low, field_type, ctx, diags, "range lower bound");
            validate_where_operand(high, field_type, ctx, diags, "range upper bound");
            // Range is only valid on numeric types
            if !is_numeric_type(field_type) {
                let actual = format_type_expr(field_type);
                let valid = vec![
                    "u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64", "f32", "f64", "fixed32",
                    "fixed64",
                ];
                diags.push(
                    Diagnostic::error(
                        expr.span,
                        ErrorClass::WhereClauseRangeInvalid,
                        format!("range constraint `..` cannot be used with type `{actual}`"),
                    )
                    .with_note(Note::ValidOptions(valid.iter().map(|s| s.to_string()).collect()))
                    .with_help("range constraints require ordering comparison, which is only defined for numeric types"),
                );
            }
        }
        WhereExpr::LenCmp { op: _, operand } => {
            // len() is only valid on string, bytes, and arrays
            if !is_collection_type(field_type) {
                let actual = format_type_expr(field_type);
                let valid = ["string", "bytes", "array<T>", "fixed_array<T, N>"];
                diags.push(
                    Diagnostic::error(
                        expr.span,
                        ErrorClass::WhereClauseLenOnNonCollection,
                        format!("len() constraint cannot be used with type `{actual}`"),
                    )
                    .with_note(Note::ValidOptions(
                        valid.iter().map(|s| s.to_string()).collect(),
                    ))
                    .with_help("len() returns the element count of a collection"),
                );
            }
            validate_where_operand(
                operand,
                &TypeExpr::Primitive(PrimitiveType::U64),
                ctx,
                diags,
                "len comparison",
            );
        }
        WhereExpr::LenRange {
            low,
            high,
            exclusive_high: _,
        } => {
            // len() is only valid on string, bytes, and arrays
            if !is_collection_type(field_type) {
                let actual = format_type_expr(field_type);
                let valid = ["string", "bytes", "array<T>", "fixed_array<T, N>"];
                diags.push(
                    Diagnostic::error(
                        expr.span,
                        ErrorClass::WhereClauseLenOnNonCollection,
                        format!("len() range constraint cannot be used with type `{actual}`"),
                    )
                    .with_note(Note::ValidOptions(
                        valid.iter().map(|s| s.to_string()).collect(),
                    ))
                    .with_help("len() returns the element count of a collection"),
                );
            }
            validate_where_operand(
                low,
                &TypeExpr::Primitive(PrimitiveType::U64),
                ctx,
                diags,
                "len range lower bound",
            );
            validate_where_operand(
                high,
                &TypeExpr::Primitive(PrimitiveType::U64),
                ctx,
                diags,
                "len range upper bound",
            );
        }
    }
}

fn validate_where_operand(
    operand: &crate::span::Spanned<WhereOperand>,
    field_type: &TypeExpr,
    ctx: &ValidationContext<'_>,
    diags: &mut Vec<Diagnostic>,
    context: &str,
) {
    match &operand.node {
        WhereOperand::ConstRef(name) => {
            // Check that the const exists
            if !ctx.const_map.contains_key(name) && !ctx.imported_names.contains(name) {
                // Collect available const names for suggestions
                let available_consts: Vec<&str> = ctx
                    .const_map
                    .keys()
                    .map(|k| k.as_str())
                    .chain(ctx.imported_names.iter().map(|k| k.as_str()))
                    .collect();

                let mut diag = Diagnostic::error(
                    operand.span,
                    ErrorClass::WhereClauseConstRefNotFound,
                    format!("const reference `{name}` not found in where clause"),
                );

                if let Some(suggestion) =
                    find_closest_match(name.as_str(), available_consts.iter().copied())
                {
                    diag = diag.with_suggestion(suggestion);
                }

                if !available_consts.is_empty() {
                    diag = diag.with_help(format!(
                        "available constants: {}",
                        available_consts.join(", ")
                    ));
                }

                diags.push(diag);
            }
        }
        WhereOperand::Int(_) => {
            // Integers are valid for numeric types
            if !is_numeric_type(field_type) {
                let expected = "numeric type (u8-u64, i8-i64, fixed32, fixed64)".to_string();
                let actual = format_type_expr(field_type);
                diags.push(
                    Diagnostic::error(
                        operand.span,
                        ErrorClass::WhereClauseTypeMismatch,
                        format!("integer literal in `{context}` requires a numeric field type"),
                    )
                    .with_expected_vs_actual(expected, actual)
                    .with_help("use a numeric field type or change the constraint operand"),
                );
            }
        }
        WhereOperand::Float(_) => {
            // Floats are only valid for f32/f64
            let expected = "f32 or f64".to_string();
            let actual = format_type_expr(field_type);
            if !matches!(
                field_type,
                TypeExpr::Primitive(PrimitiveType::F32 | PrimitiveType::F64)
            ) {
                diags.push(
                    Diagnostic::error(
                        operand.span,
                        ErrorClass::WhereClauseTypeMismatch,
                        format!("float literal in `{context}` requires an f32 or f64 field type"),
                    )
                    .with_expected_vs_actual(expected, actual),
                );
            }
        }
        WhereOperand::String(_) => {
            // Strings are only valid for string types
            let expected = "string".to_string();
            let actual = format_type_expr(field_type);
            if !matches!(field_type, TypeExpr::Semantic(SemanticType::String)) {
                diags.push(
                    Diagnostic::error(
                        operand.span,
                        ErrorClass::WhereClauseTypeMismatch,
                        format!("string literal in `{context}` requires a string field type"),
                    )
                    .with_expected_vs_actual(expected, actual),
                );
            }
        }
        WhereOperand::Bool(_) => {
            // Bools are only valid for bool types
            let expected = "bool".to_string();
            let actual = format_type_expr(field_type);
            if !matches!(field_type, TypeExpr::Primitive(PrimitiveType::Bool)) {
                diags.push(
                    Diagnostic::error(
                        operand.span,
                        ErrorClass::WhereClauseTypeMismatch,
                        format!("boolean literal in `{context}` requires a bool field type"),
                    )
                    .with_expected_vs_actual(expected, actual),
                );
            }
        }
        WhereOperand::Value => {
            // This shouldn't appear as an operand since `value` is implicit
        }
    }
}

fn validate_cmp_operator(
    op: CmpOp,
    field_type: &TypeExpr,
    span: crate::span::Span,
    diags: &mut Vec<Diagnostic>,
) {
    // Ordering comparisons (<, >, <=, >=) are only valid on numeric types
    if matches!(op, CmpOp::Lt | CmpOp::Gt | CmpOp::Le | CmpOp::Ge) && !is_numeric_type(field_type) {
        let expected = "numeric type".to_string();
        let actual = format_type_expr(field_type);
        let valid_ops = match field_type {
            TypeExpr::Primitive(PrimitiveType::Bool) => vec!["==", "!="],
            TypeExpr::Semantic(SemanticType::String) => vec!["==", "!="],
            _ => vec!["==", "!="],
        };
        diags.push(
            Diagnostic::error(
                span,
                ErrorClass::WhereClauseOperatorInvalid,
                format!("ordering comparison `{:?}` cannot be used with type `{actual}`", op),
            )
            .with_expected_vs_actual(expected, actual)
            .with_note(Note::ValidOptions(valid_ops.iter().map(|s| s.to_string()).collect()))
            .with_help("ordering comparisons (<, >, <=, >=) require a total order, which is only defined for numeric types"),
        );
    }
}

fn is_numeric_type(ty: &TypeExpr) -> bool {
    match ty {
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
            | PrimitiveType::Fixed32
            | PrimitiveType::Fixed64,
        ) => true,
        TypeExpr::SubByte(_) => true,
        TypeExpr::Optional(inner) => is_numeric_type(&inner.node),
        _ => false,
    }
}

fn is_collection_type(ty: &TypeExpr) -> bool {
    match ty {
        TypeExpr::Semantic(SemanticType::String | SemanticType::Bytes) => true,
        TypeExpr::Array(_) => true,
        TypeExpr::FixedArray(_, _) => true,
        TypeExpr::Optional(inner) => is_collection_type(&inner.node),
        _ => false,
    }
}

/// Check if a name is a valid field name (starts with lowercase or underscore,
/// contains only alphanumeric characters and underscores).
fn is_valid_field_name(name: &SmolStr) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Format a type expression for display in error messages.
fn format_type_expr(ty: &TypeExpr) -> String {
    match ty {
        TypeExpr::Primitive(p) => format!("{:?}", p).to_lowercase(),
        TypeExpr::SubByte(s) => format!("{:?}", s).to_lowercase(),
        TypeExpr::Semantic(s) => format!("{:?}", s).to_lowercase(),
        TypeExpr::Named(name) => name.to_string(),
        TypeExpr::Qualified(ns, name) => format!("{}.{}", ns, name),
        TypeExpr::Optional(inner) => format!("optional<{}>", format_type_expr(&inner.node)),
        TypeExpr::Array(inner) => format!("array<{}>", format_type_expr(&inner.node)),
        TypeExpr::FixedArray(inner, size) => {
            format!("fixed_array<{}, {}>", format_type_expr(&inner.node), size)
        }
        TypeExpr::Set(inner) => format!("set<{}>", format_type_expr(&inner.node)),
        TypeExpr::Map(key, value) => format!(
            "map<{}, {}>",
            format_type_expr(&key.node),
            format_type_expr(&value.node)
        ),
        TypeExpr::Result(ok, err) => format!(
            "result<{}, {}>",
            format_type_expr(&ok.node),
            format_type_expr(&err.node)
        ),
        TypeExpr::Vec2(inner) => format!("vec2<{}>", format_type_expr(&inner.node)),
        TypeExpr::Vec3(inner) => format!("vec3<{}>", format_type_expr(&inner.node)),
        TypeExpr::Vec4(inner) => format!("vec4<{}>", format_type_expr(&inner.node)),
        TypeExpr::Quat(inner) => format!("quat<{}>", format_type_expr(&inner.node)),
        TypeExpr::Mat3(inner) => format!("mat3<{}>", format_type_expr(&inner.node)),
        TypeExpr::Mat4(inner) => format!("mat4<{}>", format_type_expr(&inner.node)),
        TypeExpr::Generic(name, arg) => format!("{}<{}>", name, format_type_expr(&arg.node)),
        TypeExpr::BitsInline(names) => format!("bits_inline<{}>", names.join(", ")),
    }
}
