//! # Stability: Tier 2
//!
//! Lowering pass: transforms the source-faithful AST into the compiler IR.
//!
//! Resolves type references, registers declarations in a [`TypeRegistry`],
//! and populates import types from an optional `DependencyContext`.

use std::collections::{HashMap, HashSet};

use smol_str::SmolStr;

use crate::ast::{
    Annotation, AnnotationValue, BinOpKind, CmpOp, ConstDecl, ConstExpr, Decl, EnumBodyItem,
    FlagsBodyItem, ImportKind, MessageBodyItem, MessageField, PrimitiveType, Schema, TypeExpr,
    UnionBodyItem, WhereExpr, WhereOperand,
};
use crate::diagnostic::{Diagnostic, ErrorClass, Note};
use crate::errors::edit_distance;
use crate::ir::{
    self, CmpOp as IrCmpOp, CompiledSchema, ConfigDef, ConfigFieldDef, ConstValue,
    ConstraintOperand, Encoding, EnumDef, EnumVariantDef, FieldConstraint, FieldDef, FieldEncoding,
    FlagsBitDef, FlagsDef, MessageDef, NewtypeDef, ResolvedAnnotations, ResolvedType, TombstoneDef,
    TypeDef, TypeId, TypeRegistry, UnionDef, UnionVariantDef,
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
    /// Direct named import bindings, which outrank wildcard candidates.
    explicit_import_names: HashSet<SmolStr>,
    /// Defining namespace for each direct named import.
    named_import_origins: HashMap<SmolStr, String>,
    /// Aliases declared by explicit aliased imports.
    import_aliases: HashSet<SmolStr>,
    /// Names from local declarations (populated during `register_declarations`).
    local_names: HashSet<SmolStr>,
    /// Evaluated constant values.
    constants: HashMap<SmolStr, ConstValue>,
    /// Const declarations that need evaluation.
    const_decls: HashMap<SmolStr, (ConstDecl, Span)>,
    /// TypeIds of generic aliases registered during lowering.
    /// These are added to the declarations list after processing.
    generic_alias_ids: Vec<TypeId>,
    /// Local aliases known from the AST but registered on the established
    /// post-declaration lifecycle.
    deferred_alias_names: HashSet<SmolStr>,
}

