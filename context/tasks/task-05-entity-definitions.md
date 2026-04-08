# Task 05 — Entity Definitions

## Scope

Define all value types and plain entity structs. Apply `#[derive(Entity)]` to each entity struct and invoke `entity_registry!`. This task makes Tasks 03 and 04 fully concrete — replacing test stubs with real types.

---

## Files

- `src/types.rs` — value types: `Extensions`, `Raci`, `Artifact`, `HookCall`, `WorkflowStateEntry`, `TaskStateEntry`, `WorkflowSemantic`, `TaskSemantic`, `WorkflowTrigger`, `TaskTrigger`, `StateMapEntry`, `StateMapSemantic`
- `src/entities/role.rs` — `Role`
- `src/entities/hook.rs` — `Hook`, `HookInput`
- `src/entities/team.rs` — `Team`, `TeamMember`
- `src/entities/artifact_kind.rs` — `ArtifactKind`
- `src/entities/workflow.rs` — `Workflow`, `ReusableWorkflow`, `EmbeddedWorkflow`, `Step`
- `src/entities/task.rs` — `Task`
- `src/entities/relay.rs` — `Relay`, `StateMapEntry`, `StateMapSemantic`
- `src/entities/mod.rs` — re-exports all entity modules
- `src/lib.rs` — `pub mod types; pub mod entities;`
- `src/entity.rs` — add `entity_registry!` invocation at bottom (replaces stubs)

---

## Dependencies

- Task 02: `EntityRef`, `NoParent`, `WorkflowParent`, `Entity`, `TrackedEntity`, `Resolvable`, `ValidationSchema`, `EntityKind`, `AnyEntityRef`, `StoreEntity`
- Task 03: `#[derive(Entity)]`, `#[entity(...)]` attribute
- Task 04: `entity_registry!`

---

## Value Types (`src/types.rs`)

```rust
use std::collections::HashMap;
use serde_json;
use crate::entity::{EntityRef, NoParent};
// Forward refs for EntityRef<Role>, EntityRef<Team>, etc. — resolved via entity modules
// Use fully qualified paths in the entity module files; types.rs only defines the value types.

/// Open-ended metadata. Only x- prefixed keys are permitted (enforced by validation, not type system).
pub type Extensions = HashMap<String, serde_json::Value>;

/// Accountability assignment for workflows and tasks.
pub struct Raci {
    pub responsible: Vec<EntityRef<crate::entities::role::Role>>,
    pub accountable: EntityRef<crate::entities::role::Role>,
    pub consulted:   Option<Vec<EntityRef<crate::entities::role::Role>>>,
    pub informed:    Option<Vec<EntityRef<crate::entities::role::Role>>>,
}

/// Task deliverable specification.
pub struct Artifact {
    pub kind:     EntityRef<crate::entities::artifact_kind::ArtifactKind>,
    pub template: Option<String>,
}

/// Usage-site reference to a hook with optional input bindings.
pub struct HookCall {
    pub hook: EntityRef<crate::entities::hook::Hook>,
    pub with: Option<HashMap<String, String>>,
}

// --- Lifecycle state types ---

pub struct WorkflowStateEntry {
    pub id:          String,  // CamelCase
    pub description: String,
    pub semantic:    Option<WorkflowSemantic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowSemantic {
    Reviewing,
    Done,
    Blocked,
    Failed,
}

pub struct TaskStateEntry {
    pub id:          String,  // CamelCase
    pub description: String,
    pub semantic:    Option<TaskSemantic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskSemantic {
    Done,
    Blocked,
    Failed,
}

// --- Trigger enums ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkflowTrigger {
    OnStart,
    OnDone,
    OnBlocked,
    OnFailed,
    OnReviewing,  // workflow-only
    OnReject,     // workflow-only
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskTrigger {
    OnStart,
    OnDone,
    OnBlocked,
    OnFailed,
}
```

---

## Entity Definitions

### `src/entities/role.rs`

```rust
use crate::entity::{EntityRef, NoParent, EntityKind};
use crate::types::Extensions;

#[entity(kind = EntityKind::Role)]
#[derive(Entity)]
pub struct Role {
    pub entity_ref:  EntityRef<Role>,
    pub name:        String,
    pub description: Option<String>,
    pub purpose:     String,
    pub traits:      Option<Vec<String>>,
    pub extensions:  Extensions,
}
```

### `src/entities/hook.rs`

