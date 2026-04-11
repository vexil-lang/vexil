//! # Stability: Tier 2
//!
//! Enhanced diagnostic types with error codes, notes, and suggestions.

use crate::span::Span;

/// The severity level of a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    /// A fatal error that prevents compilation from proceeding.
    Error,
    /// A non-fatal warning that does not block compilation.
    Warning,
}

/// Machine-readable error codes for categorizing diagnostics.
/// Format: E### for errors, W### for warnings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // Lexer errors (E001-E009)
    E001, // Invalid character
    E002, // Invalid escape sequence
    E003, // Unterminated string

    // Structure errors (E010-E019)
    E010, // Missing namespace
    E011, // Duplicate namespace
    E012, // Import after declaration
    E013, // Invalid import syntax

    // Namespace errors (E020-E029)
    E020, // Invalid namespace component
    E021, // Reserved namespace
    E022, // Empty namespace

    // Declaration errors (E030-E039)
    E030, // Invalid declaration name
    E031, // Duplicate declaration name

    // Field errors (E040-E049)
    E040, // Invalid field name
    E041, // Duplicate field name
    E042, // Duplicate ordinal
    E043, // Ordinal too large
    E044, // Ordinal reused after removal
    E045, // Missing field (with suggestion)

    // Type errors (E050-E069)
    E050, // Unknown type (with suggestion)
    E051, // Type mismatch
    E052, // Config type as field
    E053, // Newtype over newtype
    E054, // Newtype over config
    E055, // Alias target is alias
    E056, // Alias cycle detected
    E057, // Alias target not found
    E058, // Invalid const type
    E059, // Const cycle detected
    E060, // Const division by zero
    E061, // Const reference not found
    E062, // Invalid map key type
    E063, // Geometric type invalid element
    E064, // Recursive type (infinite)
    E065, // Encoding type mismatch
    E066, // Unresolved type

    // Config errors (E070-E079)
    E070, // Config missing default
    E071, // Config has ordinal
    E072, // Config invalid field type
    E073, // Config encoding annotation

    // Enum/Flags/Union errors (E080-E099)
    E080, // Duplicate enum ordinal
    E081, // Enum ordinal too large
    E082, // Enum backing too narrow
    E083, // Invalid enum backing
    E084, // Invalid enum variant name
    E085, // Flags bit too high
    E086, // Inline bits empty
    E087, // Invalid bit name
    E088, // Duplicate union ordinal
    E089, // Union ordinal too large
    E090, // Invalid union variant name
    E091, // Unknown enum variant (with suggestion)

    // Annotation errors (E100-E119)
    E100, // Duplicate annotation
    E101, // Invalid annotation target
    E102, // Missing deprecated reason
    E103, // Missing removed reason
    E104, // Invalid limit target
    E105, // Limit exceeds global maximum
    E106, // Limit value zero
    E107, // Invalid varint target
    E108, // Invalid zigzag target
    E109, // Varint and zigzag combined
    E110, // Invalid delta target
    E111, // Type value overflow
    E112, // Version after namespace
    E113, // Duplicate version
    E114, // Invalid semver
    E115, // Non-exhaustive invalid target

    // Where clause errors (E120-E129)
    E120, // Where clause type mismatch
    E121, // Where clause range invalid
    E122, // Where clause len on non-collection
    E123, // Where clause const not found
    E124, // Where clause operator invalid
    E125, // Where clause constraint failed

    // Import errors (E130-E139)
    E130, // Import not found
    E131, // Ambiguous import
    E132, // Import name not found in namespace
    E133, // Impl function external (no body)

    // Generic errors
    E999, // Unexpected token
}

