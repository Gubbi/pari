//! [`EntityStore`] — in-memory collection of all loaded entities.
//!
//! Serves as the validation context and as the persistence input (via
//! [`collect_changes`](EntityStore::collect_changes)).

use crate::{
    schema::entities::{
        hook::{Hook, TrackedHook},
        role::{Role, TrackedRole},
        team::{Team, TrackedTeam},
        workflow::{
            SharedWorkflow, TrackedSharedWorkflow, TrackedSharedWorkStepDefinition,
            TrackedStep, TrackedWorkStepDefinition, TrackedWorkflow, Workflow,
        },
    },
    substrate::changeset::{ChangeOp, ChangeSet, EntityChange, EntityData, EntityKind},
    tracked::TrackedMap,
};

/// Unified collection of all validated entities, keyed by id for O(1) lookup.
///
/// Serves dual purpose:
/// - **Validation context**: passed to all `validate()` calls for cross-entity checks.
/// - **Persistence input**: exposes [`collect_changes`](EntityStore::collect_changes) for
///   producing a [`ChangeSet`](crate::substrate::changeset::ChangeSet).
///
/// Invariant: the incoming entity being validated MUST NOT be present in the store.
/// Callers are responsible for maintaining this guarantee.
pub struct EntityStore {
    pub(crate) roles: TrackedMap<String, TrackedRole>,
    pub(crate) hooks: TrackedMap<String, TrackedHook>,
    pub(crate) teams: TrackedMap<String, TrackedTeam>,
    pub(crate) shared_workflows: TrackedMap<String, TrackedSharedWorkflow>,
    pub(crate) workflows: TrackedMap<String, TrackedWorkflow>,
}

impl EntityStore {
    pub fn new() -> Self {
        EntityStore {
            roles: TrackedMap::new(),
            hooks: TrackedMap::new(),
            teams: TrackedMap::new(),
            shared_workflows: TrackedMap::new(),
            workflows: TrackedMap::new(),
        }
    }

    // --- Typed insertion methods ---

    pub fn insert_role(&mut self, role: Role) {
        let id = role.id.to_string();
        self.roles.insert(id, TrackedRole::from(role));
    }

    pub fn insert_hook(&mut self, hook: Hook) {
        let id = hook.id.to_string();
        self.hooks.insert(id, TrackedHook::from(hook));
    }

    pub fn insert_team(&mut self, team: Team) {
        let id = team.id.to_string();
        self.teams.insert(id, TrackedTeam::from(team));
    }

    pub fn insert_shared_workflow(&mut self, wf: SharedWorkflow) {
        let id = wf.id.to_string();
        self.shared_workflows.insert(id, TrackedSharedWorkflow::from(wf));
    }

    pub fn insert_workflow(&mut self, wf: Workflow) {
        let id = wf.id.to_string();
        self.workflows.insert(id, TrackedWorkflow::from(wf));
    }

    // --- Lookup methods ---

    pub fn has_role(&self, id: &str) -> bool {
        self.roles.get(&id.to_string()).is_some()
    }

    pub fn has_hook(&self, id: &str) -> bool {
        self.hooks.get(&id.to_string()).is_some()
    }

    pub fn has_team(&self, id: &str) -> bool {
        self.teams.get(&id.to_string()).is_some()
    }

    pub fn has_shared_workflow(&self, id: &str) -> bool {
        self.shared_workflows.get(&id.to_string()).is_some()
    }

    pub fn get_hook(&self, id: &str) -> Option<&TrackedHook> {
        self.hooks.get(&id.to_string())
    }

    pub fn get_team(&self, id: &str) -> Option<&TrackedTeam> {
        self.teams.get(&id.to_string())
    }

    pub fn get_shared_workflow_states(&self, id: &str) -> Option<Vec<String>> {
        self.shared_workflows
            .get(&id.to_string())
            .map(|sw| sw.states.iter().map(|s| s.id.clone()).collect())
    }

    // --- Mutable access methods ---

    pub fn get_role_mut(&mut self, id: &str) -> Option<&mut TrackedRole> {
        self.roles.get_mut(&id.to_string())
    }

