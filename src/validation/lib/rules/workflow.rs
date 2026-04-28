use super::{
    super::schema::{AnyCrossEntityRule, AnySemanticRule, AnyStructuralRule, ValidationSchema},
    semantic::workflow::{
        depends_on_valid, depends_on_valid_embedded, depends_on_valid_reusable, on_reject_valid,
        on_reject_valid_embedded, on_reject_valid_reusable, reviewing_state_required,
        reviewing_state_required_embedded, reviewing_state_required_reusable,
    },
    structural::{
        primitives::{non_empty_str, opt_non_empty_str, pascal_case_id, x_prefix_keys},
        raci::raci_structural,
        workflow::{states_valid_workflow, step_keys_pascal_case},
    },
};
use crate::{
    entity::entities::workflow::{
        EmbeddedWorkflow, ReusableWorkflow, TrackedEmbeddedWorkflow, TrackedReusableWorkflow,
        TrackedWorkflow, Workflow,
    },
    validation::lib::rules::cross_entity::{
        common::parent_exists,
        intercepts::{intercept_hooks_exist, intercept_inputs_valid},
        workflow::no_relay_in_tree,
    },
};

macro_rules! common_structural {
    ($E:ty, $Tracked:ty) => {{
        let mut structural: std::collections::HashMap<&'static str, Vec<AnyStructuralRule<$E>>> =
            std::collections::HashMap::new();

        structural.insert(
            "entity_ref",
            vec![Box::new(|e: &$Tracked| pascal_case_id(&e.entity_ref))],
        );
        structural.insert(
            "name",
            vec![Box::new(|e: &$Tracked| {
                e.name.get().map(|v| non_empty_str(v)).unwrap_or_default()
            })],
        );
        structural.insert(
            "description",
            vec![Box::new(|e: &$Tracked| {
                e.description
                    .get()
                    .map(|v| opt_non_empty_str(v))
                    .unwrap_or_default()
            })],
        );
        structural.insert(
            "purpose",
            vec![Box::new(|e: &$Tracked| {
                e.purpose
                    .get()
                    .map(|v| non_empty_str(v))
                    .unwrap_or_default()
            })],
        );
        structural.insert(
            "raci",
            vec![Box::new(|e: &$Tracked| {
                e.raci.get().map(|v| raci_structural(v)).unwrap_or_default()
            })],
        );
        structural.insert(
            "states",
            vec![Box::new(|e: &$Tracked| {
                e.states
                    .get()
                    .map(|v| states_valid_workflow(v.as_slice()))
                    .unwrap_or_default()
            })],
        );
        structural.insert(
            "steps",
            vec![Box::new(|e: &$Tracked| {
                e.steps
                    .get()
                    .map(|v| step_keys_pascal_case(v))
                    .unwrap_or_default()
            })],
        );
        structural.insert(
            "guidance",
            vec![Box::new(|e: &$Tracked| {
                e.guidance
                    .get()
                    .map(|v| opt_non_empty_str(v))
                    .unwrap_or_default()
            })],
        );
        structural.insert(
            "extensions",
            vec![Box::new(|e: &$Tracked| {
                e.extensions
                    .get()
                    .map(|v| x_prefix_keys(v))
                    .unwrap_or_default()
            })],
        );

        structural
    }};
}

