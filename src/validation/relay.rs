//! Structural and cross-entity validation schema for [`Relay`].

use std::collections::HashMap;

use super::{
    camel_case, camel_case_id, non_empty_str, x_prefix_keys, AnyCrossEntityRule, AnyStructuralRule,
    RuleViolation, ValidationSchema,
};
use crate::entity::entities::relay::{Relay, TrackedRelay};

fn opt_non_empty_str(value: &Option<String>) -> Vec<RuleViolation> {
    match value {
        None => vec![],
        Some(s) => non_empty_str(s),
    }
}

fn non_empty_map_state_map(
    value: &HashMap<String, crate::entity::entities::relay::StateMapEntry>,
) -> Vec<RuleViolation> {
    if value.is_empty() {
        vec![RuleViolation::field("state_map must not be empty")]
    } else {
        vec![]
    }
}

fn camel_case_state_keys(
    value: &HashMap<String, crate::entity::entities::relay::StateMapEntry>,
) -> Vec<RuleViolation> {
    value
        .keys()
        .flat_map(|k| {
            camel_case(k)
                .into_iter()
                .map(move |v| RuleViolation::sub(format!(".{k}"), v.message))
        })
        .collect()
}

pub fn relay_validation_schema() -> ValidationSchema<Relay> {
    let mut structural: std::collections::HashMap<&'static str, Vec<AnyStructuralRule<Relay>>> =
        std::collections::HashMap::new();

    structural.insert(
        "entity_ref",
        vec![Box::new(|e: &TrackedRelay| camel_case_id(&e.entity_ref))],
    );

    structural.insert(
        "name",
        vec![Box::new(|e: &TrackedRelay| {
            e.name.get().map(|v| non_empty_str(v)).unwrap_or_default()
        })],
    );

    structural.insert(
        "description",
        vec![Box::new(|e: &TrackedRelay| {
            e.description
                .get()
                .map(|v| opt_non_empty_str(v))
                .unwrap_or_default()
        })],
    );

    structural.insert(
        "purpose",
        vec![Box::new(|e: &TrackedRelay| {
            e.purpose
                .get()
                .map(|v| non_empty_str(v))
                .unwrap_or_default()
        })],
    );

    structural.insert(
        "state_map",
        vec![
            Box::new(|e: &TrackedRelay| {
                e.state_map
                    .get()
                    .map(|v| non_empty_map_state_map(v))
                    .unwrap_or_default()
            }),
            Box::new(|e: &TrackedRelay| {
                e.state_map
                    .get()
                    .map(|v| camel_case_state_keys(v))
                    .unwrap_or_default()
            }),
        ],
    );

    structural.insert(
        "extensions",
        vec![Box::new(|e: &TrackedRelay| {
            e.extensions
                .get()
                .map(|v| x_prefix_keys(v))
                .unwrap_or_default()
        })],
    );

    let mut cross_entity: std::collections::HashMap<&'static str, Vec<AnyCrossEntityRule<Relay>>> =
        std::collections::HashMap::new();

    // Stub: delegates_to ref_exists
    cross_entity.insert(
        "delegates_to",
        vec![Box::new(|_e: &TrackedRelay| Box::pin(async { vec![] }))],
    );

    // Stub: raci_roles_exist
    cross_entity.insert(
        "raci",
        vec![Box::new(|_e: &TrackedRelay| Box::pin(async { vec![] }))],
    );

    // Stub: maps_to_states_exist
    cross_entity.insert(
        "state_map",
        vec![Box::new(|_e: &TrackedRelay| Box::pin(async { vec![] }))],
    );

    ValidationSchema {
        structural,
        semantic: std::collections::HashMap::new(),
        cross_entity,
    }
}