impl ErrorCode {
    /// Returns the string representation of the error code.
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::E001 => "E001",
            ErrorCode::E002 => "E002",
            ErrorCode::E003 => "E003",
            ErrorCode::E010 => "E010",
            ErrorCode::E011 => "E011",
            ErrorCode::E012 => "E012",
            ErrorCode::E013 => "E013",
            ErrorCode::E020 => "E020",
            ErrorCode::E021 => "E021",
            ErrorCode::E022 => "E022",
            ErrorCode::E030 => "E030",
            ErrorCode::E031 => "E031",
            ErrorCode::E040 => "E040",
            ErrorCode::E041 => "E041",
            ErrorCode::E042 => "E042",
            ErrorCode::E043 => "E043",
            ErrorCode::E044 => "E044",
            ErrorCode::E045 => "E045",
            ErrorCode::E050 => "E050",
            ErrorCode::E051 => "E051",
            ErrorCode::E052 => "E052",
            ErrorCode::E053 => "E053",
            ErrorCode::E054 => "E054",
            ErrorCode::E055 => "E055",
            ErrorCode::E056 => "E056",
            ErrorCode::E057 => "E057",
            ErrorCode::E058 => "E058",
            ErrorCode::E059 => "E059",
            ErrorCode::E060 => "E060",
            ErrorCode::E061 => "E061",
            ErrorCode::E062 => "E062",
            ErrorCode::E063 => "E063",
            ErrorCode::E064 => "E064",
            ErrorCode::E065 => "E065",
            ErrorCode::E066 => "E066",
            ErrorCode::E070 => "E070",
            ErrorCode::E071 => "E071",
            ErrorCode::E072 => "E072",
            ErrorCode::E073 => "E073",
            ErrorCode::E080 => "E080",
            ErrorCode::E081 => "E081",
            ErrorCode::E082 => "E082",
            ErrorCode::E083 => "E083",
            ErrorCode::E084 => "E084",
            ErrorCode::E085 => "E085",
            ErrorCode::E086 => "E086",
            ErrorCode::E087 => "E087",
            ErrorCode::E088 => "E088",
            ErrorCode::E089 => "E089",
            ErrorCode::E090 => "E090",
            ErrorCode::E091 => "E091",
            ErrorCode::E100 => "E100",
            ErrorCode::E101 => "E101",
            ErrorCode::E102 => "E102",
            ErrorCode::E103 => "E103",
            ErrorCode::E104 => "E104",
            ErrorCode::E105 => "E105",
            ErrorCode::E106 => "E106",
            ErrorCode::E107 => "E107",
            ErrorCode::E108 => "E108",
            ErrorCode::E109 => "E109",
            ErrorCode::E110 => "E110",
            ErrorCode::E111 => "E111",
            ErrorCode::E112 => "E112",
            ErrorCode::E113 => "E113",
            ErrorCode::E114 => "E114",
            ErrorCode::E115 => "E115",
            ErrorCode::E120 => "E120",
            ErrorCode::E121 => "E121",
            ErrorCode::E122 => "E122",
            ErrorCode::E123 => "E123",
            ErrorCode::E124 => "E124",
            ErrorCode::E125 => "E125",
            ErrorCode::E130 => "E130",
            ErrorCode::E131 => "E131",
            ErrorCode::E132 => "E132",
            ErrorCode::E133 => "E133",
            ErrorCode::E999 => "E999",
        }
    }
}

