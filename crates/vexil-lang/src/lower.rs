//! # Stability: Tier 2
//!
//! Lowering pass: transforms the source-faithful AST into the compiler IR.
//!
//! Resolves type references, registers declarations in a [`TypeRegistry`],
//! and populates import types from an optional `DependencyContext`.

use std::collections::{HashMap, HashSet};

use smol_str::SmolStr;

use crate::ast::{
    Annotation, AnnotationValue, Decl, EnumBodyItem, FlagsBodyItem, ImportKind, MessageBodyItem,
    MessageField, Schema, TypeExpr, UnionBodyItem,
};
use crate::diagnostic::{Diagnostic, ErrorClass};
use crate::ir::{
    self, CompiledSchema, ConfigDef, ConfigFieldDef, Encoding, EnumDef, EnumVariantDef, FieldDef,
    FieldEncoding, FlagsBitDef, FlagsDef, MessageDef, NewtypeDef, ResolvedAnnotations,
    ResolvedType, TombstoneDef, TypeDef, TypeId, TypeRegistry, UnionDef, UnionVariantDef,
};
use crate::span::Span;

/// Pre-compiled dependency information for multi-file compilation.
pub struct DependencyContext {
    /// Maps import namespace string (e.g. "dep.types") to its compiled schema.
    pub schemas: HashMap<String, CompiledSchema>,
}

struct LowerCtx {
    registry: TypeRegistry,
    diagnostics: Vec<Diagnostic>,
    wildcard_imports: HashSet<SmolStr>,
    /// Maps type name → source namespace for wildcard imports.
    /// `None` means ambiguous (multiple wildcards provide this name).
    wildcard_origins: HashMap<SmolStr, Option<String>>,
    /// Names from local declarations (populated during `register_declarations`).
    local_names: HashSet<SmolStr>,
}

impl LowerCtx {
    fn new() -> Self {
        Self {
            registry: TypeRegistry::new(),
            diagnostics: Vec::new(),
            wildcard_imports: HashSet::new(),
            wildcard_origins: HashMap::new(),
            local_names: HashSet::new(),
        }
    }

    fn emit(&mut self, span: Span, class: ErrorClass, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error(span, class, message));
    }
}

/// Lower a parsed AST into the IR without any dependency context.
///
/// This is the single-file entry point. For multi-file compilation with
/// import resolution, use [`lower_with_deps`].
pub fn lower(schema: &Schema) -> (Option<CompiledSchema>, Vec<Diagnostic>) {
    lower_with_deps(schema, None)
}

/// Lower a parsed AST into the IR, resolving imports against the given dependencies.
pub fn lower_with_deps(
    schema: &Schema,
    deps: Option<&DependencyContext>,
) -> (Option<CompiledSchema>, Vec<Diagnostic>) {
    let mut ctx = LowerCtx::new();

    register_import_types(schema, &mut ctx, deps);
    let decl_ids = register_declarations(schema, &mut ctx);

    for (decl_spanned, &id) in schema.declarations.iter().zip(decl_ids.iter()) {
        let def = lower_decl(&decl_spanned.node, decl_spanned.span, &mut ctx);
        // Replace the placeholder registered earlier.
        // get_mut returns Some because we registered with a concrete TypeDef, not a stub.
        if let Some(slot) = ctx.registry.get_mut(id) {
            *slot = def;
        }
    }

    let namespace: Vec<SmolStr> = schema
        .namespace
        .as_ref()
        .map(|ns| ns.node.path.iter().map(|s| s.node.clone()).collect())
        .unwrap_or_default();

    let annotations = resolve_annotations(&schema.annotations);

    let compiled = CompiledSchema {
        namespace,
        annotations,
        registry: ctx.registry,
        declarations: decl_ids,
    };

    (Some(compiled), ctx.diagnostics)
}

