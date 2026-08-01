use std::fs;
use std::path::Path;
use vexil_lang::diagnostic::Severity;

fn golden_test(corpus_name: &str) {
    let corpus_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("corpus/valid");
    let source_path = corpus_dir.join(format!("{corpus_name}.vexil"));
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", source_path.display()));
    golden_source_test(corpus_name, &source);
}

fn golden_source_test(test_name: &str, source: &str) {
    let golden_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let golden_path = golden_dir.join(format!("{test_name}.rs"));

    let result = vexil_lang::compile(source);
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error),
        "compilation errors: {:?}",
        result.diagnostics
    );
    let compiled = result.compiled.expect("no compiled schema");
    let generated = vexil_codegen_rust::generate(&compiled).expect("codegen failed");

    if std::env::var("UPDATE_GOLDEN").is_ok() {
        fs::create_dir_all(&golden_dir).ok();
        fs::write(&golden_path, &generated).unwrap();
        eprintln!("Updated golden file: {}", golden_path.display());
        return;
    }

    let expected = fs::read_to_string(&golden_path)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read golden {}: {e}\nRun with UPDATE_GOLDEN=1 to create",
                golden_path.display()
            )
        })
        .replace("\r\n", "\n");

    if generated.trim_end() != expected.trim_end() {
        let diff = simple_diff(&expected, &generated);
        panic!("Golden file mismatch for {test_name}:\n{diff}");
    }
}

fn simple_diff(expected: &str, actual: &str) -> String {
    let mut out = String::new();
    for (i, (e, a)) in expected.lines().zip(actual.lines()).enumerate() {
        if e != a {
            out.push_str(&format!("Line {}:  expected: {e}\n", i + 1));
            out.push_str(&format!("Line {}:    actual: {a}\n", i + 1));
        }
    }
    let exp_lines = expected.lines().count();
    let act_lines = actual.lines().count();
    if exp_lines != act_lines {
        out.push_str(&format!(
            "Line count: expected {exp_lines}, actual {act_lines}\n"
        ));
    }
    out
}

#[test]
fn test_003_sub_byte() {
    golden_test("003_sub_byte");
}

#[test]
fn test_005_parameterized() {
    golden_test("005_parameterized");
}

#[test]
fn test_006_message() {
    golden_test("006_message");
}

#[test]
fn test_007_enum() {
    golden_test("007_enum");
}

#[test]
fn test_008_flags() {
    golden_test("008_flags");
}

#[test]
fn test_009_union() {
    golden_test("009_union");
}

#[test]
fn test_010_newtype() {
    golden_test("010_newtype");
}

#[test]
fn test_011_config() {
    golden_test("011_config");
}

#[test]
fn test_016_recursive() {
    golden_test("016_recursive");
}

#[test]
fn test_028_typed_tombstone() {
    golden_test("028_typed_tombstone");
}

#[test]
fn typed_tombstone_emits_no_codec_read() {
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/valid/028_typed_tombstone.vexil"),
    )
    .expect("typed tombstone corpus");
    let compiled = vexil_lang::compile(&source)
        .compiled
        .expect("typed tombstone schema");
    let generated = vexil_codegen_rust::generate(&compiled).expect("Rust codegen");
    assert!(!generated.contains("read_u32()?"), "{generated}");
    assert!(!generated.contains("discard @removed"), "{generated}");
}

#[test]
fn test_030_newtype_map_key() {
    golden_test("030_newtype_map_key");
}

#[test]
fn test_032_reserved_variant_names() {
    golden_test("032_reserved_variant_names");
}

#[test]
fn test_033_fixed_point() {
    golden_test("033_fixed_point");
}

#[test]
fn test_037_fixed_array() {
    golden_test("037_fixed_array");
}

#[test]
fn test_038_set() {
    golden_test("038_set");
}

#[test]
fn test_039_geometric() {
    golden_test("039_geometric");
}

#[test]
fn test_040_inline_bits() {
    golden_test("040_inline_bits");
}

#[test]
fn test_046_field_doc_placement() {
    golden_test("046_field_doc_placement");
}

#[test]
fn test_045_generic_trait() {
    golden_test("045_generic_trait");
}

#[test]
fn test_048_generic_trait_nested() {
    golden_test("048_generic_trait_nested");
}

#[test]
fn test_047_trait_function_signature() {
    golden_test("047_trait_function_signature");
}

#[test]
fn test_049_trait_function_portable_body() {
    golden_test("049_trait_function_portable_body");
}

#[test]
fn test_050_non_exhaustive_union_unknown_collision() {
    golden_test("050_non_exhaustive_union_unknown_collision");
}

#[test]
fn trait_only_generic_map() {
    golden_source_test(
        "trait_only_generic_map",
        "namespace test.trait_only_map\ntrait Lookup<T> {\n    values @0 : map<string, T>\n}",
    );
}

#[test]
fn trait_field_tags_do_not_change_generated_output() {
    let first = "namespace test.trait_tags\ntrait Tagged { value @0 : i32 label @1 : string }\nmessage Item { value @0 : i32 label @1 : string }\nimpl Tagged for Item { }";
    let retagged = "namespace test.trait_tags\ntrait Tagged { value @9 : i32 label @9 : string }\nmessage Item { value @0 : i32 label @1 : string }\nimpl Tagged for Item { }";
    let first = vexil_lang::compile(first).compiled.expect("first schema");
    let retagged = vexil_lang::compile(retagged)
        .compiled
        .expect("retagged schema");
    assert_eq!(
        vexil_codegen_rust::generate(&first).expect("first output"),
        vexil_codegen_rust::generate(&retagged).expect("retagged output")
    );
}

#[test]
fn portable_function_rust_projection_is_ownership_and_storage_aware() {
    let result = vexil_lang::compile(
        r#"
namespace test.portable_projection

trait Portable {
    fn preserve(text: string) -> string
    fn successor() -> optional<Node>
    fn replace_successor(next: optional<Node>)
}

message Node {
    label @0 : string
    next @1 : optional<Node>
}

impl Portable for Node {
    fn preserve(text: string) -> string {
        let unused: string = text
        self.label = text
        return text
    }

    fn successor() -> optional<Node> {
        return self.next
    }

    fn replace_successor(next: optional<Node>) {
        self.next = next
    }
}
"#,
    );
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error),
        "compilation errors: {:?}",
        result.diagnostics
    );
    let generated = vexil_codegen_rust::generate(&result.compiled.expect("schema should compile"))
        .expect("codegen should succeed");

    assert!(generated.contains("let _unused: String = text.clone();"));
    assert!(generated.contains("self.label = text.clone();"));
    assert!(generated.contains("fn successor(&mut self) -> Option<Node>"));
    assert!(generated.contains("self.next.clone().map(|value| *value)"));
    assert!(generated.contains("self.next = next.clone().map(Box::new);"));
}
