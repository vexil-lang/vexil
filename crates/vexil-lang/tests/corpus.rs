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

#[test]
fn valid_019_evolution_append_field() {
    parse_valid("019_evolution_append_field.vexil");
}

#[test]
fn valid_020_evolution_add_variant() {
    parse_valid("020_evolution_add_variant.vexil");
}

#[test]
fn valid_021_empty_optionals() {
    parse_valid("021_empty_optionals.vexil");
}

#[test]
fn valid_022_nested_schemas() {
    parse_valid("022_nested_schemas.vexil");
}

#[test]
fn valid_023_recursive_depth() {
    parse_valid("023_recursive_depth.vexil");
}

#[test]
fn valid_024_zero_length_payload() {
    parse_valid("024_zero_length_payload.vexil");
}

#[test]
fn valid_025_evolution_deprecate() {
    parse_valid("025_evolution_deprecate.vexil");
}

#[test]
fn valid_026_required_to_optional() {
    parse_valid("026_required_to_optional.vexil");
}

#[test]
fn valid_027_delta_on_message() {
    parse_valid("027_delta_on_message.vexil");
}

#[test]
fn valid_028_typed_tombstone() {
    parse_valid("028_typed_tombstone.vexil");
}

#[test]
fn valid_030_newtype_map_key() {
    parse_valid("030_newtype_map_key.vexil");
}

#[test]
fn valid_031_custom_annotations() {
    parse_valid("031_custom_annotations.vexil");
}

#[test]
fn valid_032_reserved_variant_names() {
    parse_valid("032_reserved_variant_names.vexil");
}

#[test]
fn valid_035_const() {
    parse_valid("035_const.vexil");
}

#[test]
fn valid_033_fixed_point() {
    parse_valid("033_fixed_point.vexil");
}

#[test]
fn valid_034_type_alias() {
    parse_valid("034_type_alias.vexil");
}

#[test]
fn valid_036_where_clause() {
    parse_valid("036_where_clause.vexil");
}

#[test]
fn valid_037_fixed_array() {
    parse_valid("037_fixed_array.vexil");
}

#[test]
fn valid_038_set() {
    parse_valid("038_set.vexil");
}

#[test]
fn valid_039_geometric() {
    parse_valid("039_geometric.vexil");
}

#[test]
fn valid_040_inline_bits() {
    parse_valid("040_inline_bits.vexil");
}

#[test]
fn valid_041_map_key_ordering() {
    parse_valid("041_map_key_ordering.vexil");
}

#[test]
fn valid_043_invariant() {
    parse_valid("043_invariant.vexil");
}

#[test]
fn valid_044_generic_alias() {
    parse_valid("044_generic_alias.vexil");
}

#[test]
fn valid_044_generic_simple() {
    parse_valid("044_generic_simple.vexil");
}

#[test]
fn valid_045_generic_trait() {
    parse_valid("045_generic_trait.vexil");
}

// =========================================================================
// Where clause semantic validation errors
// =========================================================================

#[test]
fn invalid_062_where_type_mismatch() {
    parse_invalid(
        "062_where_type_mismatch.vexil",
        ErrorClass::WhereClauseTypeMismatch,
    );
}

#[test]
fn invalid_063_where_range_invalid() {
    parse_invalid(
        "063_where_range_invalid.vexil",
        ErrorClass::WhereClauseRangeInvalid,
    );
}

