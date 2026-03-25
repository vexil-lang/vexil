# Vexil Test Corpus Manifest

Version: 0.1.0-draft-r2
Generated: 2026-03-25

This corpus exercises every normative MUST/MUST NOT in the Vexil spec.
A conformant implementation MUST accept all valid files and reject all invalid files.

## Valid Corpus (18 files)

| File | Spec sections | What it exercises |
|------|--------------|-------------------|
| 001_minimal.vexil | §2.1 | Minimum valid schema (namespace only, no declarations) |
| 002_all_primitives.vexil | §3.1 | Every primitive type: bool, u8–u64, i8–i64, f32, f64, void |
| 003_subbyte_types.vexil | §3.2 | Sub-byte unsigned (u1–u63) and signed (i2–i63) types |
| 004_semantic_types.vexil | §3.3 | All semantic types: string, bytes, rgb, uuid, timestamp, hash |
| 005_parameterized_types.vexil | §3.4 | optional, array, map, result with nesting |
| 006_messages.vexil | §4.1, §5.2 | Empty message, ordinal gaps, all annotation positions |
| 007_enums.vexil | §4.2, §12.2 | Basic, @non_exhaustive, explicit backing type, @removed tombstone |
| 008_flags.vexil | §4.3 | Basic flags, high bit positions, tombstone |
| 009_unions.vexil | §4.4 | Variant fields with ordinals, empty variants, tombstone |
| 010_newtypes.vexil | §4.5 | Newtype wrapping primitive, semantic, and user-defined types |
| 011_config.vexil | §4.6 | All default types, optional with none, array defaults |
| 012_imports.vexil | §2.7 | All 6 import forms (wildcard, named, aliased, versioned, combined) |
| 013_annotations.vexil | §5, §12 | All standard annotations: @version, @doc, @deprecated, @since, @varint, @zigzag, @delta, @type, @domain, @revision, @limit, @non_exhaustive |
| 014_keywords_as_fields.vexil | Grammar | 20 keywords used as field names (type, hash, string, etc.) |
| 015_forward_refs.vexil | §3.5 | Forward references within the same schema |
| 016_recursive.vexil | §3.5 | Self-recursive (tree, linked list) and mutual recursion |
| 017_escapes.vexil | §5.1 | All valid escape sequences: \", \\, \n, \t, \r; empty string |
| 018_comments.vexil | Lexer | Comments in all positions: file-level, inline, between elements |

## Invalid Corpus (56 files)

