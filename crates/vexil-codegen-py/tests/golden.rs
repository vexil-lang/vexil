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
