use std::collections::HashSet;

use smol_str::SmolStr;

use crate::ast::{
    Annotation, AnnotationValue, Decl, EnumBacking, EnumBodyItem, FlagsBodyItem, ImportKind,
    MessageBodyItem, MessageField, Schema, TypeExpr, UnionBodyItem,
};
use crate::diagnostic::{Diagnostic, ErrorClass};
use crate::ir::{
    self, CompiledSchema, ConfigDef, ConfigFieldDef, Encoding, EnumDef, EnumVariantDef, FieldDef,
    FieldEncoding, FlagsBitDef, FlagsDef, MessageDef, NewtypeDef, ResolvedAnnotations,
    ResolvedType, TombstoneDef, TypeDef, TypeId, TypeRegistry, UnionDef, UnionVariantDef,
};
use crate::span::Span;

struct LowerCtx {
    registry: TypeRegistry,
    diagnostics: Vec<Diagnostic>,
    wildcard_imports: HashSet<SmolStr>,
}

impl LowerCtx {
    fn new() -> Self {
        Self {
            registry: TypeRegistry::new(),
            diagnostics: Vec::new(),
            wildcard_imports: HashSet::new(),
        }
    }

    fn emit(&mut self, span: Span, class: ErrorClass, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error(span, class, message));
    }
}

pub fn lower(schema: &Schema) -> (Option<CompiledSchema>, Vec<Diagnostic>) {
    let mut ctx = LowerCtx::new();

    register_import_stubs(schema, &mut ctx);
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

fn register_import_stubs(schema: &Schema, ctx: &mut LowerCtx) {
    for imp in &schema.imports {
        match &imp.node.kind {
            ImportKind::Wildcard => {
                let ns_path: SmolStr = imp
                    .node
                    .path
                    .iter()
                    .map(|s| s.node.as_str())
                    .collect::<Vec<_>>()
                    .join(".")
                    .into();
                ctx.wildcard_imports.insert(ns_path);
            }
            ImportKind::Named { names } => {
                for name in names {
                    ctx.registry.register_stub(name.node.clone());
                }
            }
            ImportKind::Aliased { alias } => {
                ctx.registry.register_stub(alias.node.clone());
            }
        }
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
    let backing = en
        .backing
        .as_ref()
        .map(|b| b.node.clone())
        .unwrap_or(EnumBacking::U32);
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
