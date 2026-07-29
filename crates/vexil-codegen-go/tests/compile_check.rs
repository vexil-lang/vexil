use std::path::Path;

fn check_compiles(corpus_name: &str) {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
    let corpus_path = workspace_root
        .join("corpus/valid")
        .join(format!("{corpus_name}.vexil"));
    let runtime_path = workspace_root.join("packages/runtime-go");

    let source = std::fs::read_to_string(&corpus_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", corpus_path.display()));
    let compiled = vexil_lang::compile(&source)
        .compiled
        .expect("corpus file should compile");
    let code = vexil_codegen_go::generate(&compiled).expect("codegen should succeed");

    let tmp = std::env::temp_dir().join(format!("vexil-codegen-go-check-{corpus_name}"));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    std::fs::write(tmp.join("generated.go"), code).unwrap();
    let runtime_path = runtime_path
        .to_str()
        .expect("runtime path must be valid UTF-8")
        .replace('\\', "/");
    std::fs::write(
        tmp.join("go.mod"),
        format!(
            "module vexil-codegen-check\n\ngo 1.22\n\nrequire github.com/vexil-lang/vexil/packages/runtime-go v0.0.0\n\nreplace github.com/vexil-lang/vexil/packages/runtime-go => {runtime_path}\n"
        ),
    )
    .unwrap();

    let output = std::process::Command::new("go")
        .args(["test", "./..."])
        .current_dir(&tmp)
        .output()
        .expect("failed to run go test");

    let _ = std::fs::remove_dir_all(&tmp);
    assert!(
        output.status.success(),
        "Generated code for {corpus_name} failed go test:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_048_generic_trait_nested() {
    check_compiles("048_generic_trait_nested");
}
