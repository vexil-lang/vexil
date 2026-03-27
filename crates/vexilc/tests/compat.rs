use std::io::Write;
use std::process::Command;

fn vexilc() -> Command {
    Command::new(env!("CARGO_BIN_EXE_vexilc"))
}

fn corpus_path(name: &str) -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    // Navigate from crates/vexilc/ up to repo root
    format!("{manifest}/../../corpus/valid/{name}")
}

#[test]
fn compat_identical_returns_0() {
    let output = vexilc()
        .args([
            "compat",
            &corpus_path("019_evolution_append_field.vexil"),
            &corpus_path("019_evolution_append_field.vexil"),
        ])
        .output()
        .expect("failed to run vexilc");

    assert!(
        output.status.success(),
        "expected exit 0, got {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No changes"),
        "expected 'No changes' in output, got: {stdout}"
    );
}

#[test]
fn compat_json_format() {
    let output = vexilc()
        .args([
            "compat",
            &corpus_path("019_evolution_append_field.vexil"),
            &corpus_path("019_evolution_append_field.vexil"),
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run vexilc");

    assert!(
        output.status.success(),
        "expected exit 0, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");

    assert_eq!(json["result"], "compatible");
    assert!(json["changes"].as_array().unwrap().is_empty());
}

#[test]
fn compat_different_schemas_detects_namespace_change() {
    // Comparing two corpus files with different namespaces should detect NamespaceChanged
    let output = vexilc()
        .args([
            "compat",
            &corpus_path("001_minimal.vexil"),
            &corpus_path("002_primitives.vexil"),
        ])
        .output()
        .expect("failed to run vexilc");

    // Should exit 1 (breaking) because namespaces differ
    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit 1 for breaking changes\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("BREAKING"),
        "expected BREAKING in output, got: {stdout}"
    );
}

#[test]
fn compat_breaking_change_with_tempfiles() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");

    let old_path = dir.path().join("old.vexil");
    let new_path = dir.path().join("new.vexil");

    let mut old_file = std::fs::File::create(&old_path).unwrap();
    writeln!(
        old_file,
        r#"namespace test.compat
message Sensor {{
    temp @0 : f32
    humidity @1 : f32
}}"#
    )
    .unwrap();

    let mut new_file = std::fs::File::create(&new_path).unwrap();
    writeln!(
        new_file,
        r#"namespace test.compat
message Sensor {{
    temp @0 : f64
    humidity @1 : f32
    pressure @2 : f32
}}"#
    )
    .unwrap();

    let output = vexilc()
        .args([
            "compat",
            old_path.to_str().unwrap(),
            new_path.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run vexilc");

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit 1 for breaking\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");

    assert_eq!(json["result"], "breaking");
    assert_eq!(json["suggested_bump"], "major");

    let changes = json["changes"].as_array().unwrap();
    // Should have at least a type change (f32 -> f64) and a field addition (pressure)
    let kinds: Vec<&str> = changes
        .iter()
        .map(|c| c["kind"].as_str().unwrap())
        .collect();
    assert!(
        kinds.contains(&"field_type_changed"),
        "expected field_type_changed in {kinds:?}"
    );
    assert!(
        kinds.contains(&"field_added"),
        "expected field_added in {kinds:?}"
    );
}

#[test]
fn compat_compatible_addition_with_tempfiles() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");

    let old_path = dir.path().join("old.vexil");
    let new_path = dir.path().join("new.vexil");

    let mut old_file = std::fs::File::create(&old_path).unwrap();
    writeln!(
        old_file,
        r#"namespace test.compat
message Header {{
    kind @0 : u8
    status @1 : u8
}}"#
    )
    .unwrap();

    let mut new_file = std::fs::File::create(&new_path).unwrap();
    writeln!(
        new_file,
        r#"namespace test.compat
message Header {{
    kind @0 : u8
    status @1 : u8
    flags @2 : u16
}}"#
    )
    .unwrap();

    let output = vexilc()
        .args([
            "compat",
            old_path.to_str().unwrap(),
            new_path.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run vexilc");

    assert!(
        output.status.success(),
        "expected exit 0 for compatible change\nstderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");

    assert_eq!(json["result"], "compatible");
    assert_eq!(json["suggested_bump"], "minor");
}
