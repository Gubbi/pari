use super::{
    super::schema::{AnyStructuralRule, ValidationSchema},
    structural::{
        primitives::{kebab_case_id, non_empty_str, opt_non_empty_str},
        role::each_item_non_empty_str,
    },
};
use crate::entity::entities::role::{Role, TrackedRole};

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
    ValidationSchema {
        structural,
        semantic: std::collections::HashMap::new(),
        cross_entity: std::collections::HashMap::new(),
    }
}
