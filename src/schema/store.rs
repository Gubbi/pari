//! [`EntityStore`] — in-memory collection of all loaded entities.
//!
//! Serves as the validation context and as the input to [`crate::substrate::Substrate::persist`].

use std::collections::HashMap;

use crate::schema::entities::{
    hook::Hook,
    role::Role,
    team::Team,
    workflow::{SharedWorkflow, Workflow},
};

/// Unified collection of all validated entities, keyed by id for O(1) lookup.
///
/// Serves dual purpose:
/// - **Validation context**: passed to all `validate()` calls for cross-entity checks.
/// - **Persistence input**: passed to `persist()` for writing entities to a substrate.
///
/// Invariant: the incoming entity being validated MUST NOT be present in the store.
/// Callers are responsible for maintaining this guarantee.
pub struct EntityStore {
    pub roles: HashMap<String, Role>,
    pub hooks: HashMap<String, Hook>,
    pub teams: HashMap<String, Team>,
    pub shared_workflows: HashMap<String, SharedWorkflow>,
    pub workflows: HashMap<String, Workflow>,
}

impl EntityStore {
    pub fn new() -> Self {
        EntityStore {
            roles: HashMap::new(),
            hooks: HashMap::new(),
            teams: HashMap::new(),
            shared_workflows: HashMap::new(),
            workflows: HashMap::new(),
        }
    }

    pub fn has_role(&self, id: &str) -> bool {
        self.roles.contains_key(id)
    }

    pub fn has_hook(&self, id: &str) -> bool {
        self.hooks.contains_key(id)
    }

    pub fn has_team(&self, id: &str) -> bool {
        self.teams.contains_key(id)
    }

    pub fn has_shared_workflow(&self, id: &str) -> bool {
        self.shared_workflows.contains_key(id)
    }

    pub fn get_hook(&self, id: &str) -> Option<&Hook> {
        self.hooks.get(id)
    }

    pub fn get_team(&self, id: &str) -> Option<&Team> {
        self.teams.get(id)
    }

    pub fn get_shared_workflow_states(&self, id: &str) -> Option<Vec<String>> {
        self.shared_workflows
            .get(id)
            .map(|sw| sw.states.iter().map(|s| s.id.clone()).collect())
    }
}

