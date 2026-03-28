use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

use vexil_lang::codegen::{CodegenBackend, CodegenError};
use vexil_lang::ir::{CompiledSchema, ResolvedType, TypeDef, TypeId};
use vexil_lang::project::ProjectResult;

/// Go code-generation backend for Vexil schemas.
///
/// Generates Go structs, enums (const blocks), flags, unions (interface + variant
/// structs), and Pack/Unpack methods using the `github.com/vexil-lang/vexil/packages/runtime-go`
/// package.
#[derive(Debug, Clone, Copy)]
pub struct GoBackend;

impl CodegenBackend for GoBackend {
    fn name(&self) -> &str {
        "go"
    }

    fn file_extension(&self) -> &str {
        "go"
    }

    fn generate(&self, compiled: &CompiledSchema) -> Result<String, CodegenError> {
        crate::generate(compiled).map_err(|e| CodegenError::BackendSpecific(Box::new(e)))
    }

    fn generate_project(
        &self,
        result: &ProjectResult,
    ) -> Result<BTreeMap<PathBuf, String>, CodegenError> {
        let mut files = BTreeMap::new();

        // Step 1: Build a global type_name -> Go package path map.
        let mut global_type_map: HashMap<String, String> = HashMap::new();
        for (ns, compiled) in &result.schemas {
            let segments: Vec<&str> = ns.split('.').collect();
            let go_pkg = segments.join("/");
            for &type_id in &compiled.declarations {
                if let Some(typedef) = compiled.registry.get(type_id) {
                    let name = crate::type_name_of(typedef);
                    global_type_map.insert(name.to_string(), go_pkg.clone());
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

            // Step 2: Build import_types for this schema.
            let declared_ids: HashSet<TypeId> = compiled.declarations.iter().copied().collect();

            let mut import_types: HashMap<String, String> = HashMap::new();
            for &type_id in &compiled.declarations {
                if let Some(typedef) = compiled.registry.get(type_id) {
                    collect_named_ids_from_typedef(typedef, &declared_ids, |imported_id| {
                        if let Some(imported_def) = compiled.registry.get(imported_id) {
                            let name = crate::type_name_of(imported_def);
                            if let Some(go_path) = global_type_map.get(name) {
                                import_types.insert(name.to_string(), go_path.clone());
                            }
                        }
                    });
                }
            }

            // Generate code with cross-file imports.
            let imports = if import_types.is_empty() {
                None
            } else {
                Some(&import_types)
            };
            let code = crate::generate_with_imports(compiled, imports)
                .map_err(|e| CodegenError::BackendSpecific(Box::new(e)))?;

            let mut file_path = PathBuf::new();
            for seg in dir_segments {
                file_path.push(seg);
            }
            file_path.push(format!("{file_name}.go"));
            files.insert(file_path, code);
        }

        // Go doesn't need barrel/index files — imports are package-level

        Ok(files)
    }
}

/// Collect all `ResolvedType::Named(id)` from a TypeDef where `id` is NOT in
/// the declared set (i.e., it's an imported type).
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
        ResolvedType::Optional(inner) | ResolvedType::Array(inner) => {
            collect_named_ids_from_resolved(inner, declared, on_import);
        }
        ResolvedType::Map(k, v) | ResolvedType::Result(k, v) => {
            collect_named_ids_from_resolved(k, declared, on_import);
            collect_named_ids_from_resolved(v, declared, on_import);
        }
        _ => {}
    }
}
