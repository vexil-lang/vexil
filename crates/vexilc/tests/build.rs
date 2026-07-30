use std::fs;
use std::process::Command;

#[test]
fn build_with_unknown_qualified_trait_emits_no_partial_files() {
    let project = tempfile::tempdir().expect("temporary project");
    let root = project.path().join("root.vexil");
    let contracts = project.path().join("contracts.vexil");
    let output_dir = project.path().join("generated");
    fs::write(
        contracts,
        "namespace contracts\ntrait Tagged { value @0 : u64 }",
    )
    .expect("write dependency schema");
    fs::write(
        &root,
        "namespace app.root\nimport contracts as Contracts\nmessage Event { value @0 : u64 }\nimpl Missing.Tagged for Event { }",
    )
    .expect("write root schema");

    for target in ["rust", "typescript", "go", "python"] {
        let target_output = output_dir.join(target);
        let result = Command::new(env!("CARGO_BIN_EXE_vexilc"))
            .args([
                "build",
                root.to_str().expect("UTF-8 root path"),
                "--include",
                project.path().to_str().expect("UTF-8 include path"),
                "--output",
                target_output.to_str().expect("UTF-8 output path"),
                "--target",
                target,
            ])
            .output()
            .expect("run vexilc");

        assert!(!result.status.success(), "{target} unexpectedly succeeded");
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(
            stderr.contains("unknown import alias 'Missing' in impl trait reference"),
            "{target}: {stderr}"
        );
        assert!(
            !target_output.exists(),
            "{target} emitted a partial output directory"
        );
    }
}
