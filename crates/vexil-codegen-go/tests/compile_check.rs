use std::path::Path;

fn check_compiles(corpus_name: &str) {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
    let corpus_path = workspace_root
        .join("corpus/valid")
        .join(format!("{corpus_name}.vexil"));

    let source = std::fs::read_to_string(&corpus_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", corpus_path.display()));
    check_source_compiles(corpus_name, &source);
}

fn check_source_compiles(test_name: &str, source: &str) -> String {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
    let runtime_path = workspace_root.join("packages/runtime-go");

    let compiled = vexil_lang::compile(source)
        .compiled
        .expect("test schema should compile");
    let code = vexil_codegen_go::generate(&compiled).expect("codegen should succeed");

    let tmp = std::env::temp_dir().join(format!("vexil-codegen-go-check-{test_name}"));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    std::fs::write(tmp.join("generated.go"), &code).unwrap();
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
        .output();

    let _ = std::fs::remove_dir_all(&tmp);
    let output = match output {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipping generated Go compile check: `go` executable not found");
            return code;
        }
        Err(error) => panic!("failed to run go test: {error}"),
    };
    assert!(
        output.status.success(),
        "Generated code for {test_name} failed go test:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    code
}

#[test]
fn test_048_generic_trait_nested() {
    check_compiles("048_generic_trait_nested");
}

#[test]
fn trait_only_schema() {
    let code = check_source_compiles(
        "trait-only",
        "namespace test.trait_only\ntrait Container<T> {\n    items @0 : array<T>\n}",
    );
    assert!(
        !code.contains("github.com/vexil-lang/vexil/packages/runtime-go"),
        "{code}"
    );
}
