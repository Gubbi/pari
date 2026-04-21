use super::super::schema::{AnyCrossEntityRule, AnyStructuralRule, ValidationSchema};
use super::structural::{
    primitives::{camel_case_id, non_empty_list, non_empty_str, opt_non_empty_str, x_prefix_keys},
    task::states_valid_task,
};
use crate::entity::entities::task::{Task, TrackedTask};

pub fn task_validation_schema() -> ValidationSchema<Task> {
    let mut structural: std::collections::HashMap<&'static str, Vec<AnyStructuralRule<Task>>> =
        std::collections::HashMap::new();

    structural.insert(
        "entity_ref",
        vec![Box::new(|e: &TrackedTask| camel_case_id(&e.entity_ref))],
    );
    structural.insert(
        "name",
        vec![Box::new(|e: &TrackedTask| {
            e.name.get().map(|v| non_empty_str(v)).unwrap_or_default()
        })],
    );
    structural.insert(
        "description",
        vec![Box::new(|e: &TrackedTask| {
            e.description.get().map(|v| opt_non_empty_str(v)).unwrap_or_default()
        })],
    );
    structural.insert(
        "purpose",
        vec![Box::new(|e: &TrackedTask| {
            e.purpose.get().map(|v| non_empty_str(v)).unwrap_or_default()
        })],
    );
    structural.insert(
        "instructions",
        vec![Box::new(|e: &TrackedTask| {
            e.instructions.get().map(|v| non_empty_list(v.as_slice())).unwrap_or_default()
        })],
    );
    structural.insert(
        "criteria",
        vec![Box::new(|e: &TrackedTask| {
            e.criteria.get().map(|v| non_empty_list(v.as_slice())).unwrap_or_default()
        })],
    );
    structural.insert(
        "states",
        vec![Box::new(|e: &TrackedTask| {
            e.states.get().map(|v| states_valid_task(v.as_slice())).unwrap_or_default()
        })],
    );
    structural.insert(
        "extensions",
        vec![Box::new(|e: &TrackedTask| {
            e.extensions.get().map(|v| x_prefix_keys(v)).unwrap_or_default()
        })],
    );

    let mut cross_entity: std::collections::HashMap<&'static str, Vec<AnyCrossEntityRule<Task>>> =
        std::collections::HashMap::new();
    cross_entity.insert(
        "raci",
        vec![Box::new(|_e: &TrackedTask| Box::pin(async { vec![] }))],
    );
    cross_entity.insert(
        "artifact",
        vec![Box::new(|_e: &TrackedTask| Box::pin(async { vec![] }))],
    );

    ValidationSchema {
        structural,
        semantic: std::collections::HashMap::new(),
        cross_entity,
    }
}