    pub fn get_hook_mut(&mut self, id: &str) -> Option<&mut TrackedHook> {
        self.hooks.get_mut(&id.to_string())
    }

    pub fn get_team_mut(&mut self, id: &str) -> Option<&mut TrackedTeam> {
        self.teams.get_mut(&id.to_string())
    }

    pub fn get_shared_workflow_mut(&mut self, id: &str) -> Option<&mut TrackedSharedWorkflow> {
        self.shared_workflows.get_mut(&id.to_string())
    }

    pub fn get_workflow_mut(&mut self, id: &str) -> Option<&mut TrackedWorkflow> {
        self.workflows.get_mut(&id.to_string())
    }

    // --- Removal methods ---

    pub fn remove_role(&mut self, id: &str) {
        self.roles.remove(&id.to_string());
    }

    pub fn remove_hook(&mut self, id: &str) {
        self.hooks.remove(&id.to_string());
    }

    pub fn remove_team(&mut self, id: &str) {
        self.teams.remove(&id.to_string());
    }

    pub fn remove_shared_workflow(&mut self, id: &str) {
        self.shared_workflows.remove(&id.to_string());
    }

    pub fn remove_workflow(&mut self, id: &str) {
        self.workflows.remove(&id.to_string());
    }

    // --- Change tracking ---

    /// Collect all pending changes into a [`ChangeSet`] without resetting
    /// dirty state.  May be called multiple times; results are stable until
    /// [`reset_tracked`](EntityStore::reset_tracked) is called.
    pub fn collect_changes(&self) -> ChangeSet<'_> {
        let mut cs = ChangeSet::new();

        // Roles
        for id in self.roles.inserted.keys() {
            let e = self.roles.get(id).unwrap();
            cs.changes.push(EntityChange {
                path: "roles".into(),
                kind: EntityKind::Role,
                id: id.clone(),
                op: ChangeOp::Added(EntityData::Role(e)),
            });
        }
        for id in self.roles.modified.keys() {
            let e = self.roles.get(id).unwrap();
            cs.changes.push(EntityChange {
                path: "roles".into(),
                kind: EntityKind::Role,
                id: id.clone(),
                op: ChangeOp::Modified {
                    entity: EntityData::Role(e),
                    dirty_fields: e.dirty_fields().iter().map(|s| s.to_string()).collect(),
                },
            });
        }
        for id in self.roles.removed.keys() {
            cs.changes.push(EntityChange {
                path: "roles".into(),
                kind: EntityKind::Role,
                id: id.clone(),
                op: ChangeOp::Removed,
            });
        }

        // Hooks
        for id in self.hooks.inserted.keys() {
            let e = self.hooks.get(id).unwrap();
            cs.changes.push(EntityChange {
                path: "shared/hooks".into(),
                kind: EntityKind::Hook,
                id: id.clone(),
                op: ChangeOp::Added(EntityData::Hook(e)),
            });
        }
        for id in self.hooks.modified.keys() {
            let e = self.hooks.get(id).unwrap();
            cs.changes.push(EntityChange {
                path: "shared/hooks".into(),
                kind: EntityKind::Hook,
                id: id.clone(),
                op: ChangeOp::Modified {
                    entity: EntityData::Hook(e),
                    dirty_fields: e.dirty_fields().iter().map(|s| s.to_string()).collect(),
                },
            });
        }
        for id in self.hooks.removed.keys() {
            cs.changes.push(EntityChange {
                path: "shared/hooks".into(),
                kind: EntityKind::Hook,
                id: id.clone(),
                op: ChangeOp::Removed,
            });
        }

        // Teams
        for id in self.teams.inserted.keys() {
            let e = self.teams.get(id).unwrap();
            cs.changes.push(EntityChange {
                path: "teams".into(),
                kind: EntityKind::Team,
                id: id.clone(),
                op: ChangeOp::Added(EntityData::Team(e)),
            });
        }
        for id in self.teams.modified.keys() {
            let e = self.teams.get(id).unwrap();
            cs.changes.push(EntityChange {
                path: "teams".into(),
                kind: EntityKind::Team,
                id: id.clone(),
                op: ChangeOp::Modified {
                    entity: EntityData::Team(e),
                    dirty_fields: e.dirty_fields().iter().map(|s| s.to_string()).collect(),
                },
            });
        }
        for id in self.teams.removed.keys() {
            cs.changes.push(EntityChange {
                path: "teams".into(),
                kind: EntityKind::Team,
                id: id.clone(),
                op: ChangeOp::Removed,
            });
        }

