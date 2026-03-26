use std::path::Path;

fn check_compiles(corpus_name: &str) {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
    let corpus_path = workspace_root
        .join("corpus/valid")
        .join(format!("{corpus_name}.vexil"));
    let runtime_path = workspace_root.join("crates/vexil-runtime");

    let source = std::fs::read_to_string(&corpus_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", corpus_path.display()));
    let result = vexil_lang::compile(&source);
    let compiled = result.compiled.expect("corpus file should compile");
    let code = vexil_codegen_rust::generate(&compiled).expect("codegen should succeed");

    let tmp = std::env::temp_dir().join(format!("vexil-codegen-check-{corpus_name}"));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("src")).unwrap();

    std::fs::write(tmp.join("src/lib.rs"), &code).unwrap();
    // Use forward slashes in the TOML path to avoid backslash escape issues on Windows.
    let runtime_path_str = runtime_path
        .to_str()
        .expect("runtime path must be valid UTF-8")
        .replace('\\', "/");
    std::fs::write(
        tmp.join("Cargo.toml"),
        format!(
            r#"[package]
name = "codegen-check"
version = "0.1.0"
edition = "2021"

[dependencies]
vexil-runtime = {{ path = "{runtime_path_str}" }}
"#
        ),
    )
    .unwrap();

    let output = std::process::Command::new("cargo")
        .arg("check")
        .current_dir(&tmp)
        .env("CARGO_TARGET_DIR", tmp.join("target"))
        .output()
        .expect("failed to run cargo check");

    let _ = std::fs::remove_dir_all(&tmp);

    assert!(
        output.status.success(),
        "Generated code for {corpus_name} failed to compile:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_006_message() {
    check_compiles("006_message");
}

#[test]
fn test_007_enum() {
    check_compiles("007_enum");
}

#[test]
fn test_008_flags() {
    check_compiles("008_flags");
}

#[test]
fn test_009_union() {
    check_compiles("009_union");
}

#[test]
fn test_010_newtype() {
    check_compiles("010_newtype");
}

#[test]
fn test_011_config() {
    check_compiles("011_config");
}

#[test]
fn test_016_recursive() {
    check_compiles("016_recursive");
}
