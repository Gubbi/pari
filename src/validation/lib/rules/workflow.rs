use super::{
    super::schema::{AnyCrossEntityRule, AnySemanticRule, AnyStructuralRule, ValidationSchema},
    semantic::workflow::{
        depends_on_valid, depends_on_valid_reusable, on_reject_valid, on_reject_valid_embedded,
        on_reject_valid_reusable, reviewing_state_required, reviewing_state_required_embedded,
        reviewing_state_required_reusable,
    },
    structural::{
        primitives::{camel_case_id, non_empty_str, opt_non_empty_str, x_prefix_keys},
        raci::raci_structural,
        workflow::states_valid_workflow,
    },
};
use crate::entity::entities::workflow::{
    EmbeddedWorkflow, ReusableWorkflow, TrackedEmbeddedWorkflow, TrackedReusableWorkflow,
    TrackedWorkflow, Workflow,
};

macro_rules! common_structural {
    ($E:ty, $Tracked:ty) => {{
        let mut structural: std::collections::HashMap<&'static str, Vec<AnyStructuralRule<$E>>> =
            std::collections::HashMap::new();

        structural.insert(
            "entity_ref",
            vec![Box::new(|e: &$Tracked| camel_case_id(&e.entity_ref))],
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
        vec![crate::ref_check_rule!(TrackedReusableWorkflow, steps)],
    );
    cross_entity.insert(
        "raci",
        vec![crate::ref_check_rule!(TrackedReusableWorkflow, raci)],
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
            camel_case_id(&e.entity_ref)
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
            Box::new(|e: &TrackedEmbeddedWorkflow| Box::pin(on_reject_valid_embedded(e))),
            Box::new(|e: &TrackedEmbeddedWorkflow| Box::pin(reviewing_state_required_embedded(e))),
        ],
    );

    let mut cross_entity: std::collections::HashMap<
        &'static str,
        Vec<AnyCrossEntityRule<EmbeddedWorkflow>>,
    > = std::collections::HashMap::new();
    cross_entity.insert(
        "steps",
        vec![crate::ref_check_rule!(TrackedEmbeddedWorkflow, steps)],
    );
    cross_entity.insert(
        "raci",
        vec![crate::ref_check_rule!(TrackedEmbeddedWorkflow, raci)],
    );

    ValidationSchema {
        structural,
        semantic,
        cross_entity,
    }
}
