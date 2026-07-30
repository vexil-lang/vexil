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

fn check_source_compiles(test_name: &str, source: &str) {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
    let runtime_path = workspace_root.join("crates/vexil-runtime");
    let result = vexil_lang::compile(source);
    let compiled = result.compiled.expect("schema should compile");
    let code = vexil_codegen_rust::generate(&compiled).expect("codegen should succeed");

    let tmp = std::env::temp_dir().join(format!("vexil-codegen-check-{test_name}"));
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
        .args(["clippy", "--", "-D", "warnings"])
        .current_dir(&tmp)
        .env("CARGO_TARGET_DIR", tmp.join("target"))
        .output()
        .expect("failed to run cargo check");

    let _ = std::fs::remove_dir_all(&tmp);

    assert!(
        output.status.success(),
        "Generated code for {test_name} failed Clippy:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_generated_project(test_name: &str, source: &str, appended: &str) -> std::process::Output {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
    let runtime_path = workspace_root.join("crates/vexil-runtime");
    let compiled = vexil_lang::compile(source)
        .compiled
        .expect("schema should compile");
    let mut code = vexil_codegen_rust::generate(&compiled).expect("codegen should succeed");
    code.push_str(appended);

    let tmp = std::env::temp_dir().join(format!("vexil-codegen-contract-{test_name}"));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("src")).unwrap();
    std::fs::write(tmp.join("src/lib.rs"), code).unwrap();
    let runtime_path = runtime_path
        .to_str()
        .expect("runtime path must be valid UTF-8")
        .replace('\\', "/");
    std::fs::write(
        tmp.join("Cargo.toml"),
        format!(
            r#"[package]
name = "codegen-contract-{test_name}"
version = "0.1.0"
edition = "2021"

[dependencies]
vexil-runtime = {{ path = "{runtime_path}" }}
"#
        ),
    )
    .unwrap();
    let output = std::process::Command::new("cargo")
        .arg("test")
        .current_dir(&tmp)
        .env("CARGO_TARGET_DIR", tmp.join("target"))
        .output()
        .expect("failed to run generated Rust contract test");
    let _ = std::fs::remove_dir_all(&tmp);
    output
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
    let output = run_generated_project(
        "trait-function-behavior",
        &source,
        r#"

#[cfg(test)]
mod generated_contract {
    use super::{Adjustable, Counter};
    use vexil_runtime::{BitWriter, Pack};

    #[test]
    fn methods_and_wire_bytes_agree() {
        let mut counter = Counter {
            value: 5,
            _unknown: Vec::new(),
        };
        assert_eq!(Adjustable::adjust(&mut counter, 3), 5);
        assert_eq!(counter.value, 8);

        let mut writer = BitWriter::new();
        counter.pack(&mut writer).unwrap();
        assert_eq!(writer.finish(), [0x08, 0x00, 0x00, 0x00]);

        Adjustable::reset(&mut counter);
        assert_eq!(counter.value, 0);
    }
}
"#,
    );
    assert!(
        output.status.success(),
        "generated trait behavior failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn incomplete_rust_trait_implementation_is_rejected() {
    let source = r#"
namespace test.rust_negative
trait Adjustable<T> {
    value @0 : T
    fn adjust(delta: T) -> T
    fn reset()
}
"#;
    let output = run_generated_project(
        "trait-negative",
        source,
        r#"

struct Incomplete {
    value: i32,
}

impl Adjustable<i32> for Incomplete {
    fn value(&self) -> &i32 {
        &self.value
    }
}
"#,
    );
    assert!(
        !output.status.success(),
        "incomplete impl unexpectedly compiled"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("missing") && stderr.contains("adjust") && stderr.contains("reset"),
        "unexpected compiler failure:\n{stderr}"
    );
}

#[test]
fn trait_only_generic_map() {
    check_source_compiles(
        "trait-only-generic-map",
        "namespace test.trait_only_map\ntrait Lookup<T> {\n    values @0 : map<string, T>\n}",
    );
}

#[test]
fn portable_functions_preserve_non_copy_values_and_unused_locals() {
    check_source_compiles(
        "portable-non-copy-values",
        r#"
namespace test.portable_non_copy

trait Renamable {
    fn assign_and_return(text: string) -> string
    fn bind_assign_and_return(text: string) -> string
    fn ignore_binding(text: string) -> string
}

message Record {
    value @0 : string
}

impl Renamable for Record {
    fn assign_and_return(text: string) -> string {
        self.value = text
        return text
    }

    fn bind_assign_and_return(text: string) -> string {
        let copy: string = text
        self.value = copy
        return copy
    }

    fn ignore_binding(text: string) -> string {
        let unused: string = text
        return text
    }
}
"#,
    );
}

#[test]
fn portable_functions_project_recursive_storage_to_logical_values() {
    let source = r#"
namespace test.portable_recursive

trait Linked {
    fn successor() -> optional<Node>
    fn replace_successor(next: optional<Node>)
}

message Node {
    value @0 : string
    next @1 : optional<Node>
}

impl Linked for Node {
    fn successor() -> optional<Node> {
        return self.next
    }

    fn replace_successor(next: optional<Node>) {
        self.next = next
    }
}
"#;
    check_source_compiles("portable-recursive-storage", source);
    let output = run_generated_project(
        "portable-recursive-storage-behavior",
        source,
        r#"

#[cfg(test)]
mod generated_contract {
    use super::{Linked, Node};

    fn node(value: &str) -> Node {
        Node {
            value: value.to_string(),
            next: None,
            _unknown: Vec::new(),
        }
    }

    #[test]
    fn methods_convert_between_logical_values_and_boxed_storage() {
        let mut head = node("head");
        head.next = Some(Box::new(node("old")));

        let successor = Linked::successor(&mut head).expect("successor");
        assert_eq!(successor.value, "old");

        Linked::replace_successor(&mut head, Some(node("new")));
        assert_eq!(
            head.next.as_deref().map(|next| next.value.as_str()),
            Some("new")
        );
    }
}
"#,
    );
    assert!(
        output.status.success(),
        "generated recursive trait behavior failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
