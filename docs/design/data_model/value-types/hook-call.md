# hook-call

**Data Model → `data_model/value-types/`**

---

## Purpose

`HookCall` is the usage-site reference to a hook — which hook to invoke and how to bind its inputs. It appears as the value type in `HooksMap`, keyed by trigger name.

---

## Definition

```rust
pub struct HookCall {
    pub hook: EntityRef<Hook>,
    pub with: Option<HashMap<String, String>>,
}
```

- `hook` — identifies which hook to invoke
- `with` — binds values to the hook's declared inputs by name; absent when the hook has no inputs

---

## Input Binding

Keys in `with` correspond to `HookInput.name` values on the referenced hook. Validation checks that all required inputs are bound and no unknown keys are present.

---

## Usage via Intercepts

`HookCall` is the value type in `Intercepts`:

```rust
pub type Intercepts = HashMap<Trigger, HookCall>;
```

`Trigger` is an enum of lifecycle events (variants defined alongside the entities that use them). Entities that support hooks carry `intercepts: Option<Intercepts>`.

Example in YAML:

```yaml
intercepts:
  on_complete:
    hook: { id: "notify-slack", kind: "Hook" }
    with:
      channel: "#general"
      message: "Task completed"
  on_start:
    hook: { id: "log-event", kind: "Hook" }
```
