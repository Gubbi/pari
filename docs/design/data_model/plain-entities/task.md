# task-plain

**Data Model → `data_model/plain-entities/`**

---

## Purpose

`Task` is an embedded-only entity representing an atomic unit of work. It lives inside a workflow's `definitions` map and is never a top-level standalone entity.

---

## Definition

```rust
pub struct Task {
    pub entity_ref: EntityRef<Task, WorkflowParent>,
    pub name: String,
    pub description: Option<String>,
    pub purpose: String,
    pub instructions: Vec<String>,
    pub criteria: Vec<String>,
    pub raci: Option<Raci>,
    pub artifact: Artifact,
    pub states: Vec<TaskStateEntry>,
    pub intercepts: Option<HashMap<TaskTrigger, HookCall>>,
    pub guidance: Option<String>,
    pub extensions: Extensions,
}
```

---

## Fields

- `entity_ref` — carries the task's id and kind; parent is `WorkflowParent` (Workflow, ReusableWorkflow, or EmbeddedWorkflow)
- `name` — human-readable display name
- `description` — optional short summary
- `purpose` — describes the operational goal of this task
- `instructions` — ordered steps the agent or person performing the task follows
- `criteria` — acceptance criteria; conditions that must be met for the task to be considered done
- `raci` — optional; task-level accountability can be inherited from the parent workflow when absent
- `artifact` — the deliverable produced by this task (see [16 · artifact](../value-types/artifact.md))
- `states` — lifecycle states for this task; must include at least one `Done` semantic state (see [17 · state-entries](../value-types/state-entries.md))
- `intercepts` — optional lifecycle hooks keyed by `TaskTrigger` (see [18 · intercepts](../value-types/intercepts.md))
- `guidance` — optional freeform guidance for tooling or agents
- `extensions` — open-ended metadata; only `x-` prefixed keys are permitted (see [13 · extensions](../value-types/extensions.md))
