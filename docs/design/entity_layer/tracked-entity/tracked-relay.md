# tracked-relay

**Owning layer: `entity`**

---

## Purpose

`TrackedRelay` is the store's mutable, load-aware representation of a `Relay`. It demonstrates the nested entity case — `entity_ref` carries a `WorkflowParent`, and `delegates_to` is a typed cross-entity reference.

---

## Structure

```rust
struct TrackedRelay {
    entity_ref:   EntityRef<Relay, WorkflowParent>,
    name:         Arc<TrackedField<String>>,
    description:  Arc<TrackedField<Option<String>>>,
    purpose:      Arc<TrackedField<String>>,
    raci:         Arc<TrackedField<Option<Raci>>>,
    delegates_to: Arc<TrackedField<EntityRef<ReusableWorkflow>>>,
    briefing:     Arc<TrackedField<Option<String>>>,
    debriefing:   Arc<TrackedField<Option<String>>>,
    state_map:    Arc<TrackedField<HashMap<String, StateMapEntry>>>,
    intercepts:   Arc<TrackedField<Option<HashMap<TaskTrigger, HookCall>>>>,
    guidance:     Arc<TrackedField<Option<String>>>,
    extensions:   Arc<TrackedField<Extensions>>,
}
```

`delegates_to` stores the ref as a typed value — no live link to the referenced workflow. Resolving the `ReusableWorkflow` is a separate `EntityClient::resolve()` call by the caller.

`state_map` and `intercepts` are stored as single atomic `TrackedField`s — mutations replace the entire value, not individual entries.

---

## ensure_mutable

Loads all fields before any mutation. `RepoSubstrate` writes the full parent workflow file at persist — a partial field snapshot would corrupt the file. The cross-entity ref in `delegates_to` does not affect this: `ensure_mutable` only loads `TrackedRelay`'s own fields.