        // Workflows (with nested step entries)
        for id in self.workflows.inserted.keys() {
            let wf = self.workflows.get(id).unwrap();
            let path = format!("workflows/{id}");
            cs.changes.push(EntityChange {
                path: path.clone(),
                kind: EntityKind::Workflow,
                id: id.clone(),
                op: ChangeOp::Added(EntityData::Workflow(wf)),
            });
            collect_work_step_changes(&mut cs, wf, &path);
        }
        for id in self.workflows.modified.keys() {
            let wf = self.workflows.get(id).unwrap();
            let path = format!("workflows/{id}");
            cs.changes.push(EntityChange {
                path: path.clone(),
                kind: EntityKind::Workflow,
                id: id.clone(),
                op: ChangeOp::Modified {
                    entity: EntityData::Workflow(wf),
                    dirty_fields: wf.dirty_fields().iter().map(|s| s.to_string()).collect(),
                },
            });
            collect_work_step_changes(&mut cs, wf, &path);
        }
        for id in self.workflows.removed.keys() {
            cs.changes.push(EntityChange {
                path: format!("workflows/{id}"),
                kind: EntityKind::Workflow,
                id: id.clone(),
                op: ChangeOp::Removed,
            });
        }

        // SharedWorkflows (with nested step entries)
        for id in self.shared_workflows.inserted.keys() {
            let wf = self.shared_workflows.get(id).unwrap();
            let path = format!("shared/workflows/{id}");
            cs.changes.push(EntityChange {
                path: path.clone(),
                kind: EntityKind::SharedWorkflow,
                id: id.clone(),
                op: ChangeOp::Added(EntityData::SharedWorkflow(wf)),
            });
            collect_shared_step_changes(&mut cs, wf, &path);
        }
        for id in self.shared_workflows.modified.keys() {
            let wf = self.shared_workflows.get(id).unwrap();
            let path = format!("shared/workflows/{id}");
            cs.changes.push(EntityChange {
                path: path.clone(),
                kind: EntityKind::SharedWorkflow,
                id: id.clone(),
                op: ChangeOp::Modified {
                    entity: EntityData::SharedWorkflow(wf),
                    dirty_fields: wf.dirty_fields().iter().map(|s| s.to_string()).collect(),
                },
            });
            collect_shared_step_changes(&mut cs, wf, &path);
        }
        for id in self.shared_workflows.removed.keys() {
            cs.changes.push(EntityChange {
                path: format!("shared/workflows/{id}"),
                kind: EntityKind::SharedWorkflow,
                id: id.clone(),
                op: ChangeOp::Removed,
            });
        }

        cs
    }

    /// Reset all dirty state across all entity collections.
    /// Only call after a successful `atomic_persist()`.
    pub fn reset_tracked(&mut self) {
        reset_all_workflows(&mut self.workflows);
        reset_all_workflows(&mut self.shared_workflows);
        self.roles.reset_tracked();
        self.hooks.reset_tracked();
        self.teams.reset_tracked();
    }
}

/// Reset tracking state for all workflows in a TrackedMap, including their
/// nested steps TrackedMap.
fn reset_all_workflows<TS>(wf_map: &mut TrackedMap<String, crate::schema::entities::workflow::TrackedWorkflowDef<TS>>)
{
    for wf in wf_map.iter_mut() {
        wf.1.steps.reset_tracked();
    }
    wf_map.reset_tracked();
}