/// Classification of a diagnostic by its root cause.
///
/// Used for programmatic matching on error types (e.g. in test assertions
/// or IDE quick-fix suggestions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorClass {
    // Lexer
    InvalidCharacter,
    InvalidEscape,
    UnterminatedString,

    // Structure
    MissingNamespace,
    DuplicateNamespace,
    ImportAfterDecl,
    ImportNamedAliasedCombined,

    // Namespace
    NamespaceInvalidComponent,
    NamespaceReserved,
    NamespaceEmpty,

    // Declaration
    DeclNameInvalid,
    DeclNameDuplicate,

    // Field
    FieldNameInvalid,
    FieldNameDuplicate,
    OrdinalDuplicate,
    OrdinalTooLarge,
    OrdinalReusedAfterRemoved,

    // Type
    UnknownType,
    ConfigTypeAsField,
    NewtypeOverNewtype,
    NewtypeOverConfig,
    AliasTargetIsAlias,
    AliasCycleDetected,
    AliasTargetNotFound,
    ConstTypeInvalid,
    ConstCycleDetected,
    ConstDivByZero,
    ConstRefNotFound,
    InvalidMapKey,
    GeometricInvalidElementType,

    // Config
    ConfigMissingDefault,
    ConfigHasOrdinal,
    ConfigInvalidType,
    ConfigEncodingAnnotation,

    // Enum/Flags/Union
    EnumOrdinalDuplicate,
    EnumOrdinalTooLarge,
    EnumBackingTooNarrow,
    EnumBackingInvalid,
    EnumVariantNameInvalid,
    FlagsBitTooHigh,
    BitsInlineEmpty,
    InvalidBitName,
    UnionOrdinalDuplicate,
    UnionOrdinalTooLarge,
    UnionVariantNameInvalid,

    // Annotation
    DuplicateAnnotation,
    NonExhaustiveInvalidTarget,
    DeprecatedMissingReason,
    RemovedMissingReason,
    LimitInvalidTarget,
    LimitExceedsGlobal,
    LimitZero,
    VarintInvalidTarget,
    ZigzagInvalidTarget,
    VarintZigzagCombined,
    DeltaInvalidTarget,
    TypeValueOverflow,
    VersionAfterNamespace,
    VersionDuplicate,
    VersionInvalidSemver,

    // IR / Type checker
    RecursiveTypeInfinite,
    EncodingTypeMismatch,
    UnresolvedType,

    // Where clause
    WhereClauseTypeMismatch,
    WhereClauseRangeInvalid,
    WhereClauseLenOnNonCollection,
    WhereClauseConstRefNotFound,
    WhereClauseOperatorInvalid,

    // Impl
    ImplFnExternal,

    // Generic
    UnexpectedToken,
    UnexpectedEof,
}

