// crates/vexil-codegen-rust/src/backend.rs

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

use vexil_lang::codegen::{CodegenBackend, CodegenError};
use vexil_lang::ir::{CompiledSchema, ResolvedType, TypeDef, TypeId};
use vexil_lang::project::ProjectResult;

/// Rust code-generation backend for Vexil schemas.
///
/// Generates idiomatic Rust structs, enums, and encode/decode implementations
/// using the `vexil-runtime` crate.
#[derive(Debug, Clone, Copy)]
pub struct RustBackend;

impl CodegenBackend for RustBackend {
    fn name(&self) -> &str {
        "rust"
    }

    fn file_extension(&self) -> &str {
        "rs"
    }

    fn generate(&self, compiled: &CompiledSchema) -> Result<String, CodegenError> {
        crate::generate(compiled).map_err(|e| CodegenError::BackendSpecific(Box::new(e)))
    }

    fn generate_project(
        &self,
        result: &ProjectResult,
    ) -> Result<BTreeMap<PathBuf, String>, CodegenError> {
        let mut files = BTreeMap::new();
        let mut mod_tree: BTreeMap<String, Vec<String>> = BTreeMap::new();

        // Step 1: Build a global type_name -> Rust path map from all schemas' declarations.
        let mut global_type_map: HashMap<String, String> = HashMap::new();
        for (ns, compiled) in &result.schemas {
            let segments: Vec<&str> = ns.split('.').collect();
            let rust_module = segments.join("::");
            for &type_id in &compiled.declarations {
                if let Some(typedef) = compiled.registry.get(type_id) {
                    let name = crate::type_name_of(typedef);
                    let rust_path = format!("crate::{rust_module}::{name}");
                    global_type_map.insert(name.to_string(), rust_path);
                }
            }
        }

        for (ns, compiled) in &result.schemas {
            let segments: Vec<&str> = ns.split('.').collect();
            if segments.is_empty() {
                continue;
            }
            let file_name = segments[segments.len() - 1];
            let dir_segments = &segments[..segments.len() - 1];

            // Track mod.rs entries
            for i in 0..segments.len() - 1 {
                let parent_key = segments[..i].join("/");
                let child = segments[i].to_string();
                let entry = mod_tree.entry(parent_key).or_default();
                if !entry.contains(&child) {
                    entry.push(child);
                }
            }
            if segments.len() >= 2 {
                let parent_key = dir_segments.join("/");
                let child = file_name.to_string();
                let entry = mod_tree.entry(parent_key).or_default();
                if !entry.contains(&child) {
                    entry.push(child);
                }
            } else {
                let entry = mod_tree.entry(String::new()).or_default();
                let child = file_name.to_string();
                if !entry.contains(&child) {
                    entry.push(child);
                }
            }

            // Step 2: Build import_paths for this schema.
            let declared_ids: HashSet<TypeId> = compiled.declarations.iter().copied().collect();

            // Collect all Named TypeIds referenced by declared types.
            let mut import_paths: HashMap<TypeId, String> = HashMap::new();
            for &type_id in &compiled.declarations {
                if let Some(typedef) = compiled.registry.get(type_id) {
                    collect_named_ids_from_typedef(typedef, &declared_ids, |imported_id| {
                        if let Some(imported_def) = compiled.registry.get(imported_id) {
                            let name = crate::type_name_of(imported_def);
                            if let Some(rust_path) = global_type_map.get(name) {
                                import_paths.insert(imported_id, rust_path.clone());
                            }
                        }
                    });
                }
            }

            // Generate code with cross-file imports.
            let imports = if import_paths.is_empty() {
                None
            } else {
                Some(&import_paths)
            };
            let code = crate::generate_with_imports(compiled, imports)
                .map_err(|e| CodegenError::BackendSpecific(Box::new(e)))?;

            let mut file_path = PathBuf::new();
            for seg in dir_segments {
                file_path.push(seg);
            }
            file_path.push(format!("{file_name}.rs"));
            files.insert(file_path, code);
        }

        // Generate mod.rs files
        for (dir_key, children) in &mod_tree {
            let mut mod_path = PathBuf::new();
            if !dir_key.is_empty() {
                for seg in dir_key.split('/') {
                    mod_path.push(seg);
                }
            }
            mod_path.push("mod.rs");

            let child_refs: Vec<&str> = children.iter().map(|s| s.as_str()).collect();
            let mod_content = crate::generate_mod_file(&child_refs);
            files.insert(mod_path, mod_content);
        }

        Ok(files)
    }
}

/// Collect all `ResolvedType::Named(id)` from a TypeDef where `id` is NOT in
/// the declared set (i.e., it's an imported type). Calls `on_import` for each.
fn collect_named_ids_from_typedef(
    typedef: &TypeDef,
    declared: &HashSet<TypeId>,
    mut on_import: impl FnMut(TypeId),
) {
    match typedef {
        TypeDef::Message(msg) => {
            for f in &msg.fields {
                collect_named_ids_from_resolved(&f.resolved_type, declared, &mut on_import);
            }
        }
        TypeDef::Union(un) => {
            for v in &un.variants {
                for f in &v.fields {
                    collect_named_ids_from_resolved(&f.resolved_type, declared, &mut on_import);
                }
            }
        }
        TypeDef::Newtype(nt) => {
            collect_named_ids_from_resolved(&nt.inner_type, declared, &mut on_import);
        }
        TypeDef::Config(cfg) => {
            for f in &cfg.fields {
                collect_named_ids_from_resolved(&f.resolved_type, declared, &mut on_import);
            }
        }
        _ => {}
    }
}

/// Recursively collect imported Named type IDs from a ResolvedType tree.
fn collect_named_ids_from_resolved(
    ty: &ResolvedType,
    declared: &HashSet<TypeId>,
    on_import: &mut impl FnMut(TypeId),
) {
    match ty {
        ResolvedType::Named(id) => {
            if !declared.contains(id) {
                on_import(*id);
            }
        }
        ResolvedType::Optional(inner) | ResolvedType::Array(inner) | ResolvedType::Set(inner) => {
            collect_named_ids_from_resolved(inner, declared, on_import);
        }
        ResolvedType::Map(k, v) | ResolvedType::Result(k, v) => {
            collect_named_ids_from_resolved(k, declared, on_import);
            collect_named_ids_from_resolved(v, declared, on_import);
        }
        _ => {}
    }
}
