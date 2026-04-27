use super::{
    super::schema::{AnyCrossEntityRule, AnyStructuralRule, ValidationSchema},
    structural::{
        primitives::{
            each_item_non_empty, non_empty_list, non_empty_str, opt_non_empty_str, pascal_case_id,
            x_prefix_keys,
        },
        raci::raci_structural,
        task::states_valid_task,
    },
};
use crate::{
    entity::entities::task::{Task, TrackedTask},
    validation::lib::rules::cross_entity::intercepts::{
        intercept_hooks_exist, intercept_inputs_valid,
    },
};

pub fn task_validation_schema() -> ValidationSchema<Task> {
    let mut structural: std::collections::HashMap<&'static str, Vec<AnyStructuralRule<Task>>> =
        std::collections::HashMap::new();

    structural.insert(
        "entity_ref",
        vec![Box::new(|e: &TrackedTask| pascal_case_id(&e.entity_ref))],
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
            e.description
                .get()
                .map(|v| opt_non_empty_str(v))
                .unwrap_or_default()
        })],
    );
    structural.insert(
        "purpose",
        vec![Box::new(|e: &TrackedTask| {
            e.purpose
                .get()
                .map(|v| non_empty_str(v))
                .unwrap_or_default()
        })],
    );
    structural.insert(
        "instructions",
        vec![Box::new(|e: &TrackedTask| {
            e.instructions
                .get()
                .map(|v| {
                    let mut violations = non_empty_list(v.as_slice());
                    violations.extend(each_item_non_empty(v.as_slice()));
                    violations
                })
                .unwrap_or_default()
        })],
    );
    structural.insert(
        "criteria",
        vec![Box::new(|e: &TrackedTask| {
            e.criteria
                .get()
                .map(|v| {
                    let mut violations = non_empty_list(v.as_slice());
                    violations.extend(each_item_non_empty(v.as_slice()));
                    violations
                })
                .unwrap_or_default()
        })],
    );
    structural.insert(
        "raci",
        vec![Box::new(|e: &TrackedTask| {
            e.raci
                .get()
                .map(|opt_raci| {
                    opt_raci
                        .as_ref()
                        .map(|r| raci_structural(r))
                        .unwrap_or_default()
                })
                .unwrap_or_default()
        })],
    );
    structural.insert(
        "guidance",
        vec![Box::new(|e: &TrackedTask| {
            e.guidance
                .get()
                .map(|v| opt_non_empty_str(v))
                .unwrap_or_default()
        })],
    );
    structural.insert(
        "states",
        vec![Box::new(|e: &TrackedTask| {
            e.states
                .get()
                .map(|v| states_valid_task(v.as_slice()))
                .unwrap_or_default()
        })],
    );
    structural.insert(
        "extensions",
        vec![Box::new(|e: &TrackedTask| {
            e.extensions
                .get()
                .map(|v| x_prefix_keys(v))
                .unwrap_or_default()
        })],
    );

    let mut cross_entity: std::collections::HashMap<&'static str, Vec<AnyCrossEntityRule<Task>>> =
        std::collections::HashMap::new();
    cross_entity.insert("raci", vec![crate::ref_check_rule!(TrackedTask, raci)]);
    cross_entity.insert(
        "artifact",
        vec![crate::ref_check_rule!(TrackedTask, artifact)],
    );
    cross_entity.insert(
        "intercepts",
        vec![
            Box::new(|e: &TrackedTask| {
                let map = e
                    .intercepts
                    .get()
                    .and_then(|v| v.as_ref())
                    .cloned()
                    .unwrap_or_default();
                Box::pin(async move { intercept_hooks_exist(map, "intercepts").await })
            }),
            Box::new(|e: &TrackedTask| {
                let map = e
                    .intercepts
                    .get()
                    .and_then(|v| v.as_ref())
                    .cloned()
                    .unwrap_or_default();
                Box::pin(async move { intercept_inputs_valid(map, "intercepts").await })
            }),
        ],
    );

    ValidationSchema {
        structural,
        semantic: std::collections::HashMap::new(),
        cross_entity,
    }
}
