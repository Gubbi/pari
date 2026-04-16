# any-entity-ref

**Entity Layer → `entity_layer/entity-identity/`**

---

## Purpose

`AnyEntityRef` is a concrete enum that provides type-erased access to any entity ref. Where code needs to work with refs of different entity types uniformly — `all_refs()`, EntityLoadContext resolution, change tracking sets — it uses `AnyEntityRef` rather than `dyn Trait`.

`EntityRef<T, P>` remains the typed form used in field declarations. `AnyEntityRef` is the erasure form used at boundaries.

Because the wrapped ref includes its `ParentKind`, `AnyEntityRef` preserves hierarchical identity even after type erasure. This is what lets the Store keep a flat map without losing parent-child distinctions.

---

## Enum Definition

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AnyEntityRef {
    Role(EntityRef<Role, NoParent>),
    Hook(EntityRef<Hook, NoParent>),
    Team(EntityRef<Team, NoParent>),
    ArtifactKind(EntityRef<ArtifactKind, NoParent>),
    Workflow(EntityRef<Workflow, NoParent>),
    ReusableWorkflow(EntityRef<ReusableWorkflow, NoParent>),
    EmbeddedWorkflow(EntityRef<EmbeddedWorkflow, WorkflowParent>),
    Task(EntityRef<Task, WorkflowParent>),
    Relay(EntityRef<Relay, WorkflowParent>),
}
```

`Hash + Eq` delegate to the inner `EntityRef<T, P>`. `AnyEntityRef` is a plain value type — no boxing, no vtable.

---

## Accessors

```rust
impl AnyEntityRef {
    pub fn kind(&self) -> EntityKind { ... }   // match on variant, return T::KIND
    pub fn id(&self) -> &str { ... }           // match on variant, return inner id
    pub fn parent(&self) -> Option<AnyEntityRef> { ... }  // None for top-level, Some(parent) otherwise
}
```

`AnyEntityRef::parent()` is the erasure-layer view of the same relationship exposed by `EntityRef::parent()`. It returns the immediate parent as another erased ref when one exists.

---

## Use Cases

**`all_refs()`** — each `Arc<TrackedField<EntityRef<T, P>>>` field on a tracked entity wraps its ref into the appropriate variant:

```rust
fn all_refs(&self) -> Vec<AnyEntityRef> {
    let mut refs = vec![];
    if let Some(r) = self.accountable.value.get() { refs.push(AnyEntityRef::Role(r.clone())); }
    // ...
    refs
}
```

**Store index and change tracking sets** — `HashMap<AnyEntityRef, TrackedEntity>` and `HashSet<AnyEntityRef>` in the store for `added`, `modified`, `removed`, `checked_out`. Parent-qualified refs ensure embedded entities remain distinct while still being stored flat.
