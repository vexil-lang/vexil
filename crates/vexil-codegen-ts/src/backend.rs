use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

use vexil_lang::codegen::portable::{PortableExpr, PortableExprKind, PortableStatement};
use vexil_lang::codegen::{CodegenBackend, CodegenError};
use vexil_lang::ir::{CompiledSchema, ResolvedType, TypeDef, TypeId};
use vexil_lang::project::ProjectResult;

/// TypeScript code-generation backend for Vexil schemas.
///
/// Generates TypeScript interfaces, encode/decode functions, and type-safe
/// discriminated unions using the `@vexil-lang/runtime` package.
#[derive(Debug, Clone, Copy)]
pub struct TypeScriptBackend;

impl CodegenBackend for TypeScriptBackend {
    fn name(&self) -> &str {
        "typescript"
    }

    fn file_extension(&self) -> &str {
        "ts"
    }

    fn generate(&self, compiled: &CompiledSchema) -> Result<String, CodegenError> {
        crate::generate(compiled).map_err(|e| CodegenError::BackendSpecific(Box::new(e)))
    }

    fn generate_project(
        &self,
        result: &ProjectResult,
    ) -> Result<BTreeMap<PathBuf, String>, CodegenError> {
        let mut files = BTreeMap::new();
        let mut index_tree: BTreeMap<String, Vec<String>> = BTreeMap::new();

        // Step 1: Build a global type_name -> TS module path map.
        let mut global_type_map: HashMap<String, String> = HashMap::new();
        for (ns, compiled) in &result.schemas {
            let segments: Vec<&str> = ns.split('.').collect();
            let ts_module = format!("./{}", segments.join("/"));
            for &type_id in &compiled.declarations {
                if let Some(typedef) = compiled.registry.get(type_id) {
                    let name = crate::type_name_of(typedef);
                    global_type_map.insert(name.to_string(), ts_module.clone());
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

            // Track index.ts entries
            for i in 0..segments.len() - 1 {
                let parent_key = segments[..i].join("/");
                let child = segments[i].to_string();
                let entry = index_tree.entry(parent_key).or_default();
                if !entry.contains(&child) {
                    entry.push(child);
                }
            }
            if segments.len() >= 2 {
                let parent_key = dir_segments.join("/");
                let child = file_name.to_string();
                let entry = index_tree.entry(parent_key).or_default();
                if !entry.contains(&child) {
                    entry.push(child);
                }
            } else {
                let entry = index_tree.entry(String::new()).or_default();
                let child = file_name.to_string();
                if !entry.contains(&child) {
                    entry.push(child);
                }
            }

            // Step 2: Build import_paths for this schema.
            let declared_ids: HashSet<TypeId> = compiled.declarations.iter().copied().collect();

            let mut import_paths: HashMap<String, String> = HashMap::new();
            let impl_ids = compiled.impls().map(|(id, _)| id).collect::<Vec<_>>();
            for &type_id in compiled.declarations.iter().chain(impl_ids.iter()) {
                if let Some(typedef) = compiled.registry.get(type_id) {
                    collect_named_ids_from_typedef(
                        typedef,
                        compiled,
                        &declared_ids,
                        |imported_id| {
                            if let Some(imported_def) = compiled.registry.get(imported_id) {
                                let name = crate::type_name_of(imported_def);
                                if let Some(ts_path) = global_type_map.get(name) {
                                    import_paths.insert(name.to_string(), ts_path.clone());
                                }
                            }
                        },
                    );
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
            file_path.push(format!("{file_name}.ts"));
            files.insert(file_path, code);
        }

        // Generate index.ts files
        for (dir_key, children) in &index_tree {
            let mut index_path = PathBuf::new();
            if !dir_key.is_empty() {
                for seg in dir_key.split('/') {
                    index_path.push(seg);
                }
            }
            index_path.push("index.ts");

            let mut content = String::from("// Code generated by vexilc. DO NOT EDIT.\n\n");
            for child in children {
                content.push_str(&format!("export * from './{child}';\n"));
            }
            files.insert(index_path, content);
        }

        Ok(files)
    }
}

/// Collect all `ResolvedType::Named(id)` from a TypeDef where `id` is NOT in
/// the declared set (i.e., it's an imported type).
fn collect_named_ids_from_typedef(
    typedef: &TypeDef,
    compiled: &CompiledSchema,
    declared: &HashSet<TypeId>,
    mut on_import: impl FnMut(TypeId),
) {
    let registry = &compiled.registry;
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
        TypeDef::Trait(trait_def) => {
            for field in &trait_def.fields {
                collect_named_ids_from_resolved(&field.ty, declared, &mut on_import);
            }
            for function in &trait_def.functions {
                for parameter in &function.params {
                    collect_named_ids_from_resolved(&parameter.ty, declared, &mut on_import);
                }
                if let Some(return_type) = &function.return_type {
                    collect_named_ids_from_resolved(return_type, declared, &mut on_import);
                }
            }
        }
        TypeDef::Impl(impl_def) => {
            collect_named_ids_from_resolved(&impl_def.target_type, declared, &mut on_import);
            for arg in &impl_def.type_args {
                collect_named_ids_from_resolved(arg, declared, &mut on_import);
            }
            for function in &impl_def.functions {
                for parameter in &function.params {
                    collect_named_ids_from_resolved(&parameter.ty, declared, &mut on_import);
                }
                if let Some(return_type) = &function.return_type {
                    collect_named_ids_from_resolved(return_type, declared, &mut on_import);
                }
            }
            if let Ok(functions) = vexil_lang::codegen::portable::project_impl(compiled, impl_def) {
                for function in &functions {
                    for statement in &function.statements {
                        collect_named_ids_from_statement(statement, declared, &mut on_import);
                    }
                }
            }
            for (id, def) in registry.iter() {
                if matches!(def, TypeDef::Trait(t) if t.name == impl_def.trait_name)
                    && !declared.contains(&id)
                {
                    on_import(id);
                }
            }
        }
        _ => {}
    }
}

fn collect_named_ids_from_statement(
    statement: &PortableStatement,
    declared: &HashSet<TypeId>,
    on_import: &mut impl FnMut(TypeId),
) {
    match statement {
        PortableStatement::Let { ty, value, .. } => {
            collect_named_ids_from_resolved(ty, declared, on_import);
            collect_named_ids_from_expr(value, declared, on_import);
        }
        PortableStatement::Return(Some(value))
        | PortableStatement::AssignSelfField { value, .. } => {
            collect_named_ids_from_expr(value, declared, on_import);
        }
        PortableStatement::Return(None) => {}
    }
}

fn collect_named_ids_from_expr(
    expression: &PortableExpr,
    declared: &HashSet<TypeId>,
    on_import: &mut impl FnMut(TypeId),
) {
    collect_named_ids_from_resolved(&expression.ty, declared, on_import);
    match &expression.kind {
        PortableExprKind::Binary(_, left, right) => {
            collect_named_ids_from_expr(left, declared, on_import);
            collect_named_ids_from_expr(right, declared, on_import);
        }
        PortableExprKind::Unary(_, value) => {
            collect_named_ids_from_expr(value, declared, on_import);
        }
        PortableExprKind::Int(_)
        | PortableExprKind::UInt(_)
        | PortableExprKind::Float(_)
        | PortableExprKind::Bool(_)
        | PortableExprKind::String(_)
        | PortableExprKind::Local(_)
        | PortableExprKind::SelfRef
        | PortableExprKind::SelfField(_) => {}
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
        ResolvedType::Optional(inner)
        | ResolvedType::Array(inner)
        | ResolvedType::Set(inner)
        | ResolvedType::Vec2(inner)
        | ResolvedType::Vec3(inner)
        | ResolvedType::Vec4(inner)
        | ResolvedType::Quat(inner)
        | ResolvedType::Mat3(inner)
        | ResolvedType::Mat4(inner) => {
            collect_named_ids_from_resolved(inner, declared, on_import);
        }
        ResolvedType::FixedArray(inner, _) => {
            collect_named_ids_from_resolved(inner, declared, on_import);
        }
        ResolvedType::Map(k, v) | ResolvedType::Result(k, v) => {
            collect_named_ids_from_resolved(k, declared, on_import);
            collect_named_ids_from_resolved(v, declared, on_import);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vexil_lang::codegen::CodegenBackend;
    use vexil_lang::ir::POISON_TYPE_ID;
    use vexil_lang::resolve::InMemoryLoader;

    #[test]
    fn collects_named_ids_from_every_resolved_container() {
        let named = ResolvedType::Named(POISON_TYPE_ID);
        let containers = [
            ResolvedType::Optional(Box::new(named.clone())),
            ResolvedType::Array(Box::new(named.clone())),
            ResolvedType::FixedArray(Box::new(named.clone()), 2),
            ResolvedType::Set(Box::new(named.clone())),
            ResolvedType::Map(
                Box::new(ResolvedType::Primitive(vexil_lang::ast::PrimitiveType::U8)),
                Box::new(named.clone()),
            ),
            ResolvedType::Result(
                Box::new(named.clone()),
                Box::new(ResolvedType::Primitive(vexil_lang::ast::PrimitiveType::U8)),
            ),
            ResolvedType::Vec2(Box::new(named.clone())),
            ResolvedType::Vec3(Box::new(named.clone())),
            ResolvedType::Vec4(Box::new(named.clone())),
            ResolvedType::Quat(Box::new(named.clone())),
            ResolvedType::Mat3(Box::new(named.clone())),
            ResolvedType::Mat4(Box::new(named)),
        ];

        for container in containers {
            let mut collected = Vec::new();
            collect_named_ids_from_resolved(&container, &HashSet::new(), &mut |id| {
                collected.push(id);
            });
            assert_eq!(collected, vec![POISON_TYPE_ID], "{container:?}");
        }
    }

    #[test]
    fn project_codegen_collects_function_only_imports_through_diamond() {
        let result = function_import_diamond();
        let files = TypeScriptBackend.generate_project(&result).unwrap();

        let left = project_file(&files, "left.ts");
        assert!(
            left.contains(
                "import { Payload, encodePayload, decodePayload } from './imports/base';"
            ),
            "{left}"
        );
        assert!(left.contains("left(input: Payload): Payload;"), "{left}");

        let root = project_file(&files, "root.ts");
        assert!(
            root.contains(
                "import { Payload, encodePayload, decodePayload } from './imports/base';"
            ),
            "{root}"
        );
        assert!(
            root.contains("import { LeftTransformer } from './imports/left';"),
            "{root}"
        );
        assert!(
            root.contains("import { RightTransformer } from './imports/right';"),
            "{root}"
        );
        assert_eq!(result.schemas.len(), 4);
    }

    fn function_import_diamond() -> ProjectResult {
        let mut loader = InMemoryLoader::new();
        loader.schemas.insert(
            "imports.base".into(),
            "namespace imports.base\nmessage Payload { value @0 : i32 }".into(),
        );
        loader.schemas.insert(
            "imports.left".into(),
            "namespace imports.left\nimport { Payload } from imports.base\ntrait LeftTransformer { fn left(input: Payload) -> Payload }".into(),
        );
        loader.schemas.insert(
            "imports.right".into(),
            "namespace imports.right\nimport { Payload } from imports.base\ntrait RightTransformer { fn right(input: Payload) -> Payload }".into(),
        );
        let root = "namespace imports.root\nimport { LeftTransformer } from imports.left\nimport { RightTransformer } from imports.right\nmessage Host { marker @0 : i32 }\nimpl LeftTransformer for Host { fn left(input: Payload) -> Payload { let staged: Payload = input return staged } }\nimpl RightTransformer for Host { fn right(input: Payload) -> Payload { return input } }";
        let result =
            vexil_lang::compile_project(root, &PathBuf::from("imports/root.vexil"), &loader)
                .unwrap();
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        result
    }

    fn project_file<'a>(files: &'a BTreeMap<PathBuf, String>, suffix: &str) -> &'a str {
        files
            .iter()
            .find(|(path, _)| path.to_string_lossy().ends_with(suffix))
            .map(|(_, code)| code.as_str())
            .unwrap()
    }
}
