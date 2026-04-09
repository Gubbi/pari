# entity-ref

**Data Model → `data_model/entity-identity/`**

---

## Purpose

`EntityRef<T, P>` is the primary identity type for entities. It carries an entity's id and its parent chain, with the entity type `T` encoded at the type level. It is the canonical way to refer to any entity across the codebase, and it is the authoritative representation of hierarchical relationships between entities.

---

## Structure

```rust
pub struct EntityRef<T: Entity, P: ParentKind = NoParent> {
    pub id: String,
    parent: P,
    _marker: PhantomData<T>,  // covariant in T; satisfies compiler's unused type parameter rule
}
```

- `id` — the entity's own identifier (kebab-case for Role/Hook/Team, CamelCase for Workflow/Task/Relay)
- `parent` — the parent ref in the chain; `NoParent` is a ZST (zero bytes)
- `_marker` — zero bytes; declares the struct as covariant in T; no data stored

`T` is compile-time only. The struct carries no runtime value for T — `T::KIND` and `T::PREFIX` are available in generic contexts via monomorphization.

The parent chain is semantic identity, not storage nesting. A `Task` may be "under" a `Workflow` in the domain model because its `EntityRef<Task, WorkflowParent>` says so, while the Store still indexes that task as a standalone entry by its full ref.

---

## Construction

```rust
impl<T: Entity, P: ParentKind> EntityRef<T, P> {
    pub fn new(id: impl Into<String>, parent: P) -> Self {
        Self { id: id.into(), parent, _marker: PhantomData }
    }
}

// Top-level entity (P = NoParent inferred)
let role_ref = EntityRef::<Role>::new("eng-lead", NoParent);

// Embedded entity with parent
let task_ref = EntityRef::<Task, _>::new("WriteProposal", WorkflowParent::Workflow(workflow_ref));

// Nested sub-workflow
let sub_wf_ref = EntityRef::<EmbeddedWorkflow, _>::new(
    "OnboardingFlow",
    WorkflowParent::Workflow(parent_workflow_ref),
);
```

The constructor model should stay generic: when a parent is part of the ref type, construction should take the parent entity ref (wrapped in the appropriate `ParentKind`) rather than relying on hierarchy-specific helpers.

---

## Accessing the Parent

```rust
impl<T: Entity, P: ParentKind> EntityRef<T, P> {
    pub fn parent(&self) -> &P { &self.parent }
}
```

`parent()` returns `&NoParent` (meaningless ZST) for top-level entities, or the appropriate `WorkflowParent` chain for hierarchy-bearing entities.

---

## Hash and Eq

`T::KIND`, `id`, and the full parent chain are all included in `Hash` and `Eq`. Two refs are equal only if they share the same entity type, id, and parent chain:

```rust
impl<T: Entity, P: ParentKind + Hash> Hash for EntityRef<T, P> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        T::KIND.hash(state);
        self.id.hash(state);
        self.parent.hash(state);
    }
}
```

`T::KIND` is baked in at compile time via monomorphization — no stored field needed.

---

## Serde

`T::KIND` provides the `type` tag in the wire format at compile time. Full wire format design is in [10 · entity-ref-serde](entity-ref-serde.md).
