// crates/vexil-codegen-rust/src/backend.rs

use std::collections::BTreeMap;
use std::path::PathBuf;

use vexil_lang::codegen::{CodegenBackend, CodegenError};
use vexil_lang::ir::CompiledSchema;
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

            // Generate code
            let code = crate::generate(compiled)
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