| File | Spec section | Error class | What it rejects |
|------|-------------|-------------|-----------------|
| 001_missing_namespace.vexil | §2.1 | Parse | Schema without namespace declaration |
| 002_duplicate_namespace.vexil | §2.1 | Parse | Two namespace declarations |
| 003_namespace_invalid_component.vexil | §2.2 | Parse | Uppercase in namespace component |
| 004_namespace_reserved.vexil | §2.2 | Semantic | Namespace starting with `vexil.` |
| 005_namespace_empty.vexil | §2.2 | Parse | Bare `namespace` keyword with no path |
| 006_decl_name_lowercase.vexil | §2.3 | Parse | Declaration name starting with lowercase |
| 007_decl_name_underscore.vexil | §2.3 | Parse | Declaration name containing underscore |
| 008_decl_name_duplicate.vexil | §2.3 | Semantic | Two declarations with the same name |
| 009_field_name_uppercase.vexil | §2.4 | Parse | Field name starting with uppercase |
| 010_duplicate_ordinal.vexil | §2.4 | Semantic | Two fields sharing ordinal @0 |
| 011_ordinal_too_large.vexil | §2.4 | Semantic | Ordinal @65536 exceeds maximum |
| 012_duplicate_field_name.vexil | §2.4 | Semantic | Two fields with the same name |
| 013_field_references_config.vexil | §3.5 | Type | Wire field referencing a config type |
| 014_newtype_over_newtype.vexil | §4.5 | Type | Newtype wrapping another newtype |
| 015_newtype_over_config.vexil | §4.5 | Type | Newtype wrapping a config type |
| 016_config_missing_default.vexil | §4.6 | Parse | Config field without default value |
| 017_config_with_ordinal.vexil | §4.6 | Parse | Ordinal on config field |
| 018_config_map_type.vexil | §4.6 | Type | map<K,V> as config field type |
| 019_config_result_type.vexil | §4.6 | Type | result<T,E> as config field type |
| 020_config_encoding_annotation.vexil | §4.6 | Semantic | @varint on config field |
| 021_invalid_escape.vexil | §5.1 | Parse | Unrecognized escape sequence \a |
| 022_duplicate_annotation.vexil | §5.3 | Semantic | @non_exhaustive appearing twice |
| 023_import_after_decl.vexil | §2.7 | Parse | Import appearing after type declaration |
| 024_import_named_aliased.vexil | §2.7 | Parse | Named + aliased import combined |
| 025_varint_on_subbyte.vexil | §12.4 | Semantic | @varint on sub-byte type (u4) |
| 026_zigzag_on_unsigned.vexil | §12.4 | Semantic | @zigzag on unsigned type (u32) |
| 027_varint_on_signed.vexil | §12.4 | Semantic | @varint on signed type (i32) |
| 028_varint_zigzag_combined.vexil | §12.4 | Semantic | @varint and @zigzag on same field |
| 029_map_invalid_key.vexil | §3.4 | Type | f64 as map key type |
| 030_map_void_key.vexil | §3.4 | Type | void as map key type |
| 031_map_message_key.vexil | §3.4 | Type | message as map key type |
| 032_map_optional_key.vexil | §3.4 | Type | optional<T> as map key type |
| 033_enum_duplicate_ordinal.vexil | §4.2 | Semantic | Two enum variants with ordinal @0 |
| 034_enum_ordinal_overflow.vexil | §4.2 | Semantic | Enum ordinal @65536 |
| 035_enum_backing_too_narrow.vexil | §4.2 | Semantic | u8 backing with ordinal 256 |
| 036_flags_bit_too_high.vexil | §4.3 | Semantic | Flags bit position @64 (max is 63) |
| 037_union_duplicate_ordinal.vexil | §4.4 | Semantic | Two union variants with ordinal @0 |
| 038_union_ordinal_overflow.vexil | §4.4 | Semantic | Union variant ordinal @65536 |
| 039_union_variant_lowercase.vexil | §4.4 | Parse | Union variant starting with lowercase |
| 040_removed_missing_reason.vexil | §12.3 | Semantic | @removed without reason argument |
| 041_removed_reuses_ordinal.vexil | §12.3 | Semantic | Active field reusing tombstoned ordinal |
| 042_version_not_semver.vexil | §6.3 | Parse | Version constraint ^1.0 missing patch |
| 043_non_exhaustive_on_message.vexil | §12.2 | Semantic | @non_exhaustive on message (enum/union only) |
| 044_limit_on_invalid_type.vexil | §12.5 | Semantic | @limit on u32 (string/bytes/array/map only) |
| 045_limit_exceeds_global.vexil | §12.5 | Semantic | @limit(16777217) exceeds array max |
| 046_type_unknown.vexil | §3.5 | Type | Reference to undefined type |
| 047_enum_variant_lowercase.vexil | §4.2 | Parse | Enum variant starting with lowercase |
| 048_deprecated_missing_reason.vexil | §12.2 | Semantic | @deprecated without reason argument |
| 049_delta_on_string.vexil | §12.4 | Semantic | @delta on string type |
| 050_type_domain_bad_arg.vexil | §12.6 | Semantic | @type(0x100) exceeds u8 range |
| 051_enum_backing_invalid_type.vexil | §4.2 | Semantic | i32 as enum backing (u8/u16/u32/u64 only) |
| 052_zigzag_on_subbyte.vexil | §12.4 | Semantic | @zigzag on sub-byte type (i4) |
| 053_varint_on_float.vexil | §12.4 | Semantic | @varint on f32 |
| 054_limit_zero.vexil | §12.5 | Semantic | @limit(0) — N must be positive |
| 055_namespace_before_version.vexil | §12.1 | Parse | @version after namespace (must be before) |
| 056_duplicate_version.vexil | §12.1 | Semantic | Two @version annotations on same schema |

## Error class taxonomy

- **Parse**: Rejected during parsing; implementation MUST NOT continue analysis of this file (§1.4).
- **Semantic**: Rejected during semantic analysis; implementation MUST continue to report all errors (§1.4).
- **Type**: Rejected during type checking; a subclass of semantic errors.

## Coverage notes

The following MUST NOT conditions are tested indirectly or require multi-file setups:

- **Circular imports** (§6.2): Requires two files that import each other. Not testable as a single file.
- **Duplicate namespace across compilation unit** (§2.2): Requires two files with the same namespace.
- **Wildcard import conflict** (§2.7): Requires two imported schemas exporting the same name.
- **Version constraint mismatch** (§6.3): Requires an imported schema with a non-matching @version.

These should be tested as multi-file integration tests in the reference implementation.