impl ErrorClass {
    /// Maps the error class to a machine-readable error code.
    pub fn to_code(&self) -> ErrorCode {
        match self {
            // Lexer
            ErrorClass::InvalidCharacter => ErrorCode::E001,
            ErrorClass::InvalidEscape => ErrorCode::E002,
            ErrorClass::UnterminatedString => ErrorCode::E003,

            // Structure
            ErrorClass::MissingNamespace => ErrorCode::E010,
            ErrorClass::DuplicateNamespace => ErrorCode::E011,
            ErrorClass::ImportAfterDecl => ErrorCode::E012,
            ErrorClass::ImportNamedAliasedCombined => ErrorCode::E013,

            // Namespace
            ErrorClass::NamespaceInvalidComponent => ErrorCode::E020,
            ErrorClass::NamespaceReserved => ErrorCode::E021,
            ErrorClass::NamespaceEmpty => ErrorCode::E022,

            // Declaration
            ErrorClass::DeclNameInvalid => ErrorCode::E030,
            ErrorClass::DeclNameDuplicate => ErrorCode::E031,

            // Field
            ErrorClass::FieldNameInvalid => ErrorCode::E040,
            ErrorClass::FieldNameDuplicate => ErrorCode::E041,
            ErrorClass::OrdinalDuplicate => ErrorCode::E042,
            ErrorClass::OrdinalTooLarge => ErrorCode::E043,
            ErrorClass::OrdinalReusedAfterRemoved => ErrorCode::E044,

            // Type
            ErrorClass::UnknownType => ErrorCode::E050,
            ErrorClass::ConfigTypeAsField => ErrorCode::E052,
            ErrorClass::NewtypeOverNewtype => ErrorCode::E053,
            ErrorClass::NewtypeOverConfig => ErrorCode::E054,
            ErrorClass::AliasTargetIsAlias => ErrorCode::E055,
            ErrorClass::AliasCycleDetected => ErrorCode::E056,
            ErrorClass::AliasTargetNotFound => ErrorCode::E057,
            ErrorClass::ConstTypeInvalid => ErrorCode::E058,
            ErrorClass::ConstCycleDetected => ErrorCode::E059,
            ErrorClass::ConstDivByZero => ErrorCode::E060,
            ErrorClass::ConstRefNotFound => ErrorCode::E061,
            ErrorClass::InvalidMapKey => ErrorCode::E062,
            ErrorClass::GeometricInvalidElementType => ErrorCode::E063,
            ErrorClass::RecursiveTypeInfinite => ErrorCode::E064,
            ErrorClass::EncodingTypeMismatch => ErrorCode::E065,
            ErrorClass::UnresolvedType => ErrorCode::E066,

            // Config
            ErrorClass::ConfigMissingDefault => ErrorCode::E070,
            ErrorClass::ConfigHasOrdinal => ErrorCode::E071,
            ErrorClass::ConfigInvalidType => ErrorCode::E072,
            ErrorClass::ConfigEncodingAnnotation => ErrorCode::E073,

            // Enum/Flags/Union
            ErrorClass::EnumOrdinalDuplicate => ErrorCode::E080,
            ErrorClass::EnumOrdinalTooLarge => ErrorCode::E081,
            ErrorClass::EnumBackingTooNarrow => ErrorCode::E082,
            ErrorClass::EnumBackingInvalid => ErrorCode::E083,
            ErrorClass::EnumVariantNameInvalid => ErrorCode::E084,
            ErrorClass::FlagsBitTooHigh => ErrorCode::E085,
            ErrorClass::BitsInlineEmpty => ErrorCode::E086,
            ErrorClass::InvalidBitName => ErrorCode::E087,
            ErrorClass::UnionOrdinalDuplicate => ErrorCode::E088,
            ErrorClass::UnionOrdinalTooLarge => ErrorCode::E089,
            ErrorClass::UnionVariantNameInvalid => ErrorCode::E090,

            // Annotation
            ErrorClass::DuplicateAnnotation => ErrorCode::E100,
            ErrorClass::NonExhaustiveInvalidTarget => ErrorCode::E115,
            ErrorClass::DeprecatedMissingReason => ErrorCode::E102,
            ErrorClass::RemovedMissingReason => ErrorCode::E103,
            ErrorClass::LimitInvalidTarget => ErrorCode::E104,
            ErrorClass::LimitExceedsGlobal => ErrorCode::E105,
            ErrorClass::LimitZero => ErrorCode::E106,
            ErrorClass::VarintInvalidTarget => ErrorCode::E107,
            ErrorClass::ZigzagInvalidTarget => ErrorCode::E108,
            ErrorClass::VarintZigzagCombined => ErrorCode::E109,
            ErrorClass::DeltaInvalidTarget => ErrorCode::E110,
            ErrorClass::TypeValueOverflow => ErrorCode::E111,
            ErrorClass::VersionAfterNamespace => ErrorCode::E112,
            ErrorClass::VersionDuplicate => ErrorCode::E113,
            ErrorClass::VersionInvalidSemver => ErrorCode::E114,

            // Where clause
            ErrorClass::WhereClauseTypeMismatch => ErrorCode::E120,
            ErrorClass::WhereClauseRangeInvalid => ErrorCode::E121,
            ErrorClass::WhereClauseLenOnNonCollection => ErrorCode::E122,
            ErrorClass::WhereClauseConstRefNotFound => ErrorCode::E123,
            ErrorClass::WhereClauseOperatorInvalid => ErrorCode::E124,

            // Impl
            ErrorClass::ImplFnExternal => ErrorCode::E133,

            // Generic
            ErrorClass::UnexpectedToken => ErrorCode::E999,
            ErrorClass::UnexpectedEof => ErrorCode::E999,
        }
    }
}

/// A note or suggestion attached to a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    /// A general note providing additional context.
    Note(String),
    /// A "did you mean" suggestion for typos.
    DidYouMean(String),
    /// A list of valid options (e.g., valid types, valid annotations).
    ValidOptions(Vec<String>),
    /// Expected vs actual values for type mismatch errors.
    ExpectedVsActual { expected: String, actual: String },
    /// Help text explaining how to fix the error.
    Help(String),
}

