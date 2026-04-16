# relay-plain

**Entity Layer → `entity_layer/plain-entities/`**

---

## Purpose

`Relay` is an embedded entity that delegates a segment of work to a `ReusableWorkflow`. It defines its own states and maps each to a state in the delegated workflow, bridging the parent workflow's execution context with the reusable subprocess.

---

## Definition

```rust
pub struct Relay {
    pub entity_ref: EntityRef<Relay, WorkflowParent>,
    pub name: String,
    pub description: Option<String>,
    pub purpose: String,
    pub raci: Option<Raci>,
    pub delegates_to: EntityRef<ReusableWorkflow>,
    pub briefing: Option<String>,
    pub debriefing: Option<String>,
    pub state_map: HashMap<String, StateMapEntry>,
    pub intercepts: Option<HashMap<TaskTrigger, HookCall>>,
    pub guidance: Option<String>,
    pub extensions: Extensions,
}

pub struct StateMapEntry {
    pub maps_to: String,
    pub description: Option<String>,
    pub semantic: Option<StateMapSemantic>,
}

pub enum StateMapSemantic {
    Done,
    Blocked,
    Failed,
}
```

---

## Fields

- `entity_ref` — carries the relay's id and kind; parent is `WorkflowParent`, which establishes where the relay sits in the workflow tree; `Relay` may not appear in a `ReusableWorkflow` or any `EmbeddedWorkflow` within one (enforced by validation)
- `delegates_to` — the `ReusableWorkflow` this relay hands off to
- `briefing` — optional instructions for setting up the delegated workflow invocation
- `debriefing` — optional instructions for interpreting results when the delegated workflow completes
- `state_map` — defines the relay's own states (map keys, CamelCase) and maps each to a state in the delegated `ReusableWorkflow`; minimum one entry required
- `raci`, `intercepts`, `guidance`, `extensions` — same semantics as on `Task`

---

## `StateMapEntry`

Each entry defines one of the relay's own states:

- `maps_to` — id of the corresponding state in the delegated `ReusableWorkflow`
- `description` — optional description of this relay state
- `semantic` — optional machine-readable meaning: `Done`, `Blocked`, or `Failed`

The `state_map` keys are CamelCase state ids. No separate `states` list — the relay's states are fully defined by the `state_map`.
