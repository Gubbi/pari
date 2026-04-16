# raci

**Owning layer: `entity`**

---

## Purpose

`Raci` captures accountability assignments for workflows and tasks. Each field holds one or more `EntityRef<Role>` references — not raw strings — giving type safety and enabling cross-reference validation and navigation.

---

## Definition

```rust
pub struct Raci {
    pub responsible: Vec<EntityRef<Role>>,
    pub accountable: EntityRef<Role>,
    pub consulted: Option<Vec<EntityRef<Role>>>,
    pub informed: Option<Vec<EntityRef<Role>>>,
}
```

- `responsible` — one or more roles who do the work; required, at least one
- `accountable` — exactly one role who owns the outcome
- `consulted` — roles whose input is sought; optional
- `informed` — roles kept in the loop; optional

---

## Why `EntityRef<Role>` Not `String`

**Type safety** — the compiler rejects any non-Role ref in a Raci field. `EntityRef<Hook>` or a bare string cannot appear here.

**Validation** — at load time, each ref can be checked against the store to confirm the role exists. A string id would require a manual lookup with no compiler-enforced contract.

**Navigation** — code holding a `Raci` can load the actual `Role` via the ref without a separate id-to-role resolution step.

**Self-describing wire format** — the serialized form includes the kind tag (`"kind": "Role"`), making mismatches detectable on deserialization rather than silently passing through as opaque strings.

---

## Usage

`Raci` appears as the `raci` field on `Workflow` (required) and `Task` (optional). The field is named `raci` to avoid confusion with the `accountable` field inside the struct itself:

```rust
pub struct WorkflowDef {
    pub raci: Raci,
    // ...
}

pub struct Task {
    pub raci: Option<Raci>,
    // ...
}
```
