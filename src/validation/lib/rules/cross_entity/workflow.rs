use std::collections::HashSet;

use crate::{
    entity::{
        entities::workflow::{EmbeddedWorkflow, Step},
        AnyEntityRef, Entity,
    },
    error::primitive::PrimitiveError,
    workspace::EntityClient,
};

/// BFS over all work steps in the `ReusableWorkflow`'s step tree (including steps
/// inside nested `EmbeddedWorkflow` entities). Returns an error if any `Relay`
/// step is found anywhere in the tree.
///
/// A single error is reported at the `steps` field regardless of depth.
pub async fn no_relay_in_tree(steps: indexmap::IndexMap<String, Step>) -> Vec<PrimitiveError> {
    let mut visited: HashSet<String> = HashSet::new();
    if search_for_relay(steps, &mut visited).await {
        vec![PrimitiveError::workflow_graph_inconsistency(
            "reusable workflow must not contain Relay steps (directly or via EmbeddedWorkflow)",
            "relay_in_tree",
        )]
    } else {
        vec![]
    }
}

fn search_for_relay(
    steps: indexmap::IndexMap<String, Step>,
    visited: &mut HashSet<String>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + '_>> {
    Box::pin(async move {
        for step in steps.into_values() {
            match step {
                Step::Relay { .. } => return true,
                Step::EmbeddedWorkflow { entity_ref, .. } => {
                    let key = entity_ref.id().to_owned();
                    if !visited.insert(key) {
                        continue; // already visited — skip to avoid cycles
                    }
                    let any_ref: AnyEntityRef = AnyEntityRef::EmbeddedWorkflow(entity_ref);
                    let tracked = match EntityClient::resolve(any_ref).await {
                        Ok(t) => t,
                        Err(_) => continue,
                    };
                    if let Some(embedded) = EmbeddedWorkflow::extract(&tracked) {
                        if let Some(nested_steps) = embedded.steps.get() {
                            if search_for_relay(nested_steps.clone(), visited).await {
                                return true;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        false
    })
}