```rust
use crate::entity::{EntityRef, NoParent, EntityKind};
use crate::types::Extensions;

#[entity(kind = EntityKind::Hook)]
#[derive(Entity)]
pub struct Hook {
    pub entity_ref:   EntityRef<Hook>,
    pub name:         String,
    pub description:  Option<String>,
    pub instructions: Vec<String>,
    pub inputs:       Option<Vec<HookInput>>,
    pub extensions:   Extensions,
}

#[derive(Debug, Clone)]
pub struct HookInput {
    pub name:        String,
    pub description: Option<String>,
    pub required:    bool,
}
```

### `src/entities/team.rs`

```rust
use std::collections::HashMap;
use crate::entity::{EntityRef, NoParent, EntityKind};
use crate::types::Extensions;
use crate::entities::role::Role;

#[entity(kind = EntityKind::Team)]
#[derive(Entity)]
pub struct Team {
    pub entity_ref:  EntityRef<Team>,
    pub name:        String,
    pub description: Option<String>,
    pub members:     Option<Vec<TeamMember>>,
    pub include:     Option<HashMap<EntityRef<Team>, EntityRef<Role>>>,
    pub import:      Option<Vec<EntityRef<Team>>>,
    pub extensions:  Extensions,
}

#[derive(Debug, Clone)]
pub struct TeamMember {
    pub handle: String,
    pub role:   EntityRef<Role>,
}
```

### `src/entities/artifact_kind.rs`

```rust
use crate::entity::{EntityRef, NoParent, EntityKind};
use crate::types::Extensions;

#[entity(kind = EntityKind::ArtifactKind)]
#[derive(Entity)]
pub struct ArtifactKind {
    pub entity_ref:  EntityRef<ArtifactKind>,
    pub name:        String,
    pub description: Option<String>,
    pub service:     String,
    pub access:      Option<String>,
    pub guidance:    Option<String>,
    pub extensions:  Extensions,
}
```

### `src/entities/workflow.rs`

```rust
use std::collections::HashMap;
use indexmap::IndexMap;
use crate::entity::{EntityRef, NoParent, WorkflowParent, EntityKind};
use crate::types::{Raci, WorkflowStateEntry, WorkflowTrigger, HookCall, Extensions};
use crate::entities::role::Role;
use crate::entities::task::Task;
use crate::entities::relay::Relay;

// Step type — not an Entity; lives inside Workflow.steps
pub enum Step {
    Task {
        entity_ref: EntityRef<Task, WorkflowParent>,
        depends_on: Option<Vec<String>>,
    },
    Relay {
        entity_ref: EntityRef<Relay, WorkflowParent>,
        depends_on: Option<Vec<String>>,
    },
    EmbeddedWorkflow {
        entity_ref: EntityRef<EmbeddedWorkflow, WorkflowParent>,
        depends_on: Option<Vec<String>>,
    },
    Review {
        approver:  Vec<EntityRef<Role>>,
        on_reject: String,
    },
}

#[entity(kind = EntityKind::Workflow)]
#[derive(Entity)]
pub struct Workflow {
    pub entity_ref:   EntityRef<Workflow>,
    pub name:         String,
    pub description:  Option<String>,
    pub purpose:      String,
    pub raci:         Raci,
    pub states:       Vec<WorkflowStateEntry>,
    pub steps:        IndexMap<String, Step>,
    pub intercepts:   Option<HashMap<WorkflowTrigger, HookCall>>,
    pub guidance:     Option<String>,
    pub extensions:   Extensions,
}

#[entity(kind = EntityKind::ReusableWorkflow)]
#[derive(Entity)]
pub struct ReusableWorkflow {
    pub entity_ref:   EntityRef<ReusableWorkflow>,
    pub name:         String,
    pub description:  Option<String>,
    pub purpose:      String,
    pub raci:         Raci,
    pub states:       Vec<WorkflowStateEntry>,
    pub steps:        IndexMap<String, Step>,
    pub intercepts:   Option<HashMap<WorkflowTrigger, HookCall>>,
    pub guidance:     Option<String>,
    pub extensions:   Extensions,
}

#[entity(kind = EntityKind::EmbeddedWorkflow, parent = WorkflowParent)]
#[derive(Entity)]
pub struct EmbeddedWorkflow {
    pub entity_ref:   EntityRef<EmbeddedWorkflow, WorkflowParent>,
    pub name:         String,
    pub description:  Option<String>,
    pub purpose:      String,
    pub raci:         Option<Raci>,
    pub states:       Vec<WorkflowStateEntry>,
    pub steps:        IndexMap<String, Step>,
    pub intercepts:   Option<HashMap<WorkflowTrigger, HookCall>>,
    pub guidance:     Option<String>,
    pub extensions:   Extensions,
}
```

