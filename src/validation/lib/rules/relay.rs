use super::{
    super::schema::{AnyCrossEntityRule, AnyStructuralRule, ValidationSchema},
    structural::{
        primitives::{camel_case_id, non_empty_str, opt_non_empty_str, x_prefix_keys},
        relay::{camel_case_state_keys, non_empty_map_state_map},
    },
};
use crate::entity::entities::relay::{Relay, TrackedRelay};

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
    cross_entity.insert(
        "delegates_to",
        vec![crate::ref_check_rule!(TrackedRelay, delegates_to)],
    );
    cross_entity.insert("raci", vec![crate::ref_check_rule!(TrackedRelay, raci)]);
    cross_entity.insert(
        "state_map",
        vec![crate::ref_check_rule!(TrackedRelay, state_map)],
    );

    ValidationSchema {
        structural,
        semantic: std::collections::HashMap::new(),
        cross_entity,
    }
}
