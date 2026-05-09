use std::hash::Hash;

use crate::{
    entity::{Entity, EntityRef, ParentKind},
    error::primitive::PrimitiveError,
};

/// Id must match `[a-z0-9]+(-[a-z0-9]+)*`
pub fn kebab_case(value: &str) -> Vec<PrimitiveError> {
    let valid = !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--");
    if valid {
        vec![]
    } else {
        vec![PrimitiveError::naming_format_violation(
            format!("'{value}' is not kebab-case"),
            None::<String>,
            "kebab_case",
        )]
    }
}

/// Id must match `[A-Z][a-zA-Z0-9]*`
pub fn pascal_case(value: &str) -> Vec<PrimitiveError> {
    let valid = !value.is_empty()
        && value.starts_with(|c: char| c.is_ascii_uppercase())
        && value.chars().all(|c| c.is_ascii_alphanumeric());
    if valid {
        vec![]
    } else {
        vec![PrimitiveError::naming_format_violation(
            format!("'{value}' is not PascalCase"),
            None::<String>,
            "pascal_case",
        )]
    }
}

/// `EntityRef` id must be kebab-case
pub fn kebab_case_id<T: Entity, P: ParentKind>(
    entity_ref: &EntityRef<T, P>,
) -> Vec<PrimitiveError> {
    kebab_case(entity_ref.id())
}

/// `EntityRef` id must be PascalCase
pub fn pascal_case_id<T: Entity, P: ParentKind>(
    entity_ref: &EntityRef<T, P>,
) -> Vec<PrimitiveError> {
    pascal_case(entity_ref.id())
}

/// String must not be empty or whitespace-only
pub fn non_empty_str(value: &str) -> Vec<PrimitiveError> {
    if value.trim().is_empty() {
        vec![PrimitiveError::empty_required_value(
            "must not be empty",
            None::<String>,
            "non_empty",
        )]
    } else {
        vec![]
    }
}

/// Optional string must be non-empty if present
pub fn opt_non_empty_str(value: &Option<String>) -> Vec<PrimitiveError> {
    match value {
        None => vec![],
        Some(s) => non_empty_str(s),
    }
}

/// Slice must have at least one element
pub fn non_empty_list<T>(value: &[T]) -> Vec<PrimitiveError> {
    if value.is_empty() {
        vec![PrimitiveError::malformed_collection_value(
            "must not be empty",
            "non_empty",
        )]
    } else {
        vec![]
    }
}

/// Each string item in the slice must be non-empty (not whitespace-only)
pub fn each_item_non_empty(value: &[String]) -> Vec<PrimitiveError> {
    let mut violations = vec![];
    for (i, item) in value.iter().enumerate() {
        if item.trim().is_empty() {
            violations.push(PrimitiveError::empty_required_value(
                "must not be empty",
                Some(format!("[{i}]")),
                "non_empty",
            ));
        }
    }
    violations
}

/// Map must have at least one entry
pub fn non_empty_map<K, V>(value: &std::collections::HashMap<K, V>) -> Vec<PrimitiveError> {
    if value.is_empty() {
        vec![PrimitiveError::malformed_collection_value(
            "must not be empty",
            "non_empty",
        )]
    } else {
        vec![]
    }
}

/// Slice must have at least `min` elements
pub fn min_length<T>(value: &[T], min: usize) -> Vec<PrimitiveError> {
    if value.len() < min {
        vec![PrimitiveError::malformed_collection_value(
            format!("must have at least {min} elements, got {}", value.len()),
            "min_length",
        )]
    } else {
        vec![]
    }
}

/// All elements must produce distinct keys via `key_fn`
pub fn unique_by<T, K: Eq + Hash>(value: &[T], key_fn: fn(&T) -> K) -> Vec<PrimitiveError> {
    let mut seen = std::collections::HashSet::new();
    let mut violations = vec![];
    for (i, item) in value.iter().enumerate() {
        let key = key_fn(item);
        if !seen.insert(key) {
            violations.push(PrimitiveError::duplicate_entry_violation(
                "duplicate entry",
                format!("[{i}]"),
                "unique",
            ));
        }
    }
    violations
}

#[cfg(test)]
mod tests {
    //! Parameterized coverage of the small naming / shape primitives.
    //! Each has a tight regex-like contract; functional tests exercise
    //! one or two cases via specific failures, but the input space is
    //! small enough that direct enumeration is the cheapest way to
    //! cover the corners (empty, whitespace, unicode, leading hyphen,
    //! trailing hyphen, mixed casing, …).

    use rstest::rstest;

    use super::*;

    fn passes(v: Vec<PrimitiveError>) -> bool {
        v.is_empty()
    }

    fn fails(v: Vec<PrimitiveError>) -> bool {
        !v.is_empty()
    }

    // -----------------------------------------------------------------------
    // kebab_case
    // -----------------------------------------------------------------------

    #[rstest]
    #[case::single_word("eng", true)]
    #[case::multi_word("eng-lead", true)]
    #[case::with_digits("v1-roadmap", true)]
    #[case::all_digits("123", true)]
    #[case::three_segments("eng-platform-team", true)]
    #[case::empty("", false)]
    #[case::leading_hyphen("-eng", false)]
    #[case::trailing_hyphen("eng-", false)]
    #[case::double_hyphen("eng--lead", false)]
    #[case::uppercase("Eng", false)]
    #[case::camel_case("engLead", false)]
    #[case::underscore("eng_lead", false)]
    #[case::space("eng lead", false)]
    #[case::special_char("eng!", false)]
    #[case::unicode("éng", false)]
    fn kebab_case_cases(#[case] input: &str, #[case] should_pass: bool) {
        if should_pass {
            assert!(passes(kebab_case(input)), "expected `{input}` to pass");
        } else {
            assert!(fails(kebab_case(input)), "expected `{input}` to fail");
        }
    }

