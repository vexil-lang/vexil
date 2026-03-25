use std::collections::HashSet;

use vexil_lang::ast::DefaultValue;
use vexil_lang::ir::{ConfigDef, ConfigFieldDef, ResolvedType, TypeDef, TypeId, TypeRegistry};

use crate::annotations::{emit_field_annotations, emit_type_annotations};
use crate::emit::CodeWriter;
use crate::types::rust_type;

/// Emit a config struct with a `Default` implementation.
///
/// Config types are compile-time only — no `Pack`/`Unpack` is generated.
pub fn emit_config(
    w: &mut CodeWriter,
    cfg: &ConfigDef,
    registry: &TypeRegistry,
    needs_box: &HashSet<(TypeId, usize)>,
) {
    let name = cfg.name.as_str();

    // ── Type-level annotations ───────────────────────────────────────────────
    emit_type_annotations(w, &cfg.annotations);

    // ── Struct definition ────────────────────────────────────────────────────
    w.line("#[derive(Debug, Clone, PartialEq)]");
    w.open_block(&format!("pub struct {name}"));
    for field in &cfg.fields {
        emit_field_annotations(w, &field.annotations);
        let field_rust_type = rust_type(&field.resolved_type, registry, needs_box, None);
        w.line(&format!("pub {}: {},", field.name, field_rust_type));
    }
    w.close_block();
    w.blank();

    // ── Default impl ─────────────────────────────────────────────────────────
    w.open_block(&format!("impl Default for {name}"));
    w.open_block("fn default() -> Self");
    w.open_block("Self");
    for field in &cfg.fields {
        let expr = default_value_expr(&field.default_value, field, registry);
        w.line(&format!("{}: {},", field.name, expr));
    }
    w.close_block();
    w.close_block();
    w.close_block();
    w.blank();
}

/// Convert a `DefaultValue` to its Rust literal / expression.
fn default_value_expr(
    value: &DefaultValue,
    field: &ConfigFieldDef,
    registry: &TypeRegistry,
) -> String {
    match value {
        DefaultValue::None => "None".to_string(),
        DefaultValue::Bool(b) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        DefaultValue::Int(n) => format!("{n}"),
        DefaultValue::UInt(n) => format!("{n}"),
        DefaultValue::Float(f) => format!("{f}"),
        DefaultValue::Str(s) => format!("String::from(\"{s}\")"),
        // An identifier that matches a named type's variant — treated as an
        // enum variant reference using the field's resolved type name.
        DefaultValue::Ident(name) => {
            let type_name = resolve_type_name(&field.resolved_type, registry);
            format!("{type_name}::{name}")
        }
        // Upper-case identifiers are always enum variant references.
        DefaultValue::UpperIdent(name) => {
            let type_name = resolve_type_name(&field.resolved_type, registry);
            format!("{type_name}::{name}")
        }
        DefaultValue::Array(items) => {
            let exprs: Vec<String> = items
                .iter()
                .map(|spanned| default_value_expr(&spanned.node, field, registry))
                .collect();
            format!("vec![{}]", exprs.join(", "))
        }
    }
}

/// Given a resolved type, return the Rust type name string for use in enum
/// variant expressions like `TypeName::Variant`.
fn resolve_type_name(ty: &ResolvedType, registry: &TypeRegistry) -> String {
    match ty {
        ResolvedType::Named(id) => match registry.get(*id) {
            Some(TypeDef::Enum(e)) => e.name.to_string(),
            Some(TypeDef::Flags(f)) => f.name.to_string(),
            Some(TypeDef::Newtype(n)) => n.name.to_string(),
            Some(TypeDef::Message(m)) => m.name.to_string(),
            Some(TypeDef::Union(u)) => u.name.to_string(),
            Some(TypeDef::Config(c)) => c.name.to_string(),
            _ => "UnresolvedType".to_string(),
        },
        // For optional/array/etc., unwrap one layer
        ResolvedType::Optional(inner) => resolve_type_name(inner, registry),
        _ => "UnresolvedType".to_string(),
    }
}
