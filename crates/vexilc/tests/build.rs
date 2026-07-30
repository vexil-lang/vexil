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

#[test]
fn build_rejects_distinct_imported_trait_name_collisions_without_partial_files() {
    let project = tempfile::tempdir().expect("temporary project");
    let root = project.path().join("root.vexil");
    fs::write(
        project.path().join("left.vexil"),
        "namespace left\ntrait Tagged { left @0 : u64 }",
    )
    .expect("write left trait schema");
    fs::write(
        project.path().join("right.vexil"),
        "namespace right\ntrait Tagged { right @0 : u64 }",
    )
    .expect("write right trait schema");
    fs::write(
        &root,
        "namespace app.root\nimport left as Left\nimport right as Right\nmessage LeftEvent { left @0 : u64 }\nmessage RightEvent { right @0 : u64 }\nimpl Left.Tagged for LeftEvent { }\nimpl Right.Tagged for RightEvent { }",
    )
    .expect("write root schema");

    for (target, target_name) in [
        ("rust", "Rust"),
        ("typescript", "TypeScript"),
        ("go", "Go"),
        ("python", "Python"),
    ] {
        let target_output = project.path().join("generated").join(target);
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
            stderr.contains(&format!("{target_name} identifier collision for 'Tagged'")),
            "{target}: {stderr}"
        );
        assert!(
            !target_output.exists(),
            "{target} emitted a partial output directory"
        );
    }
}

#[test]
fn build_rejects_imported_trait_local_name_collisions_without_partial_files() {
    let project = tempfile::tempdir().expect("temporary project");
    let root = project.path().join("root.vexil");
    fs::write(
        project.path().join("contracts.vexil"),
        "namespace contracts\ntrait Tagged { external @0 : u64 }",
    )
    .expect("write imported trait schema");
    fs::write(
        &root,
        "namespace app.root\nimport contracts as Contracts\ntrait Tagged { local @0 : u64 }\nmessage Event { external @0 : u64 }\nimpl Contracts.Tagged for Event { }",
    )
    .expect("write root schema");

    for (target, target_name) in [
        ("rust", "Rust"),
        ("typescript", "TypeScript"),
        ("go", "Go"),
        ("python", "Python"),
    ] {
        let target_output = project.path().join("generated").join(target);
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
            stderr.contains(&format!("{target_name} identifier collision for 'Tagged'")),
            "{target}: {stderr}"
        );
        assert!(
            !target_output.exists(),
            "{target} emitted a partial output directory"
        );
    }
}