### `src/entities/task.rs`

```rust
use std::collections::HashMap;
use crate::entity::{EntityRef, WorkflowParent, EntityKind};
use crate::types::{Raci, Artifact, TaskStateEntry, TaskTrigger, HookCall, Extensions};

#[entity(kind = EntityKind::Task, parent = WorkflowParent)]
#[derive(Entity)]
pub struct Task {
    pub entity_ref:   EntityRef<Task, WorkflowParent>,
    pub name:         String,
    pub description:  Option<String>,
    pub purpose:      String,
    pub instructions: Vec<String>,
    pub criteria:     Vec<String>,
    pub raci:         Option<Raci>,
    pub artifact:     Artifact,
    pub states:       Vec<TaskStateEntry>,
    pub intercepts:   Option<HashMap<TaskTrigger, HookCall>>,
    pub guidance:     Option<String>,
    pub extensions:   Extensions,
}
```

### `src/entities/relay.rs`

```rust
use std::collections::HashMap;
use crate::entity::{EntityRef, WorkflowParent, NoParent, EntityKind};
use crate::types::{Raci, TaskTrigger, HookCall, Extensions};
use crate::entities::workflow::ReusableWorkflow;

#[entity(kind = EntityKind::Relay, parent = WorkflowParent)]
#[derive(Entity)]
pub struct Relay {
    pub entity_ref:    EntityRef<Relay, WorkflowParent>,
    pub name:          String,
    pub description:   Option<String>,
    pub purpose:       String,
    pub raci:          Option<Raci>,
    pub delegates_to:  EntityRef<ReusableWorkflow>,
    pub briefing:      Option<String>,
    pub debriefing:    Option<String>,
    pub state_map:     HashMap<String, StateMapEntry>,
    pub intercepts:    Option<HashMap<TaskTrigger, HookCall>>,
    pub guidance:      Option<String>,
    pub extensions:    Extensions,
}

#[derive(Debug, Clone)]
pub struct StateMapEntry {
    pub maps_to:     String,
    pub description: Option<String>,
    pub semantic:    Option<StateMapSemantic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateMapSemantic {
    Done,
    Blocked,
    Failed,
}
```

### `src/entities/mod.rs`

```rust
pub mod role;
pub mod hook;
pub mod team;
pub mod artifact_kind;
pub mod workflow;
pub mod task;
pub mod relay;
```

---

## `entity_registry!` Invocation (add to `src/entity.rs`)

Remove the hand-written `EntityKind` stub and add at the bottom:

```rust
use crate::entities::{
    role::Role, hook::Hook, team::Team, artifact_kind::ArtifactKind,
    workflow::{Workflow, ReusableWorkflow, EmbeddedWorkflow},
    task::Task, relay::Relay,
};

entity_registry! {
    Role             => NoParent,
    Hook             => NoParent,
    Team             => NoParent,
    Workflow         => NoParent,
    ReusableWorkflow => NoParent,
    ArtifactKind     => NoParent,
    Task             => WorkflowParent,
    Relay            => WorkflowParent,
    EmbeddedWorkflow => WorkflowParent,
}
```

---

## TDD: Tests to Write First

