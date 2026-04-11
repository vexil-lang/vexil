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
    let golden_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");

    let source_path = corpus_dir.join(format!("{corpus_name}.vexil"));
    let golden_path = golden_dir.join(format!("{corpus_name}.py"));

    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", source_path.display()));
    let result = vexil_lang::compile(&source);
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error),
        "compilation errors: {:?}",
        result.diagnostics
    );
    let compiled = result.compiled.expect("no compiled schema");
    let generated = vexil_codegen_py::generate(&compiled).expect("codegen failed");

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
        panic!("Golden file mismatch for {corpus_name}:\n{diff}");
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
fn test_003_sub_byte() {
    golden_test("003_sub_byte");
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
fn test_001_minimal() {
    golden_test("001_minimal");
}

#[test]
fn test_002_primitives() {
    golden_test("002_primitives");
}

#[test]
fn test_004_semantic_types() {
    golden_test("004_semantic_types");
}

#[test]
fn test_005_parameterized() {
    golden_test("005_parameterized");
}

#[test]
fn test_012_imports() {
    golden_test("012_imports");
}

#[test]
fn test_013_annotations() {
    golden_test("013_annotations");
}

#[test]
fn test_014_keywords_as_fields() {
    golden_test("014_keywords_as_fields");
}

#[test]
fn test_015_forward_refs() {
    golden_test("015_forward_refs");
}

#[test]
fn test_017_escapes() {
    golden_test("017_escapes");
}

#[test]
fn test_018_comments() {
    golden_test("018_comments");
}

#[test]
fn test_019_evolution_append_field() {
    golden_test("019_evolution_append_field");
}

#[test]
fn test_020_evolution_add_variant() {
    golden_test("020_evolution_add_variant");
}

#[test]
fn test_021_empty_optionals() {
    golden_test("021_empty_optionals");
}

#[test]
fn test_022_nested_schemas() {
    golden_test("022_nested_schemas");
}

#[test]
fn test_023_recursive_depth() {
    golden_test("023_recursive_depth");
}

#[test]
fn test_024_zero_length_payload() {
    golden_test("024_zero_length_payload");
}

#[test]
fn test_025_evolution_deprecate() {
    golden_test("025_evolution_deprecate");
}

#[test]
fn test_026_required_to_optional() {
    golden_test("026_required_to_optional");
}

#[test]
fn test_029_import_then_annotation() {
    golden_test("029_import_then_annotation");
}

#[test]
fn test_031_custom_annotations() {
    golden_test("031_custom_annotations");
}

#[test]
fn test_034_type_alias() {
    golden_test("034_type_alias");
}

#[test]
fn test_035_const() {
    golden_test("035_const");
}

#[test]
fn test_036_where_clause() {
    golden_test("036_where_clause");
}

#[test]
fn test_041_map_key_ordering() {
    golden_test("041_map_key_ordering");
}

#[test]
fn test_043_invariant() {
    golden_test("043_invariant");
}
