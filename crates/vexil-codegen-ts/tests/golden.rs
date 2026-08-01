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
    let golden_path = golden_dir.join(format!("{test_name}.ts"));

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
    let generated = vexil_codegen_ts::generate(&compiled).expect("codegen failed");

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

    if generated != expected {
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
fn test_013_annotations() {
    golden_test("013_annotations");
}

#[test]
fn test_016_recursive() {
    golden_test("016_recursive");
}

#[test]
fn test_027_delta_on_message() {
    golden_test("027_delta_on_message");
}

#[test]
fn test_028_typed_tombstone() {
    golden_test("028_typed_tombstone");
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
fn trait_field_tags_do_not_change_generated_output() {
    let first = "namespace test.trait_tags\ntrait Tagged { value @0 : i32 label @1 : string }\nmessage Item { value @0 : i32 label @1 : string }\nimpl Tagged for Item { }";
    let retagged = "namespace test.trait_tags\ntrait Tagged { value @9 : i32 label @9 : string }\nmessage Item { value @0 : i32 label @1 : string }\nimpl Tagged for Item { }";
    let first = vexil_lang::compile(first).compiled.expect("first schema");
    let retagged = vexil_lang::compile(retagged)
        .compiled
        .expect("retagged schema");
    assert_eq!(
        vexil_codegen_ts::generate(&first).expect("first output"),
        vexil_codegen_ts::generate(&retagged).expect("retagged output")
    );
}

/// A trait-only schema emits an interface and a type guard but no codec, so it
/// must not import `BitReader` or `BitWriter`.
#[test]
fn trait_only_generic_map() {
    golden_source_test(
        "trait_only_generic_map",
        "namespace test.trait_only_map\ntrait Lookup<T> {\n    values @0 : map<string, T>\n}",
    );
}

#[test]
fn nested_optional_constraint() {
    golden_source_test(
        "nested_optional_constraint",
        "namespace test.nested_optional_constraint\nmessage M { v @0 : optional<optional<u16>> where value in 1..1000 }",
    );
}

#[test]
fn nested_optional_tail() {
    golden_source_test(
        "nested_optional_tail",
        "namespace test.nested_optional_none_tail\nmessage M { v @0 : optional<optional<u16>> tail @1 : bool }",
    );
}

/// Trait names that happen to match runtime symbols must not manufacture an
/// import when the schema emits no codec.
#[test]
fn trait_names_codec_symbols() {
    golden_source_test(
        "trait_names_codec_symbols",
        "namespace test.trait_codec_names\ntrait BitReader {\n    value @0 : u32\n}\ntrait BitWriter {\n    value @0 : u32\n}",
    );
}