    // -----------------------------------------------------------------------
    // pascal_case
    // -----------------------------------------------------------------------

    #[rstest]
    #[case::single_word("Design", true)]
    #[case::multi_word("DesignFlow", true)]
    #[case::with_digits("V1Roadmap", true)]
    #[case::trailing_digits("Version2", true)]
    #[case::single_letter("A", true)]
    #[case::empty("", false)]
    #[case::lowercase_first("design", false)]
    #[case::leading_digit("1Design", false)]
    #[case::kebab("design-flow", false)]
    #[case::snake("design_flow", false)]
    #[case::space("Design Flow", false)]
    #[case::special_char("Design!", false)]
    #[case::unicode("Désign", false)]
    fn pascal_case_cases(#[case] input: &str, #[case] should_pass: bool) {
        if should_pass {
            assert!(passes(pascal_case(input)), "expected `{input}` to pass");
        } else {
            assert!(fails(pascal_case(input)), "expected `{input}` to fail");
        }
    }

    // -----------------------------------------------------------------------
    // non_empty_str / opt_non_empty_str
    // -----------------------------------------------------------------------

    #[rstest]
    #[case::populated("hello", true)]
    #[case::single_char("a", true)]
    #[case::with_whitespace_around("  text  ", true)]
    #[case::empty("", false)]
    #[case::whitespace_only("   ", false)]
    #[case::tab_only("\t", false)]
    #[case::newline_only("\n", false)]
    fn non_empty_str_cases(#[case] input: &str, #[case] should_pass: bool) {
        if should_pass {
            assert!(passes(non_empty_str(input)), "expected `{input:?}` to pass");
        } else {
            assert!(fails(non_empty_str(input)), "expected `{input:?}` to fail");
        }
    }

    #[test]
    fn opt_non_empty_str_none_passes() {
        assert!(passes(opt_non_empty_str(&None)));
    }

    #[test]
    fn opt_non_empty_str_some_delegates_to_non_empty_str() {
        assert!(passes(opt_non_empty_str(&Some("hello".to_string()))));
        assert!(fails(opt_non_empty_str(&Some("".to_string()))));
        assert!(fails(opt_non_empty_str(&Some("   ".to_string()))));
    }

    // -----------------------------------------------------------------------
    // non_empty_list / each_item_non_empty
    // -----------------------------------------------------------------------

    #[test]
    fn non_empty_list_empty_fails() {
        let empty: Vec<i32> = vec![];
        assert!(fails(non_empty_list(&empty)));
    }

    #[test]
    fn non_empty_list_one_element_passes() {
        assert!(passes(non_empty_list(&[1])));
    }

    #[test]
    fn each_item_non_empty_all_populated() {
        let v = vec!["a".to_string(), "b".to_string()];
        assert!(passes(each_item_non_empty(&v)));
    }

    #[test]
    fn each_item_non_empty_empty_list_passes() {
        // Vacuously true — no items, nothing to violate.
        let v: Vec<String> = vec![];
        assert!(passes(each_item_non_empty(&v)));
    }

    #[test]
    fn each_item_non_empty_collects_per_offending_item() {
        let v = vec![
            "ok".to_string(),
            "".to_string(),
            "also-ok".to_string(),
            "   ".to_string(),
        ];
        let violations = each_item_non_empty(&v);
        assert_eq!(violations.len(), 2);
    }

    // -----------------------------------------------------------------------
    // non_empty_map
    // -----------------------------------------------------------------------

    #[test]
    fn non_empty_map_empty_fails() {
        let m: std::collections::HashMap<&str, i32> = std::collections::HashMap::new();
        assert!(fails(non_empty_map(&m)));
    }

    #[test]
    fn non_empty_map_with_entry_passes() {
        let mut m = std::collections::HashMap::new();
        m.insert("k", 1);
        assert!(passes(non_empty_map(&m)));
    }

    // -----------------------------------------------------------------------
    // min_length
    // -----------------------------------------------------------------------

    #[rstest]
    #[case::below_min(vec![1], 2, false)]
    #[case::at_min(vec![1, 2], 2, true)]
    #[case::above_min(vec![1, 2, 3], 2, true)]
    #[case::zero_min_empty(vec![], 0, true)]
    #[case::empty_below(vec![], 1, false)]
    fn min_length_cases(#[case] input: Vec<i32>, #[case] min: usize, #[case] should_pass: bool) {
        let result = min_length(&input, min);
        assert_eq!(result.is_empty(), should_pass);
    }

    // -----------------------------------------------------------------------
    // unique_by
    // -----------------------------------------------------------------------

    #[test]
    fn unique_by_empty_passes() {
        let v: Vec<i32> = vec![];
        assert!(passes(unique_by(&v, |x| *x)));
    }

    #[test]
    fn unique_by_all_distinct_passes() {
        let v = vec![1, 2, 3, 4];
        assert!(passes(unique_by(&v, |x| *x)));
    }

    #[test]
    fn unique_by_single_duplicate_violates() {
        let v = vec![1, 2, 1];
        let violations = unique_by(&v, |x| *x);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn unique_by_collects_per_repeat_after_first() {
        // First occurrence is OK; every subsequent repeat is a
        // violation. With [1, 2, 1, 1] that's two violations.
        let v = vec![1, 2, 1, 1];
        let violations = unique_by(&v, |x| *x);
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn unique_by_uses_key_fn_not_item_identity() {
        // Items differ but their derived keys collide.
        let v = vec![("a", 1), ("b", 1)];
        let violations = unique_by(&v, |t| t.1);
        assert_eq!(violations.len(), 1);
    }
}