pub fn workflow_validation_schema() -> ValidationSchema<Workflow> {
    let structural = common_structural!(Workflow, TrackedWorkflow);

    let mut semantic: std::collections::HashMap<&'static str, Vec<AnySemanticRule<Workflow>>> =
        std::collections::HashMap::new();
    semantic.insert(
        "steps",
        vec![
            Box::new(|e: &TrackedWorkflow| Box::pin(depends_on_valid(e))),
            Box::new(|e: &TrackedWorkflow| Box::pin(on_reject_valid(e))),
            Box::new(|e: &TrackedWorkflow| Box::pin(reviewing_state_required(e))),
        ],
    );

    let mut cross_entity: std::collections::HashMap<
        &'static str,
        Vec<AnyCrossEntityRule<Workflow>>,
    > = std::collections::HashMap::new();
    cross_entity.insert(
        "steps",
        vec![crate::ref_check_rule!(TrackedWorkflow, steps)],
    );
    cross_entity.insert("raci", vec![crate::ref_check_rule!(TrackedWorkflow, raci)]);
    cross_entity.insert(
        "intercepts",
        vec![
            Box::new(|e: &TrackedWorkflow| {
                let map = e
                    .intercepts
                    .get()
                    .and_then(|v| v.as_ref())
                    .cloned()
                    .unwrap_or_default();
                Box::pin(async move { intercept_hooks_exist(map, "intercepts").await })
            }),
            Box::new(|e: &TrackedWorkflow| {
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
        semantic,
        cross_entity,
    }
}

pub fn reusable_workflow_validation_schema() -> ValidationSchema<ReusableWorkflow> {
    let structural = common_structural!(ReusableWorkflow, TrackedReusableWorkflow);

    let mut semantic: std::collections::HashMap<
        &'static str,
        Vec<AnySemanticRule<ReusableWorkflow>>,
    > = std::collections::HashMap::new();
    semantic.insert(
        "steps",
        vec![
            Box::new(|e: &TrackedReusableWorkflow| Box::pin(depends_on_valid_reusable(e))),
            Box::new(|e: &TrackedReusableWorkflow| Box::pin(on_reject_valid_reusable(e))),
            Box::new(|e: &TrackedReusableWorkflow| Box::pin(reviewing_state_required_reusable(e))),
        ],
    );

    let mut cross_entity: std::collections::HashMap<
        &'static str,
        Vec<AnyCrossEntityRule<ReusableWorkflow>>,
    > = std::collections::HashMap::new();
    cross_entity.insert(
        "steps",
        vec![
            crate::ref_check_rule!(TrackedReusableWorkflow, steps),
            Box::new(|e: &TrackedReusableWorkflow| {
                let steps = e.steps.get().cloned().unwrap_or_default();
                Box::pin(async move { no_relay_in_tree(steps).await })
            }),
        ],
    );
    cross_entity.insert(
        "raci",
        vec![crate::ref_check_rule!(TrackedReusableWorkflow, raci)],
    );
    cross_entity.insert(
        "intercepts",
        vec![
            Box::new(|e: &TrackedReusableWorkflow| {
                let map = e
                    .intercepts
                    .get()
                    .and_then(|v| v.as_ref())
                    .cloned()
                    .unwrap_or_default();
                Box::pin(async move { intercept_hooks_exist(map, "intercepts").await })
            }),
            Box::new(|e: &TrackedReusableWorkflow| {
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
        semantic,
        cross_entity,
    }
}

pub fn embedded_workflow_validation_schema() -> ValidationSchema<EmbeddedWorkflow> {
    let mut structural: std::collections::HashMap<
        &'static str,
        Vec<AnyStructuralRule<EmbeddedWorkflow>>,
    > = std::collections::HashMap::new();
    structural.insert(
        "entity_ref",
        vec![Box::new(|e: &TrackedEmbeddedWorkflow| {
            pascal_case_id(&e.entity_ref)
        })],
    );
    structural.insert(
        "name",
        vec![Box::new(|e: &TrackedEmbeddedWorkflow| {
            e.name.get().map(|v| non_empty_str(v)).unwrap_or_default()
        })],
    );
    structural.insert(
        "description",
        vec![Box::new(|e: &TrackedEmbeddedWorkflow| {
            e.description
                .get()
                .map(|v| opt_non_empty_str(v))
                .unwrap_or_default()
        })],
    );
    structural.insert(
        "purpose",
        vec![Box::new(|e: &TrackedEmbeddedWorkflow| {
            e.purpose
                .get()
                .map(|v| non_empty_str(v))
                .unwrap_or_default()
        })],
    );
    structural.insert(
        "raci",
        vec![Box::new(|e: &TrackedEmbeddedWorkflow| {
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
        "states",
        vec![Box::new(|e: &TrackedEmbeddedWorkflow| {
            e.states
                .get()
                .map(|v| states_valid_workflow(v.as_slice()))
                .unwrap_or_default()
        })],
    );
    structural.insert(
        "steps",
        vec![Box::new(|e: &TrackedEmbeddedWorkflow| {
            e.steps
                .get()
                .map(|v| step_keys_pascal_case(v))
                .unwrap_or_default()
        })],
    );
    structural.insert(
        "guidance",
        vec![Box::new(|e: &TrackedEmbeddedWorkflow| {
            e.guidance
                .get()
                .map(|v| opt_non_empty_str(v))
                .unwrap_or_default()
        })],
    );
    structural.insert(
        "extensions",
        vec![Box::new(|e: &TrackedEmbeddedWorkflow| {
            e.extensions
                .get()
                .map(|v| x_prefix_keys(v))
                .unwrap_or_default()
        })],
    );

    let mut semantic: std::collections::HashMap<
        &'static str,
        Vec<AnySemanticRule<EmbeddedWorkflow>>,
    > = std::collections::HashMap::new();
    semantic.insert(
        "steps",
        vec![
            Box::new(|e: &TrackedEmbeddedWorkflow| Box::pin(depends_on_valid_embedded(e))),
            Box::new(|e: &TrackedEmbeddedWorkflow| Box::pin(on_reject_valid_embedded(e))),
            Box::new(|e: &TrackedEmbeddedWorkflow| Box::pin(reviewing_state_required_embedded(e))),
        ],
    );

    let mut cross_entity: std::collections::HashMap<
        &'static str,
        Vec<AnyCrossEntityRule<EmbeddedWorkflow>>,
    > = std::collections::HashMap::new();
    cross_entity.insert(
        "entity_ref",
        vec![Box::new(|e: &TrackedEmbeddedWorkflow| {
            let parent = e.entity_ref.parent().cloned();
            Box::pin(async move {
                match parent {
                    Some(p) => parent_exists(p).await,
                    None => vec![],
                }
            })
        })],
    );
    cross_entity.insert(
        "steps",
        vec![crate::ref_check_rule!(TrackedEmbeddedWorkflow, steps)],
    );
    cross_entity.insert(
        "raci",
        vec![crate::ref_check_rule!(TrackedEmbeddedWorkflow, raci)],
    );
    cross_entity.insert(
        "intercepts",
        vec![
            Box::new(|e: &TrackedEmbeddedWorkflow| {
                let map = e
                    .intercepts
                    .get()
                    .and_then(|v| v.as_ref())
                    .cloned()
                    .unwrap_or_default();
                Box::pin(async move { intercept_hooks_exist(map, "intercepts").await })
            }),
            Box::new(|e: &TrackedEmbeddedWorkflow| {
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
        semantic,
        cross_entity,
    }
}
