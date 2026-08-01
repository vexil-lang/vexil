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
    let golden_path = golden_dir.join(format!("{test_name}.py"));

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
fn typed_tombstone_emits_no_codec_read() {
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/valid/028_typed_tombstone.vexil"),
    )
    .expect("typed tombstone corpus");
    let compiled = vexil_lang::compile(&source)
        .compiled
        .expect("typed tombstone schema");
    let generated = vexil_codegen_py::generate(&compiled).expect("Python codegen");
    assert!(!generated.contains("read_u32()"), "{generated}");
    assert!(!generated.contains("discard @removed"), "{generated}");
}

#[test]
fn typed_tombstone_shapes() {
    golden_source_test(
        "typed_tombstone_shapes",
        r#"namespace test.typed.tombstone.shapes

message LegacyShapes {
    @removed(0, reason: "bytes payload removed") : bytes
    @removed(1, reason: "set payload removed") : set<u16>
    @removed(2, reason: "fixed array payload removed") : array<u8, 3>
    @removed(3, reason: "geometric payload removed") : vec3<f32>
    @removed(4, reason: "inline bits payload removed") : bits { read, write, execute }
    current @5 : u32
}
"#,
    );
}

#[test]
fn reader_helpers_are_declared_once_per_type() {
    let source = r#"namespace test.reader.helpers

enum Status {
    Ok @0
}

flags Mode {
    Read @0
}

newtype Identifier : u64
"#;
    let result = vexil_lang::compile(source);
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error),
        "compilation errors: {:?}",
        result.diagnostics
    );
    let compiled = result.compiled.expect("no compiled schema");
    let generated = vexil_codegen_py::generate(&compiled).expect("codegen failed");

    assert_eq!(
        generated.matches("    def decode_from(").count(),
        3,
        "each generated enum, flags, and newtype must declare one reader helper:\n{generated}"
    );
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
        vexil_codegen_py::generate(&first).expect("first output"),
        vexil_codegen_py::generate(&retagged).expect("retagged output")
    );
}

#[test]
fn trait_only_non_generic() {
    golden_source_test(
        "trait_only_non_generic",
        "namespace test.trait_only\ntrait Named {\n    name @0 : string\n}",
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
fn identifier_conflicts() {
    golden_source_test(
        "identifier_conflicts",
        "namespace test.identifier_conflicts\nconfig Settings {\n    self : u8 = 0\n    unknown : u8 = 0\n    from : u8 = 0\n}\ntrait Fields {\n    self @0 : u8\n    unknown @1 : u8\n}\nmessage Collision {\n    from @0 : u8\n    from_ @1 : u8\n    self @2 : u8\n    unknown @3 : bytes\n}\nunion Choice {\n    Named @0 { self @0 : u8  pr @1 : u16 }\n}",
    );
}

#[test]
fn generic_trait_type_param_conflicts() {
    golden_source_test(
        "generic_trait_type_param_conflicts",
        "namespace test.generic_trait_conflicts\ntrait Wrapper<Protocol> {\n    value @0 : Protocol\n}",
    );
}
