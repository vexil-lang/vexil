use vexil_lang::diagnostic::{ErrorClass, Severity};

fn parse_valid(file: &str) {
    let path = format!("{}/../../corpus/valid/{file}", env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
    let result = vexil_lang::parse(&source);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "expected no errors in {file}, got: {errors:#?}"
    );
}

fn parse_invalid(file: &str, expected: ErrorClass) {
    let path = format!("{}/../../corpus/invalid/{file}", env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
    let result = vexil_lang::parse(&source);
    let has_expected = result
        .diagnostics
        .iter()
        .any(|d| d.class == expected && d.severity == Severity::Error);
    assert!(
        has_expected,
        "expected {expected:?} in {file}, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn valid_001_minimal() {
    parse_valid("001_minimal.vexil");
}

#[test]
fn invalid_001_missing_namespace() {
    parse_invalid("001_missing_namespace.vexil", ErrorClass::MissingNamespace);
}
