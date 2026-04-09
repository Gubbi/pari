# src/validation — Validation Framework

## ValidationSchema<E>

Each entity has a `&'static ValidationSchema<E>` returned by `Entity::validation_schema()`.
Built with three rule maps — rules run in order, all violations accumulated before returning.

```rust
ValidationSchema<E> {
    structural:   IndexMap<&'static str, Box<dyn Fn(&E) -> Vec<RuleViolation> + Send + Sync>>,
    semantic:     IndexMap<&'static str, Box<dyn Fn(&E, &ValidationContext) -> BoxFuture<Vec<RuleViolation>> + Send + Sync>>,
    cross_entity: IndexMap<&'static str, Box<dyn Fn(&E, &ValidationContext) -> BoxFuture<Vec<RuleViolation>> + Send + Sync>>,
}
```

**Rule kinds:**
- **Structural** — sync, no context needed; field format checks (kebab-case, non-empty, etc.)
- **Semantic** — async, requires `ValidationContext`; business logic checks
- **Cross-entity** — async, requires `ValidationContext`; referential integrity across entities

---

## Validation Entry Point

```rust
// src/validation/mod.rs
pub async fn run_validations<E: Entity>(
    entity: &E,
    context: &ValidationContext,
) -> Result<(), ValidationErrors>
```

Runs all three rule kinds in sequence. Returns `Err(ValidationErrors)` if any violation found.

---

## RuleViolation

```rust
pub struct RuleViolation {
    pub message:  String,
    pub sub_path: Option<String>,  // e.g. "steps[0].id"
}
```

---

## Structural Primitives (`src/validation/mod.rs`)

```rust
kebab_case(val: &str)              -> Vec<RuleViolation>   // id format check
camel_case(val: &str)              -> Vec<RuleViolation>
non_empty_str(field: &str, val: &str)  -> Vec<RuleViolation>
non_empty_list(field: &str, list: &[T])  -> Vec<RuleViolation>
min_length(field: &str, val: &str, n: usize)  -> Vec<RuleViolation>
unique_by<T, K>(field: &str, items: &[T], key_fn: fn(&T) -> K)  -> Vec<RuleViolation>
x_prefix_keys(extensions: &Extensions)  -> Vec<RuleViolation>
states_valid_workflow(states: &[WorkflowStateEntry])  -> Vec<RuleViolation>
states_valid_task(states: &[TaskStateEntry])           -> Vec<RuleViolation>
raci_structural(raci: &Raci)       -> Vec<RuleViolation>
```

---

## Per-Entity Validation Modules

Each entity has a module (`role.rs`, `team.rs`, `hook.rs`, `artifact_kind.rs`, `task.rs`, `relay.rs`, `workflow.rs`) exporting a `*_validation_schema()` function:

```rust
// example from role.rs:
pub fn role_validation_schema() -> &'static ValidationSchema<Role> { ... }
```

`cross_entity.rs` holds cross-entity rules shared across modules.

---

## Error Types (`src/validation/error.rs`)

```rust
ValidationErrors   // Vec<FieldValidationError>; implements Display
FieldValidationError { field: String, violations: Vec<RuleViolation> }
SetterError        // returned by generated async setters when validation fails
```
