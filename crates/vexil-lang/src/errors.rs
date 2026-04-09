//! # Stability: Tier 2
//!
//! Error utilities including edit distance calculation for "did you mean?" suggestions.

use smol_str::SmolStr;

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
pub fn did_you_mean(target: &str, candidates: &[SmolStr]) -> Option<SmolStr> {
    let target_lower = target.to_lowercase();
    let mut best_match: Option<SmolStr> = None;
    let mut best_distance = usize::MAX;

    // Threshold: allow matches up to 1/3 of the target length or 3, whichever is larger
    let threshold = (target.len() / 3).max(3);

    for candidate in candidates {
        let candidate_lower = candidate.to_lowercase();
        let distance = edit_distance(&target_lower, &candidate_lower);

        // Exact match (case-insensitive) - return immediately
        if distance == 0 {
            return Some(candidate.clone());
        }

        if distance < best_distance && distance <= threshold {
            best_distance = distance;
            best_match = Some(candidate.clone());
        }
    }

    best_match
}

/// Find all candidates that are similar to the target (for showing multiple suggestions).
pub fn find_similar_matches(
    target: &str,
    candidates: &[SmolStr],
    max_results: usize,
) -> Vec<SmolStr> {
    let target_lower = target.to_lowercase();
    let threshold = (target.len() / 3).max(3);

    let mut matches: Vec<(SmolStr, usize)> = candidates
        .iter()
        .map(|c| {
            let dist = edit_distance(&target_lower, &c.to_lowercase());
            (c.clone(), dist)
        })
        .filter(|(_, d)| *d <= threshold && *d > 0) // Exclude exact matches
        .collect();

    // Sort by distance, then alphabetically
    matches.sort_by(|(a, da), (b, db)| da.cmp(db).then(a.cmp(b)));
    matches.truncate(max_results);

    matches.into_iter().map(|(s, _)| s).collect()
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
    fn test_did_you_mean() {
        let candidates: Vec<SmolStr> = vec![
            "username".into(),
            "user_id".into(),
            "user_name".into(),
            "email".into(),
            "password".into(),
        ];

        // Exact match
        assert_eq!(did_you_mean("email", &candidates), Some("email".into()));

        // Close match
        assert_eq!(
            did_you_mean("usrname", &candidates),
            Some("username".into())
        );

        // No close match
        assert_eq!(did_you_mean("xyz", &candidates), None);

        // Case insensitive
        assert_eq!(did_you_mean("EMAIL", &candidates), Some("email".into()));
    }

    #[test]
    fn test_find_similar_matches() {
        let candidates: Vec<SmolStr> = vec![
            "username".into(),
            "user_id".into(),
            "user_name".into(),
            "email".into(),
            "password".into(),
        ];

        let similar = find_similar_matches("usrname", &candidates, 3);
        assert!(!similar.is_empty());
        assert!(similar.contains(&"username".into()));
    }

    #[test]
    fn test_did_you_mean_with_import_names() {
        // Simulate import error scenario
        let available_exports: Vec<SmolStr> =
            vec!["Foo".into(), "Bar".into(), "Baz".into(), "MyMessage".into()];

        // Typo in import
        assert_eq!(did_you_mean("Baaz", &available_exports), Some("Baz".into()));

        // Another typo
        assert_eq!(
            did_you_mean("MyMesage", &available_exports),
            Some("MyMessage".into())
        );

        // No suggestion for completely different name
        assert_eq!(did_you_mean("SomethingElse", &available_exports), None);
    }
}
