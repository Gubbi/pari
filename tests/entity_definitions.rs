use pari::entity::{Entity, EntityKind, EntityRef, NoParent, WorkflowParent};
use pari::entities::{
    role::Role,
    hook::{Hook, HookInput},
    team::{Team, TeamMember},
    artifact_kind::ArtifactKind,
    workflow::{EmbeddedWorkflow, ReusableWorkflow, Step, Workflow},
    task::Task,
    relay::{Relay, StateMapEntry, StateMapSemantic},
};
use pari::types::{
    Artifact, Extensions, HookCall, Raci, TaskSemantic, TaskStateEntry, TaskTrigger,
    WorkflowSemantic, WorkflowStateEntry, WorkflowTrigger,
};
use indexmap::IndexMap;
use std::collections::HashMap;

// --- Helper builders ---

fn role_ref(id: &str) -> EntityRef<Role> {
    EntityRef::new(id)
}

fn workflow_parent(id: &str) -> WorkflowParent {
    WorkflowParent::Workflow(EntityRef::new(id))
}

fn make_raci(responsible_id: &str, accountable_id: &str) -> Raci {
    Raci {
        responsible: vec![role_ref(responsible_id)],
        accountable: role_ref(accountable_id),
        consulted: None,
        informed: None,
    }
}

fn done_state(id: &str) -> WorkflowStateEntry {
    WorkflowStateEntry {
        id: id.to_string(),
        description: "Terminal state".to_string(),
        semantic: Some(WorkflowSemantic::Done),
    }
}

fn task_done_state() -> TaskStateEntry {
    TaskStateEntry {
        id: "Done".to_string(),
        description: "Completed".to_string(),
        semantic: Some(TaskSemantic::Done),
    }
}

// --- EntityKind values ---

#[test]
fn role_kind_is_role() {
    assert_eq!(<Role as Entity>::KIND, EntityKind::Role);
}

#[test]
fn hook_kind_is_hook() {
    assert_eq!(<Hook as Entity>::KIND, EntityKind::Hook);
}

#[test]
fn team_kind_is_team() {
    assert_eq!(<Team as Entity>::KIND, EntityKind::Team);
}

#[test]
fn workflow_kind_is_workflow() {
    assert_eq!(<Workflow as Entity>::KIND, EntityKind::Workflow);
}

#[test]
fn reusable_workflow_kind_is_reusable_workflow() {
    assert_eq!(<ReusableWorkflow as Entity>::KIND, EntityKind::ReusableWorkflow);
}

#[test]
fn artifact_kind_kind_is_artifact_kind() {
    assert_eq!(<ArtifactKind as Entity>::KIND, EntityKind::ArtifactKind);
}

#[test]
fn task_kind_is_task() {
    assert_eq!(<Task as Entity>::KIND, EntityKind::Task);
}

#[test]
fn relay_kind_is_relay() {
    assert_eq!(<Relay as Entity>::KIND, EntityKind::Relay);
}

#[test]
fn embedded_workflow_kind_is_embedded_workflow() {
    assert_eq!(<EmbeddedWorkflow as Entity>::KIND, EntityKind::EmbeddedWorkflow);
}

// --- Parent types ---

#[test]
fn role_parent_is_no_parent() {
    fn _check(_: <Role as Entity>::Parent)
    where
        <Role as Entity>::Parent: pari::entity::ParentKind,
    {
    }
    let _ = |p: NoParent| _check(p);
}

#[test]
fn task_parent_is_workflow_parent() {
    fn _check(_: <Task as Entity>::Parent)
    where
        <Task as Entity>::Parent: pari::entity::ParentKind,
    {
    }
    let _ = |p: WorkflowParent| _check(p);
}

// --- Tracked type roundtrip ---

#[test]
fn role_tracked_for_roundtrip_compiles() {
    use pari::entity::TrackedFor;
    fn _check(_: <pari::entities::role::TrackedRole as TrackedFor>::Entity) {}
    let _ = |r: Role| _check(r);
}

// --- From<Plain> for Tracked ---

#[test]
fn tracked_role_from_plain_role() {
    use pari::entities::role::TrackedRole;
    let plain = Role {
        entity_ref: EntityRef::new("eng-lead"),
        name: "Engineering Lead".to_string(),
        description: None,
        purpose: "Leads engineering".to_string(),
        traits: Some(vec!["reviewer".to_string()]),
        extensions: HashMap::new(),
    };
    let tracked = TrackedRole::from(plain);
    assert_eq!(tracked.entity_ref().id(), "eng-lead");
    assert_eq!(tracked.name.get(), Some(&"Engineering Lead".to_string()));
    assert!(!tracked.has_dirty_fields());
}