fn register_import_types(schema: &Schema, ctx: &mut LowerCtx, deps: Option<&DependencyContext>) {
    for imp in &schema.imports {
        let ns_key: String = imp
            .node
            .path
            .iter()
            .map(|s| s.node.as_str())
            .collect::<Vec<_>>()
            .join(".");

        // Emit warning for version constraints (deferred to Milestone G).
        if let Some(ref ver) = imp.node.version {
            ctx.diagnostics.push(Diagnostic::warning(
                ver.span,
                ErrorClass::UnexpectedToken,
                format!(
                    "version constraints are not yet enforced; ignoring `@ {}`",
                    ver.node
                ),
            ));
        }

        match deps.and_then(|d| d.schemas.get(&ns_key)) {
            Some(dep_compiled) => {
                // Real dependency available — inject types by import kind.
                match &imp.node.kind {
                    ImportKind::Named { names } => {
                        for name_spanned in names {
                            let name = &name_spanned.node;
                            let found = dep_compiled.declarations.iter().find(|&&id| {
                                dep_compiled
                                    .registry
                                    .get(id)
                                    .map(|d| crate::remap::type_def_name(d) == name.as_str())
                                    .unwrap_or(false)
                            });
                            match found {
                                Some(&id) => {
                                    crate::remap::clone_types_into(
                                        &dep_compiled.registry,
                                        &[id],
                                        &mut ctx.registry,
                                    );
                                }
                                None => {
                                    ctx.emit(
                                        name_spanned.span,
                                        ErrorClass::UnresolvedType,
                                        format!(
                                            "imported name `{name}` not found in namespace `{ns_key}`"
                                        ),
                                    );
                                }
                            }
                        }
                    }
                    ImportKind::Wildcard => {
                        crate::remap::clone_types_into(
                            &dep_compiled.registry,
                            &dep_compiled.declarations,
                            &mut ctx.registry,
                        );
                        // Track wildcard origins for collision detection.
                        for &old_id in &dep_compiled.declarations {
                            if let Some(def) = dep_compiled.registry.get(old_id) {
                                let type_name = SmolStr::new(crate::remap::type_def_name(def));
                                match ctx.wildcard_origins.get(&type_name) {
                                    None => {
                                        ctx.wildcard_origins
                                            .insert(type_name, Some(ns_key.clone()));
                                    }
                                    Some(Some(existing_ns)) if *existing_ns != ns_key => {
                                        // Ambiguous: same name from different namespaces.
                                        ctx.wildcard_origins.insert(type_name, None);
                                    }
                                    _ => {} // same namespace or already ambiguous
                                }
                            }
                        }
                        ctx.wildcard_imports.insert(SmolStr::new(&ns_key));
                    }
                    ImportKind::Aliased { alias } => {
                        // Clone all types with proper ID remapping, then rename
                        // to qualified "Alias.TypeName" form.
                        let id_map = crate::remap::clone_types_into(
                            &dep_compiled.registry,
                            &dep_compiled.declarations,
                            &mut ctx.registry,
                        );
                        // Collect renames first to avoid borrow conflicts.
                        let renames: Vec<_> = id_map
                            .values()
                            .filter_map(|&new_id| {
                                ctx.registry.get(new_id).map(|def| {
                                    let orig = crate::remap::type_def_name(def).to_owned();
                                    let qualified = format!("{}.{}", alias.node, orig);
                                    (new_id, orig, qualified)
                                })
                            })
                            .collect();
                        for (new_id, orig, qualified) in renames {
                            if let Some(def) = ctx.registry.get_mut(new_id) {
                                set_type_name(def, SmolStr::new(&qualified));
                            }
                            ctx.registry.rename(new_id, &orig, SmolStr::new(&qualified));
                        }
                    }
                }
            }
            None => {
                // No dependency context — fall back to stubs (existing behavior).
                match &imp.node.kind {
                    ImportKind::Named { names } => {
                        for name_spanned in names {
                            ctx.registry.register_stub(name_spanned.node.clone());
                        }
                    }
                    ImportKind::Wildcard => {
                        ctx.wildcard_imports.insert(SmolStr::new(&ns_key));
                    }
                    ImportKind::Aliased { alias } => {
                        ctx.registry.register_stub(alias.node.clone());
                    }
                }
            }
        }
    }
}

