use std::collections::{HashMap, HashSet};

/// Minimal info about a Hook's inputs, stored in RepoContext for invocation validation.
pub struct HookInputInfo {
    pub name: String,
    pub required: bool,
}

/// Hook definition as known to the repo context.
pub struct HookDefinition {
    pub inputs: Vec<HookInputInfo>,
}

/// Shared workflow info stored in repo context.
pub struct SharedWorkflowInfo {
    pub state_ids: Vec<String>,
}

/// Holds all validated entities known to the repository. Used for cross-entity
/// referential integrity checks. Contains only already-validated, cycle-free data.
/// The incoming entity being validated is never present in RepoContext.
pub struct RepoContext {
    pub role_ids: HashSet<String>,
    pub hook_ids: HashSet<String>,
    pub team_ids: HashSet<String>,
    /// Direct include/import references for each known team (used for cycle detection).
    pub team_direct_refs: HashMap<String, HashSet<String>>,
    /// Shared workflows (keyed by id) and their state names.
    pub shared_workflows: HashMap<String, SharedWorkflowInfo>,
    /// Hook definitions for input validation at invocation sites.
    pub hook_definitions: HashMap<String, HookDefinition>,
}

impl RepoContext {
    pub fn new() -> Self {
        RepoContext {
            role_ids: HashSet::new(),
            hook_ids: HashSet::new(),
            team_ids: HashSet::new(),
            team_direct_refs: HashMap::new(),
            shared_workflows: HashMap::new(),
            hook_definitions: HashMap::new(),
        }
    }

    pub fn has_role(&self, id: &str) -> bool {
        self.role_ids.contains(id)
    }

    pub fn has_hook(&self, id: &str) -> bool {
        self.hook_ids.contains(id)
    }

    pub fn has_team(&self, id: &str) -> bool {
        self.team_ids.contains(id)
    }

    pub fn has_shared_workflow(&self, id: &str) -> bool {
        self.shared_workflows.contains_key(id)
    }

    pub fn get_shared_workflow_states(&self, id: &str) -> Option<&[String]> {
        self.shared_workflows.get(id).map(|w| w.state_ids.as_slice())
    }

    pub fn get_team_refs(&self, team_id: &str) -> Option<&HashSet<String>> {
        self.team_direct_refs.get(team_id)
    }

    pub fn get_hook_definition(&self, hook_id: &str) -> Option<&HookDefinition> {
        self.hook_definitions.get(hook_id)
    }
}

impl Default for RepoContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx() -> RepoContext {
        let mut ctx = RepoContext::new();
        ctx.role_ids.insert("eng-lead".to_string());
        ctx.role_ids.insert("pm".to_string());
        ctx.hook_ids.insert("UpdateJira".to_string());
        ctx.team_ids.insert("platform-team".to_string());
        ctx.team_ids.insert("backend-team".to_string());

        let mut refs = HashSet::new();
        refs.insert("backend-team".to_string());
        ctx.team_direct_refs.insert("platform-team".to_string(), refs);

        ctx.shared_workflows.insert(
            "LegalReview".to_string(),
            SharedWorkflowInfo {
                state_ids: vec!["Active".to_string(), "Done".to_string()],
            },
        );

        ctx.hook_definitions.insert(
            "UpdateJira".to_string(),
            HookDefinition {
                inputs: vec![
                    HookInputInfo {
                        name: "status".to_string(),
                        required: true,
                    },
                    HookInputInfo {
                        name: "comment".to_string(),
                        required: false,
                    },
                ],
            },
        );

        ctx
    }

    // --- 9.1: RepoContext construction and lookup tests ---

    #[test]
    fn has_role_returns_true_for_known_role() {
        let ctx = make_ctx();
        assert!(ctx.has_role("eng-lead"));
        assert!(ctx.has_role("pm"));
    }

    #[test]
    fn has_role_returns_false_for_unknown_role() {
        let ctx = make_ctx();
        assert!(!ctx.has_role("unknown-role"));
    }

    #[test]
    fn has_hook_returns_true_for_known_hook() {
        let ctx = make_ctx();
        assert!(ctx.has_hook("UpdateJira"));
    }

    #[test]
    fn has_hook_returns_false_for_unknown_hook() {
        let ctx = make_ctx();
        assert!(!ctx.has_hook("UnknownHook"));
    }

    #[test]
    fn has_team_returns_true_for_known_team() {
        let ctx = make_ctx();
        assert!(ctx.has_team("platform-team"));
    }

    #[test]
    fn has_team_returns_false_for_unknown_team() {
        let ctx = make_ctx();
        assert!(!ctx.has_team("unknown-team"));
    }

    #[test]
    fn has_shared_workflow_returns_true_for_known() {
        let ctx = make_ctx();
        assert!(ctx.has_shared_workflow("LegalReview"));
    }

    #[test]
    fn has_shared_workflow_returns_false_for_unknown() {
        let ctx = make_ctx();
        assert!(!ctx.has_shared_workflow("Unknown"));
    }

    #[test]
    fn get_shared_workflow_states_returns_state_names() {
        let ctx = make_ctx();
        let states = ctx.get_shared_workflow_states("LegalReview").unwrap();
        assert_eq!(states, &["Active", "Done"]);
    }

    #[test]
    fn get_shared_workflow_states_returns_none_for_unknown() {
        let ctx = make_ctx();
        assert!(ctx.get_shared_workflow_states("Unknown").is_none());
    }

    #[test]
    fn get_team_refs_returns_direct_references() {
        let ctx = make_ctx();
        let refs = ctx.get_team_refs("platform-team").unwrap();
        assert!(refs.contains("backend-team"));
    }

    #[test]
    fn get_team_refs_returns_none_for_unknown_team() {
        let ctx = make_ctx();
        assert!(ctx.get_team_refs("unknown-team").is_none());
    }

    #[test]
    fn get_hook_definition_returns_definition() {
        let ctx = make_ctx();
        let def = ctx.get_hook_definition("UpdateJira").unwrap();
        assert_eq!(def.inputs.len(), 2);
        assert_eq!(def.inputs[0].name, "status");
        assert!(def.inputs[0].required);
        assert_eq!(def.inputs[1].name, "comment");
        assert!(!def.inputs[1].required);
    }

    #[test]
    fn get_hook_definition_returns_none_for_unknown() {
        let ctx = make_ctx();
        assert!(ctx.get_hook_definition("Unknown").is_none());
    }
}
