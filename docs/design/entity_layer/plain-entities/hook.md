# hook-plain

**Owning layer: `entity`**

---

## Purpose

`Hook` is a top-level entity representing an invocable unit of work — a set of instructions for an agent or executor to follow. Hooks are referenced from entity intercepts via `HookCall` and invoked at lifecycle trigger points.

---

## Definition

```rust
pub struct Hook {
    pub entity_ref: EntityRef<Hook>,
    pub name: String,
    pub description: Option<String>,
    pub instructions: Vec<String>,
    pub inputs: Option<Vec<HookInput>>,
    pub extensions: Extensions,
}

pub struct HookInput {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}
```

---

## Fields

- `entity_ref` — carries the hook's id and kind; top-level entity, defaults to `NoParent`
- `name` — human-readable display name
- `description` — optional short summary
- `instructions` — ordered steps the agent or executor follows when the hook is invoked; may include script invocations, output interpretation, and conditional branching guidance
- `inputs` — declared input slots; absent when the hook requires no external values; when present, each slot declares whether it is required; callers bind values via `HookCall.with` (see [15 · hook-call](../value-types/hook-call.md))
- `extensions` — open-ended metadata; only `x-` prefixed keys are permitted (see [13 · extensions](../value-types/extensions.md))

---

## `HookInput`

Each input declares a named slot. `name` is the binding key; `description` documents the expected value. `required: true` means the caller must provide a binding in `HookCall.with`; `required: false` means it is optional and the hook handles its absence.