fn set_type_name(def: &mut TypeDef, name: SmolStr) {
    match def {
        TypeDef::Message(m) => m.name = name,
        TypeDef::Enum(e) => e.name = name,
        TypeDef::Flags(f) => f.name = name,
        TypeDef::Union(u) => u.name = name,
        TypeDef::Newtype(n) => n.name = name,
        TypeDef::Config(c) => c.name = name,
    }
}

fn register_declarations(schema: &Schema, ctx: &mut LowerCtx) -> Vec<TypeId> {
    let mut ids = Vec::new();
    for decl_spanned in &schema.declarations {
        let name = match &decl_spanned.node {
            Decl::Message(d) => d.name.node.clone(),
            Decl::Enum(d) => d.name.node.clone(),
            Decl::Flags(d) => d.name.node.clone(),
            Decl::Union(d) => d.name.node.clone(),
            Decl::Newtype(d) => d.name.node.clone(),
            Decl::Config(d) => d.name.node.clone(),
        };
        ctx.local_names.insert(name.clone());
        // Register a concrete placeholder so get_mut will find it later.
        // Safe: placeholder is overwritten in the lowering loop before any code reads it.
        // All declarations are registered before any are lowered (forward pass).
        let placeholder = TypeDef::Message(MessageDef {
            name: name.clone(),
            span: decl_spanned.span,
            fields: Vec::new(),
            tombstones: Vec::new(),
            annotations: ResolvedAnnotations::default(),
            wire_size: None,
        });
        let id = ctx.registry.register(name, placeholder);
        ids.push(id);
    }
    ids
}

fn lower_decl(decl: &Decl, span: Span, ctx: &mut LowerCtx) -> TypeDef {
    match decl {
        Decl::Message(d) => TypeDef::Message(lower_message(d, span, ctx)),
        Decl::Enum(d) => TypeDef::Enum(lower_enum(d, span, ctx)),
        Decl::Flags(d) => TypeDef::Flags(lower_flags(d, span, ctx)),
        Decl::Union(d) => TypeDef::Union(lower_union(d, span, ctx)),
        Decl::Newtype(d) => TypeDef::Newtype(lower_newtype(d, span, ctx)),
        Decl::Config(d) => TypeDef::Config(lower_config(d, span, ctx)),
    }
}

fn lower_message(msg: &crate::ast::MessageDecl, span: Span, ctx: &mut LowerCtx) -> MessageDef {
    let mut fields = Vec::new();
    let mut tombstones = Vec::new();
    for item in &msg.body {
        match item {
            MessageBodyItem::Field(f) => fields.push(lower_field(&f.node, f.span, ctx)),
            MessageBodyItem::Tombstone(t) => tombstones.push(lower_tombstone(&t.node, t.span)),
        }
    }
    MessageDef {
        name: msg.name.node.clone(),
        span,
        fields,
        tombstones,
        annotations: resolve_annotations(&msg.annotations),
        wire_size: None,
    }
}

fn lower_field(field: &MessageField, span: Span, ctx: &mut LowerCtx) -> FieldDef {
    let resolved_type = resolve_type_expr(&field.ty.node, field.ty.span, ctx);
    let all_annotations: Vec<&Annotation> = field
        .pre_annotations
        .iter()
        .chain(field.post_ordinal_annotations.iter())
        .chain(field.post_type_annotations.iter())
        .collect();
    let encoding = compute_field_encoding(&all_annotations);
    let annotations = resolve_annotations_refs(&all_annotations);
    FieldDef {
        name: field.name.node.clone(),
        span,
        ordinal: field.ordinal.node,
        resolved_type,
        encoding,
        annotations,
    }
}