/// Walk inserted/modified/removed steps of a `TrackedWorkflow` and push
/// `EntityChange` entries into `cs`.  Recurses into nested inline Workflows.
fn collect_work_step_changes<'a>(
    cs: &mut ChangeSet<'a>,
    wf: &'a TrackedWorkflow,
    wf_path: &str,
) {
    for step_id in wf.steps.inserted.keys() {
        if let Some(TrackedStep::Work(ws)) = wf.steps.get(step_id) {
            let path = format!("{wf_path}/{step_id}");
            match &ws.definition {
                TrackedWorkStepDefinition::Task(t) => cs.changes.push(EntityChange {
                    path,
                    kind: EntityKind::Task,
                    id: step_id.clone(),
                    op: ChangeOp::Added(EntityData::Task(t)),
                }),
                TrackedWorkStepDefinition::Relay(r) => cs.changes.push(EntityChange {
                    path,
                    kind: EntityKind::Relay,
                    id: step_id.clone(),
                    op: ChangeOp::Added(EntityData::Relay(r)),
                }),
                TrackedWorkStepDefinition::Workflow(inner) => {
                    cs.changes.push(EntityChange {
                        path: path.clone(),
                        kind: EntityKind::Workflow,
                        id: step_id.clone(),
                        op: ChangeOp::Added(EntityData::Workflow(inner)),
                    });
                    collect_work_step_changes(cs, inner, &path);
                }
            }
        }
    }
    for step_id in wf.steps.modified.keys() {
        if let Some(step @ TrackedStep::Work(ws)) = wf.steps.get(step_id) {
            let path = format!("{wf_path}/{step_id}");
            let dirty = step.dirty_fields().iter().map(|s| s.to_string()).collect();
            match &ws.definition {
                TrackedWorkStepDefinition::Task(t) => cs.changes.push(EntityChange {
                    path,
                    kind: EntityKind::Task,
                    id: step_id.clone(),
                    op: ChangeOp::Modified { entity: EntityData::Task(t), dirty_fields: dirty },
                }),
                TrackedWorkStepDefinition::Relay(r) => cs.changes.push(EntityChange {
                    path,
                    kind: EntityKind::Relay,
                    id: step_id.clone(),
                    op: ChangeOp::Modified { entity: EntityData::Relay(r), dirty_fields: dirty },
                }),
                TrackedWorkStepDefinition::Workflow(inner) => {
                    cs.changes.push(EntityChange {
                        path: path.clone(),
                        kind: EntityKind::Workflow,
                        id: step_id.clone(),
                        op: ChangeOp::Modified {
                            entity: EntityData::Workflow(inner),
                            dirty_fields: dirty,
                        },
                    });
                    collect_work_step_changes(cs, inner, &path);
                }
            }
        }
    }
    for (step_id, step) in &wf.steps.removed {
        if let TrackedStep::Work(ws) = step {
            let kind = match &ws.definition {
                TrackedWorkStepDefinition::Task(_) => EntityKind::Task,
                TrackedWorkStepDefinition::Relay(_) => EntityKind::Relay,
                TrackedWorkStepDefinition::Workflow(_) => EntityKind::Workflow,
            };
            cs.changes.push(EntityChange {
                path: format!("{wf_path}/{step_id}"),
                kind,
                id: step_id.clone(),
                op: ChangeOp::Removed,
            });
        }
    }
}

