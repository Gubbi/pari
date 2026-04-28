//! Canonical [`Hook`] sample data for tests.

use pari::{
    entities::hook::{Hook, HookInput, TrackedHook},
    entity::{EntityRef, TrackedEntity},
};

/// Bare hook with required fields populated; no declared inputs.
pub fn a_minimal_hook(id: &str) -> TrackedEntity {
    hook(id, vec![])
}

/// Hook declaring a single required input named `input_name`.
pub fn a_hook_with_required_input(id: &str, input_name: &str) -> TrackedEntity {
    hook(
        id,
        vec![HookInput {
            name: input_name.to_string(),
            description: Some("A required input.".to_string()),
            required: true,
        }],
    )
}

fn hook(id: &str, inputs: Vec<HookInput>) -> TrackedEntity {
    let inputs = if inputs.is_empty() {
        None
    } else {
        Some(inputs)
    };
    TrackedEntity::from_hook(TrackedHook::from(Hook {
        entity_ref: EntityRef::new(id),
        name: "Notify Slack".to_string(),
        description: Some("Send a notification.".to_string()),
        instructions: vec!["Send a message to the #ops channel.".to_string()],
        inputs,
        extensions: Default::default(),
    }))
}