impl LowerCtx {
    fn new() -> Self {
        Self {
            registry: TypeRegistry::new(),
            diagnostics: Vec::new(),
            wildcard_imports: HashSet::new(),
            wildcard_origins: HashMap::new(),
            explicit_import_names: HashSet::new(),
            named_import_origins: HashMap::new(),
            import_aliases: HashSet::new(),
            local_names: HashSet::new(),
            constants: HashMap::new(),
            const_decls: HashMap::new(),
            generic_alias_ids: Vec::new(),
            deferred_alias_names: HashSet::new(),
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
    reject_unsupported_invariants(schema, &mut ctx);
    if !ctx.diagnostics.is_empty() {
        return (None, ctx.diagnostics);
    }
    ctx.local_names.extend(
        schema
            .declarations
            .iter()
            .filter_map(|decl| match &decl.node {
                Decl::Message(d) => Some(d.name.node.clone()),
                Decl::Enum(d) => Some(d.name.node.clone()),
                Decl::Flags(d) => Some(d.name.node.clone()),
                Decl::Union(d) => Some(d.name.node.clone()),
                Decl::Newtype(d) => Some(d.name.node.clone()),
                Decl::Config(d) => Some(d.name.node.clone()),
                Decl::Alias(d) => Some(d.name.node.clone()),
                Decl::Trait(d) => Some(d.name.node.clone()),
                Decl::Const(_) | Decl::Impl(_) => None,
            }),
    );
    ctx.explicit_import_names.extend(
        schema
            .imports
            .iter()
            .filter_map(|import| match &import.node.kind {
                ImportKind::Named { names } => Some(names),
                _ => None,
            })
            .flatten()
            .map(|name| name.node.clone()),
    );
    let namespace_key = schema
        .namespace
        .as_ref()
        .map(|namespace| {
            namespace
                .node
                .path
                .iter()
                .map(|segment| segment.node.as_str())
                .collect::<Vec<_>>()
                .join(".")
        })
        .unwrap_or_default();

    register_import_types(schema, &mut ctx, deps);
    let diagnostic_count = ctx.diagnostics.len();
    reject_imported_alias_chains(schema, &mut ctx);
    if ctx.diagnostics.len() > diagnostic_count {
        return (None, ctx.diagnostics);
    }
    let decl_ids = register_declarations(schema, &namespace_key, &mut ctx);

    // First pass: lower non-alias, non-const, non-impl declarations
    let mut alias_decls: Vec<&crate::ast::AliasDecl> = Vec::new();
    let mut const_decls: Vec<&crate::ast::ConstDecl> = Vec::new();
    let mut impl_decls: Vec<(&crate::ast::ImplDecl, Span)> = Vec::new();
    for decl_spanned in &schema.declarations {
        match &decl_spanned.node {
            Decl::Alias(alias) => {
                ctx.deferred_alias_names.insert(alias.name.node.clone());
                alias_decls.push(alias);
            }
            Decl::Const(c) => {
                const_decls.push(c);
                // Store for later evaluation
                ctx.const_decls
                    .insert(c.name.node.clone(), (c.clone(), decl_spanned.span));
            }
            Decl::Impl(i) => {
                impl_decls.push((i, decl_spanned.span));
            }
            _ => {
                // Skip - handled by register_declarations.id_map
            }
        }
    }
    // Register transparent aliases once all concrete names have stable TypeIds,
    // before fields and trait signatures resolve those aliases.
    for alias in alias_decls {
        lower_alias(alias, &namespace_key, &mut ctx);
    }
    ctx.deferred_alias_names.clear();

    // Second pass: lower the actual type definitions for non-impl declarations
    let mut concrete_ids = decl_ids.iter().copied();
    for decl_spanned in &schema.declarations {
        match &decl_spanned.node {
            Decl::Alias(_) | Decl::Const(_) | Decl::Impl(_) => {}
            _ => {
                let id = concrete_ids
                    .next()
                    .expect("registered declaration must have a TypeId");
                let def = lower_decl(&decl_spanned.node, decl_spanned.span, &mut ctx);
                if let Some(slot) = ctx.registry.get_mut(id) {
                    *slot = def;
                }
                if let Decl::Trait(trait_decl) = &decl_spanned.node {
                    for function in &trait_decl.functions {
                        if let Some(return_type) = &function.return_type {
                            ctx.registry.set_trait_fn_return_type(
                                id,
                                function.name.node.clone(),
                                return_type.node.clone(),
                            );
                        }
                    }
                }
            }
        }
    }

    // Reserve impl slots in their established registry positions. Function
    // signatures and bodies are filled after aliases are registered so their
    // wire-inert types can use transparent aliases without changing TypeIds.
    let mut impl_slots = Vec::with_capacity(impl_decls.len());
    for (impl_decl, span) in impl_decls {
        let target_type = resolve_impl_target(impl_decl, &mut ctx);
        // Generate a unique name for the impl based on trait and target type
        let impl_name = SmolStr::new(format!(
            "__impl_{:?}_{:?}",
            impl_decl.trait_name.node, target_type
        ));
        let impl_id = ctx.registry.register_stub(impl_name);
        impl_slots.push((impl_decl, span, impl_id));
    }

    for (impl_decl, span, impl_id) in impl_slots {
        let impl_def = lower_impl(impl_decl, span, &mut ctx);
        ctx.registry.fill_stub(impl_id, TypeDef::Impl(impl_def));
        ctx.registry
            .set_impl_trait_span(impl_id, impl_decl.trait_name.span);
        if let Some(trait_id) = resolve_impl_trait(impl_decl, &mut ctx) {
            ctx.registry.set_impl_trait_id(impl_id, trait_id);
        }
    }

    // Third pass: evaluate constants after all types are resolved
    evaluate_constants(&mut ctx);

    let namespace: Vec<SmolStr> = schema
        .namespace
        .as_ref()
        .map(|ns| ns.node.path.iter().map(|s| s.node.clone()).collect())
        .unwrap_or_default();

    let annotations = resolve_annotations(&schema.annotations);

    // Filter out aliases and consts from declarations list.
    // Generic aliases are handled separately (see generic_alias_ids below).
    // Consts are skipped here because they don't have TypeIds (handled separately).
    let mut declaration_ids = decl_ids;

    // Add TypeIds of generic aliases (registered in lower_alias) to declarations
    declaration_ids.extend(ctx.generic_alias_ids.iter().copied());

    let compiled = CompiledSchema {
        namespace,
        annotations,
        registry: ctx.registry,
        declarations: declaration_ids,
        constants: ctx.constants,
    };

    (Some(compiled), ctx.diagnostics)
}

fn reject_unsupported_invariants(schema: &Schema, ctx: &mut LowerCtx) {
    for declaration in &schema.declarations {
        let Decl::Message(message) = &declaration.node else {
            continue;
        };
        for item in &message.body {
            if let MessageBodyItem::Invariant(invariant) = item {
                ctx.emit(
                    invariant.span,
                    ErrorClass::InvariantUnsupported,
                    "message invariants are not yet supported",
                );
            }
        }
    }
}

fn reject_imported_alias_chains(schema: &Schema, ctx: &mut LowerCtx) {
    for declaration in &schema.declarations {
        let Decl::Alias(alias) = &declaration.node else {
            continue;
        };
        let type_params: HashSet<&str> = alias
            .type_params
            .iter()
            .map(|parameter| parameter.name.node.as_str())
            .collect();
        if type_expr_references_registered_alias(&alias.target.node, ctx, &type_params) {
            ctx.emit(
                alias.target.span,
                ErrorClass::AliasTargetIsAlias,
                format!(
                    "alias `{}` references an imported alias; must reference a terminal type directly",
                    alias.name.node
                ),
            );
        }
    }
}

fn type_expr_references_registered_alias(
    target: &TypeExpr,
    ctx: &LowerCtx,
    type_params: &HashSet<&str>,
) -> bool {
    let is_alias_name = |name: &str| {
        ctx.registry.lookup_alias(name).is_some()
            || ctx
                .registry
                .lookup(name)
                .and_then(|id| ctx.registry.get(id))
                .is_some_and(|definition| matches!(definition, TypeDef::GenericAlias(_)))
    };

    match target {
        TypeExpr::Named(name) => {
            !type_params.contains(name.as_str()) && is_alias_name(name.as_str())
        }
        TypeExpr::Qualified(namespace, name) => is_alias_name(&format!("{namespace}.{name}")),
        TypeExpr::Generic(name, argument) => {
            is_alias_name(name.as_str())
                || type_expr_references_registered_alias(&argument.node, ctx, type_params)
        }
        TypeExpr::Optional(inner)
        | TypeExpr::Array(inner)
        | TypeExpr::FixedArray(inner, _)
        | TypeExpr::Set(inner)
        | TypeExpr::Vec2(inner)
        | TypeExpr::Vec3(inner)
        | TypeExpr::Vec4(inner)
        | TypeExpr::Quat(inner)
        | TypeExpr::Mat3(inner)
        | TypeExpr::Mat4(inner) => {
            type_expr_references_registered_alias(&inner.node, ctx, type_params)
        }
        TypeExpr::Map(key, value) | TypeExpr::Result(key, value) => {
            type_expr_references_registered_alias(&key.node, ctx, type_params)
                || type_expr_references_registered_alias(&value.node, ctx, type_params)
        }
        TypeExpr::Primitive(_)
        | TypeExpr::SubByte(_)
        | TypeExpr::Semantic(_)
        | TypeExpr::BitsInline(_) => false,
    }
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

        match deps.and_then(|d| d.schemas.get(&ns_key)) {
            Some(dep_compiled) => {
                if let Some(requirement) = &imp.node.version {
                    let parsed_requirement = semver::VersionReq::parse(&requirement.node);
                    match (&dep_compiled.annotations.version, parsed_requirement) {
                        (Some(version), Ok(requirement_value)) => {
                            match semver::Version::parse(version) {
                                Ok(version_value) if !requirement_value.matches(&version_value) => {
                                    ctx.emit(
                                        requirement.span,
                                        ErrorClass::ImportVersionMismatch,
                                        format!(
                                            "import `{ns_key}` requires `{}` but schema declares `{version}`",
                                            requirement.node
                                        ),
                                    );
                                }
                                Ok(_) => {}
                                Err(error) => ctx.emit(
                                    requirement.span,
                                    ErrorClass::VersionInvalidSemver,
                                    format!(
                                        "imported schema `{ns_key}` declares invalid version `{version}`: {error}"
                                    ),
                                ),
                            }
                        }
                        (None, Ok(_)) => ctx.diagnostics.push(
                            Diagnostic::warning(
                                requirement.span,
                                ErrorClass::ImportVersionUnavailable,
                                format!(
                                    "cannot verify import requirement `{}` because schema `{ns_key}` has no @version",
                                    requirement.node
                                ),
                            )
                            .with_help("add @version to the imported schema"),
                        ),
                        (_, Err(error)) => ctx.emit(
                            requirement.span,
                            ErrorClass::VersionInvalidSemver,
                            format!("invalid import version requirement: {error}"),
                        ),
                    }
                }

                // Real dependency available — inject types by import kind.
                match &imp.node.kind {
                    ImportKind::Named { names } => {
                        for name_spanned in names {
                            let name = &name_spanned.node;
                            if let Some(existing_namespace) =
                                ctx.named_import_origins.get(name.as_str()).cloned()
                            {
                                if existing_namespace != ns_key {
                                    ctx.emit(
                                        name_spanned.span,
                                        ErrorClass::UnresolvedType,
                                        format!(
                                            "named import '{name}' conflicts between namespaces '{existing_namespace}' and '{ns_key}'"
                                        ),
                                    );
                                    continue;
                                }
                            } else {
                                ctx.named_import_origins
                                    .insert(name.clone(), ns_key.clone());
                            }
                            let found = dep_compiled.declarations.iter().find(|&&id| {
                                dep_compiled
                                    .registry
                                    .get(id)
                                    .map(|d| crate::remap::type_def_name(d) == name.as_str())
                                    .unwrap_or(false)
                            });
                            let alias_target = dep_compiled
                                .registry
                                .find_alias_origin(&ns_key, name.as_str())
                                .cloned();
                            match (found, alias_target) {
                                (Some(&id), _) => {
                                    let id_map = crate::remap::clone_types_into_unbound(
                                        &dep_compiled.registry,
                                        &[id],
                                        &mut ctx.registry,
                                    );
                                    if let Some(&new_id) = id_map.get(&id) {
                                        ctx.registry.bind_name(name.clone(), new_id);
                                    }
                                }
                                (None, Some(target)) => {
                                    let remapped = crate::remap::clone_resolved_type_into_unbound(
                                        &dep_compiled.registry,
                                        &target,
                                        &mut ctx.registry,
                                    );
                                    ctx.registry.register_resolved_alias(name.clone(), remapped);
                                    ctx.registry.set_alias_origin(
                                        name.as_str(),
                                        SmolStr::new(&ns_key),
                                        name.clone(),
                                    );
                                }
                                (None, None) => {
                                    // Collect available export names for suggestions
                                    let mut available_names: Vec<&str> = dep_compiled
                                        .declarations
                                        .iter()
                                        .filter_map(|&id| {
                                            dep_compiled
                                                .registry
                                                .get(id)
                                                .map(crate::remap::type_def_name)
                                        })
                                        .collect();
                                    available_names.extend(
                                        dep_compiled
                                            .registry
                                            .aliases_from_origin(&ns_key)
                                            .map(|(name, _)| name),
                                    );

                                    // Find closest match using edit distance (threshold 3)
                                    let target_lower = name.as_str().to_lowercase();
                                    let mut best_match: Option<&str> = None;
                                    let mut best_distance = usize::MAX;
                                    const THRESHOLD: usize = 3;

                                    for candidate in &available_names {
                                        let candidate_lower = candidate.to_lowercase();
                                        let distance =
                                            edit_distance(&target_lower, &candidate_lower);
                                        if distance < best_distance && distance <= THRESHOLD {
                                            best_distance = distance;
                                            best_match = Some(*candidate);
                                        }
                                    }

                                    let message = if let Some(sugg) = best_match {
                                        format!("unknown import `{name}`. Did you mean `{sugg}`?")
                                    } else {
                                        format!(
                                            "imported name `{name}` not found in namespace `{ns_key}`"
                                        )
                                    };

                                    let mut diag = Diagnostic::error(
                                        name_spanned.span,
                                        ErrorClass::UnresolvedType,
                                        message,
                                    );

                                    // Add list of available exports
                                    if !available_names.is_empty() {
                                        let exports_list: Vec<String> = available_names
                                            .iter()
                                            .take(10) // Limit to avoid overwhelming output
                                            .map(|s| s.to_string())
                                            .collect();
                                        diag = diag.with_note(Note::ValidOptions(exports_list));
                                        if available_names.len() > 10 {
                                            diag = diag.with_note(Note::Note(format!(
                                                "... and {} more",
                                                available_names.len() - 10
                                            )));
                                        }
                                    } else {
                                        diag = diag.with_help("the imported namespace exports no types (namespace may be empty or all declarations may be private)");
                                    }

                                    ctx.diagnostics.push(diag);
                                }
                            }
                        }
                    }
                    ImportKind::Wildcard => {
                        let id_map = crate::remap::clone_types_into_unbound(
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
                                            .insert(type_name.clone(), Some(ns_key.clone()));
                                        if !ctx.local_names.contains(type_name.as_str())
                                            && !ctx
                                                .explicit_import_names
                                                .contains(type_name.as_str())
                                        {
                                            if let Some(&new_id) = id_map.get(&old_id) {
                                                ctx.registry.bind_name(type_name, new_id);
                                            }
                                        }
                                    }
                                    Some(Some(existing_ns)) if *existing_ns != ns_key => {
                                        // Ambiguous: same name from different namespaces.
                                        ctx.wildcard_origins.insert(type_name, None);
                                    }
                                    _ => {} // same namespace or already ambiguous
                                }
                            }
                        }
                        let aliases: Vec<_> = dep_compiled
                            .registry
                            .aliases_from_origin(&ns_key)
                            .map(|(name, target)| (SmolStr::new(name), target.clone()))
                            .collect();
                        for (name, target) in aliases {
                            match ctx.wildcard_origins.get(&name) {
                                None => {
                                    ctx.wildcard_origins
                                        .insert(name.clone(), Some(ns_key.clone()));
                                    if !ctx.local_names.contains(name.as_str())
                                        && !ctx.explicit_import_names.contains(name.as_str())
                                    {
                                        let remapped =
                                            crate::remap::clone_resolved_type_into_unbound(
                                                &dep_compiled.registry,
                                                &target,
                                                &mut ctx.registry,
                                            );
                                        ctx.registry.register_resolved_alias(name, remapped);
                                    }
                                }
                                Some(Some(existing_ns)) if *existing_ns != ns_key => {
                                    ctx.wildcard_origins.insert(name, None);
                                }
                                _ => {}
                            }
                        }
                        ctx.wildcard_imports.insert(SmolStr::new(&ns_key));
                    }
                    ImportKind::Aliased { alias } => {
                        if ctx.local_names.contains(&alias.node) {
                            ctx.emit(
                                alias.span,
                                ErrorClass::UnresolvedType,
                                format!(
                                    "import alias '{}' conflicts with a local declaration",
                                    alias.node
                                ),
                            );
                            continue;
                        }
                        if !ctx.import_aliases.insert(alias.node.clone()) {
                            ctx.emit(
                                alias.span,
                                ErrorClass::UnresolvedType,
                                format!("duplicate import alias '{}'", alias.node),
                            );
                            continue;
                        }
                        let id_map = crate::remap::clone_types_into_unbound(
                            &dep_compiled.registry,
                            &dep_compiled.declarations,
                            &mut ctx.registry,
                        );
                        for &old_id in &dep_compiled.declarations {
                            if let (Some(def), Some(&new_id)) =
                                (dep_compiled.registry.get(old_id), id_map.get(&old_id))
                            {
                                let qualified = SmolStr::new(format!(
                                    "{}.{}",
                                    alias.node,
                                    crate::remap::type_def_name(def)
                                ));
                                ctx.registry.register_alias(qualified, new_id);
                            }
                        }
                        let aliases: Vec<_> = dep_compiled
                            .registry
                            .aliases_from_origin(&ns_key)
                            .map(|(name, target)| (SmolStr::new(name), target.clone()))
                            .collect();
                        for (name, target) in aliases {
                            let remapped = crate::remap::clone_resolved_type_into_unbound(
                                &dep_compiled.registry,
                                &target,
                                &mut ctx.registry,
                            );
                            let qualified = SmolStr::new(format!("{}.{}", alias.node, name));
                            ctx.registry.register_resolved_alias(qualified, remapped);
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

fn register_declarations(schema: &Schema, namespace: &str, ctx: &mut LowerCtx) -> Vec<TypeId> {
    let mut ids = Vec::new();
    for decl_spanned in &schema.declarations {
        let name = match &decl_spanned.node {
            Decl::Message(d) => d.name.node.clone(),
            Decl::Enum(d) => d.name.node.clone(),
            Decl::Flags(d) => d.name.node.clone(),
            Decl::Union(d) => d.name.node.clone(),
            Decl::Newtype(d) => d.name.node.clone(),
            Decl::Config(d) => d.name.node.clone(),
            Decl::Trait(d) => d.name.node.clone(),
            Decl::Alias(_) => continue, // Aliases don't get TypeIds, they use alias_map
            Decl::Const(_) => continue, // Consts don't get TypeIds, they use constants map
            Decl::Impl(_) => continue,  // Impls don't get TypeIds
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
        let declaration_name = name.clone();
        let id = ctx.registry.register(name, placeholder);
        ctx.registry
            .set_origin(id, SmolStr::new(namespace), declaration_name);
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
        Decl::Trait(d) => TypeDef::Trait(lower_trait(d, span, ctx)),
        Decl::Alias(_) | Decl::Const(_) | Decl::Impl(_) => {
            // Aliases and consts are handled separately in lower_with_deps after all regular
            // declarations are lowered. This ensures dependencies are resolved.
            // Return a dummy TypeDef that should never be used.
            TypeDef::Message(MessageDef {
                name: SmolStr::new("__placeholder"),
                span,
                fields: Vec::new(),
                tombstones: Vec::new(),
                annotations: ResolvedAnnotations::default(),
                wire_size: None,
            })
        }
    }
}

fn lower_message(msg: &crate::ast::MessageDecl, span: Span, ctx: &mut LowerCtx) -> MessageDef {
    let has_message_delta = msg.annotations.iter().any(|a| a.name.node == "delta");

    let mut fields = Vec::new();
    let mut tombstones = Vec::new();
    for item in &msg.body {
        match item {
            MessageBodyItem::Field(f) => fields.push(lower_field(&f.node, f.span, ctx)),
            MessageBodyItem::Tombstone(t) => tombstones.push(lower_tombstone(&t.node, t.span, ctx)),
            MessageBodyItem::Invariant(_) => {} // rejected by the lowering entry-point guard
        }
    }

    // Desugar @delta on message: apply to all eligible fields that don't already have @delta.
    // Message-level @delta implies optimal variable-length inner encoding:
    //   unsigned integers → Delta(Varint)
    //   signed integers   → Delta(ZigZag)
    //   floats            → Delta(Default)  (varint doesn't apply to floats)
    if has_message_delta {
        for field in &mut fields {
            if is_delta_eligible(&field.resolved_type) && !is_already_delta(&field.encoding) {
                let inner = optimal_delta_inner(&field.resolved_type, &field.encoding);
                field.encoding.encoding = Encoding::Delta(Box::new(inner));
            }
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
    let constraint = field
        .where_clause
        .as_ref()
        .map(|w| lower_where_expr(&w.node));
    FieldDef {
        name: field.name.node.clone(),
        span,
        ordinal: field.ordinal.node,
        resolved_type,
        encoding,
        annotations,
        constraint,
    }
}

fn lower_enum(en: &crate::ast::EnumDecl, span: Span, ctx: &mut LowerCtx) -> EnumDef {
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
            EnumBodyItem::Tombstone(t) => tombstones.push(lower_tombstone(&t.node, t.span, ctx)),
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

fn lower_flags(flags: &crate::ast::FlagsDecl, span: Span, ctx: &mut LowerCtx) -> FlagsDef {
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
            FlagsBodyItem::Tombstone(t) => tombstones.push(lower_tombstone(&t.node, t.span, ctx)),
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
                            tombstones.push(lower_tombstone(&t.node, t.span, ctx))
                        }
                        MessageBodyItem::Invariant(_) => {} // invariants lowered with message
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
            UnionBodyItem::Tombstone(t) => {
                top_tombstones.push(lower_tombstone(&t.node, t.span, ctx))
            }
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
    // Terminal type is the innermost non-newtype.
    // Validation rejects newtype-over-newtype chains, so this is safe.
    let terminal_type = inner_type.clone();
    NewtypeDef {
        name: nt.name.node.clone(),
        span,
        inner_type,
        terminal_type,
        annotations: resolve_annotations(&nt.annotations),
    }
}

/// Lower a type alias declaration.
/// Non-generic aliases are transparent — they don't create TypeDef entries.
/// Instead, they add name-to-resolved-type mappings in the registry.
/// Generic aliases are stored as GenericAlias TypeDef entries.
fn lower_alias(alias: &crate::ast::AliasDecl, namespace: &str, ctx: &mut LowerCtx) {
    // Check if this is a generic alias (has type parameters)
    if !alias.type_params.is_empty() {
        // Generic alias: store as GenericAlias TypeDef
        let type_param_names: Vec<SmolStr> = alias
            .type_params
            .iter()
            .map(|p| p.name.node.clone())
            .collect();

        let generic_alias_def = TypeDef::GenericAlias(ir::GenericAliasDef {
            name: alias.name.node.clone(),
            span: alias.name.span,
            type_params: type_param_names,
            target_type: alias.target.node.clone(),
            annotations: resolve_annotations(&alias.annotations),
        });

        // Register the generic alias in the registry
        let id = ctx
            .registry
            .register(alias.name.node.clone(), generic_alias_def);
        ctx.generic_alias_ids.push(id);
        return;
    }

    // Non-generic aliases remain transparent regardless of their concrete target.
    let target_type = resolve_type_expr(&alias.target.node, alias.target.span, ctx);
    ctx.registry
        .register_resolved_alias(alias.name.node.clone(), target_type);
    ctx.registry.set_alias_origin(
        alias.name.node.as_str(),
        SmolStr::new(namespace),
        alias.name.node.clone(),
    );
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

/// Check if a type expression contains any of the given type parameters.
fn contains_type_param(expr: &TypeExpr, type_params: &std::collections::HashSet<SmolStr>) -> bool {
    match expr {
        TypeExpr::Named(name) => type_params.contains(name),
        TypeExpr::Primitive(_) | TypeExpr::SubByte(_) | TypeExpr::Semantic(_) => false,
        TypeExpr::Generic(_, arg) => contains_type_param(&arg.node, type_params),
        TypeExpr::Optional(inner) => contains_type_param(&inner.node, type_params),
        TypeExpr::Array(inner) => contains_type_param(&inner.node, type_params),
        TypeExpr::FixedArray(inner, _) => contains_type_param(&inner.node, type_params),
        TypeExpr::Set(inner) => contains_type_param(&inner.node, type_params),
        TypeExpr::Map(key, value) => {
            contains_type_param(&key.node, type_params)
                || contains_type_param(&value.node, type_params)
        }
        TypeExpr::Result(ok, err) => {
            contains_type_param(&ok.node, type_params)
                || contains_type_param(&err.node, type_params)
        }
        TypeExpr::Vec2(inner) | TypeExpr::Vec3(inner) | TypeExpr::Vec4(inner) => {
            contains_type_param(&inner.node, type_params)
        }
        TypeExpr::Quat(inner) => contains_type_param(&inner.node, type_params),
        TypeExpr::Mat3(inner) | TypeExpr::Mat4(inner) => {
            contains_type_param(&inner.node, type_params)
        }
        TypeExpr::BitsInline(_) => false,
        TypeExpr::Qualified(_, _) => false,
    }
}

/// Lower a trait declaration to IR.
fn lower_trait(
    decl: &crate::ast::TraitDecl,
    span: Span,
    ctx: &mut LowerCtx,
) -> crate::ir::TraitDef {
    let name = decl.name.node.clone();

    // Lower type parameters
    let type_params = decl.type_params.clone();

    // Lower required fields
    // Note: we don't resolve type parameters during trait lowering.
    // For generic traits, the trait field types may reference type parameters.
    // We store both the resolved type (for wire size calculation) and the
    // unresolved type expression (for conformance checking with impl type args).
    let mut fields = Vec::new();
    let type_param_names: std::collections::HashSet<SmolStr> = decl
        .type_params
        .iter()
        .map(|p| p.name.node.clone())
        .collect();

    for field in &decl.fields {
        let all_annotations: Vec<&crate::ast::Annotation> = field
            .pre_annotations
            .iter()
            .chain(field.post_ordinal_annotations.iter())
            .chain(field.post_type_annotations.iter())
            .collect();

        // Check if the field type contains any type parameters from this trait
        let field_ty_resolved = if contains_type_param(&field.ty.node, &type_param_names) {
            // Don't resolve type parameters - leave as POISON_TYPE_ID marker
            ResolvedType::Named(ir::types::POISON_TYPE_ID)
        } else {
            resolve_type_expr(&field.ty.node, field.ty.span, ctx)
        };

        let field_def = crate::ir::TraitFieldDef {
            name: field.name.node.clone(),
            ty: field_ty_resolved,
            unresolved_ty: field.ty.node.clone(),
            ordinal: field.ordinal.node,
            annotations: resolve_annotations_refs(&all_annotations),
        };
        fields.push(field_def);
    }

    // Lower function signatures
    let functions = decl
        .functions
        .iter()
        .map(|fn_decl| crate::ir::TraitFnDef {
            name: fn_decl.name.node.clone(),
            params: fn_decl
                .params
                .iter()
                .map(|p| crate::ir::FnParamDef {
                    name: p.name.node.clone(),
                    ty: resolve_trait_function_type(&p.ty.node, &type_param_names, p.ty.span, ctx),
                    unresolved_ty: p.ty.node.clone(),
                })
                .collect(),
            return_type: fn_decl
                .return_type
                .as_ref()
                .map(|ty| resolve_trait_function_type(&ty.node, &type_param_names, ty.span, ctx)),
        })
        .collect();

    crate::ir::TraitDef {
        name,
        type_params,
        fields,
        functions,
        annotations: resolve_annotations(&decl.annotations),
        span,
    }
}

/// Lower an impl declaration to IR.
fn resolve_impl_trait(decl: &crate::ast::ImplDecl, ctx: &mut LowerCtx) -> Option<TypeId> {
    let lookup = decl.trait_name.node.as_str();
    if let Some((alias, member)) = lookup.split_once('.') {
        if !ctx.import_aliases.contains(alias) {
            ctx.emit(
                decl.trait_name.span,
                ErrorClass::UnresolvedType,
                format!("unknown import alias '{alias}' in impl trait reference"),
            );
            return None;
        }
        let Some(id) = ctx.registry.lookup(lookup) else {
            ctx.emit(
                decl.trait_name.span,
                ErrorClass::UnresolvedType,
                format!("import alias '{alias}' has no member '{member}'"),
            );
            return None;
        };
        return match ctx.registry.get(id) {
            Some(TypeDef::Trait(_)) => Some(id),
            Some(_) => {
                ctx.emit(
                    decl.trait_name.span,
                    ErrorClass::UnresolvedType,
                    format!("'{lookup}' does not name a trait"),
                );
                None
            }
            None => {
                ctx.emit(
                    decl.trait_name.span,
                    ErrorClass::UnresolvedType,
                    format!("import alias '{alias}' has no member '{member}'"),
                );
                None
            }
        };
    }

    let wildcard_is_ambiguous = matches!(ctx.wildcard_origins.get(lookup), Some(None))
        && !ctx.local_names.contains(lookup)
        && !ctx.explicit_import_names.contains(lookup);
    if wildcard_is_ambiguous {
        ctx.emit(
            decl.trait_name.span,
            ErrorClass::UnresolvedType,
            format!("trait name '{lookup}' is ambiguous across wildcard imports"),
        );
        return None;
    }

    let Some(id) = ctx.registry.lookup(lookup) else {
        ctx.emit(
            decl.trait_name.span,
            ErrorClass::UnresolvedType,
            format!("impl references unknown trait '{lookup}'"),
        );
        return None;
    };
    match ctx.registry.get(id) {
        Some(TypeDef::Trait(_)) => Some(id),
        Some(_) => {
            ctx.emit(
                decl.trait_name.span,
                ErrorClass::UnresolvedType,
                format!("'{lookup}' does not name a trait"),
            );
            None
        }
        None => {
            ctx.emit(
                decl.trait_name.span,
                ErrorClass::UnresolvedType,
                format!("impl references unknown trait '{lookup}'"),
            );
            None
        }
    }
}

fn lower_impl(decl: &crate::ast::ImplDecl, span: Span, ctx: &mut LowerCtx) -> crate::ir::ImplDef {
    let trait_name = decl.trait_name.node.clone();
    let target_type = resolve_impl_target(decl, ctx);

    // Lower type arguments for generic traits
    let type_args: Vec<ResolvedType> = decl
        .type_args
        .iter()
        .map(|arg| resolve_type_expr(&arg.node, arg.span, ctx))
        .collect();

    // Lower function implementations
    let functions = decl
        .functions
        .iter()
        .map(|f| {
            let body = match &f.body {
                crate::ast::ImplFnBody::External => crate::ir::FnBody::External,
                crate::ast::ImplFnBody::Block(stmts) => {
                    let ir_stmts = stmts.iter().map(|s| lower_statement(s, ctx)).collect();
                    crate::ir::FnBody::Block(ir_stmts)
                }
            };
            crate::ir::ImplFnDef {
                name: f.name.node.clone(),
                params: f
                    .params
                    .iter()
                    .map(|p| crate::ir::FnParamDef {
                        name: p.name.node.clone(),
                        ty: resolve_type_expr(&p.ty.node, p.name.span, ctx),
                        unresolved_ty: p.ty.node.clone(),
                    })
                    .collect(),
                return_type: f
                    .return_type
                    .as_ref()
                    .map(|t| resolve_type_expr(&t.node, span, ctx)),
                body,
            }
        })
        .collect();

    crate::ir::ImplDef {
        trait_name,
        target_type,
        type_args,
        functions,
        annotations: resolve_annotations(&decl.annotations),
        span,
    }
}

fn resolve_impl_target(decl: &crate::ast::ImplDecl, ctx: &mut LowerCtx) -> ResolvedType {
    let target_type_name = decl.target_type.node.as_str();
    if let Some(id) = ctx.registry.lookup(target_type_name) {
        ResolvedType::Named(id)
    } else if ctx.local_names.contains(target_type_name) {
        ResolvedType::Named(ir::types::POISON_TYPE_ID)
    } else {
        let id = ctx.registry.register_stub(SmolStr::new(target_type_name));
        ResolvedType::Named(id)
    }
}

fn resolve_trait_function_type(
    expr: &TypeExpr,
    type_params: &HashSet<SmolStr>,
    span: Span,
    ctx: &mut LowerCtx,
) -> ResolvedType {
    match expr {
        TypeExpr::Named(name) if type_params.contains(name) => {
            ResolvedType::Named(ir::types::POISON_TYPE_ID)
        }
        TypeExpr::Named(name) if ctx.deferred_alias_names.contains(name) => {
            ResolvedType::Named(ir::types::POISON_TYPE_ID)
        }
        TypeExpr::Optional(inner) => ResolvedType::Optional(Box::new(resolve_trait_function_type(
            &inner.node,
            type_params,
            inner.span,
            ctx,
        ))),
        TypeExpr::Array(inner) => ResolvedType::Array(Box::new(resolve_trait_function_type(
            &inner.node,
            type_params,
            inner.span,
            ctx,
        ))),
        TypeExpr::FixedArray(inner, length) => ResolvedType::FixedArray(
            Box::new(resolve_trait_function_type(
                &inner.node,
                type_params,
                inner.span,
                ctx,
            )),
            *length,
        ),
        TypeExpr::Set(inner) => ResolvedType::Set(Box::new(resolve_trait_function_type(
            &inner.node,
            type_params,
            inner.span,
            ctx,
        ))),
        TypeExpr::Map(key, value) => ResolvedType::Map(
            Box::new(resolve_trait_function_type(
                &key.node,
                type_params,
                key.span,
                ctx,
            )),
            Box::new(resolve_trait_function_type(
                &value.node,
                type_params,
                value.span,
                ctx,
            )),
        ),
        TypeExpr::Result(ok, error) => ResolvedType::Result(
            Box::new(resolve_trait_function_type(
                &ok.node,
                type_params,
                ok.span,
                ctx,
            )),
            Box::new(resolve_trait_function_type(
                &error.node,
                type_params,
                error.span,
                ctx,
            )),
        ),
        TypeExpr::Vec2(inner) => ResolvedType::Vec2(Box::new(resolve_trait_function_type(
            &inner.node,
            type_params,
            inner.span,
            ctx,
        ))),
        TypeExpr::Vec3(inner) => ResolvedType::Vec3(Box::new(resolve_trait_function_type(
            &inner.node,
            type_params,
            inner.span,
            ctx,
        ))),
        TypeExpr::Vec4(inner) => ResolvedType::Vec4(Box::new(resolve_trait_function_type(
            &inner.node,
            type_params,
            inner.span,
            ctx,
        ))),
        TypeExpr::Quat(inner) => ResolvedType::Quat(Box::new(resolve_trait_function_type(
            &inner.node,
            type_params,
            inner.span,
            ctx,
        ))),
        TypeExpr::Mat3(inner) => ResolvedType::Mat3(Box::new(resolve_trait_function_type(
            &inner.node,
            type_params,
            inner.span,
            ctx,
        ))),
        TypeExpr::Mat4(inner) => ResolvedType::Mat4(Box::new(resolve_trait_function_type(
            &inner.node,
            type_params,
            inner.span,
            ctx,
        ))),
        TypeExpr::Generic(name, _)
            if ctx.deferred_alias_names.contains(name)
                || contains_type_param(expr, type_params) =>
        {
            ResolvedType::Named(ir::types::POISON_TYPE_ID)
        }
        _ => resolve_type_expr(expr, span, ctx),
    }
}

/// Lower an expression from AST to IR.
#[allow(clippy::only_used_in_recursion)]
fn lower_expr(expr: &crate::ast::Expr, ctx: &mut LowerCtx) -> crate::ir::Expr {
    use crate::ast::Expr as AstExpr;

    match expr {
        AstExpr::Int(v) => crate::ir::Expr::Int(*v),
        AstExpr::UInt(v) => crate::ir::Expr::UInt(*v),
        AstExpr::Float(v) => crate::ir::Expr::Float(*v),
        AstExpr::Bool(v) => crate::ir::Expr::Bool(*v),
        AstExpr::String(s) => crate::ir::Expr::String(s.clone()),
        AstExpr::Ident(name) => crate::ir::Expr::Local(name.clone()),
        AstExpr::SelfRef => crate::ir::Expr::SelfRef,
        AstExpr::FieldAccess(obj, field) => {
            let obj = lower_expr(obj, ctx);
            crate::ir::Expr::FieldAccess(Box::new(obj), field.node.clone())
        }
        AstExpr::Call(func, args) => {
            let func_name = match func.as_ref() {
                AstExpr::Ident(name) => name.clone(),
                _ => SmolStr::new("__error"),
            };
            let args = args.iter().map(|a| lower_expr(a, ctx)).collect();
            crate::ir::Expr::Call(func_name, args)
        }
        AstExpr::MethodCall(receiver, method, args) => {
            let receiver = lower_expr(receiver, ctx);
            let args: Vec<_> = args.iter().map(|a| lower_expr(a, ctx)).collect();

            // For now, emit trait method call - resolution happens later
            crate::ir::Expr::TraitMethodCall {
                trait_name: SmolStr::new("__unresolved"), // filled in by typeck
                method_name: method.node.clone(),
                receiver: Box::new(receiver),
                args,
            }
        }
        AstExpr::Binary(op, lhs, rhs) => {
            let lhs = lower_expr(lhs, ctx);
            let rhs = lower_expr(rhs, ctx);
            let ir_op = lower_bin_op(*op);
            crate::ir::Expr::Binary(ir_op, Box::new(lhs), Box::new(rhs))
        }
        AstExpr::Unary(op, expr) => {
            let expr = lower_expr(expr, ctx);
            let ir_op = lower_unary_op(*op);
            crate::ir::Expr::Unary(ir_op, Box::new(expr))
        }
    }
}

/// Lower a binary operator from AST to IR.
fn lower_bin_op(op: crate::ast::BinOpKind) -> crate::ir::BinOp {
    use crate::ast::BinOpKind as Ast;
    use crate::ir::BinOp as Ir;

    match op {
        Ast::Add => Ir::Add,
        Ast::Sub => Ir::Sub,
        Ast::Mul => Ir::Mul,
        Ast::Div => Ir::Div,
        Ast::Eq => Ir::Eq,
        Ast::Ne => Ir::Ne,
        Ast::Lt => Ir::Lt,
        Ast::Le => Ir::Le,
        Ast::Gt => Ir::Gt,
        Ast::Ge => Ir::Ge,
    }
}

/// Lower a unary operator from AST to IR.
fn lower_unary_op(op: crate::ast::UnaryOpKind) -> crate::ir::UnaryOp {
    use crate::ast::UnaryOpKind as Ast;
    use crate::ir::UnaryOp as Ir;

    match op {
        Ast::Neg => Ir::Neg,
        Ast::Not => Ir::Not,
    }
}

/// Lower a statement from AST to IR.
fn lower_statement(stmt: &crate::ast::Statement, ctx: &mut LowerCtx) -> crate::ir::Statement {
    use crate::ast::Statement as Ast;

    match stmt {
        Ast::Expr(e) => crate::ir::Statement::Expr(lower_expr(e, ctx)),
        Ast::Let { name, ty, value } => crate::ir::Statement::Let {
            name: name.node.clone(),
            ty: ty
                .as_ref()
                .map(|t| resolve_type_expr(&t.node, name.span, ctx)),
            value: lower_expr(value, ctx),
        },
        Ast::Return(v) => crate::ir::Statement::Return(v.as_ref().map(|e| lower_expr(e, ctx))),
        Ast::Assign { target, value } => crate::ir::Statement::Assign {
            target: lower_expr(target, ctx),
            value: lower_expr(value, ctx),
        },
    }
}

fn resolve_type_expr(expr: &TypeExpr, span: Span, ctx: &mut LowerCtx) -> ResolvedType {
    match expr {
        TypeExpr::Primitive(p) => ResolvedType::Primitive(*p),
        TypeExpr::SubByte(s) => ResolvedType::SubByte(*s),
        TypeExpr::Semantic(s) => ResolvedType::Semantic(*s),
        TypeExpr::Named(name) => {
            if let Some(target) = ctx.registry.lookup_alias(name.as_str()) {
                return target.clone();
            }
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
                // Collect available type names for suggestions
                let available_types: Vec<&str> = ctx
                    .local_names
                    .iter()
                    .map(|s| s.as_str())
                    .chain(ctx.registry.iter_names())
                    .collect();

                let mut diag = Diagnostic::error(
                    span,
                    ErrorClass::UnresolvedType,
                    format!("unresolved type `{name}`"),
                );

                if let Some(suggestion) = crate::diagnostic::find_closest_match(
                    name.as_str(),
                    available_types.clone().into_iter(),
                ) {
                    diag = diag.with_suggestion(suggestion);
                }

                if !available_types.is_empty() {
                    let type_list: Vec<String> = available_types
                        .iter()
                        .take(15)
                        .map(|s| s.to_string())
                        .collect();
                    diag = diag.with_note(Note::ValidOptions(type_list));
                    if available_types.len() > 15 {
                        diag = diag.with_note(Note::Note(format!(
                            "... and {} more types",
                            available_types.len() - 15
                        )));
                    }
                }

                ctx.diagnostics.push(diag);
                ResolvedType::Named(ir::types::POISON_TYPE_ID)
            }
        }
        TypeExpr::Qualified(ns, name) => {
            let qualified_name: SmolStr = format!("{ns}.{name}").into();
            if let Some(target) = ctx.registry.lookup_alias(qualified_name.as_str()) {
                target.clone()
            } else if let Some(id) = ctx.registry.lookup(qualified_name.as_str()) {
                ResolvedType::Named(id)
            } else if ctx.registry.lookup(ns.as_str()).is_some() {
                // Namespace alias is known — register qualified stub.
                let id = ctx.registry.register_stub(qualified_name);
                ResolvedType::Named(id)
            } else {
                // Collect available namespace aliases for suggestions
                let available_ns: Vec<&str> = ctx
                    .registry
                    .iter_names()
                    .filter(|n| !n.contains('.')) // Only top-level aliases
                    .collect();

                let mut diag = Diagnostic::error(
                    span,
                    ErrorClass::UnresolvedType,
                    format!("unresolved qualified type `{ns}.{name}`"),
                );

                // Suggest if namespace alias is typo
                if let Some(suggestion) = crate::diagnostic::find_closest_match(
                    ns.as_str(),
                    available_ns.clone().into_iter(),
                ) {
                    diag = diag.with_suggestion(format!("{}.{}", suggestion, name));
                }

                if !available_ns.is_empty() {
                    diag = diag.with_help(format!(
                        "available namespace aliases: {}",
                        available_ns.join(", ")
                    ));
                } else {
                    diag = diag.with_help(
                        "use `import <namespace> as <alias>` to create a namespace alias",
                    );
                }

                ctx.diagnostics.push(diag);
                ResolvedType::Named(ir::types::POISON_TYPE_ID)
            }
        }
        TypeExpr::Optional(inner) => {
            ResolvedType::Optional(Box::new(resolve_type_expr(&inner.node, inner.span, ctx)))
        }
        TypeExpr::Array(inner) => {
            ResolvedType::Array(Box::new(resolve_type_expr(&inner.node, inner.span, ctx)))
        }
        TypeExpr::FixedArray(inner, size) => ResolvedType::FixedArray(
            Box::new(resolve_type_expr(&inner.node, inner.span, ctx)),
            *size,
        ),
        TypeExpr::Set(inner) => {
            ResolvedType::Set(Box::new(resolve_type_expr(&inner.node, inner.span, ctx)))
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
        TypeExpr::Vec2(inner) => {
            ResolvedType::Vec2(Box::new(resolve_type_expr(&inner.node, inner.span, ctx)))
        }
        TypeExpr::Vec3(inner) => {
            ResolvedType::Vec3(Box::new(resolve_type_expr(&inner.node, inner.span, ctx)))
        }
        TypeExpr::Vec4(inner) => {
            ResolvedType::Vec4(Box::new(resolve_type_expr(&inner.node, inner.span, ctx)))
        }
        TypeExpr::Quat(inner) => {
            ResolvedType::Quat(Box::new(resolve_type_expr(&inner.node, inner.span, ctx)))
        }
        TypeExpr::Mat3(inner) => {
            ResolvedType::Mat3(Box::new(resolve_type_expr(&inner.node, inner.span, ctx)))
        }
        TypeExpr::Mat4(inner) => {
            ResolvedType::Mat4(Box::new(resolve_type_expr(&inner.node, inner.span, ctx)))
        }
        TypeExpr::Generic(name, arg) => {
            let alias_id = ctx.registry.lookup(name.as_str());
            match alias_id {
                Some(id) => {
                    if let Some(TypeDef::GenericAlias(alias_def)) = ctx.registry.get(id).cloned() {
                        resolve_type_expr_with_substitution(
                            &alias_def.target_type,
                            &alias_def.type_params,
                            std::slice::from_ref(&arg.node),
                            span,
                            ctx,
                        )
                    } else {
                        ctx.emit(
                            span,
                            ErrorClass::AliasTargetNotFound,
                            format!("`{name}` is not a generic alias"),
                        );
                        ResolvedType::Named(ir::types::POISON_TYPE_ID)
                    }
                }
                None => {
                    ctx.emit(
                        span,
                        ErrorClass::UnresolvedType,
                        format!("unresolved generic type `{name}`"),
                    );
                    ResolvedType::Named(ir::types::POISON_TYPE_ID)
                }
            }
        }
        TypeExpr::BitsInline(names) => ResolvedType::BitsInline(names.clone()),
    }
}

/// Substitute type parameters in a target type expression.
///
/// `type_params` contains the parameter names (e.g., `["T"]`).
/// `type_args` contains the concrete types to substitute (e.g., `[u64]`).
/// For each `Named(name)` in `expr`, if `name` matches a parameter, replace with the
/// corresponding resolved type from `type_args`. Otherwise, resolve normally.
fn resolve_type_expr_with_substitution(
    expr: &TypeExpr,
    type_params: &[SmolStr],
    type_args: &[TypeExpr],
    span: Span,
    ctx: &mut LowerCtx,
) -> ResolvedType {
    match expr {
        TypeExpr::Named(name) => {
            if let Some(idx) = type_params.iter().position(|p| p.as_str() == name.as_str()) {
                if idx < type_args.len() {
                    return resolve_type_expr(&type_args[idx], span, ctx);
                }
            }
            resolve_type_expr(expr, span, ctx)
        }
        TypeExpr::Optional(inner) => {
            ResolvedType::Optional(Box::new(resolve_type_expr_with_substitution(
                &inner.node,
                type_params,
                type_args,
                inner.span,
                ctx,
            )))
        }
        TypeExpr::Array(inner) => {
            ResolvedType::Array(Box::new(resolve_type_expr_with_substitution(
                &inner.node,
                type_params,
                type_args,
                inner.span,
                ctx,
            )))
        }
        TypeExpr::FixedArray(inner, size) => ResolvedType::FixedArray(
            Box::new(resolve_type_expr_with_substitution(
                &inner.node,
                type_params,
                type_args,
                inner.span,
                ctx,
            )),
            *size,
        ),
        TypeExpr::Set(inner) => ResolvedType::Set(Box::new(resolve_type_expr_with_substitution(
            &inner.node,
            type_params,
            type_args,
            inner.span,
            ctx,
        ))),
        TypeExpr::Map(key, value) => ResolvedType::Map(
            Box::new(resolve_type_expr_with_substitution(
                &key.node,
                type_params,
                type_args,
                key.span,
                ctx,
            )),
            Box::new(resolve_type_expr_with_substitution(
                &value.node,
                type_params,
                type_args,
                value.span,
                ctx,
            )),
        ),
        TypeExpr::Result(ok, err) => ResolvedType::Result(
            Box::new(resolve_type_expr_with_substitution(
                &ok.node,
                type_params,
                type_args,
                ok.span,
                ctx,
            )),
            Box::new(resolve_type_expr_with_substitution(
                &err.node,
                type_params,
                type_args,
                err.span,
                ctx,
            )),
        ),
        TypeExpr::Vec2(inner) => ResolvedType::Vec2(Box::new(resolve_type_expr_with_substitution(
            &inner.node,
            type_params,
            type_args,
            inner.span,
            ctx,
        ))),
        TypeExpr::Vec3(inner) => ResolvedType::Vec3(Box::new(resolve_type_expr_with_substitution(
            &inner.node,
            type_params,
            type_args,
            inner.span,
            ctx,
        ))),
        TypeExpr::Vec4(inner) => ResolvedType::Vec4(Box::new(resolve_type_expr_with_substitution(
            &inner.node,
            type_params,
            type_args,
            inner.span,
            ctx,
        ))),
        TypeExpr::Quat(inner) => ResolvedType::Quat(Box::new(resolve_type_expr_with_substitution(
            &inner.node,
            type_params,
            type_args,
            inner.span,
            ctx,
        ))),
        TypeExpr::Mat3(inner) => ResolvedType::Mat3(Box::new(resolve_type_expr_with_substitution(
            &inner.node,
            type_params,
            type_args,
            inner.span,
            ctx,
        ))),
        TypeExpr::Mat4(inner) => ResolvedType::Mat4(Box::new(resolve_type_expr_with_substitution(
            &inner.node,
            type_params,
            type_args,
            inner.span,
            ctx,
        ))),
        TypeExpr::Generic(name, arg) => {
            let alias_id = ctx.registry.lookup(name.as_str());
            match alias_id {
                Some(id) => {
                    if let Some(TypeDef::GenericAlias(alias_def)) = ctx.registry.get(id).cloned() {
                        resolve_type_expr_with_substitution(
                            &alias_def.target_type,
                            &alias_def.type_params,
                            std::slice::from_ref(&arg.node),
                            span,
                            ctx,
                        )
                    } else {
                        resolve_type_expr(expr, span, ctx)
                    }
                }
                None => {
                    ctx.emit(
                        span,
                        ErrorClass::UnresolvedType,
                        format!("unresolved type `{name}`"),
                    );
                    ResolvedType::Named(ir::types::POISON_TYPE_ID)
                }
            }
        }
        _ => resolve_type_expr(expr, span, ctx),
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

/// Returns true if the type is eligible for @delta encoding.
fn is_delta_eligible(ty: &ResolvedType) -> bool {
    match ty {
        ResolvedType::Primitive(p) => matches!(
            p,
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
        ),
        ResolvedType::SubByte(_) => true,
        _ => false,
    }
}

/// Returns true if the encoding is already Delta-wrapped.
fn is_already_delta(enc: &FieldEncoding) -> bool {
    matches!(enc.encoding, Encoding::Delta(_))
}

/// Pick the optimal inner encoding for message-level @delta desugaring.
/// Unsigned integers get Varint (small deltas → 1-2 bytes).
/// Signed integers get ZigZag (small +/- deltas → 1-2 bytes).
/// Floats keep the field's existing encoding (varint doesn't apply).
/// Sub-byte types keep Default (already compact).
fn optimal_delta_inner(ty: &ResolvedType, enc: &FieldEncoding) -> Encoding {
    match ty {
        ResolvedType::Primitive(p) => match p {
            PrimitiveType::U8 | PrimitiveType::U16 | PrimitiveType::U32 | PrimitiveType::U64 => {
                Encoding::Varint
            }
            PrimitiveType::I8 | PrimitiveType::I16 | PrimitiveType::I32 | PrimitiveType::I64 => {
                Encoding::ZigZag
            }
            _ => enc.encoding.clone(), // f32, f64 — keep existing
        },
        ResolvedType::SubByte(_) => enc.encoding.clone(), // already sub-byte
        _ => enc.encoding.clone(),
    }
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
            _ => {
                result.custom.push(lower_custom_annotation(ann));
            }
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

fn lower_custom_annotation(ann: &Annotation) -> ir::CustomAnnotation {
    ir::CustomAnnotation {
        name: ann.name.node.clone(),
        args: ann
            .args
            .as_ref()
            .map(|args| {
                args.iter()
                    .map(|arg| ir::CustomAnnotationArg {
                        key: arg.key.as_ref().map(|k| k.node.clone()),
                        value: match &arg.value.node {
                            AnnotationValue::Int(v) => ir::CustomAnnotationValue::Int(*v),
                            AnnotationValue::Hex(v) => ir::CustomAnnotationValue::Hex(*v),
                            AnnotationValue::Str(s) => {
                                ir::CustomAnnotationValue::Str(SmolStr::new(s))
                            }
                            AnnotationValue::Bool(b) => ir::CustomAnnotationValue::Bool(*b),
                            AnnotationValue::Ident(s) => {
                                ir::CustomAnnotationValue::Ident(s.clone())
                            }
                            AnnotationValue::UpperIdent(s) => {
                                ir::CustomAnnotationValue::Ident(s.clone())
                            }
                        },
                    })
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn lower_tombstone(
    tombstone: &crate::ast::Tombstone,
    span: Span,
    ctx: &mut LowerCtx,
) -> TombstoneDef {
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
    let original_type = tombstone
        .original_type
        .as_ref()
        .map(|t| resolve_type_expr(&t.node, t.span, ctx));
    TombstoneDef {
        span,
        ordinal: tombstone.ordinal.node,
        reason,
        since,
        original_type,
    }
}

// ---------------------------------------------------------------------------
// Where clause lowering
// ---------------------------------------------------------------------------

fn lower_where_expr(expr: &WhereExpr) -> FieldConstraint {
    match expr {
        WhereExpr::And(left, right) => FieldConstraint::And(
            Box::new(lower_where_expr(&left.node)),
            Box::new(lower_where_expr(&right.node)),
        ),
        WhereExpr::Or(left, right) => FieldConstraint::Or(
            Box::new(lower_where_expr(&left.node)),
            Box::new(lower_where_expr(&right.node)),
        ),
        WhereExpr::Not(inner) => FieldConstraint::Not(Box::new(lower_where_expr(&inner.node))),
        WhereExpr::Cmp { op, operand } => FieldConstraint::Cmp {
            op: lower_cmp_op(*op),
            operand: lower_where_operand(&operand.node),
        },
        WhereExpr::Range {
            low,
            high,
            exclusive_high,
        } => FieldConstraint::Range {
            low: lower_where_operand(&low.node),
            high: lower_where_operand(&high.node),
            exclusive_high: *exclusive_high,
        },
        WhereExpr::LenCmp { op, operand } => FieldConstraint::LenCmp {
            op: lower_cmp_op(*op),
            operand: lower_where_operand(&operand.node),
        },
        WhereExpr::LenRange {
            low,
            high,
            exclusive_high,
        } => FieldConstraint::LenRange {
            low: lower_where_operand(&low.node),
            high: lower_where_operand(&high.node),
            exclusive_high: *exclusive_high,
        },
    }
}

fn lower_cmp_op(op: CmpOp) -> IrCmpOp {
    match op {
        CmpOp::Eq => IrCmpOp::Eq,
        CmpOp::Ne => IrCmpOp::Ne,
        CmpOp::Lt => IrCmpOp::Lt,
        CmpOp::Gt => IrCmpOp::Gt,
        CmpOp::Le => IrCmpOp::Le,
        CmpOp::Ge => IrCmpOp::Ge,
    }
}

fn lower_where_operand(operand: &WhereOperand) -> ConstraintOperand {
    match operand {
        WhereOperand::Int(v) => ConstraintOperand::Int(*v),
        WhereOperand::Float(v) => ConstraintOperand::Float(*v),
        WhereOperand::String(s) => ConstraintOperand::String(s.clone()),
        WhereOperand::Bool(b) => ConstraintOperand::Bool(*b),
        WhereOperand::Value => ConstraintOperand::Int(0), // placeholder, value is implicit
        WhereOperand::ConstRef(s) => ConstraintOperand::ConstRef(s.clone()),
    }
}

// ---------------------------------------------------------------------------
// Constant evaluation
// ---------------------------------------------------------------------------

fn evaluate_constants(ctx: &mut LowerCtx) {
    if ctx.const_decls.is_empty() {
        return;
    }

    // Build dependency graph - collect data first to avoid borrow issues
    let const_entries: Vec<(SmolStr, ConstDecl, Span)> = ctx
        .const_decls
        .iter()
        .map(|(name, (c, span))| (name.clone(), c.clone(), *span))
        .collect();

    let mut deps: HashMap<SmolStr, Vec<SmolStr>> = HashMap::new();
    for (name, c, _) in &const_entries {
        let mut refs = Vec::new();
        collect_const_refs(&c.value.node, &mut refs);
        deps.insert(name.clone(), refs.into_iter().cloned().collect());
    }

    // Topological sort using Kahn's algorithm
    let mut in_degree: HashMap<&SmolStr, usize> = HashMap::new();
    for (name, _, _) in &const_entries {
        in_degree.insert(name, 0);
    }

    for (name, refs) in &deps {
        for ref_name in refs {
            if ctx.const_decls.contains_key(ref_name) {
                *in_degree.entry(name).or_insert(0) += 1;
            }
        }
    }

    let mut queue: Vec<&SmolStr> = in_degree
        .iter()
        .filter(|(_, &count)| count == 0)
        .map(|(name, _)| *name)
        .collect();

    let mut eval_order: Vec<SmolStr> = Vec::new();

    while let Some(name) = queue.pop() {
        eval_order.push(name.clone());

        // Find all consts that depend on this one
        for (other_name, other_deps) in &deps {
            if other_deps.contains(name) {
                let count = in_degree.get_mut(other_name).unwrap();
                *count -= 1;
                if *count == 0 {
                    queue.push(other_name);
                }
            }
        }
    }

    // Evaluate in dependency order
    for name in &eval_order {
        // Clone the data we need to avoid borrow conflicts
        let const_data = ctx.const_decls.get(name).cloned();
        if let Some((c, span)) = const_data {
            let resolved_type = resolve_type_expr(&c.ty.node, c.ty.span, ctx);
            match eval_const_expr(&c.value.node, &ctx.constants) {
                Some(value) => {
                    ctx.constants.insert(
                        name.clone(),
                        ConstValue {
                            ty: resolved_type,
                            value,
                            span,
                        },
                    );
                }
                None => {
                    // Reference not found - emit error
                    ctx.emit(
                        c.value.span,
                        ErrorClass::ConstRefNotFound,
                        format!("could not evaluate constant `{}`", name),
                    );
                }
            }
        }
    }
}

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

fn eval_const_expr(expr: &ConstExpr, values: &HashMap<SmolStr, ConstValue>) -> Option<i64> {
    match expr {
        ConstExpr::Int(v) => Some(*v),
        ConstExpr::UInt(v) => Some(*v as i64),
        ConstExpr::Hex(v) => Some(*v as i64),
        ConstExpr::ConstRef(name) => values.get(name).map(|cv| cv.value),
        ConstExpr::BinOp { op, left, right } => {
            let left_val = eval_const_expr(left, values)?;
            let right_val = eval_const_expr(right, values)?;

            match op {
                BinOpKind::Add => left_val.checked_add(right_val),
                BinOpKind::Sub => left_val.checked_sub(right_val),
                BinOpKind::Mul => left_val.checked_mul(right_val),
                BinOpKind::Div => {
                    if right_val == 0 {
                        None // Division by zero — caught in validate phase
                    } else {
                        left_val.checked_div(right_val)
                    }
                }
                // Comparison operators not valid in const expressions
                _ => None,
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod dep_tests {
    use super::*;

    #[test]
    fn transparent_alias_does_not_displace_following_declaration() {
        let result = crate::compile(
            "namespace test.alias_order\ntype Count = u16\nmessage M { value @0 : Count }",
        );
        let compiled = result.compiled.expect("schema should compile");
        assert_eq!(compiled.declarations.len(), 1);
        let id = compiled.declarations[0];
        let Some(TypeDef::Message(message)) = compiled.registry.get(id) else {
            panic!("following declaration must remain a message");
        };
        assert_eq!(message.name.as_str(), "M");
        assert_eq!(message.fields.len(), 1);
        assert!(matches!(
            message.fields[0].resolved_type,
            ResolvedType::Primitive(PrimitiveType::U16)
        ));
    }

    #[test]
    fn public_lowering_api_rejects_invariants_without_partial_ir() {
        let parsed = crate::parse(
            "namespace test.guard\nmessage M { value @0 : u32 invariant { value > 0 } }",
        );
        let schema = parsed.schema.expect("schema parses");
        let (compiled, diagnostics) = lower(&schema);
        assert!(compiled.is_none());
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.class == ErrorClass::InvariantUnsupported
                && diagnostic.message == "message invariants are not yet supported"
        }));
    }

    #[test]
    fn nested_alias_chains_are_rejected_regardless_of_source_order() {
        for source in [
            "namespace test.alias_order\ntype Base = u32\ntype Wrapped = optional<Base>",
            "namespace test.alias_order\ntype Wrapped = optional<Base>\ntype Base = u32",
        ] {
            let result = crate::compile(source);
            assert!(result.compiled.is_none());
            assert!(result
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.class == ErrorClass::AliasTargetIsAlias }));
        }
    }

    #[test]
    fn concrete_alias_spelling_has_no_hash_wire_or_compatibility_effect() {
        let aliased = crate::compile(
            "namespace test.alias_contract\ntype Values = array<u16>\nmessage M { values @0 : Values }",
        )
        .compiled
        .expect("aliased schema compiles");
        let expanded =
            crate::compile("namespace test.alias_contract\nmessage M { values @0 : array<u16> }")
                .compiled
                .expect("expanded schema compiles");

        assert_eq!(
            crate::canonical::schema_hash(&aliased),
            crate::canonical::schema_hash(&expanded)
        );
        let report = crate::compat::check(&aliased, &expanded);
        assert_eq!(report.result, crate::compat::CompatResult::Compatible);
        assert!(report.changes.is_empty(), "{:?}", report.changes);
    }

    #[test]
    fn concrete_container_aliases_resolve_transparently() {
        let result = crate::compile(
            "namespace test.container_aliases\n\
             type Names = array<string>\n\
             type Lookup = map<string, optional<u32>>\n\
             type Outcome = result<array<string>, string>\n\
             message M { names @0 : Names lookup @1 : Lookup outcome @2 : Outcome }",
        );
        let compiled = result.compiled.expect("schema should compile");
        let message = compiled
            .declarations
            .iter()
            .filter_map(|id| compiled.registry.get(*id))
            .find_map(|definition| match definition {
                TypeDef::Message(message) => Some(message),
                _ => None,
            })
            .expect("message declaration");
        assert!(matches!(
            message.fields[0].resolved_type,
            ResolvedType::Array(_)
        ));
        assert!(matches!(
            message.fields[1].resolved_type,
            ResolvedType::Map(_, _)
        ));
        assert!(matches!(
            message.fields[2].resolved_type,
            ResolvedType::Result(_, _)
        ));
    }

    #[test]
    fn concrete_aliases_are_importable_by_name_wildcard_and_namespace() {
        let dep = crate::compile(
            "namespace dep.aliases\nmessage Item { id @0 : u32 }\ntype Names = array<Item>",
        )
        .compiled
        .expect("dependency compiles");

        for source in [
            "namespace root.named\nimport { Names } from dep.aliases\nmessage M { value @0 : Names }",
            "namespace root.wildcard\nimport dep.aliases\nmessage M { value @0 : Names }",
            "namespace root.qualified\nimport dep.aliases as DT\nmessage M { value @0 : DT.Names }",
        ] {
            let schema = crate::parse(source).schema.expect("root parses");
            let mut deps = DependencyContext {
                schemas: HashMap::new(),
            };
            deps.schemas.insert("dep.aliases".to_owned(), dep.clone());
            let (compiled, diagnostics) = lower_with_deps(&schema, Some(&deps));
            let errors: Vec<_> = diagnostics
                .iter()
                .filter(|diagnostic| {
                    diagnostic.severity == crate::diagnostic::Severity::Error
                })
                .collect();
            assert!(errors.is_empty(), "unexpected errors: {errors:?}");
            let compiled = compiled.expect("root compiles");
            let message = compiled
                .declarations
                .iter()
                .filter_map(|id| compiled.registry.get(*id))
                .find_map(|definition| match definition {
                    TypeDef::Message(message) => Some(message),
                    _ => None,
                })
                .expect("message declaration");
            let ResolvedType::Array(inner) = &message.fields[0].resolved_type else {
                panic!("alias should resolve to an array");
            };
            let ResolvedType::Named(item_id) = **inner else {
                panic!("alias target should retain its named element type");
            };
            assert!(matches!(
                compiled.registry.get(item_id),
                Some(TypeDef::Message(item)) if item.name == "Item"
            ));
        }
    }

    #[test]
    fn imported_alias_targets_remap_through_transitive_and_diamond_graphs() {
        let leaf = crate::compile("namespace dep.leaf\nmessage Item { id @0 : u32 }")
            .compiled
            .expect("leaf compiles");

        let compile_branch = |namespace: &str, alias: &str| {
            let source = format!(
                "namespace {namespace}\nimport {{ Item }} from dep.leaf\ntype {alias} = array<Item>"
            );
            let schema = crate::parse(&source).schema.expect("branch parses");
            let mut dependencies = DependencyContext {
                schemas: HashMap::new(),
            };
            dependencies
                .schemas
                .insert("dep.leaf".to_owned(), leaf.clone());
            let (compiled, diagnostics) = lower_with_deps(&schema, Some(&dependencies));
            assert!(
                diagnostics.iter().all(|diagnostic| {
                    diagnostic.severity != crate::diagnostic::Severity::Error
                }),
                "branch errors: {diagnostics:?}"
            );
            compiled.expect("branch compiles")
        };

        let left = compile_branch("dep.left", "LeftItems");
        let right = compile_branch("dep.right", "RightItems");
        let root = crate::parse(
            "namespace root\n\
             import { LeftItems } from dep.left\n\
             import { RightItems } from dep.right\n\
             message M { left @0 : LeftItems right @1 : RightItems }",
        )
        .schema
        .expect("root parses");
        let mut dependencies = DependencyContext {
            schemas: HashMap::new(),
        };
        dependencies.schemas.insert("dep.left".to_owned(), left);
        dependencies.schemas.insert("dep.right".to_owned(), right);

        let (compiled, diagnostics) = lower_with_deps(&root, Some(&dependencies));
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.severity != crate::diagnostic::Severity::Error),
            "root errors: {diagnostics:?}"
        );
        let compiled = compiled.expect("root compiles");
        let message = compiled
            .declarations
            .iter()
            .filter_map(|id| compiled.registry.get(*id))
            .find_map(|definition| match definition {
                TypeDef::Message(message) => Some(message),
                _ => None,
            })
            .expect("message declaration");
        let element_id = |field: &FieldDef| match &field.resolved_type {
            ResolvedType::Array(inner) => match **inner {
                ResolvedType::Named(id) => id,
                _ => panic!("array element must remain named"),
            },
            _ => panic!("alias must remain an array"),
        };
        assert_eq!(
            element_id(&message.fields[0]),
            element_id(&message.fields[1]),
            "diamond imports must deduplicate the defining Item identity"
        );
    }

    #[test]
    fn imported_aliases_cannot_be_targets_by_name_wildcard_or_namespace() {
        let dependency = crate::compile("namespace dep.aliases\ntype Names = array<string>")
            .compiled
            .expect("dependency compiles");

        for source in [
            "namespace root.named\nimport { Names } from dep.aliases\ntype Local = optional<Names>",
            "namespace root.wildcard\nimport dep.aliases\ntype Local = optional<Names>",
            "namespace root.qualified\nimport dep.aliases as DT\ntype Local = optional<DT.Names>",
        ] {
            let schema = crate::parse(source).schema.expect("root parses");
            let mut dependencies = DependencyContext {
                schemas: HashMap::new(),
            };
            dependencies
                .schemas
                .insert("dep.aliases".to_owned(), dependency.clone());
            let (compiled, diagnostics) = lower_with_deps(&schema, Some(&dependencies));
            assert!(compiled.is_none(), "alias chain emitted partial IR");
            assert!(diagnostics.iter().any(|diagnostic| {
                diagnostic.class == ErrorClass::AliasTargetIsAlias
                    && diagnostic.message.contains("imported alias")
            }));
        }
    }

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
        assert!(diags
            .iter()
            .any(|d| d.message.contains("unknown import") || d.message.contains("not found")));
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