/// Walk inserted/modified/removed steps of a `TrackedSharedWorkflow` and push
/// `EntityChange` entries into `cs`.  Recurses into nested inline SharedWorkflows.
fn collect_shared_step_changes<'a>(
    cs: &mut ChangeSet<'a>,
    wf: &'a TrackedSharedWorkflow,
    wf_path: &str,
) {
    for step_id in wf.steps.inserted.keys() {
        if let Some(TrackedStep::Work(ws)) = wf.steps.get(step_id) {
            let path = format!("{wf_path}/{step_id}");
            match &ws.definition {
                TrackedSharedWorkStepDefinition::Task(t) => cs.changes.push(EntityChange {
                    path,
                    kind: EntityKind::Task,
                    id: step_id.clone(),
                    op: ChangeOp::Added(EntityData::Task(t)),
                }),
                TrackedSharedWorkStepDefinition::SharedWorkflow(inner) => {
                    cs.changes.push(EntityChange {
                        path: path.clone(),
                        kind: EntityKind::SharedWorkflow,
                        id: step_id.clone(),
                        op: ChangeOp::Added(EntityData::SharedWorkflow(inner)),
                    });
                    collect_shared_step_changes(cs, inner, &path);
                }
            }
        }
    }
    for step_id in wf.steps.modified.keys() {
        if let Some(step @ TrackedStep::Work(ws)) = wf.steps.get(step_id) {
            let path = format!("{wf_path}/{step_id}");
            let dirty = step.dirty_fields().iter().map(|s| s.to_string()).collect();
            match &ws.definition {
                TrackedSharedWorkStepDefinition::Task(t) => cs.changes.push(EntityChange {
                    path,
                    kind: EntityKind::Task,
                    id: step_id.clone(),
                    op: ChangeOp::Modified { entity: EntityData::Task(t), dirty_fields: dirty },
                }),
                TrackedSharedWorkStepDefinition::SharedWorkflow(inner) => {
                    cs.changes.push(EntityChange {
                        path: path.clone(),
                        kind: EntityKind::SharedWorkflow,
                        id: step_id.clone(),
                        op: ChangeOp::Modified {
                            entity: EntityData::SharedWorkflow(inner),
                            dirty_fields: dirty,
                        },
                    });
                    collect_shared_step_changes(cs, inner, &path);
                }
            }
        }
    }
    for (step_id, step) in &wf.steps.removed {
        if let TrackedStep::Work(ws) = step {
            let kind = match &ws.definition {
                TrackedSharedWorkStepDefinition::Task(_) => EntityKind::Task,
                TrackedSharedWorkStepDefinition::SharedWorkflow(_) => EntityKind::SharedWorkflow,
            };
            cs.changes.push(EntityChange {
                path: format!("{wf_path}/{step_id}"),
                kind,
                id: step_id.clone(),
                op: ChangeOp::Removed,
            });
        }
    }
}