fn lower_enum(en: &crate::ast::EnumDecl, span: Span, _ctx: &mut LowerCtx) -> EnumDef {
    // Preserve the explicit backing type as-is; None means auto-sized.
    let backing = en.backing.as_ref().map(|b| b.node.clone());
    let mut variants = Vec::new();
    let mut tombstones = Vec::new();
    for item in &en.body {
        match item {
            EnumBodyItem::Variant(v) => {
                variants.push(EnumVariantDef {
                    name: v.node.name.node.clone(),
                    span: v.span,
                    ordinal: v.node.ordinal.node,
                    annotations: resolve_annotations(&v.node.annotations),
                });
            }
            EnumBodyItem::Tombstone(t) => tombstones.push(lower_tombstone(&t.node, t.span)),
        }
    }
    EnumDef {
        name: en.name.node.clone(),
        span,
        backing,
        variants,
        tombstones,
        annotations: resolve_annotations(&en.annotations),
        wire_bits: 0, // filled in by typeck
    }
}

fn lower_flags(flags: &crate::ast::FlagsDecl, span: Span, _ctx: &mut LowerCtx) -> FlagsDef {
    let mut bits = Vec::new();
    let mut tombstones = Vec::new();
    for item in &flags.body {
        match item {
            FlagsBodyItem::Bit(b) => {
                bits.push(FlagsBitDef {
                    name: b.node.name.node.clone(),
                    span: b.span,
                    bit: b.node.ordinal.node,
                    annotations: resolve_annotations(&b.node.annotations),
                });
            }
            FlagsBodyItem::Tombstone(t) => tombstones.push(lower_tombstone(&t.node, t.span)),
        }
    }
    FlagsDef {
        name: flags.name.node.clone(),
        span,
        bits,
        tombstones,
        annotations: resolve_annotations(&flags.annotations),
        wire_bytes: 0, // filled in by typeck
    }
}

fn lower_union(un: &crate::ast::UnionDecl, span: Span, ctx: &mut LowerCtx) -> UnionDef {
    let mut variants = Vec::new();
    let mut top_tombstones = Vec::new();
    for item in &un.body {
        match item {
            UnionBodyItem::Variant(v) => {
                let mut fields = Vec::new();
                let mut tombstones = Vec::new();
                for body_item in &v.node.fields {
                    match body_item {
                        MessageBodyItem::Field(f) => fields.push(lower_field(&f.node, f.span, ctx)),
                        MessageBodyItem::Tombstone(t) => {
                            tombstones.push(lower_tombstone(&t.node, t.span))
                        }
                    }
                }
                variants.push(UnionVariantDef {
                    name: v.node.name.node.clone(),
                    span: v.span,
                    ordinal: v.node.ordinal.node,
                    fields,
                    tombstones,
                    annotations: resolve_annotations(&v.node.annotations),
                });
            }
            UnionBodyItem::Tombstone(t) => top_tombstones.push(lower_tombstone(&t.node, t.span)),
        }
    }
    UnionDef {
        name: un.name.node.clone(),
        span,
        variants,
        tombstones: top_tombstones,
        annotations: resolve_annotations(&un.annotations),
        wire_size: None,
    }
}

fn lower_newtype(nt: &crate::ast::NewtypeDecl, span: Span, ctx: &mut LowerCtx) -> NewtypeDef {
    let inner_type = resolve_type_expr(&nt.inner_type.node, nt.inner_type.span, ctx);
    // TODO(typeck): resolve terminal_type through newtype chains.
    // Currently safe because validate.rs rejects newtype-over-newtype.
    let terminal_type = inner_type.clone();
    NewtypeDef {
        name: nt.name.node.clone(),
        span,
        inner_type,
        terminal_type,
        annotations: resolve_annotations(&nt.annotations),
    }
}

