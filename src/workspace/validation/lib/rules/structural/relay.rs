use std::collections::HashMap;

use crate::{entity::entities::relay::StateMapEntry, error::primitive::PrimitiveError};

pub fn non_empty_map_state_map(value: &HashMap<String, StateMapEntry>) -> Vec<PrimitiveError> {
    if value.is_empty() {
        vec![PrimitiveError::malformed_collection_value(
            "state_map must not be empty",
            "non_empty",
        )]
    } else {
        vec![]
    }
}

pub fn pascal_case_state_keys(value: &HashMap<String, StateMapEntry>) -> Vec<PrimitiveError> {
    value
        .keys()
        .filter(|k| {
            k.is_empty()
                || !k.starts_with(|c: char| c.is_ascii_uppercase())
                || !k.chars().all(|c| c.is_ascii_alphanumeric())
        })
        .map(|k| {
            PrimitiveError::naming_format_violation(
                format!("'{k}' is not PascalCase"),
                Some(format!(".{k}")),
                "pascal_case",
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(maps_to: &str) -> StateMapEntry {
        StateMapEntry {
            maps_to: maps_to.to_string(),
            description: None,
            semantic: None,
        }
    }

    fn map_with(keys: &[&str]) -> HashMap<String, StateMapEntry> {
        keys.iter()
            .map(|k| (k.to_string(), entry("Done")))
            .collect()
    }

    // -----------------------------------------------------------------------
    // non_empty_map_state_map
    // -----------------------------------------------------------------------

    #[test]
    fn non_empty_passes_with_one_entry() {
        let m = map_with(&["Done"]);
        assert!(non_empty_map_state_map(&m).is_empty());
    }

    #[test]
    fn non_empty_violates_when_empty() {
        let v = non_empty_map_state_map(&HashMap::new());
        assert_eq!(v.len(), 1);
        assert!(matches!(
            v[0],
            PrimitiveError::MalformedCollectionValue { .. }
        ));
    }

    // -----------------------------------------------------------------------
    // pascal_case_state_keys
    // -----------------------------------------------------------------------

    #[test]
    fn pascal_keys_all_valid_passes() {
        let m = map_with(&["Done", "InProgress"]);
        assert!(pascal_case_state_keys(&m).is_empty());
    }

    #[test]
    fn pascal_keys_lowercase_first_violates() {
        let m = map_with(&["done"]);
        assert_eq!(pascal_case_state_keys(&m).len(), 1);
    }

    #[test]
    fn pascal_keys_kebab_violates() {
        let m = map_with(&["in-progress"]);
        assert_eq!(pascal_case_state_keys(&m).len(), 1);
    }

    #[test]
    fn pascal_keys_empty_key_violates() {
        let m = map_with(&[""]);
        assert_eq!(pascal_case_state_keys(&m).len(), 1);
    }

    #[test]
    fn pascal_keys_collects_per_bad_key() {
        let m = map_with(&["Done", "in-progress", "lowercase"]);
        // Done passes; the other two fail.
        assert_eq!(pascal_case_state_keys(&m).len(), 2);
    }
}
