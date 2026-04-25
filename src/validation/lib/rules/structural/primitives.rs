use std::hash::Hash;

use crate::{
    entity::{types::Extensions, Entity, EntityRef, ParentKind},
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
pub fn kebab_case_id<T: Entity, P: ParentKind>(
    entity_ref: &EntityRef<T, P>,
) -> Vec<PrimitiveError> {
    kebab_case(entity_ref.id())
}

/// `EntityRef` id must be CamelCase
pub fn camel_case_id<T: Entity, P: ParentKind>(
    entity_ref: &EntityRef<T, P>,
) -> Vec<PrimitiveError> {
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