#[test]
fn invalid_064_where_len_invalid() {
    parse_invalid(
        "064_where_len_invalid.vexil",
        ErrorClass::WhereClauseLenOnNonCollection,
    );
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
fn invalid_059_alias_chain() {
    parse_invalid("059_alias_chain.vexil", ErrorClass::AliasTargetIsAlias);
}

#[test]
fn invalid_060_const_div_zero() {
    parse_invalid("060_const_div_zero.vexil", ErrorClass::ConstDivByZero);
}

#[test]
fn invalid_061_const_cycle() {
    parse_invalid("061_const_cycle.vexil", ErrorClass::ConstCycleDetected);
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

// =========================================================================
// Semantic validation errors
// =========================================================================

// Namespace semantic
#[test]
fn invalid_004() {
    parse_invalid(
        "004_namespace_reserved.vexil",
        ErrorClass::NamespaceReserved,
    );
}

// Declaration semantic
#[test]
fn invalid_008() {
    parse_invalid(
        "008_decl_name_duplicate.vexil",
        ErrorClass::DeclNameDuplicate,
    );
}

// Ordinal semantic
#[test]
fn invalid_010() {
    parse_invalid("010_duplicate_ordinal.vexil", ErrorClass::OrdinalDuplicate);
}

#[test]
fn invalid_011() {
    parse_invalid("011_ordinal_too_large.vexil", ErrorClass::OrdinalTooLarge);
}

#[test]
fn invalid_012() {
    parse_invalid(
        "012_duplicate_field_name.vexil",
        ErrorClass::FieldNameDuplicate,
    );
}

#[test]
fn invalid_041() {
    parse_invalid(
        "041_removed_reuses_ordinal.vexil",
        ErrorClass::OrdinalReusedAfterRemoved,
    );
}

// Enum/flags/union semantic
#[test]
fn invalid_033() {
    parse_invalid(
        "033_enum_duplicate_ordinal.vexil",
        ErrorClass::EnumOrdinalDuplicate,
    );
}

#[test]
fn invalid_034() {
    parse_invalid(
        "034_enum_ordinal_overflow.vexil",
        ErrorClass::EnumOrdinalTooLarge,
    );
}

#[test]
fn invalid_035() {
    parse_invalid(
        "035_enum_backing_too_narrow.vexil",
        ErrorClass::EnumBackingTooNarrow,
    );
}

#[test]
fn invalid_036() {
    parse_invalid("036_flags_bit_too_high.vexil", ErrorClass::FlagsBitTooHigh);
}

#[test]
fn invalid_037() {
    parse_invalid(
        "037_union_duplicate_ordinal.vexil",
        ErrorClass::UnionOrdinalDuplicate,
    );
}

#[test]
fn invalid_038() {
    parse_invalid(
        "038_union_ordinal_overflow.vexil",
        ErrorClass::UnionOrdinalTooLarge,
    );
}

#[test]
fn invalid_051() {
    parse_invalid(
        "051_enum_backing_invalid_type.vexil",
        ErrorClass::EnumBackingInvalid,
    );
}

// Annotation semantic
#[test]
fn invalid_022() {
    parse_invalid(
        "022_duplicate_annotation.vexil",
        ErrorClass::DuplicateAnnotation,
    );
}

#[test]
fn invalid_056() {
    parse_invalid("056_duplicate_version.vexil", ErrorClass::VersionDuplicate);
}

#[test]
fn invalid_054() {
    parse_invalid("054_limit_zero.vexil", ErrorClass::LimitZero);
}

#[test]
fn invalid_045() {
    parse_invalid(
        "045_limit_exceeds_global.vexil",
        ErrorClass::LimitExceedsGlobal,
    );
}

#[test]
fn invalid_050() {
    parse_invalid(
        "050_type_domain_bad_arg.vexil",
        ErrorClass::TypeValueOverflow,
    );
}

// Type-level errors
#[test]
fn invalid_046() {
    parse_invalid("046_type_unknown.vexil", ErrorClass::UnknownType);
}

#[test]
fn invalid_013() {
    parse_invalid(
        "013_field_references_config.vexil",
        ErrorClass::ConfigTypeAsField,
    );
}

#[test]
fn invalid_014() {
    parse_invalid(
        "014_newtype_over_newtype.vexil",
        ErrorClass::NewtypeOverNewtype,
    );
}

#[test]
fn invalid_015() {
    parse_invalid(
        "015_newtype_over_config.vexil",
        ErrorClass::NewtypeOverConfig,
    );
}

#[test]
fn invalid_029() {
    parse_invalid("029_map_invalid_key.vexil", ErrorClass::InvalidMapKey);
}

#[test]
fn invalid_030() {
    parse_invalid("030_map_void_key.vexil", ErrorClass::InvalidMapKey);
}

#[test]
fn invalid_031() {
    parse_invalid("031_map_message_key.vexil", ErrorClass::InvalidMapKey);
}

#[test]
fn invalid_032() {
    parse_invalid("032_map_optional_key.vexil", ErrorClass::InvalidMapKey);
}

#[test]
fn invalid_057_newtype_message_map_key() {
    parse_invalid(
        "057_newtype_message_map_key.vexil",
        ErrorClass::InvalidMapKey,
    );
}

#[test]
fn invalid_018() {
    parse_invalid("018_config_map_type.vexil", ErrorClass::ConfigInvalidType);
}

#[test]
fn invalid_019() {
    parse_invalid(
        "019_config_result_type.vexil",
        ErrorClass::ConfigInvalidType,
    );
}

// Annotation-target errors
#[test]
fn invalid_043() {
    parse_invalid(
        "043_non_exhaustive_on_message.vexil",
        ErrorClass::NonExhaustiveInvalidTarget,
    );
}

#[test]
fn invalid_025() {
    parse_invalid(
        "025_varint_on_subbyte.vexil",
        ErrorClass::VarintInvalidTarget,
    );
}

#[test]
fn invalid_027() {
    parse_invalid(
        "027_varint_on_signed.vexil",
        ErrorClass::VarintInvalidTarget,
    );
}

#[test]
fn invalid_053() {
    parse_invalid("053_varint_on_float.vexil", ErrorClass::VarintInvalidTarget);
}

#[test]
fn invalid_026() {
    parse_invalid(
        "026_zigzag_on_unsigned.vexil",
        ErrorClass::ZigzagInvalidTarget,
    );
}

#[test]
fn invalid_052() {
    parse_invalid(
        "052_zigzag_on_subbyte.vexil",
        ErrorClass::ZigzagInvalidTarget,
    );
}

#[test]
fn invalid_028() {
    parse_invalid(
        "028_varint_zigzag_combined.vexil",
        ErrorClass::VarintZigzagCombined,
    );
}

#[test]
fn invalid_049() {
    parse_invalid("049_delta_on_string.vexil", ErrorClass::DeltaInvalidTarget);
}

#[test]
fn invalid_044() {
    parse_invalid(
        "044_limit_on_invalid_type.vexil",
        ErrorClass::LimitInvalidTarget,
    );
}

#[test]
fn invalid_048() {
    parse_invalid(
        "048_deprecated_missing_reason.vexil",
        ErrorClass::DeprecatedMissingReason,
    );
}

#[test]
fn invalid_020() {
    parse_invalid(
        "020_config_encoding_annotation.vexil",
        ErrorClass::ConfigEncodingAnnotation,
    );
}

// =========================================================================
// Impl semantic validation errors
// =========================================================================

#[test]
fn invalid_065_impl_unknown_trait() {
    parse_invalid("065_impl_unknown_trait.vexil", ErrorClass::UnknownType);
}

#[test]
fn invalid_066_external_fn() {
    parse_invalid("066_external_fn.vexil", ErrorClass::ImplFnExternal);
}
