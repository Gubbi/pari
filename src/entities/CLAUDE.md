# src/entities — Entity Type Definitions

Each entity is a plain Rust struct with `#[derive(pari_macros::Entity)]`. The macro generates:
- A `Tracked*` companion struct (all fields wrapped in `Arc<TrackedField<T>>`)
- `From<Plain>` impl for the tracked struct
- `has_dirty_fields()`, `dirty_fields() -> Vec<&'static str>`, `merge_dirty_into()`, `reset_dirty()`
- Async accessor: `async fn name(&self) -> Result<&str, LoadError>`
- Async setter: `async fn set_name(&mut self, v: String) -> Result<(), SetterError>`

Entity structs are registered in `src/entity.rs` via `entity_registry!`.

---

## Top-Level Entities (parent = NoParent)

### Role
```rust
pub struct Role {
    pub entity_ref:  EntityRef<Role>,
    pub name:        String,
    pub description: Option<String>,
    pub purpose:     String,
    pub traits:      Option<Vec<String>>,
    pub extensions:  Extensions,
}
```

### Hook
```rust
pub struct Hook {
    pub entity_ref:   EntityRef<Hook>,
    pub name:         String,
    pub description:  Option<String>,
    pub instructions: Vec<String>,
    pub inputs:       Vec<HookInput>,
    pub extensions:   Extensions,
}
pub struct HookInput { pub name: String, pub description: String, pub required: bool }
```

### Team
```rust
pub struct Team {
    pub entity_ref:  EntityRef<Team>,
    pub name:        String,
    pub description: Option<String>,
    pub members:     Vec<TeamMember>,
    pub include:     HashMap<EntityRef<Team>, EntityRef<Role>>,
    pub import:      Vec<EntityRef<Team>>,
    pub extensions:  Extensions,
}
pub struct TeamMember { pub handle: String, pub role: EntityRef<Role> }
```

### ArtifactKind
```rust
pub struct ArtifactKind {
    pub entity_ref:  EntityRef<ArtifactKind>,
    pub name:        String,
    pub description: Option<String>,
    pub service:     Option<String>,
    pub access:      Option<String>,
    pub guidance:    Option<String>,
    pub extensions:  Extensions,
}
```

### Workflow / ReusableWorkflow
```rust
// Both share this shape; differ only in entity_ref type and kind constant.
pub struct Workflow {
    pub entity_ref:  EntityRef<Workflow>,   // ReusableWorkflow for the other
    pub name:        String,
    pub description: Option<String>,
    pub purpose:     String,
    pub raci:        Raci,
    pub states:      Vec<WorkflowStateEntry>,
    pub steps:       IndexMap<String, Step>,
    pub intercepts:  Option<HashMap<WorkflowTrigger, HookCall>>,
    pub guidance:    Option<String>,
    pub extensions:  Extensions,
}
```

---

## Embedded Entities (parent = WorkflowParent)

These live inside `Workflow`/`ReusableWorkflow` steps. Their `EntityRef` carries `workflow_id`.

### Task
```rust
pub struct Task {
    pub entity_ref:  EntityRef<Task, WorkflowParent>,
    pub name:        String,
    pub description: Option<String>,
    pub purpose:     String,
    pub instructions: Vec<String>,
    pub criteria:    Vec<String>,
    pub raci:        Option<Raci>,
    pub artifact:    Artifact,
    pub states:      Vec<TaskStateEntry>,
    pub intercepts:  Option<HashMap<TaskTrigger, HookCall>>,
    pub guidance:    Option<String>,
    pub extensions:  Extensions,
}
// TaskTrigger = OnStart | OnDone | OnBlocked | OnFailed
```

### Relay
```rust
pub struct Relay {
    pub entity_ref:   EntityRef<Relay, WorkflowParent>,
    pub name:         String,
    pub description:  Option<String>,
    pub purpose:      String,
    pub raci:         Option<Raci>,
    pub delegates_to: EntityRef<ReusableWorkflow>,
    pub briefing:     Option<String>,
    pub debriefing:   Option<String>,
    pub state_map:    HashMap<String, StateMapEntry>,
    pub intercepts:   Option<HashMap<WorkflowTrigger, HookCall>>,
    pub guidance:     Option<String>,
    pub extensions:   Extensions,
}
pub struct StateMapEntry { pub maps_to: String, pub description: Option<String>, pub semantic: StateMapSemantic }
pub enum StateMapSemantic { Done, Blocked, Failed }
```

### EmbeddedWorkflow
Same shape as `Workflow` but `raci` is `Option<Raci>` and `entity_ref` is `EntityRef<EmbeddedWorkflow, WorkflowParent>`.

---

## Step enum (workflow.rs)

`Step` is not an entity — no `EntityRef`, no `#[derive(Entity)]`:
```rust
pub enum Step {
    Task             { entity_ref: EntityRef<Task, WorkflowParent>,             depends_on: Option<Vec<String>> },
    Relay            { entity_ref: EntityRef<Relay, WorkflowParent>,            depends_on: Option<Vec<String>> },
    EmbeddedWorkflow { entity_ref: EntityRef<EmbeddedWorkflow, WorkflowParent>, depends_on: Option<Vec<String>> },
    Review           { approver: Vec<EntityRef<Role>>, on_reject: String },
}
```

---

## EntityKind discriminants (generated in src/entity.rs)

```
Role | Hook | Team | Workflow | ReusableWorkflow | ArtifactKind | Task | Relay | EmbeddedWorkflow
```
