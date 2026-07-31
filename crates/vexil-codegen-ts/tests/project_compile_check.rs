//! Integration tests for multi-file project compilation + TypeScript codegen.
//!
//! For each corpus project:
//! 1. Load root file, create FilesystemLoader with project dir as include root
//! 2. Compile project via compile_project()
//! 3. Assert no error-level diagnostics
//! 4. Generate TypeScript files via generate_project()
//! 5. Verify generated files are non-empty and contain expected TypeScript content

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use vexil_lang::codegen::CodegenBackend;
use vexil_lang::diagnostic::Severity;
use vexil_lang::resolve::FilesystemLoader;

static TEMP_PROJECT_COUNTER: AtomicU64 = AtomicU64::new(0);

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("corpus")
        .join("projects")
}

fn native_tsc_check(project_name: &str, files: &BTreeMap<PathBuf, String>) {
    let Some(tsc) = std::env::var_os("VEXIL_TSC") else {
        return;
    };

    let sequence = TEMP_PROJECT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_dir = std::env::temp_dir().join(format!(
        "vexil-ts-project-{project_name}-{}-{sequence}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_dir)
        .unwrap_or_else(|e| panic!("failed to create {}: {e}", temp_dir.display()));

    for (path, content) in files {
        let output = temp_dir.join(path);
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|e| panic!("failed to create {}: {e}", parent.display()));
        }
        std::fs::write(&output, content)
            .unwrap_or_else(|e| panic!("failed to write {}: {e}", output.display()));
    }

    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    let config = format!(
        r#"{{
  "compilerOptions": {{
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "noEmit": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "skipLibCheck": false,
    "types": [],
    "baseUrl": "{workspace}",
    "paths": {{
      "@vexil-lang/runtime": ["packages/runtime-ts/src/index.ts"]
    }}
  }},
  "include": ["./**/*.ts"]
}}
"#
    );
    let config_path = temp_dir.join("tsconfig.json");
    std::fs::write(&config_path, config)
        .unwrap_or_else(|e| panic!("failed to write {}: {e}", config_path.display()));

    let mut command = if Path::new(&tsc)
        .extension()
        .is_some_and(|extension| extension.to_string_lossy().eq_ignore_ascii_case("cmd"))
    {
        let mut command = Command::new("cmd");
        command.arg("/c").arg(&tsc);
        command
    } else {
        Command::new(&tsc)
    };
    let output = command
        .arg("-p")
        .arg(&config_path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run TypeScript compiler: {e}"));

    std::fs::remove_dir_all(&temp_dir)
        .unwrap_or_else(|e| panic!("failed to remove {}: {e}", temp_dir.display()));
    assert!(
        output.status.success(),
        "native tsc failed for {project_name}:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn compile_and_generate(project_name: &str, root_ns: &str, expected_schema_count: usize) {
    let project_dir = corpus_dir().join(project_name);

    // Build root file path from namespace: "simple.main" -> "simple/main.vexil"
    let root_segments: Vec<&str> = root_ns.split('.').collect();
    let mut root_path = project_dir.clone();
    for seg in &root_segments {
        root_path.push(seg);
    }
    root_path.set_extension("vexil");

    let source = std::fs::read_to_string(&root_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", root_path.display()));

    let loader = FilesystemLoader::new(vec![project_dir]);

    let result = vexil_lang::compile_project(&source, &root_path, &loader)
        .unwrap_or_else(|e| panic!("compile_project failed for {project_name}: {e}"));

    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "unexpected errors in {project_name}: {errors:?}"
    );

    assert_eq!(
        result.schemas.len(),
        expected_schema_count,
        "{project_name}: expected {expected_schema_count} schemas, got {}",
        result.schemas.len()
    );

    // Generate TypeScript project files via the backend trait.
    let backend = vexil_codegen_ts::TypeScriptBackend;
    let files = backend
        .generate_project(&result)
        .unwrap_or_else(|e| panic!("generate_project failed for {project_name}: {e}"));

    assert!(!files.is_empty(), "{project_name}: no files generated");

    // Verify all generated files contain TypeScript content.
    for (path, content) in &files {
        assert!(
            !content.is_empty(),
            "{project_name}: empty file generated at {path:?}"
        );
        // Every generated file should contain import or export statements.
        assert!(
            content.contains("import") || content.contains("export"),
            "{project_name}: {path:?} doesn't look like TypeScript"
        );
    }

    native_tsc_check(project_name, &files);

    // Also verify single-file generation works for each schema.
    for (ns, compiled) in &result.schemas {
        let code = vexil_codegen_ts::generate(compiled)
            .unwrap_or_else(|e| panic!("codegen failed for {ns}: {e}"));
        assert!(!code.is_empty(), "empty codegen output for {ns}");
        assert!(
            code.contains("// Code generated by vexilc"),
            "missing header in {ns}"
        );
    }

    eprintln!(
        "{project_name}: {} schemas compiled and generated successfully ({} files)",
        result.schemas.len(),
        files.len()
    );
}

/// Simple two-file project: main imports types.
#[test]
fn project_simple() {
    compile_and_generate("simple", "simple.main", 2);
}

/// Diamond dependency: root -> left + right -> base.
/// Verifies diamond dedup (base compiled only once).
#[test]
fn project_diamond() {
    compile_and_generate("diamond", "diamond.root", 4);
}

/// Mixed project with enum + message across three files.
#[test]
fn project_mixed() {
    compile_and_generate("mixed", "mix.app", 3);
}
