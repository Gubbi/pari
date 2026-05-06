use std::collections::HashSet;

use crate::{
    entity::{
        entities::workflow::{EmbeddedWorkflow, Step},
        AnyEntityRef, Entity,
    },
    error::primitive::PrimitiveError,
    workspace::Workspace,
};

/// BFS over all work steps in the `ReusableWorkflow`'s step tree (including steps
/// inside nested `EmbeddedWorkflow` entities). Returns an error if any `Relay`
/// step is found anywhere in the tree.
///
/// A single error is reported at the `steps` field regardless of depth.
pub async fn no_relay_in_tree(
    workspace: &Workspace,
    steps: indexmap::IndexMap<String, Step>,
) -> Vec<PrimitiveError> {
    let mut visited: HashSet<String> = HashSet::new();
    if search_for_relay(workspace, steps, &mut visited).await {
        vec![PrimitiveError::workflow_graph_inconsistency(
            "reusable workflow must not contain Relay steps (directly or via EmbeddedWorkflow)",
            "relay_in_tree",
        )]
    } else {
        vec![]
    }
}

fn search_for_relay<'a>(
    workspace: &'a Workspace,
    steps: indexmap::IndexMap<String, Step>,
    visited: &'a mut HashSet<String>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + 'a>> {
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
                    let tracked = match workspace.resolve_any(any_ref).await {
                        Ok(t) => t,
                        Err(_) => continue,
                    };
                    if let Some(embedded) = EmbeddedWorkflow::extract(&tracked) {
                        if let Some(nested_steps) = embedded.steps.get() {
                            if search_for_relay(workspace, nested_steps.clone(), visited).await {
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
