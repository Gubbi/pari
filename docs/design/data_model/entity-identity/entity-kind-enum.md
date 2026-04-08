# entity-kind-enum

**Data Model → `data_model/entity-identity/`**

---

## Purpose

`EntityKind` is a runtime enum that identifies the type of any entity that has an `EntityRef`. It is used for hash discrimination, runtime dispatch, and serde wire format type tags.

---

## Variants

```rust
pub enum EntityKind {
    Role,
    Hook,
    Team,
    ArtifactKind,
    Workflow,
    ReusableWorkflow,
    EmbeddedWorkflow,  // embedded-only; lives inside Workflow/ReusableWorkflow/EmbeddedWorkflow steps
    Task,              // embedded-only; lives inside Workflow/ReusableWorkflow/EmbeddedWorkflow steps
    Relay,             // embedded-only; lives inside Workflow/EmbeddedWorkflow steps (not ReusableWorkflow)
}
```

All entity types with an `EntityRef` get a variant — including the embedded-only types, which are not independently loadable. "Independently loadable" is a separate property not encoded in `EntityKind`.

`WorkStepDefinition` is an enum wrapper, not itself an entity with an `EntityRef`. It has no `EntityKind` variant.

---

## Uses

### 1. Hash and Eq discrimination

`EntityRef` includes `EntityKind` in its `Hash` and `Eq` implementations. This ensures refs to different entity types with the same id string do not collide:

```rust
EntityRef<Role>("eng-lead")  ≠  EntityRef<Hook>("eng-lead")
// Different EntityKind → different hash → no collision
```

### 2. Runtime dispatch via `AnyEntityRef`

`AnyEntityRef` carries an `EntityKind` (via its enum variant) to route store lookups. The store's flat `HashMap<AnyEntityRef, TrackedEntity>` uses `AnyEntityRef` as the key — `kind()` on a key identifies which `TrackedEntity` variant to expect in the value. For embedded entities (EmbeddedWorkflow, Task, Relay), the `parent()` accessor on `AnyEntityRef` provides the parent chain needed to navigate context.

---

## Naming Note

`EntityKind` (runtime enum) is distinct from `Entity` (compile-time trait). They address the same concept at different levels — the enum carries the kind as a runtime value; the trait provides `const KIND: EntityKind` at compile time. See [105 · entity-kind-naming](../../codegen/entity-kind-naming.md).