```rust
// tests/entity_definitions.rs
use pari::entity::{Entity, EntityKind, EntityRef, NoParent, WorkflowParent};
use pari::entities::{
    role::Role,
    hook::{Hook, HookInput},
    team::{Team, TeamMember},
    artifact_kind::ArtifactKind,
    workflow::{Workflow, ReusableWorkflow, EmbeddedWorkflow, Step},
    task::Task,
    relay::{Relay, StateMapEntry, StateMapSemantic},
};
use pari::types::{
    Raci, Artifact, HookCall, WorkflowStateEntry, TaskStateEntry,
    WorkflowSemantic, TaskSemantic, WorkflowTrigger, TaskTrigger, Extensions,
};
use indexmap::IndexMap;
use std::collections::HashMap;

// Helper builders

fn role_ref(id: &str) -> EntityRef<Role> {
    EntityRef::new(id)
}

fn make_raci(responsible_id: &str, accountable_id: &str) -> Raci {
    Raci {
        responsible: vec![role_ref(responsible_id)],
        accountable: role_ref(accountable_id),
        consulted:   None,
        informed:    None,
    }
}

fn done_state(id: &str) -> WorkflowStateEntry {
    WorkflowStateEntry {
        id:          id.to_string(),
        description: "Terminal state".to_string(),
        semantic:    Some(WorkflowSemantic::Done),
    }
}

fn task_done_state() -> TaskStateEntry {
    TaskStateEntry {
        id:          "Done".to_string(),
        description: "Completed".to_string(),
        semantic:    Some(TaskSemantic::Done),
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
    fn _check(_: <Role as Entity>::Parent) where <Role as Entity>::Parent: pari::entity::ParentKind {}
    let _ = |p: NoParent| _check(p);
}

#[test]
fn task_parent_is_workflow_parent() {
    fn _check(_: <Task as Entity>::Parent) where <Task as Entity>::Parent: pari::entity::ParentKind {}
    let _ = |p: WorkflowParent { workflow_id: String::new() }| _check(p);
}

// --- Tracked type roundtrip ---

#[test]
fn role_tracked_entity_roundtrip_compiles() {
    use pari::entity::TrackedEntity;
    fn _check(_: <pari::entities::role::TrackedRole as TrackedEntity>::Entity) {}
    let _ = |r: Role| _check(r);
}

// --- From<Plain> for Tracked ---

#[test]
fn tracked_role_from_plain_role() {
    use pari::entities::role::TrackedRole;
    let plain = Role {
        entity_ref:  EntityRef::new("eng-lead"),
        name:        "Engineering Lead".to_string(),
        description: None,
        purpose:     "Leads engineering".to_string(),
        traits:      Some(vec!["reviewer".to_string()]),
        extensions:  HashMap::new(),
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
        entity_ref:   EntityRef::new("notify-slack"),
        name:         "Notify Slack".to_string(),
        description:  None,
        instructions: vec!["Post a message".to_string()],
        inputs:       None,
        extensions:   HashMap::new(),
    };
    let tracked = TrackedHook::from(plain);
    assert_eq!(tracked.entity_ref().id(), "notify-slack");
    assert!(!tracked.has_dirty_fields());
}

#[test]
fn tracked_task_from_plain_task() {
    use pari::entities::task::TrackedTask;
    use pari::entities::artifact_kind::ArtifactKind;
    let plain = Task {
        entity_ref:   EntityRef::new_embedded("WriteProposal", "InitiativeWorkflow"),
        name:         "Write Proposal".to_string(),
        description:  None,
        purpose:      "Draft the initiative proposal".to_string(),
        instructions: vec!["Write a clear proposal".to_string()],
        criteria:     vec!["Proposal is approved".to_string()],
        raci:         None,
        artifact:     Artifact {
            kind:     EntityRef::new("doc"),
            template: None,
        },
        states:       vec![task_done_state()],
        intercepts:   None,
        guidance:     None,
        extensions:   HashMap::new(),
    };
    let tracked = TrackedTask::from(plain);
    assert_eq!(tracked.entity_ref().id(), "WriteProposal");
    assert_eq!(tracked.entity_ref().parent.workflow_id, "InitiativeWorkflow");
}

// --- Resolvable ---

#[test]
fn role_to_any_ref_wraps_in_role_variant() {
    use pari::entity::{Resolvable, AnyEntityRef};
    let r: EntityRef<Role> = EntityRef::new("eng-lead");
    let any = Role::to_any_ref(&r);
    assert_eq!(any.kind(), EntityKind::Role);
    assert_eq!(any.id(), "eng-lead");
    assert!(any.parent().is_none());
}

#[test]
fn task_to_any_ref_wraps_in_task_variant() {
    use pari::entity::{Resolvable, AnyEntityRef};
    let r = EntityRef::<Task, WorkflowParent>::new_embedded("WriteProposal", "InitiativeWorkflow");
    let any = Task::to_any_ref(&r);
    assert_eq!(any.kind(), EntityKind::Task);
    assert_eq!(any.id(), "WriteProposal");
    let parent = any.parent().unwrap();
    assert_eq!(parent.id(), "InitiativeWorkflow");
    assert_eq!(parent.kind(), EntityKind::Workflow);
}

#[test]
fn relay_to_any_ref_has_workflow_parent() {
    use pari::entity::{Resolvable, AnyEntityRef};
    let r = EntityRef::<Relay, WorkflowParent>::new_embedded("HandoffToReview", "InitiativeWorkflow");
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
        id:          "UnderReview".to_string(),
        description: "Awaiting approval".to_string(),
        semantic:    Some(WorkflowSemantic::Reviewing),
    };
    assert!(matches!(state.semantic, Some(WorkflowSemantic::Reviewing)));
}

#[test]
fn task_trigger_enum_does_not_have_reviewing() {
    // TaskTrigger must not have OnReviewing or OnReject variants — type-level constraint.
    // Verify by exhaustive match that only the four allowed variants exist:
    let t = TaskTrigger::OnStart;
    let _ = match t {
        TaskTrigger::OnStart  => (),
        TaskTrigger::OnDone   => (),
        TaskTrigger::OnBlocked => (),
        TaskTrigger::OnFailed  => (),
    };
}

#[test]
fn step_enum_variants_compile() {
    // Construct each variant to verify they compile with correct types
    let _task_step = Step::Task {
        entity_ref: EntityRef::new_embedded("T1", "WF1"),
        depends_on: None,
    };
    let _review_step = Step::Review {
        approver: vec![role_ref("pm")],
        on_reject: "T1".to_string(),
    };
}
```