fn lower_config(cfg: &crate::ast::ConfigDecl, span: Span, ctx: &mut LowerCtx) -> ConfigDef {
    let fields = cfg
        .fields
        .iter()
        .map(|f| {
            let resolved_type = resolve_type_expr(&f.node.ty.node, f.node.ty.span, ctx);
            ConfigFieldDef {
                name: f.node.name.node.clone(),
                span: f.span,
                resolved_type,
                default_value: f.node.default_value.node.clone(),
                annotations: resolve_annotations(&f.node.annotations),
            }
        })
        .collect();
    ConfigDef {
        name: cfg.name.node.clone(),
        span,
        fields,
        annotations: resolve_annotations(&cfg.annotations),
    }
}

fn resolve_type_expr(expr: &TypeExpr, span: Span, ctx: &mut LowerCtx) -> ResolvedType {
    match expr {
        TypeExpr::Primitive(p) => ResolvedType::Primitive(*p),
        TypeExpr::SubByte(s) => ResolvedType::SubByte(*s),
        TypeExpr::Semantic(s) => ResolvedType::Semantic(*s),
        TypeExpr::Named(name) => {
            // 1. Local declarations always win (shadow wildcards).
            if ctx.local_names.contains(name.as_str()) {
                if let Some(id) = ctx.registry.lookup(name.as_str()) {
                    return ResolvedType::Named(id);
                }
            }
            // 2. Check for wildcard ambiguity (only for non-local names).
            if let Some(origin) = ctx.wildcard_origins.get(name.as_str()) {
                if origin.is_none() {
                    ctx.emit(
                        span,
                        ErrorClass::UnresolvedType,
                        format!(
                            "ambiguous type `{name}`: provided by multiple wildcard imports; \
                             use a named or aliased import to disambiguate"
                        ),
                    );
                    return ResolvedType::Named(ir::types::POISON_TYPE_ID);
                }
            }
            // 3. Normal lookup (named imports or unambiguous wildcard types).
            if let Some(id) = ctx.registry.lookup(name.as_str()) {
                ResolvedType::Named(id)
            } else if !ctx.wildcard_imports.is_empty() {
                // Could be from a wildcard import — register a stub.
                let id = ctx.registry.register_stub(name.clone());
                ResolvedType::Named(id)
            } else {
                ctx.emit(
                    span,
                    ErrorClass::UnresolvedType,
                    format!("unresolved type `{name}`"),
                );
                ResolvedType::Named(ir::types::POISON_TYPE_ID)
            }
        }
        TypeExpr::Qualified(ns, name) => {
            let qualified_name: SmolStr = format!("{ns}.{name}").into();
            if let Some(id) = ctx.registry.lookup(qualified_name.as_str()) {
                ResolvedType::Named(id)
            } else if ctx.registry.lookup(ns.as_str()).is_some() {
                // Namespace alias is known — register qualified stub.
                let id = ctx.registry.register_stub(qualified_name);
                ResolvedType::Named(id)
            } else {
                ctx.emit(
                    span,
                    ErrorClass::UnresolvedType,
                    format!("unresolved qualified type `{ns}.{name}`"),
                );
                ResolvedType::Named(ir::types::POISON_TYPE_ID)
            }
        }
        TypeExpr::Optional(inner) => {
            ResolvedType::Optional(Box::new(resolve_type_expr(&inner.node, inner.span, ctx)))
        }
        TypeExpr::Array(inner) => {
            ResolvedType::Array(Box::new(resolve_type_expr(&inner.node, inner.span, ctx)))
        }
        TypeExpr::Map(key, value) => {
            let rk = resolve_type_expr(&key.node, key.span, ctx);
            let rv = resolve_type_expr(&value.node, value.span, ctx);
            ResolvedType::Map(Box::new(rk), Box::new(rv))
        }
        TypeExpr::Result(ok, err) => {
            let ro = resolve_type_expr(&ok.node, ok.span, ctx);
            let re = resolve_type_expr(&err.node, err.span, ctx);
            ResolvedType::Result(Box::new(ro), Box::new(re))
        }
    }
}

