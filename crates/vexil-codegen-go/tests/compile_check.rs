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
fn unused_immutable_local_compiles() {
    let code = check_source_compiles(
        "trait-function-unused-local",
        r#"
namespace test.trait_function_unused_local
trait Identity {
    fn identity(input: i32) -> i32
}
message Counter {
    value @0 : i32
}
impl Identity for Counter {
    fn identity(input: i32) -> i32 {
        let unused: i32 = input
        return input
    }
}
"#,
    );
    assert!(code.contains("var unused int32 = input"), "{code}");
    assert!(code.contains("_ = unused"), "{code}");
}

fn run_generated_go(test_name: &str, source: &str, extra_go: &str) -> Option<std::process::Output> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
    let runtime_path = workspace_root.join("packages/runtime-go");
    let compiled = vexil_lang::compile(source)
        .compiled
        .expect("test schema should compile");
    let code = vexil_codegen_go::generate(&compiled).expect("codegen should succeed");

    let tmp = std::env::temp_dir().join(format!("vexil-codegen-go-contract-{test_name}"));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::write(tmp.join("generated.go"), code).unwrap();
    std::fs::write(tmp.join("generated_test.go"), extra_go).unwrap();
    let runtime_path = runtime_path
        .to_str()
        .expect("runtime path must be valid UTF-8")
        .replace('\\', "/");
    std::fs::write(
        tmp.join("go.mod"),
        format!(
            "module vexil-codegen-contract\n\ngo 1.22\n\nrequire github.com/vexil-lang/vexil/packages/runtime-go v0.0.0\n\nreplace github.com/vexil-lang/vexil/packages/runtime-go => {runtime_path}\n"
        ),
    )
    .unwrap();
    let output = std::process::Command::new("go")
        .args(["test", "./..."])
        .current_dir(&tmp)
        .output();
    let _ = std::fs::remove_dir_all(&tmp);
    match output {
        Ok(output) => Some(output),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipping generated Go contract check: `go` executable not found");
            None
        }
        Err(error) => panic!("failed to run go test: {error}"),
    }
}

#[test]
fn test_048_generic_trait_nested() {
    check_compiles("048_generic_trait_nested");
}

#[test]
fn test_049_trait_function_portable_body() {
    check_compiles("049_trait_function_portable_body");

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
    let source = std::fs::read_to_string(
        workspace_root.join("corpus/valid/049_trait_function_portable_body.vexil"),
    )
    .unwrap();
    let Some(output) = run_generated_go(
        "trait-function-behavior",
        &source,
        r#"package trait_function_portable_body

import (
    "bytes"
    "testing"

    vexil "github.com/vexil-lang/vexil/packages/runtime-go"
)

func TestGeneratedTraitMethodsAndWire(t *testing.T) {
    counter := &Counter{Value: 5}
    if got := counter.Adjust(3); got != 5 || counter.Value != 8 {
        t.Fatalf("adjust mismatch: previous=%d value=%d", got, counter.Value)
    }
    writer := vexil.NewBitWriter()
    if err := counter.Pack(writer); err != nil {
        t.Fatal(err)
    }
    if got, want := writer.Finish(), []byte{0x08, 0x00, 0x00, 0x00}; !bytes.Equal(got, want) {
        t.Fatalf("wire mismatch: got %x want %x", got, want)
    }
    counter.Reset()
    if counter.Value != 0 {
        t.Fatalf("reset mismatch: value=%d", counter.Value)
    }
}
"#,
    ) else {
        return;
    };
    assert!(
        output.status.success(),
        "generated Go trait behavior failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn float_literals_preserve_fractional_go_semantics() {
    let source = r#"
namespace test.trait_float_literal

trait Divider {
    fn half() -> f64
}

message Calculator {}

impl Divider for Calculator {
    fn half() -> f64 {
        return 1.0 / 2.0
    }
}
"#;
    let code = check_source_compiles("trait-float-literal", source);
    assert!(code.contains("return (1.0 / 2.0)"), "{code}");

    let Some(output) = run_generated_go(
        "trait-float-literal-behavior",
        source,
        r#"package trait_float_literal

import "testing"

func TestGeneratedFloatDivision(t *testing.T) {
    calculator := &Calculator{}
    if got := calculator.Half(); got != 0.5 {
        t.Fatalf("half mismatch: got %v want 0.5", got)
    }
}
"#,
    ) else {
        return;
    };
    assert!(
        output.status.success(),
        "generated Go float behavior failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn incomplete_go_trait_implementation_is_rejected() {
    let source = r#"
namespace test.go_negative
trait Adjustable<T> {
    value @0 : T
    fn adjust(delta: T) -> T
    fn reset()
}
"#;
    let Some(output) = run_generated_go(
        "trait-negative",
        source,
        r#"package go_negative

type incomplete struct {
    Value int32
}

func (m *incomplete) GetValue() int32 { return m.Value }

var _ Adjustable[int32] = (*incomplete)(nil)
"#,
    ) else {
        return;
    };
    assert!(
        !output.status.success(),
        "incomplete Go impl unexpectedly compiled"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does not implement") && stderr.contains("Adjust"),
        "unexpected Go compiler failure:\n{stderr}"
    );
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

#[test]
fn bytes_map_keys_compile() {
    check_compiles("041_map_key_ordering");
}

#[test]
fn transparent_alias_fields_compile() {
    check_compiles("034_type_alias");
}

#[test]
fn optional_container_access_compiles() {
    check_source_compiles(
        "optional-container-access",
        "namespace test.optional_container\nmessage M { value @0 : optional<map<u8, set<u16>>> }",
    );
}
