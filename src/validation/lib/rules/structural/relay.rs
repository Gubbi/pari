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

pub fn camel_case_state_keys(value: &HashMap<String, StateMapEntry>) -> Vec<PrimitiveError> {
    value
        .keys()
        .filter(|k| {
            k.is_empty()
                || !k.starts_with(|c: char| c.is_ascii_uppercase())
                || !k.chars().all(|c| c.is_ascii_alphanumeric())
        })
        .map(|k| {
            PrimitiveError::naming_format_violation(
                format!("'{k}' is not CamelCase"),
                Some(format!(".{k}")),
                "camel_case",
            )
        })
        .collect()
}
