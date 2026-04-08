//! Structural and cross-entity validation schema for [`Role`].

use super::{
    AnyStructuralRule, RuleViolation, ValidationSchema,
    kebab_case_id, non_empty_str, x_prefix_keys,
};
use crate::entities::role::{Role, TrackedRole};

fn opt_non_empty_str(value: &Option<String>) -> Vec<RuleViolation> {
    match value {
        None => vec![],
        Some(s) => non_empty_str(s),
    }
}

fn each_item_non_empty_str(value: &Option<Vec<String>>) -> Vec<RuleViolation> {
    match value {
        None => vec![],
        Some(items) => items
            .iter()
            .enumerate()
            .flat_map(|(i, s)| {
                non_empty_str(s)
                    .into_iter()
                    .map(move |v| RuleViolation::sub(format!("[{i}]"), v.message))
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
            e.description.get().map(|v| opt_non_empty_str(v)).unwrap_or_default()
        })],
    );

    structural.insert(
        "purpose",
        vec![Box::new(|e: &TrackedRole| {
            e.purpose.get().map(|v| non_empty_str(v)).unwrap_or_default()
        })],
    );

    structural.insert(
        "traits",
        vec![Box::new(|e: &TrackedRole| {
            e.traits.get().map(|v| each_item_non_empty_str(v)).unwrap_or_default()
        })],
    );

    structural.insert(
        "extensions",
        vec![Box::new(|e: &TrackedRole| {
            e.extensions.get().map(|v| x_prefix_keys(v)).unwrap_or_default()
        })],
    );

    ValidationSchema {
        structural,
        semantic: std::collections::HashMap::new(),
        cross_entity: std::collections::HashMap::new(),
    }
}
