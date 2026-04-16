# intercepts

**Owning layer: `entity`**

---

## Purpose

Intercepts bind lifecycle trigger points to `HookCall`s. Each entity type defines its own trigger enum — giving compile-time safety over which triggers are valid per entity, the same way semantic enums constrain state entries.

---

## Trigger Enums

```rust
pub enum WorkflowTrigger {
    OnStart,
    OnDone,
    OnBlocked,
    OnFailed,
    OnReviewing,  // workflow only — fires when entering a reviewing state
    OnReject,     // workflow only — fires when a ReviewStep rejects
}

pub enum TaskTrigger {
    OnStart,
    OnDone,
    OnBlocked,
    OnFailed,
}
```

`OnReviewing` and `OnReject` are workflow-only — a `TaskTrigger` cannot reference them. This is enforced at the type level, not just validation.

---

## Usage on Entities

Each entity declares its intercepts field directly with its specific trigger type:

```rust
// on Workflow:
pub intercepts: Option<HashMap<WorkflowTrigger, HookCall>>,

// on Task:
pub intercepts: Option<HashMap<TaskTrigger, HookCall>>,
```

No shared `Intercepts` type alias — the key type differs per entity and the alias would obscure that difference.

---

## Relation to HookCall

`HookCall` (topic 15) is the value type — it identifies which hook to invoke and how to bind its inputs. The trigger map is the dispatch layer on top of it.