---

## Implementation Notes

### `indexmap` Dependency

`Workflow.steps` and `ReusableWorkflow.steps`/`EmbeddedWorkflow.steps` use `IndexMap<String, Step>` (ordered map preserving insertion order). Ensure `indexmap` is in `Cargo.toml`:

```toml
[dependencies]
indexmap = "2"
```

### `serde_json` Dependency

`Extensions = HashMap<String, serde_json::Value>` requires `serde_json`. Ensure in `Cargo.toml`:

```toml
[dependencies]
serde_json = "1"
```

### Circular Reference: `Team.include` / `Team.import`

`Team` references `EntityRef<Team>` in `include` and `import`. This is a same-type ref, not a circular type definition — `EntityRef<T>` is a thin wrapper and does not embed `T` inline. This is valid Rust.

### `Step` is Not an Entity

`Step` is a plain enum — not annotated with `#[derive(Entity)]`. It lives inside `Workflow.steps` as a value type. It has no `entity_ref`, no `EntityKind`, and is not in the `entity_registry!`.

### `HookInput`, `TeamMember`, `StateMapEntry` are Not Entities

These are plain structs with no `EntityRef` or `#[derive(Entity)]`. They are value types embedded in their parent entities.

### `entity_ref` field type for embedded entities

`Task`, `Relay`, `EmbeddedWorkflow` use `EntityRef<T, WorkflowParent>`, not `EntityRef<T>` (which defaults to `NoParent`). The `#[entity(parent = WorkflowParent)]` attribute signals the proc macro to generate `TrackedX.entity_ref: EntityRef<T, WorkflowParent>` rather than `EntityRef<T, NoParent>`.

### `Clone` bounds for `reset_dirty`

The `#[derive(Entity)]` macro generates `reset_dirty` which calls `value.clone()`. All field types on all entities must implement `Clone`:
- `String`, `Option<String>`, `Vec<String>`: ✓
- `Raci`, `Artifact`, `HookCall`: must `#[derive(Clone)]`
- `WorkflowStateEntry`, `TaskStateEntry`: must `#[derive(Clone)]`
- `IndexMap<String, Step>`: requires `Step: Clone` — `Step` must `#[derive(Clone)]`
- `Extensions` (`HashMap<String, serde_json::Value>`): `serde_json::Value: Clone` ✓
- `HookInput`, `TeamMember`, `StateMapEntry`: already specified as `#[derive(Debug, Clone)]` above

Add `#[derive(Clone)]` to: `Raci`, `Artifact`, `HookCall`, `WorkflowStateEntry`, `TaskStateEntry`, `WorkflowSemantic`, `TaskSemantic`, `StateMapEntry`, `StateMapSemantic`, `Step`.

---

## Acceptance Criteria

- `cargo build` succeeds — all entity types compile with `#[derive(Entity)]`
- `cargo test entity_definitions` passes — all tests in `tests/entity_definitions.rs` green
- `entity_registry!` invocation compiles, replacing all Task 02/04 hand-written stubs
- `EntityKind` has the correct 9 variants matching the registry
- `From<PlainEntity>` works for all entities (Role, Hook, Team, ArtifactKind, Workflow, ReusableWorkflow, EmbeddedWorkflow, Task, Relay)
- `AnyEntityRef::parent()` returns `Some` for Task/Relay/EmbeddedWorkflow, `None` for others
- Task 02, 03, 04 tests still pass