fn compute_field_encoding(annotations: &[&Annotation]) -> FieldEncoding {
    let has_varint = annotations.iter().any(|a| a.name.node == "varint");
    let has_zigzag = annotations.iter().any(|a| a.name.node == "zigzag");
    let has_delta = annotations.iter().any(|a| a.name.node == "delta");
    let base = if has_varint {
        Encoding::Varint
    } else if has_zigzag {
        Encoding::ZigZag
    } else {
        Encoding::Default
    };
    let encoding = if has_delta {
        Encoding::Delta(Box::new(base))
    } else {
        base
    };
    let limit = annotations.iter().find_map(|a| {
        if a.name.node == "limit" {
            a.args.as_ref().and_then(|args| {
                args.iter().find_map(|arg| {
                    if arg.key.is_none() {
                        match &arg.value.node {
                            AnnotationValue::Int(v) => Some(*v),
                            AnnotationValue::Hex(v) => Some(*v),
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
            })
        } else {
            None
        }
    });
    FieldEncoding { encoding, limit }
}

fn resolve_annotations(annotations: &[Annotation]) -> ResolvedAnnotations {
    let refs: Vec<&Annotation> = annotations.iter().collect();
    resolve_annotations_refs(&refs)
}

fn resolve_annotations_refs(annotations: &[&Annotation]) -> ResolvedAnnotations {
    let mut result = ResolvedAnnotations::default();
    for ann in annotations {
        match ann.name.node.as_str() {
            "deprecated" => {
                let reason = extract_string_arg(ann, "reason").unwrap_or_default();
                let since = extract_string_arg(ann, "since");
                result.deprecated = Some(ir::DeprecatedInfo { reason, since });
            }
            "since" => {
                result.since = extract_first_string_arg(ann);
            }
            "doc" => {
                if let Some(s) = extract_first_string_arg(ann) {
                    result.doc.push(s);
                }
            }
            "revision" => {
                result.revision = extract_first_int_arg(ann);
            }
            "non_exhaustive" => {
                result.non_exhaustive = true;
            }
            "version" => {
                result.version = extract_first_string_arg(ann);
            }
            _ => {}
        }
    }
    result
}

fn extract_string_arg(ann: &Annotation, key: &str) -> Option<SmolStr> {
    ann.args.as_ref().and_then(|args| {
        args.iter().find_map(|arg| {
            if arg.key.as_ref().is_some_and(|k| k.node == key) {
                match &arg.value.node {
                    AnnotationValue::Str(s) => Some(SmolStr::new(s)),
                    _ => None,
                }
            } else {
                None
            }
        })
    })
}

fn extract_first_string_arg(ann: &Annotation) -> Option<SmolStr> {
    ann.args.as_ref().and_then(|args| {
        args.first().and_then(|arg| match &arg.value.node {
            AnnotationValue::Str(s) => Some(SmolStr::new(s)),
            _ => None,
        })
    })
}

fn extract_first_int_arg(ann: &Annotation) -> Option<u64> {
    ann.args.as_ref().and_then(|args| {
        args.first().and_then(|arg| match &arg.value.node {
            AnnotationValue::Int(v) => Some(*v),
            AnnotationValue::Hex(v) => Some(*v),
            _ => None,
        })
    })
}

fn lower_tombstone(tombstone: &crate::ast::Tombstone, span: Span) -> TombstoneDef {
    let reason = tombstone
        .args
        .iter()
        .find_map(|arg| {
            if arg.key.node == "reason" {
                match &arg.value.node {
                    AnnotationValue::Str(s) => Some(SmolStr::new(s)),
                    _ => None,
                }
            } else {
                None
            }
        })
        .unwrap_or_else(|| SmolStr::new("(no reason)"));
    let since = tombstone.args.iter().find_map(|arg| {
        if arg.key.node == "since" {
            match &arg.value.node {
                AnnotationValue::Str(s) => Some(SmolStr::new(s)),
                _ => None,
            }
        } else {
            None
        }
    });
    TombstoneDef {
        span,
        ordinal: tombstone.ordinal.node,
        reason,
        since,
    }
}

#[cfg(test)]
mod dep_tests {
    use super::*;

    #[test]
    fn lower_with_dependency_resolves_named_import() {
        let dep_result = crate::compile("namespace dep.types\nmessage Foo { x @0 : u32 }");
        let dep_compiled = dep_result.compiled.unwrap();

        let root_source =
            "namespace root\nimport { Foo } from dep.types\nmessage Bar { f @0 : Foo }";
        let root_schema = crate::parse(root_source).schema.unwrap();

        let mut dep_ctx = DependencyContext {
            schemas: HashMap::new(),
        };
        dep_ctx
            .schemas
            .insert("dep.types".to_string(), dep_compiled);

        let (compiled, diags) = lower_with_deps(&root_schema, Some(&dep_ctx));
        assert!(compiled.is_some(), "should compile: {:?}", diags);
        let compiled = compiled.unwrap();
        for &id in &compiled.declarations {
            if let Some(TypeDef::Message(m)) = compiled.registry.get(id) {
                if m.name == "Bar" {
                    if let ResolvedType::Named(ref_id) = &m.fields[0].resolved_type {
                        assert!(
                            !compiled.registry.is_stub(*ref_id),
                            "Foo should not be a stub"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn local_declaration_shadows_wildcard() {
        let dep_result = crate::compile("namespace dep\nmessage Foo { x @0 : u32 }");
        let dep_compiled = dep_result.compiled.unwrap();

        let root_source =
            "namespace root\nimport dep\nmessage Foo { y @0 : string }\nmessage Bar { f @0 : Foo }";
        let root_schema = crate::parse(root_source).schema.unwrap();

        let mut dep_ctx = DependencyContext {
            schemas: HashMap::new(),
        };
        dep_ctx.schemas.insert("dep".to_string(), dep_compiled);

        let (compiled, diags) = lower_with_deps(&root_schema, Some(&dep_ctx));
        assert!(compiled.is_some());
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == crate::diagnostic::Severity::Error)
            .collect();
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    }

    #[test]
    fn named_import_nonexistent_type_errors() {
        let dep_result = crate::compile("namespace dep\nmessage Foo { x @0 : u32 }");
        let dep_compiled = dep_result.compiled.unwrap();

        let root_source = "namespace root\nimport { Bar } from dep\nmessage Baz { x @0 : u32 }";
        let root_schema = crate::parse(root_source).schema.unwrap();

        let mut dep_ctx = DependencyContext {
            schemas: HashMap::new(),
        };
        dep_ctx.schemas.insert("dep".to_string(), dep_compiled);

        let (_compiled, diags) = lower_with_deps(&root_schema, Some(&dep_ctx));
        assert!(diags.iter().any(|d| d.message.contains("not found")));
    }

    #[test]
    fn lower_without_deps_falls_back_to_stubs() {
        let source = "namespace root\nimport { Foo } from dep.types\nmessage Bar { f @0 : Foo }";
        let schema = crate::parse(source).schema.unwrap();

        let (compiled, _diags) = lower_with_deps(&schema, None);
        assert!(compiled.is_some());
        let compiled = compiled.unwrap();
        // Foo should be a stub when no deps provided.
        if let Some(id) = compiled.registry.lookup("Foo") {
            assert!(compiled.registry.is_stub(id), "Foo should be a stub");
        }
    }

    #[test]
    fn aliased_import_creates_qualified_names() {
        let dep_result = crate::compile("namespace dep.types\nmessage Foo { x @0 : u32 }");
        let dep_compiled = dep_result.compiled.unwrap();

        let root_source = "namespace root\nimport dep.types as DT\nmessage Bar { f @0 : DT.Foo }";
        let root_schema = crate::parse(root_source).schema.unwrap();

        let mut dep_ctx = DependencyContext {
            schemas: HashMap::new(),
        };
        dep_ctx
            .schemas
            .insert("dep.types".to_string(), dep_compiled);

        let (compiled, diags) = lower_with_deps(&root_schema, Some(&dep_ctx));
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == crate::diagnostic::Severity::Error)
            .collect();
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
        assert!(compiled.is_some());
        let compiled = compiled.unwrap();
        assert!(
            compiled.registry.lookup("DT.Foo").is_some(),
            "DT.Foo should be registered"
        );
    }

    #[test]
    fn wildcard_collision_on_use_errors() {
        let dep_a = crate::compile("namespace dep.a\nmessage Foo { x @0 : u32 }")
            .compiled
            .unwrap();
        let dep_b = crate::compile("namespace dep.b\nmessage Foo { y @0 : string }")
            .compiled
            .unwrap();

        let root_source = "namespace root\nimport dep.a\nimport dep.b\nmessage Bar { f @0 : Foo }";
        let root_schema = crate::parse(root_source).schema.unwrap();

        let mut dep_ctx = DependencyContext {
            schemas: HashMap::new(),
        };
        dep_ctx.schemas.insert("dep.a".to_string(), dep_a);
        dep_ctx.schemas.insert("dep.b".to_string(), dep_b);

        let (_compiled, diags) = lower_with_deps(&root_schema, Some(&dep_ctx));
        assert!(
            diags.iter().any(|d| d.message.contains("ambiguous")),
            "expected ambiguity error, got: {:?}",
            diags
        );
    }

    #[test]
    fn named_import_overrides_wildcard_ambiguity() {
        let dep_a = crate::compile("namespace dep.a\nmessage Foo { x @0 : u32 }")
            .compiled
            .unwrap();
        let dep_b = crate::compile("namespace dep.b\nmessage Foo { y @0 : string }")
            .compiled
            .unwrap();

        // Named import from dep.a, wildcard from dep.b — named should win.
        let root_source =
            "namespace root\nimport { Foo } from dep.a\nimport dep.b\nmessage Bar { f @0 : Foo }";
        let root_schema = crate::parse(root_source).schema.unwrap();

        let mut dep_ctx = DependencyContext {
            schemas: HashMap::new(),
        };
        dep_ctx.schemas.insert("dep.a".to_string(), dep_a);
        dep_ctx.schemas.insert("dep.b".to_string(), dep_b);

        let (compiled, diags) = lower_with_deps(&root_schema, Some(&dep_ctx));
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == crate::diagnostic::Severity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "named import should override wildcard ambiguity, got: {:?}",
            errors
        );
        assert!(compiled.is_some());
    }

    #[test]
    fn local_declaration_shadows_wildcard_collision() {
        let dep_a = crate::compile("namespace dep.a\nmessage Foo { x @0 : u32 }")
            .compiled
            .unwrap();
        let dep_b = crate::compile("namespace dep.b\nmessage Foo { y @0 : string }")
            .compiled
            .unwrap();

        // Local Foo declaration should shadow the ambiguous wildcards.
        let root_source = "namespace root\nimport dep.a\nimport dep.b\nmessage Foo { z @0 : bool }\nmessage Bar { f @0 : Foo }";
        let root_schema = crate::parse(root_source).schema.unwrap();

        let mut dep_ctx = DependencyContext {
            schemas: HashMap::new(),
        };
        dep_ctx.schemas.insert("dep.a".to_string(), dep_a);
        dep_ctx.schemas.insert("dep.b".to_string(), dep_b);

        let (compiled, diags) = lower_with_deps(&root_schema, Some(&dep_ctx));
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == crate::diagnostic::Severity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "local declaration should shadow wildcard collision, got: {:?}",
            errors
        );
        assert!(compiled.is_some());
    }
}
