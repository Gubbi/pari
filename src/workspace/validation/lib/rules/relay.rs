use super::{
    super::schema::{AnyCrossEntityRule, AnyStructuralRule, ValidationSchema},
    structural::{
        primitives::{non_empty_str, opt_non_empty_str, pascal_case_id},
        raci::raci_structural,
        relay::{non_empty_map_state_map, pascal_case_state_keys},
    },
};
use crate::{
    entity::entities::relay::{Relay, TrackedRelay},
    validation::lib::rules::cross_entity::{
        common::parent_exists,
        intercepts::{intercept_hooks_exist, intercept_inputs_valid},
        relay::maps_to_states_exist,
    },
    workspace::XViewer,
};

pub fn relay_validation_schema() -> ValidationSchema<Relay> {
    let mut structural: std::collections::HashMap<&'static str, Vec<AnyStructuralRule<Relay>>> =
        std::collections::HashMap::new();

    structural.insert(
        "entity_ref",
        vec![Box::new(|e: &TrackedRelay| pascal_case_id(&e.entity_ref))],
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
        "raci",
        vec![Box::new(|e: &TrackedRelay| {
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
        "briefing",
        vec![Box::new(|e: &TrackedRelay| {
            e.briefing
                .get()
                .map(|v| opt_non_empty_str(v))
                .unwrap_or_default()
        })],
    );
    structural.insert(
        "debriefing",
        vec![Box::new(|e: &TrackedRelay| {
            e.debriefing
                .get()
                .map(|v| opt_non_empty_str(v))
                .unwrap_or_default()
        })],
    );
    structural.insert(
        "guidance",
        vec![Box::new(|e: &TrackedRelay| {
            e.guidance
                .get()
                .map(|v| opt_non_empty_str(v))
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
                    .map(|v| pascal_case_state_keys(v))
                    .unwrap_or_default()
            }),
        ],
    );
    let mut cross_entity: std::collections::HashMap<&'static str, Vec<AnyCrossEntityRule<Relay>>> =
        std::collections::HashMap::new();
    cross_entity.insert(
        "entity_ref",
        vec![Box::new(|viewer: &XViewer<'_, Relay>| {
            let parent = viewer.tracked().entity_ref.parent().cloned();
            let workspace = viewer.workspace();
            Box::pin(async move {
                match parent {
                    Some(p) => parent_exists(workspace, p).await,
                    None => vec![],
                }
            })
        })],
    );
    cross_entity.insert(
        "delegates_to",
        vec![crate::ref_check_rule!(Relay, delegates_to)],
    );
    cross_entity.insert("raci", vec![crate::ref_check_rule!(Relay, raci)]);
    cross_entity.insert(
        "state_map",
        vec![
            crate::ref_check_rule!(Relay, state_map),
            Box::new(|viewer: &XViewer<'_, Relay>| {
                let delegates_to_id = viewer
                    .tracked()
                    .delegates_to
                    .get()
                    .map(|r| r.id().to_owned())
                    .unwrap_or_default();
                let state_map = viewer
                    .tracked()
                    .state_map
                    .get()
                    .cloned()
                    .unwrap_or_default();
                let workspace = viewer.workspace();
                Box::pin(async move {
                    maps_to_states_exist(workspace, &delegates_to_id, state_map).await
                })
            }),
        ],
    );
    cross_entity.insert(
        "intercepts",
        vec![
            Box::new(|viewer: &XViewer<'_, Relay>| {
                let map = viewer
                    .tracked()
                    .intercepts
                    .get()
                    .and_then(|v| v.as_ref())
                    .cloned()
                    .unwrap_or_default();
                let workspace = viewer.workspace();
                Box::pin(async move { intercept_hooks_exist(workspace, map, "intercepts").await })
            }),
            Box::new(|viewer: &XViewer<'_, Relay>| {
                let map = viewer
                    .tracked()
                    .intercepts
                    .get()
                    .and_then(|v| v.as_ref())
                    .cloned()
                    .unwrap_or_default();
                let workspace = viewer.workspace();
                Box::pin(async move { intercept_inputs_valid(workspace, map, "intercepts").await })
            }),
        ],
    );

    ValidationSchema {
        structural,
        semantic: std::collections::HashMap::new(),
        cross_entity,
    }
}
