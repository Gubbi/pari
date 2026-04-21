use std::hash::Hash;

use crate::entity::{Entity, EntityRef, ParentKind, types::Extensions};
use crate::error::primitive::PrimitiveError;

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
pub fn camel_case(value: &str) -> Vec<PrimitiveError> {
    let valid = !value.is_empty()
        && value.starts_with(|c: char| c.is_ascii_uppercase())
        && value.chars().all(|c| c.is_ascii_alphanumeric());
    if valid {
        vec![]
    } else {
        vec![PrimitiveError::naming_format_violation(
            format!("'{value}' is not CamelCase"),
            None::<String>,
            "camel_case",
        )]
    }
}

/// `EntityRef` id must be kebab-case
pub fn kebab_case_id<T: Entity, P: ParentKind>(entity_ref: &EntityRef<T, P>) -> Vec<PrimitiveError> {
    kebab_case(entity_ref.id())
}

/// `EntityRef` id must be CamelCase
pub fn camel_case_id<T: Entity, P: ParentKind>(entity_ref: &EntityRef<T, P>) -> Vec<PrimitiveError> {
    camel_case(entity_ref.id())
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

/// All keys in extensions must start with `"x-"`
pub fn x_prefix_keys(value: &Extensions) -> Vec<PrimitiveError> {
    value
        .keys()
        .filter(|k| !k.starts_with("x-"))
        .map(|k| {
            PrimitiveError::naming_format_violation(
                format!("extension key '{k}' must start with 'x-'"),
                Some(format!(".{k}")),
                "x_prefix_keys",
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::primitive::PrimitiveError;

    #[test]
    fn kebab_case_valid_ids() {
        assert!(kebab_case("eng-lead").is_empty());
        assert!(kebab_case("abc").is_empty());
        assert!(kebab_case("a-b-c").is_empty());
        assert!(kebab_case("abc123").is_empty());
    }

    #[test]
    fn kebab_case_rejects_uppercase() {
        assert!(!kebab_case("EngLead").is_empty());
    }

    #[test]
    fn kebab_case_rejects_leading_dash() {
        assert!(!kebab_case("-eng").is_empty());
    }

    #[test]
    fn kebab_case_rejects_trailing_dash() {
        assert!(!kebab_case("eng-").is_empty());
    }

    #[test]
    fn kebab_case_rejects_double_dash() {
        assert!(!kebab_case("eng--lead").is_empty());
    }

    #[test]
    fn kebab_case_rejects_empty() {
        assert!(!kebab_case("").is_empty());
    }

    #[test]
    fn camel_case_valid_ids() {
        assert!(camel_case("WriteProposal").is_empty());
        assert!(camel_case("Done").is_empty());
        assert!(camel_case("InitiativeWorkflow").is_empty());
    }

    #[test]
    fn camel_case_rejects_lowercase_start() {
        assert!(!camel_case("writeProposal").is_empty());
    }

    #[test]
    fn camel_case_rejects_hyphens() {
        assert!(!camel_case("Write-Proposal").is_empty());
    }

    #[test]
    fn camel_case_rejects_empty() {
        assert!(!camel_case("").is_empty());
    }

    #[test]
    fn non_empty_str_valid() {
        assert!(non_empty_str("hello").is_empty());
    }

    #[test]
    fn non_empty_str_rejects_empty() {
        assert!(!non_empty_str("").is_empty());
    }

    #[test]
    fn non_empty_str_rejects_whitespace_only() {
        assert!(!non_empty_str("   ").is_empty());
    }

    #[test]
    fn non_empty_list_valid() {
        assert!(non_empty_list(&[1u32, 2]).is_empty());
    }

    #[test]
    fn non_empty_list_rejects_empty() {
        assert!(!non_empty_list::<u32>(&[]).is_empty());
    }

    #[test]
    fn min_length_passes_when_met() {
        assert!(min_length(&[1u32, 2], 2).is_empty());
    }

    #[test]
    fn min_length_fails_when_short() {
        assert!(!min_length(&[1u32], 2).is_empty());
    }

    #[test]
    fn unique_by_no_duplicates() {
        let v = vec!["a", "b", "c"];
        assert!(unique_by(&v, |s| s.to_string()).is_empty());
    }

    #[test]
    fn unique_by_reports_duplicate_index() {
        let v = vec!["a", "b", "a"];
        let violations = unique_by(&v, |s| s.to_string());
        assert_eq!(violations.len(), 1);
        match &violations[0] {
            PrimitiveError::DuplicateEntryViolation { sub_path, .. } => {
                assert_eq!(sub_path.as_str(), "[2]");
            }
            other => panic!("expected DuplicateEntryViolation, got {other:?}"),
        }
    }

    #[test]
    fn x_prefix_keys_valid() {
        let mut ext = Extensions::new();
        ext.insert("x-owner".to_string(), serde_json::json!("alice"));
        assert!(x_prefix_keys(&ext).is_empty());
    }

    #[test]
    fn x_prefix_keys_rejects_non_x_keys() {
        let mut ext = Extensions::new();
        ext.insert("owner".to_string(), serde_json::json!("alice"));
        let v = x_prefix_keys(&ext);
        assert_eq!(v.len(), 1);
        match &v[0] {
            PrimitiveError::NamingFormatViolation { sub_path, .. } => {
                assert!(sub_path.as_ref().unwrap().contains("owner"));
            }
            other => panic!("expected NamingFormatViolation, got {other:?}"),
        }
    }
}