impl Note {
    /// Format the note as a display string.
    pub fn format(&self) -> String {
        match self {
            Note::Note(msg) => format!("note: {msg}"),
            Note::DidYouMean(suggestion) => format!("help: did you mean `{suggestion}`?"),
            Note::ValidOptions(options) => {
                let list = options.join(", ");
                format!("note: valid options are: {list}")
            }
            Note::ExpectedVsActual { expected, actual } => {
                format!("note: expected `{expected}`, found `{actual}`")
            }
            Note::Help(msg) => format!("help: {msg}"),
        }
    }
}

/// A compiler diagnostic (error or warning) with location and classification.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    /// Whether this is an error or a warning.
    pub severity: Severity,
    /// Source location where the diagnostic was raised.
    pub span: Span,
    /// Machine-readable error classification.
    pub class: ErrorClass,
    /// Human-readable description of the problem.
    pub message: String,
    /// Source file path, if known (set during multi-file compilation).
    pub source_file: Option<std::path::PathBuf>,
    /// Additional notes and suggestions.
    pub notes: Vec<Note>,
    /// Error code for machine-readable identification.
    pub code: ErrorCode,
}

impl Diagnostic {
    /// Create an error-severity diagnostic.
    pub fn error(span: Span, class: ErrorClass, message: impl Into<String>) -> Self {
        let class_clone = class.clone();
        Self {
            severity: Severity::Error,
            span,
            class,
            message: message.into(),
            source_file: None,
            notes: Vec::new(),
            code: class_clone.to_code(),
        }
    }

    /// Create a warning-severity diagnostic.
    pub fn warning(span: Span, class: ErrorClass, message: impl Into<String>) -> Self {
        let class_clone = class.clone();
        Self {
            severity: Severity::Warning,
            span,
            class,
            message: message.into(),
            source_file: None,
            notes: Vec::new(),
            code: class_clone.to_code(),
        }
    }

    /// Attach a source file path to this diagnostic (builder pattern).
    pub fn with_file(mut self, path: std::path::PathBuf) -> Self {
        self.source_file = Some(path);
        self
    }

    /// Add a note to this diagnostic (builder pattern).
    pub fn with_note(mut self, note: Note) -> Self {
        self.notes.push(note);
        self
    }

    /// Add multiple notes to this diagnostic (builder pattern).
    pub fn with_notes(mut self, notes: impl IntoIterator<Item = Note>) -> Self {
        self.notes.extend(notes);
        self
    }

    /// Add a "did you mean" suggestion.
    pub fn with_suggestion(self, suggestion: impl Into<String>) -> Self {
        self.with_note(Note::DidYouMean(suggestion.into()))
    }

    /// Add expected vs actual note for type mismatch.
    pub fn with_expected_vs_actual(
        self,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        self.with_note(Note::ExpectedVsActual {
            expected: expected.into(),
            actual: actual.into(),
        })
    }

    /// Add valid options note.
    pub fn with_valid_options(self, options: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let opts: Vec<String> = options.into_iter().map(|s| s.into()).collect();
        self.with_note(Note::ValidOptions(opts))
    }

    /// Add help text.
    pub fn with_help(self, help: impl Into<String>) -> Self {
        self.with_note(Note::Help(help.into()))
    }
}

/// Calculate the Levenshtein edit distance between two strings.
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a_len = a.chars().count();
    let b_len = b.chars().count();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    // Use a single row for space efficiency
    let mut prev_row: Vec<usize> = (0..=b_len).collect();
    let mut curr_row = vec![0; b_len + 1];

    for (i, a_char) in a.chars().enumerate() {
        curr_row[0] = i + 1;

        for (j, b_char) in b.chars().enumerate() {
            let cost = if a_char == b_char { 0 } else { 1 };
            curr_row[j + 1] = (prev_row[j + 1] + 1) // deletion
                .min(curr_row[j] + 1) // insertion
                .min(prev_row[j] + cost); // substitution
        }

        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[b_len]
}

