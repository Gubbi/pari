//! Structural and cross-entity validation schema for [`Role`].

use super::{
    kebab_case_id, non_empty_str, x_prefix_keys, AnyStructuralRule, ValidationSchema,
};
use crate::entity::entities::role::{Role, TrackedRole};
use crate::error::primitive::PrimitiveError;

fn opt_non_empty_str(value: &Option<String>) -> Vec<PrimitiveError> {
    match value {
        None => vec![],
        Some(s) => non_empty_str(s),
    }
}

fn each_item_non_empty_str(value: &Option<Vec<String>>) -> Vec<PrimitiveError> {
    match value {
        None => vec![],
        Some(items) => items
            .iter()
            .enumerate()
            .flat_map(|(i, s)| {
                if s.trim().is_empty() {
                    vec![PrimitiveError::empty_required_value(
                        "must not be empty",
                        Some(format!("[{i}]")),
                        "non_empty",
                    )]
                } else {
                    vec![]
                }
            })
            .collect(),
    }
}

pub fn role_validation_schema() -> ValidationSchema<Role> {
    let mut structural: std::collections::HashMap<&'static str, Vec<AnyStructuralRule<Role>>> =
        std::collections::HashMap::new();

    structural.insert(
        "entity_ref",
        vec![Box::new(|e: &TrackedRole| kebab_case_id(&e.entity_ref))],
    );

    structural.insert(
        "name",
        vec![Box::new(|e: &TrackedRole| {
            e.name.get().map(|v| non_empty_str(v)).unwrap_or_default()
        })],
    );

    structural.insert(
        "description",
        vec![Box::new(|e: &TrackedRole| {
            e.description
                .get()
                .map(|v| opt_non_empty_str(v))
                .unwrap_or_default()
        })],
    );

    structural.insert(
        "purpose",
        vec![Box::new(|e: &TrackedRole| {
            e.purpose
                .get()
                .map(|v| non_empty_str(v))
                .unwrap_or_default()
        })],
    );

    structural.insert(
        "traits",
        vec![Box::new(|e: &TrackedRole| {
            e.traits
                .get()
                .map(|v| each_item_non_empty_str(v))
                .unwrap_or_default()
        })],
    );

    structural.insert(
        "extensions",
        vec![Box::new(|e: &TrackedRole| {
            e.extensions
                .get()
                .map(|v| x_prefix_keys(v))
                .unwrap_or_default()
        })],
    );

    ValidationSchema {
        structural,
        semantic: std::collections::HashMap::new(),
        cross_entity: std::collections::HashMap::new(),
    }
}