impl Default for EntityStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{
        entities::{
            hook::HookInput,
            workflow::{ReviewStep, SharedStep, WorkflowDef},
        },
        types::{Extensions, Raci, WorkflowSemantic, WorkflowStateEntry},
    };

    fn minimal_role(id: &str) -> Role {
        Role {
            id: id.into(),
            name: id.to_string(),
            purpose: "test".to_string(),
            traits: None,
            extensions: Extensions::default(),
        }
    }

    fn minimal_hook(id: &str) -> Hook {
        Hook {
            id: id.into(),
            name: id.to_string(),
            description: "test".to_string(),
            instructions: vec!["do it".to_string()],
            inputs: Some(vec![
                HookInput {
                    name: "status".to_string(),
                    description: "desc".to_string(),
                    required: true,
                },
                HookInput {
                    name: "comment".to_string(),
                    description: "desc".to_string(),
                    required: false,
                },
            ]),
            extensions: Extensions::default(),
        }
    }

    fn minimal_team(id: &str) -> Team {
        Team {
            id: id.into(),
            name: id.to_string(),
            description: None,
            members: None,
            include: None,
            import: None,
            extensions: Extensions::default(),
        }
    }

    fn minimal_shared_workflow(id: &str, state_ids: &[&str]) -> SharedWorkflow {
        let states = state_ids
            .iter()
            .enumerate()
            .map(|(i, sid)| WorkflowStateEntry {
                id: sid.to_string(),
                description: "desc".to_string(),
                semantic: if i == state_ids.len() - 1 {
                    Some(WorkflowSemantic::Complete)
                } else {
                    None
                },
            })
            .collect();
        WorkflowDef {
            id: id.into(),
            name: id.to_string(),
            description: None,
            purpose: "test".to_string(),
            accountability: Raci {
                responsible: "eng-lead".to_string(),
                accountable: "pm".to_string(),
                consulted: vec![],
                informed: vec![],
            },
            steps: vec![SharedStep::Review(ReviewStep {
                id: "Approve".to_string(),
                approver: "eng-lead".to_string(),
                on_reject: "Approve".to_string(),
            })],
            states,
            hooks: None,
            guidance: None,
            extensions: Extensions::default(),
        }
    }

    // --- construction ---

    #[test]
    fn new_creates_empty_store() {
        let store = EntityStore::new();
        assert!(store.roles.is_empty());
        assert!(store.hooks.is_empty());
        assert!(store.teams.is_empty());
        assert!(store.shared_workflows.is_empty());
        assert!(store.workflows.is_empty());
    }

    // --- has_role ---

    #[test]
    fn has_role_returns_true_for_known_role() {
        let mut store = EntityStore::new();
        store
            .roles
            .insert("eng-lead".to_string(), minimal_role("eng-lead"));
        assert!(store.has_role("eng-lead"));
    }

    #[test]
    fn has_role_returns_false_for_unknown_role() {
        let store = EntityStore::new();
        assert!(!store.has_role("unknown"));
    }

    // --- has_hook ---

    #[test]
    fn has_hook_returns_true_for_known_hook() {
        let mut store = EntityStore::new();
        store
            .hooks
            .insert("UpdateJira".to_string(), minimal_hook("UpdateJira"));
        assert!(store.has_hook("UpdateJira"));
    }

    #[test]
    fn has_hook_returns_false_for_unknown_hook() {
        let store = EntityStore::new();
        assert!(!store.has_hook("Unknown"));
    }

    // --- has_team ---

    #[test]
    fn has_team_returns_true_for_known_team() {
        let mut store = EntityStore::new();
        store
            .teams
            .insert("platform-team".to_string(), minimal_team("platform-team"));
        assert!(store.has_team("platform-team"));
    }

    #[test]
    fn has_team_returns_false_for_unknown_team() {
        let store = EntityStore::new();
        assert!(!store.has_team("unknown-team"));
    }

    // --- has_shared_workflow ---

    #[test]
    fn has_shared_workflow_returns_true_for_known() {
        let mut store = EntityStore::new();
        store.shared_workflows.insert(
            "LegalReview".to_string(),
            minimal_shared_workflow("LegalReview", &["Active", "Done"]),
        );
        assert!(store.has_shared_workflow("LegalReview"));
    }

    #[test]
    fn has_shared_workflow_returns_false_for_unknown() {
        let store = EntityStore::new();
        assert!(!store.has_shared_workflow("Unknown"));
    }

    // --- get_hook ---

    #[test]
    fn get_hook_returns_full_hook_entity() {
        let mut store = EntityStore::new();
        store
            .hooks
            .insert("UpdateJira".to_string(), minimal_hook("UpdateJira"));
        let hook = store.get_hook("UpdateJira").unwrap();
        assert_eq!(hook.id, "UpdateJira");
        let inputs = hook.inputs.as_ref().unwrap();
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0].name, "status");
        assert!(inputs[0].required);
        assert_eq!(inputs[1].name, "comment");
        assert!(!inputs[1].required);
    }

    #[test]
    fn get_hook_returns_none_for_unknown() {
        let store = EntityStore::new();
        assert!(store.get_hook("Unknown").is_none());
    }

    // --- get_team ---

    #[test]
    fn get_team_returns_full_team_entity() {
        let mut store = EntityStore::new();
        store
            .teams
            .insert("platform-team".to_string(), minimal_team("platform-team"));
        let team = store.get_team("platform-team").unwrap();
        assert_eq!(team.id, "platform-team");
    }

    #[test]
    fn get_team_returns_none_for_unknown() {
        let store = EntityStore::new();
        assert!(store.get_team("unknown-team").is_none());
    }

    // --- get_shared_workflow_states ---

    #[test]
    fn get_shared_workflow_states_returns_state_ids_in_order() {
        let mut store = EntityStore::new();
        store.shared_workflows.insert(
            "LegalReview".to_string(),
            minimal_shared_workflow("LegalReview", &["Active", "Done"]),
        );
        let states = store.get_shared_workflow_states("LegalReview").unwrap();
        assert_eq!(states, vec!["Active".to_string(), "Done".to_string()]);
    }

    #[test]
    fn get_shared_workflow_states_returns_none_for_unknown() {
        let store = EntityStore::new();
        assert!(store.get_shared_workflow_states("Unknown").is_none());
    }
}