#[test]
fn tracked_hook_from_plain_hook() {
    use pari::entities::hook::TrackedHook;
    let plain = Hook {
        entity_ref: EntityRef::new("notify-slack"),
        name: "Notify Slack".to_string(),
        description: None,
        instructions: vec!["Post a message".to_string()],
        inputs: None,
        extensions: HashMap::new(),
    };
    let tracked = TrackedHook::from(plain);
    assert_eq!(tracked.entity_ref().id(), "notify-slack");
    assert!(!tracked.has_dirty_fields());
}

#[test]
fn tracked_task_from_plain_task() {
    use pari::entities::task::TrackedTask;
    let plain = Task {
        entity_ref: EntityRef::with_parent("WriteProposal", workflow_parent("InitiativeWorkflow")),
        name: "Write Proposal".to_string(),
        description: None,
        purpose: "Draft the initiative proposal".to_string(),
        instructions: vec!["Write a clear proposal".to_string()],
        criteria: vec!["Proposal is approved".to_string()],
        raci: None,
        artifact: Artifact { kind: EntityRef::new("doc"), template: None },
        states: vec![task_done_state()],
        intercepts: None,
        guidance: None,
        extensions: HashMap::new(),
    };
    let tracked = TrackedTask::from(plain);
    assert_eq!(tracked.entity_ref().id(), "WriteProposal");
    assert!(matches!(
        tracked.entity_ref().parent(),
        Some(WorkflowParent::Workflow(parent)) if parent.id() == "InitiativeWorkflow"
    ));
}

// --- to_any_ref ---

#[test]
fn role_to_any_ref_wraps_in_role_variant() {
    use pari::entity::AnyEntityRef;
    let r: EntityRef<Role> = EntityRef::new("eng-lead");
    let any = Role::to_any_ref(&r);
    assert_eq!(any.kind(), EntityKind::Role);
    assert_eq!(any.id(), "eng-lead");
    assert!(any.parent().is_none());
}

#[test]
fn task_to_any_ref_wraps_in_task_variant() {
    use pari::entity::AnyEntityRef;
    let r = EntityRef::<Task, WorkflowParent>::with_parent(
        "WriteProposal",
        workflow_parent("InitiativeWorkflow"),
    );
    let any = Task::to_any_ref(&r);
    assert_eq!(any.kind(), EntityKind::Task);
    assert_eq!(any.id(), "WriteProposal");
    let parent = any.parent().unwrap();
    assert_eq!(parent.id(), "InitiativeWorkflow");
    assert_eq!(parent.kind(), EntityKind::Workflow);
}

#[test]
fn relay_to_any_ref_has_workflow_parent() {
    use pari::entity::AnyEntityRef;
    let r = EntityRef::<Relay, WorkflowParent>::with_parent(
        "HandoffToReview",
        workflow_parent("InitiativeWorkflow"),
    );
    let any = Relay::to_any_ref(&r);
    assert_eq!(any.kind(), EntityKind::Relay);
    let parent = any.parent().unwrap();
    assert_eq!(parent.kind(), EntityKind::Workflow);
}

// --- Value types compile ---

#[test]
fn extensions_is_hashmap() {
    let mut ext: Extensions = HashMap::new();
    ext.insert("x-owner".to_string(), serde_json::json!("alice"));
    assert_eq!(ext.len(), 1);
}

#[test]
fn workflow_state_entry_reviewing_semantics() {
    let state = WorkflowStateEntry {
        id: "UnderReview".to_string(),
        description: "Awaiting approval".to_string(),
        semantic: Some(WorkflowSemantic::Reviewing),
    };
    assert!(matches!(state.semantic, Some(WorkflowSemantic::Reviewing)));
}

#[test]
fn task_trigger_enum_does_not_have_reviewing() {
    // Exhaustive match confirms only four variants exist
    let t = TaskTrigger::OnStart;
    let _ = match t {
        TaskTrigger::OnStart => (),
        TaskTrigger::OnDone => (),
        TaskTrigger::OnBlocked => (),
        TaskTrigger::OnFailed => (),
    };
}

#[test]
fn step_enum_variants_compile() {
    let _task_step = Step::Task {
        entity_ref: EntityRef::with_parent("T1", workflow_parent("WF1")),
        depends_on: None,
    };
    let _review_step = Step::Review { approver: vec![role_ref("pm")], on_reject: "T1".to_string() };
}
