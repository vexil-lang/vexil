use vexil_lang::diagnostic::{ErrorClass, Severity};

fn parse_valid(file: &str) {
    let path = format!("{}/../../corpus/valid/{file}", env!("CARGO_MANIFEST_DIR"));
    let source =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
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
    let source =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
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
fn valid_002_primitives() {
    parse_valid("002_primitives.vexil");
}

#[test]
fn valid_003_sub_byte() {
    parse_valid("003_sub_byte.vexil");
}

#[test]
fn valid_004_semantic_types() {
    parse_valid("004_semantic_types.vexil");
}

#[test]
fn valid_005_parameterized() {
    parse_valid("005_parameterized.vexil");
}

#[test]
fn valid_006_message() {
    parse_valid("006_message.vexil");
}

#[test]
fn valid_007_enum() {
    parse_valid("007_enum.vexil");
}

#[test]
fn valid_008_flags() {
    parse_valid("008_flags.vexil");
}

#[test]
fn valid_009_union() {
    parse_valid("009_union.vexil");
}

#[test]
fn valid_010_newtype() {
    parse_valid("010_newtype.vexil");
}

#[test]
fn valid_011_config() {
    parse_valid("011_config.vexil");
}

#[test]
fn valid_012_imports() {
    parse_valid("012_imports.vexil");
}

#[test]
fn valid_013_annotations() {
    parse_valid("013_annotations.vexil");
}

#[test]
fn valid_014_keywords_as_fields() {
    parse_valid("014_keywords_as_fields.vexil");
}

#[test]
fn valid_015_forward_refs() {
    parse_valid("015_forward_refs.vexil");
}

#[test]
fn valid_016_recursive() {
    parse_valid("016_recursive.vexil");
}

#[test]
fn valid_017_escapes() {
    parse_valid("017_escapes.vexil");
}

#[test]
fn valid_018_comments() {
    parse_valid("018_comments.vexil");
}

// Structure errors
#[test]
fn invalid_001_missing_namespace() {
    parse_invalid("001_missing_namespace.vexil", ErrorClass::MissingNamespace);
}

#[test]
fn invalid_002_duplicate_namespace() {
    parse_invalid(
        "002_duplicate_namespace.vexil",
        ErrorClass::DuplicateNamespace,
    );
}

#[test]
fn invalid_005_namespace_empty() {
    parse_invalid("005_namespace_empty.vexil", ErrorClass::NamespaceEmpty);
}

#[test]
fn invalid_023_import_after_decl() {
    parse_invalid("023_import_after_decl.vexil", ErrorClass::ImportAfterDecl);
}

#[test]
fn invalid_024_import_named_aliased() {
    parse_invalid(
        "024_import_named_aliased.vexil",
        ErrorClass::ImportNamedAliasedCombined,
    );
}

#[test]
fn invalid_042_version_not_semver() {
    parse_invalid(
        "042_version_not_semver.vexil",
        ErrorClass::VersionInvalidSemver,
    );
}

// Lexer errors
#[test]
fn invalid_021_invalid_escape() {
    parse_invalid("021_invalid_escape.vexil", ErrorClass::InvalidEscape);
}

// Namespace errors
#[test]
fn invalid_003_namespace_invalid_component() {
    parse_invalid(
        "003_namespace_invalid_component.vexil",
        ErrorClass::NamespaceInvalidComponent,
    );
}

// Name validation
#[test]
fn invalid_006_decl_name_lowercase() {
    parse_invalid("006_decl_name_lowercase.vexil", ErrorClass::DeclNameInvalid);
}

#[test]
fn invalid_007_decl_name_underscore() {
    parse_invalid(
        "007_decl_name_underscore.vexil",
        ErrorClass::DeclNameInvalid,
    );
}

#[test]
fn invalid_009_field_name_uppercase() {
    parse_invalid(
        "009_field_name_uppercase.vexil",
        ErrorClass::FieldNameInvalid,
    );
}

#[test]
fn invalid_039_union_variant_lowercase() {
    parse_invalid(
        "039_union_variant_lowercase.vexil",
        ErrorClass::UnionVariantNameInvalid,
    );
}

#[test]
fn invalid_047_enum_variant_lowercase() {
    parse_invalid(
        "047_enum_variant_lowercase.vexil",
        ErrorClass::EnumVariantNameInvalid,
    );
}

// Config parse errors
#[test]
fn invalid_016_config_missing_default() {
    parse_invalid(
        "016_config_missing_default.vexil",
        ErrorClass::ConfigMissingDefault,
    );
}

#[test]
fn invalid_017_config_with_ordinal() {
    parse_invalid(
        "017_config_with_ordinal.vexil",
        ErrorClass::ConfigHasOrdinal,
    );
}

// Tombstone
#[test]
fn invalid_040_removed_missing_reason() {
    parse_invalid(
        "040_removed_missing_reason.vexil",
        ErrorClass::RemovedMissingReason,
    );
}

// Version
#[test]
fn invalid_055_namespace_before_version() {
    parse_invalid(
        "055_namespace_before_version.vexil",
        ErrorClass::VersionAfterNamespace,
    );
}
