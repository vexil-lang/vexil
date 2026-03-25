use vexil_lang::diagnostic::Severity;

fn read_corpus(dir: &str, file: &str) -> String {
    let path = format!("{}/../../corpus/{dir}/{file}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

#[test]
fn valid_corpus_compiles() {
    let valid_files = [
        "001_minimal.vexil",
        "002_primitives.vexil",
        "003_sub_byte.vexil",
        "004_semantic_types.vexil",
        "005_parameterized.vexil",
        "006_message.vexil",
        "007_enum.vexil",
        "008_flags.vexil",
        "009_union.vexil",
        "010_newtype.vexil",
        "011_config.vexil",
        "012_imports.vexil",
        "013_annotations.vexil",
        "014_keywords_as_fields.vexil",
        "015_forward_refs.vexil",
        "016_recursive.vexil",
        "017_escapes.vexil",
        "018_comments.vexil",
    ];
    for file in &valid_files {
        let source = read_corpus("valid", file);
        let result = vexil_lang::compile(&source);
        let errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "expected no errors in {file}, got: {errors:#?}"
        );
        assert!(
            result.compiled.is_some(),
            "expected CompiledSchema for valid {file}"
        );
    }
}

#[test]
fn compile_simple_message() {
    let source = "namespace test.simple\nmessage Foo { a @0 : u32  b @1 : bool }";
    let result = vexil_lang::compile(source);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "errors: {errors:#?}");
    let compiled = result.compiled.as_ref().unwrap();
    assert_eq!(compiled.declarations.len(), 1);
    assert_eq!(compiled.registry.len(), 1);
}