impl Default for EntityStore {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fixtures::{
            hook::{hook_with_inputs, minimal_hook},
            role::minimal_role,
            shared_workflow::minimal_shared_workflow,
            task::minimal_task,
            team::minimal_team,
            workflow::minimal_workflow,
        },
        substrate::changeset::{ChangeOp, EntityKind},
    };

    // --- 5.1: EntityStore insertion API tests ---

    #[test]
    fn new_creates_empty_store() {
        let store = EntityStore::new();
        assert!(store.roles.is_empty());
        assert!(store.hooks.is_empty());
        assert!(store.teams.is_empty());
        assert!(store.shared_workflows.is_empty());
        assert!(store.workflows.is_empty());
    }

    #[test]
    fn insert_role_adds_to_inserted_set() {
        let mut store = EntityStore::new();
        store.insert_role(minimal_role("eng-lead"));
        assert!(store.roles.inserted.contains_key("eng-lead"));
        assert!(store.has_role("eng-lead"));
    }

    #[test]
    fn insert_hook_adds_to_inserted_set() {
        let mut store = EntityStore::new();
        store.insert_hook(minimal_hook("UpdateJira"));
        assert!(store.hooks.inserted.contains_key("UpdateJira"));
    }

    #[test]
    fn insert_team_adds_to_inserted_set() {
        let mut store = EntityStore::new();
        store.insert_team(minimal_team("platform-team"));
        assert!(store.teams.inserted.contains_key("platform-team"));
    }

    #[test]
    fn insert_shared_workflow_adds_to_inserted_set() {
        let mut store = EntityStore::new();
        store.insert_shared_workflow(minimal_shared_workflow("LegalReview", &["Active", "Done"]));
        assert!(store.shared_workflows.inserted.contains_key("LegalReview"));
    }

    // --- 5.4: EntityStore read access tests ---

    #[test]
    fn has_role_returns_true_for_known_role() {
        let mut store = EntityStore::new();
        store.insert_role(minimal_role("eng-lead"));
        assert!(store.has_role("eng-lead"));
    }

    #[test]
    fn has_role_returns_false_for_unknown_role() {
        let store = EntityStore::new();
        assert!(!store.has_role("unknown"));
    }

    #[test]
    fn has_hook_returns_true_for_known_hook() {
        let mut store = EntityStore::new();
        store.insert_hook(minimal_hook("UpdateJira"));
        assert!(store.has_hook("UpdateJira"));
    }

    #[test]
    fn has_hook_returns_false_for_unknown_hook() {
        let store = EntityStore::new();
        assert!(!store.has_hook("Unknown"));
    }

    #[test]
    fn has_team_returns_true_for_known_team() {
        let mut store = EntityStore::new();
        store.insert_team(minimal_team("platform-team"));
        assert!(store.has_team("platform-team"));
    }

    #[test]
    fn has_shared_workflow_returns_true_for_known() {
        let mut store = EntityStore::new();
        store.insert_shared_workflow(minimal_shared_workflow("LegalReview", &["Active", "Done"]));
        assert!(store.has_shared_workflow("LegalReview"));
    }

    #[test]
    fn get_hook_returns_full_hook_entity_via_deref() {
        let mut store = EntityStore::new();
        store.insert_hook(hook_with_inputs("UpdateJira"));
        let hook = store.get_hook("UpdateJira").unwrap();
        assert_eq!(*hook.id, "UpdateJira");
        let inputs = hook.inputs.as_ref().unwrap();
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0].name, "status");
        assert!(inputs[0].required);
    }

    #[test]
    fn get_hook_returns_none_for_unknown() {
        let store = EntityStore::new();
        assert!(store.get_hook("Unknown").is_none());
    }

    #[test]
    fn get_team_returns_full_team_entity_via_deref() {
        let mut store = EntityStore::new();
        store.insert_team(minimal_team("platform-team"));
        let team = store.get_team("platform-team").unwrap();
        assert_eq!(*team.id, "platform-team");
    }

    #[test]
    fn get_shared_workflow_states_returns_state_ids_in_order() {
        let mut store = EntityStore::new();
        store.insert_shared_workflow(minimal_shared_workflow("LegalReview", &["Active", "Done"]));
        let states = store.get_shared_workflow_states("LegalReview").unwrap();
        assert_eq!(states, vec!["Active".to_string(), "Done".to_string()]);
    }

    #[test]
    fn get_shared_workflow_states_returns_none_for_unknown() {
        let store = EntityStore::new();
        assert!(store.get_shared_workflow_states("Unknown").is_none());
    }

    // --- 5.6: EntityStore mutable access tests ---

    #[test]
    fn get_role_mut_returns_tracked_reference_and_marks_modified() {
        let mut store = EntityStore::new();
        store.insert_role(minimal_role("eng-lead"));
        store.roles.reset_tracked(); // clear inserted
        let role = store.get_role_mut("eng-lead").unwrap();
        *role.name = "Engineering Lead".to_string();
        assert!(role.name.is_dirty());
        assert!(store.roles.modified.contains_key("eng-lead"));
    }

    #[test]
    fn remove_role_records_deletion_in_removed_set() {
        let mut store = EntityStore::new();
        store.insert_role(minimal_role("eng-lead"));
        store.roles.reset_tracked();
        store.remove_role("eng-lead");
        assert!(!store.has_role("eng-lead"));
        assert!(store.roles.removed.contains_key("eng-lead"));
    }

    #[test]
    fn reset_tracked_clears_all_tracking_state() {
        let mut store = EntityStore::new();
        store.insert_role(minimal_role("eng-lead"));
        store.insert_hook(minimal_hook("UpdateJira"));
        store.reset_tracked();
        assert!(!store.roles.has_changes());
        assert!(!store.hooks.has_changes());
    }

    // --- 6.3: collect_changes tests ---

    #[test]
    fn collect_changes_empty_store_returns_empty_changeset() {
        let store = EntityStore::new();
        let cs = store.collect_changes();
        assert!(cs.is_empty());
    }

    #[test]
    fn collect_changes_inserted_role_produces_added_entry() {
        let mut store = EntityStore::new();
        store.insert_role(minimal_role("eng-lead"));
        let cs = store.collect_changes();
        assert_eq!(cs.len(), 1);
        let change = &cs.changes[0];
        assert_eq!(change.path, "roles");
        assert_eq!(change.kind, EntityKind::Role);
        assert_eq!(change.id, "eng-lead");
        assert!(matches!(change.op, ChangeOp::Added(_)));
    }

    #[test]
    fn collect_changes_inserted_hook_produces_added_entry_with_correct_path() {
        let mut store = EntityStore::new();
        store.insert_hook(minimal_hook("UpdateJira"));
        let cs = store.collect_changes();
        assert_eq!(cs.len(), 1);
        let change = &cs.changes[0];
        assert_eq!(change.path, "shared/hooks");
        assert_eq!(change.kind, EntityKind::Hook);
        assert_eq!(change.id, "UpdateJira");
    }

    #[test]
    fn collect_changes_inserted_team_produces_added_entry_with_correct_path() {
        let mut store = EntityStore::new();
        store.insert_team(minimal_team("platform-team"));
        let cs = store.collect_changes();
        assert_eq!(cs.len(), 1);
        assert_eq!(cs.changes[0].path, "teams");
        assert_eq!(cs.changes[0].kind, EntityKind::Team);
    }

    #[test]
    fn collect_changes_inserted_workflow_produces_entries_for_workflow_and_task_step() {
        let mut store = EntityStore::new();
        store.insert_workflow(minimal_workflow("Initiative", "WriteProposal"));
        let cs = store.collect_changes();
        // Workflow entity + 1 task step
        assert_eq!(cs.len(), 2);
        let paths: Vec<&str> = cs.changes.iter().map(|c| c.path.as_str()).collect();
        assert!(paths.contains(&"workflows/Initiative"));
        assert!(paths.contains(&"workflows/Initiative/WriteProposal"));
    }

    #[test]
    fn collect_changes_modified_role_produces_modified_entry_with_dirty_fields() {
        let mut store = EntityStore::new();
        store.insert_role(minimal_role("eng-lead"));
        store.roles.reset_tracked();
        {
            let role = store.get_role_mut("eng-lead").unwrap();
            *role.name = "Engineering Lead".to_string();
        }
        let cs = store.collect_changes();
        assert_eq!(cs.len(), 1);
        let change = &cs.changes[0];
        assert_eq!(change.id, "eng-lead");
        if let ChangeOp::Modified { dirty_fields, .. } = &change.op {
            assert!(dirty_fields.contains(&"name".to_string()));
        } else {
            panic!("expected Modified");
        }
    }

    #[test]
    fn collect_changes_removed_role_produces_removed_entry() {
        let mut store = EntityStore::new();
        store.insert_role(minimal_role("eng-lead"));
        store.roles.reset_tracked();
        store.remove_role("eng-lead");
        let cs = store.collect_changes();
        assert_eq!(cs.len(), 1);
        assert_eq!(cs.changes[0].id, "eng-lead");
        assert!(matches!(cs.changes[0].op, ChangeOp::Removed));
    }

    #[test]
    fn collect_changes_does_not_reset_dirty_state() {
        let mut store = EntityStore::new();
        store.insert_role(minimal_role("eng-lead"));
        let _ = store.collect_changes();
        // Calling again must produce the same result
        let cs2 = store.collect_changes();
        assert_eq!(cs2.len(), 1);
    }

    // --- 6.5: reset_tracked tests ---

    #[test]
    fn reset_tracked_after_collect_changes_produces_empty_changeset() {
        let mut store = EntityStore::new();
        store.insert_role(minimal_role("eng-lead"));
        store.insert_workflow(minimal_workflow("Initiative", "WriteProposal"));
        let _ = store.collect_changes();
        store.reset_tracked();
        let cs = store.collect_changes();
        assert!(cs.is_empty());
    }

    #[test]
    fn reset_tracked_clears_nested_step_tracking_in_workflows() {
        let mut store = EntityStore::new();
        store.insert_workflow(minimal_workflow("Initiative", "WriteProposal"));
        store.reset_tracked();
        // The workflow's steps TrackedMap should also be clean
        let wf = store.workflows.get(&"Initiative".to_string()).unwrap();
        assert!(!wf.steps.has_changes());
    }
}