/// Find the closest match to `target` from `candidates` using edit distance.
/// Returns `None` if no candidate is within a reasonable threshold.
pub fn find_closest_match<'a>(
    target: &str,
    candidates: impl Iterator<Item = &'a str>,
) -> Option<&'a str> {
    let target_lower = target.to_lowercase();
    let mut best_match: Option<&str> = None;
    let mut best_distance = usize::MAX;

    // Threshold: allow matches up to 1/3 of the target length or 3, whichever is larger
    let threshold = (target.len() / 3).max(3);

    for candidate in candidates {
        let candidate_lower = candidate.to_lowercase();
        let distance = edit_distance(&target_lower, &candidate_lower);

        // Exact match (case-insensitive) - return immediately
        if distance == 0 {
            return Some(candidate);
        }

        if distance < best_distance && distance <= threshold {
            best_distance = distance;
            best_match = Some(candidate);
        }
    }

    best_match
}

/// Find all candidates that are similar to the target (for showing multiple suggestions).
pub fn find_similar_matches<'a>(
    target: &str,
    candidates: impl Iterator<Item = &'a str>,
    max_results: usize,
) -> Vec<&'a str> {
    let target_lower = target.to_lowercase();
    let threshold = (target.len() / 3).max(3);

    let mut matches: Vec<(&str, usize)> = candidates
        .map(|c| {
            let dist = edit_distance(&target_lower, &c.to_lowercase());
            (c, dist)
        })
        .filter(|(_, d)| *d <= threshold && *d > 0) // Exclude exact matches
        .collect();

    // Sort by distance, then alphabetically
    matches.sort_by(|(a, da), (b, db)| da.cmp(db).then(a.cmp(b)));
    matches.truncate(max_results);

    matches.into_iter().map(|(s, _)| s).collect()
}

/// Format a diagnostic with notes for display.
/// This is used when ariadne is not available (e.g., in library contexts).
pub fn format_diagnostic_simple(diag: &Diagnostic, filename: Option<&str>) -> String {
    let severity_str = match diag.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    };

    let mut result = if let Some(file) = filename {
        format!(
            "{}[{}]: {}\n  --> {}:{}",
            severity_str,
            diag.code.as_str(),
            diag.message,
            file,
            diag.span.offset
        )
    } else {
        format!("{}[{}]: {}", severity_str, diag.code.as_str(), diag.message)
    };

    for note in &diag.notes {
        result.push('\n');
        result.push_str(&note.format());
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_distance() {
        assert_eq!(edit_distance("", ""), 0);
        assert_eq!(edit_distance("a", ""), 1);
        assert_eq!(edit_distance("", "a"), 1);
        assert_eq!(edit_distance("abc", "abc"), 0);
        assert_eq!(edit_distance("abc", "def"), 3);
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("sunday", "saturday"), 3);
    }

    #[test]
    fn test_find_closest_match() {
        let candidates = ["username", "user_id", "user_name", "email", "password"];
        let iter = candidates.iter().copied();

        // Exact match
        assert_eq!(find_closest_match("email", iter.clone()), Some("email"));

        // Close match
        assert_eq!(
            find_closest_match("usrname", iter.clone()),
            Some("username")
        );

        // No close match
        assert_eq!(find_closest_match("xyz", iter.clone()), None);

        // Case insensitive
        assert_eq!(find_closest_match("EMAIL", iter.clone()), Some("email"));
    }

    #[test]
    fn test_find_similar_matches() {
        let candidates = ["username", "user_id", "user_name", "email", "password"];
        let iter = candidates.iter().copied();

        let similar = find_similar_matches("usrname", iter.clone(), 3);
        assert!(!similar.is_empty());
        assert!(similar.contains(&"username"));
    }

    #[test]
    fn test_note_format() {
        assert_eq!(
            Note::Note("additional context".to_string()).format(),
            "note: additional context"
        );
        assert_eq!(
            Note::DidYouMean("foo".to_string()).format(),
            "help: did you mean `foo`?"
        );
        assert_eq!(
            Note::ValidOptions(vec!["a".to_string(), "b".to_string()]).format(),
            "note: valid options are: a, b"
        );
        assert_eq!(
            Note::ExpectedVsActual {
                expected: "u32".to_string(),
                actual: "string".to_string()
            }
            .format(),
            "note: expected `u32`, found `string`"
        );
    }
}
