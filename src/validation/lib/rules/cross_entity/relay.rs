use crate::{
    entity::{entities::workflow::ReusableWorkflow, AnyEntityRef, Entity},
    error::primitive::PrimitiveError,
    workspace::Workspace,
};

/// For each `StateMapEntry` in `state_map`, checks that `maps_to` is a valid state id
/// declared in the `delegates_to` `ReusableWorkflow`'s `states` list.
///
/// If `delegates_to` cannot be resolved (e.g. it doesn't exist), this rule silently
/// returns no errors — existence is already checked by `delegates_to_exists`.
pub async fn maps_to_states_exist(
    workspace: &Workspace,
    delegates_to_id: &str,
    state_map: std::collections::HashMap<String, crate::entity::entities::relay::StateMapEntry>,
) -> Vec<PrimitiveError> {
    let any_ref = AnyEntityRef::ReusableWorkflow(crate::entity::EntityRef::new(delegates_to_id));
    let tracked = match workspace.resolve_any(any_ref).await {
        Ok(t) => t,
        Err(_) => return vec![], // delegates_to missing — caught by delegates_to_exists
    };
    let tracked_wf = match ReusableWorkflow::extract(&tracked) {
        Some(wf) => wf,
        None => return vec![],
    };
    let states = match tracked_wf.states.get() {
        Some(s) => s.clone(),
        None => return vec![], // states not loaded — skip
    };
    let valid_state_ids: std::collections::HashSet<&str> =
        states.iter().map(|s| s.id.as_str()).collect();

    let mut errors = vec![];
    for (key, entry) in &state_map {
        if !valid_state_ids.contains(entry.maps_to.as_str()) {
            errors.push(PrimitiveError::referenced_entity_absent(
                format!(
                    "state '{}' does not exist in delegates_to workflow",
                    entry.maps_to
                ),
                format!("state_map.{key}.maps_to"),
                entry.maps_to.clone(),
            ));
        }
    }
    errors
}
