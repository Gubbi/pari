# Task 07 — Per-Entity Async Validators

## Scope

Implement all entity-specific validation schemas and rule functions. Wire each schema into the entity's `Entity::validation_schema()` function via `OnceLock`. Implement the cross-entity primitives that require `EntityServer` access (`ref_exists`, `all_refs_exist`, `hook_call_inputs_valid`, `raci_roles_exist`).

Covered entities: `Role`, `Hook`, `Team`, `ArtifactKind`, `Workflow`, `ReusableWorkflow`, `EmbeddedWorkflow`, `Task`, `Relay`.

---

## Files

- `src/validation/cross_entity.rs` — shared cross-entity primitives
- `src/validation/role.rs` — `ROLE_VALIDATION_SCHEMA` and any Role-specific additional structural rules
- `src/validation/hook.rs` — `HOOK_VALIDATION_SCHEMA` and Hook-specific structural rules
- `src/validation/team.rs` — `TEAM_VALIDATION_SCHEMA`, Team-specific structural rules, cycle detection
- `src/validation/artifact_kind.rs` — `ARTIFACT_KIND_VALIDATION_SCHEMA`
- `src/validation/workflow.rs` — `WORKFLOW_VALIDATION_SCHEMA`, `REUSABLE_WORKFLOW_VALIDATION_SCHEMA`, `EMBEDDED_WORKFLOW_VALIDATION_SCHEMA`, workflow semantic rules
- `src/validation/task.rs` — `TASK_VALIDATION_SCHEMA`
- `src/validation/relay.rs` — `RELAY_VALIDATION_SCHEMA`, relay-specific structural rules
- `src/validation/mod.rs` — module declarations
- `src/entities/role.rs`, `hook.rs`, `team.rs`, `artifact_kind.rs`, `workflow.rs`, `task.rs`, `relay.rs` — update `Entity::validation_schema()` on each type to call the corresponding `*_validation_schema()` constructor
- `src/lib.rs` — reorganize as `src/validation/mod.rs`

---

## Dependencies

- Task 05: All plain entity types and TrackedX types
- Task 06: `ValidationSchema<E>`, `RuleViolation`, `ValidationErrors`, `run_validations`, all structural primitives
- Task 09: `EntityServer` channel (for `ref_exists` and store queries) — **stubs for now; real impl in Task 09**

---

## Cross-Entity Primitives (`src/validation/cross_entity.rs`)

These require store access via `EntityServer::sender()`. For this task, define the signatures and a **stub implementation** that returns `vec![]` (no violations). Task 09 replaces the bodies with real store calls.

```rust
use crate::entity::{Entity, EntityRef, ParentKind};
use crate::types::{Raci, HookCall};
use crate::entities::hook::Hook;
use super::RuleViolation;

/// Checks that `entity_ref` exists in the store.
/// Stub: returns empty (no violation). Task 09 replaces with has_ref check.
pub async fn ref_exists<T: Entity, P: ParentKind>(entity_ref: &EntityRef<T, P>) -> Vec<RuleViolation> {
    // TODO Task 09: EntityServer::sender().has_ref(entity_ref).await
    vec![]
}

/// Checks that all refs in a slice exist. sub_path "[{i}]" for each missing.
pub async fn all_refs_exist<T: Entity>(refs: &[EntityRef<T>]) -> Vec<RuleViolation> {
    let mut v = vec![];
    for (i, r) in refs.iter().enumerate() {
        let sub = ref_exists(r).await;
        v.extend(sub.into_iter().map(|viol| RuleViolation::sub(format!("[{i}]"), viol.message)));
    }
    v
}

/// Validates HookCall.with bindings against Hook.inputs.
/// Stub: returns empty. Task 09 replaces with full validation.
pub async fn hook_call_inputs_valid(hook_call: &HookCall) -> Vec<RuleViolation> {
    // TODO Task 09: load hook, check with keys vs declared inputs
    vec![]
}

/// Checks all role refs in a Raci exist.
pub async fn raci_roles_exist(raci: &Raci) -> Vec<RuleViolation> {
    let mut v = vec![];
    v.extend(all_refs_exist(&raci.responsible).await.into_iter()
        .map(|viol| RuleViolation::sub(format!(".responsible{}", viol.sub_path.as_deref().unwrap_or("")), viol.message)));
    let accountable_check = ref_exists(&raci.accountable).await;
    v.extend(accountable_check.into_iter()
        .map(|viol| RuleViolation::sub(".accountable", viol.message)));
    if let Some(consulted) = &raci.consulted {
        v.extend(all_refs_exist(consulted).await.into_iter()
            .map(|viol| RuleViolation::sub(format!(".consulted{}", viol.sub_path.as_deref().unwrap_or("")), viol.message)));
    }
    if let Some(informed) = &raci.informed {
        v.extend(all_refs_exist(informed).await.into_iter()
            .map(|viol| RuleViolation::sub(format!(".informed{}", viol.sub_path.as_deref().unwrap_or("")), viol.message)));
    }
    v
}
```

---

## Role Schema (`src/validation/role.rs`)

```rust
use crate::validation::*;
use crate::entities::role::TrackedRole;

fn opt_non_empty_str(value: &Option<String>) -> Vec<RuleViolation> {
    match value {
        None => vec![],
        Some(s) => non_empty_str(s),
    }
}

fn each_item_non_empty_str(value: &Option<Vec<String>>) -> Vec<RuleViolation> {
    match value {
        None => vec![],
        Some(items) => items.iter().enumerate()
            .filter(|(_, s)| s.trim().is_empty())
            .map(|(i, _)| RuleViolation::sub(format!("[{i}]"), "must not be empty"))
            .collect(),
    }
}

pub fn role_validation_schema() -> ValidationSchema<crate::entities::role::Role> {
    let mut schema = ValidationSchema::empty();

    schema.structural.insert("entity_ref", vec![
        Box::new(|e: &TrackedRole| e.entity_ref().map_or(vec![], |r| kebab_case_id(r)))
    ]);
    // ... similar closures for name, description, purpose, traits, extensions
    // Full closure list:
    schema.structural.insert("name", vec![
        Box::new(|e: &TrackedRole| e.name.get().map(|v| non_empty_str(v)).unwrap_or_default())
    ]);
    schema.structural.insert("description", vec![
        Box::new(|e: &TrackedRole| e.description.get().map(|v| opt_non_empty_str(v)).unwrap_or_default())
    ]);
    schema.structural.insert("purpose", vec![
        Box::new(|e: &TrackedRole| e.purpose.get().map(|v| non_empty_str(v)).unwrap_or_default())
    ]);
    schema.structural.insert("traits", vec![
        Box::new(|e: &TrackedRole| e.traits.get().map(|v| each_item_non_empty_str(v)).unwrap_or_default())
    ]);
    schema.structural.insert("extensions", vec![
        Box::new(|e: &TrackedRole| e.extensions.get().map(|v| x_prefix_keys(v)).unwrap_or_default())
    ]);

    schema
}
```

**Note on `entity_ref` field access**: `entity_ref` is not wrapped in `TrackedField` — it is always present. The closure accesses it via `e.entity_ref()` (sync accessor) and calls `kebab_case(e.entity_ref().id())` directly.

---

## Hook Schema (`src/validation/hook.rs`)

Rules for `inputs` field:

```rust
fn hook_inputs_structural(value: &Option<Vec<crate::entities::hook::HookInput>>) -> Vec<RuleViolation> {
    let Some(inputs) = value else { return vec![]; };
    let mut v = vec![];
    // each_name_non_empty
    for (i, inp) in inputs.iter().enumerate() {
        if inp.name.trim().is_empty() {
            v.push(RuleViolation::sub(format!("[{i}].name"), "must not be empty"));
        }
    }
    // each_description_non_empty (when present)
    for (i, inp) in inputs.iter().enumerate() {
        if let Some(desc) = &inp.description {
            if desc.trim().is_empty() {
                v.push(RuleViolation::sub(format!("[{i}].description"), "must not be empty"));
            }
        }
    }
    // unique_input_names
    let names: Vec<&str> = inputs.iter().map(|i| i.name.as_str()).collect();
    let unique_violations = unique_by(&names, |n| n.to_string());
    if !unique_violations.is_empty() {
        v.push(RuleViolation::field("input names must be unique"));
    }
    v
}
```

Schema: `entity_ref`, `name`, `description` (structural only; no semantic, no cross-entity).

---

## Team Schema (`src/validation/team.rs`)

```rust
fn unique_member_handles(value: &Option<Vec<crate::entities::team::TeamMember>>) -> Vec<RuleViolation> {
    let Some(members) = value else { return vec![]; };
    let violations = unique_by(members, |m| m.handle.clone());
    if violations.is_empty() { vec![] }
    else { vec![RuleViolation::field("member handles must be unique")] }
}

// Cross-entity rules (async, stub bodies):
async fn member_roles_exist(entity: &TrackedTeam) -> Vec<RuleViolation> {
    let Some(members) = entity.members.get() else { return vec![]; };
    let Some(members) = members.as_ref() else { return vec![]; };
    let mut v = vec![];
    for (i, m) in members.iter().enumerate() {
        let sub = ref_exists(&m.role).await;
        v.extend(sub.into_iter().map(|viol| RuleViolation::sub(format!("[{i}].role"), viol.message)));
    }
    v
}

async fn include_teams_exist(entity: &TrackedTeam) -> Vec<RuleViolation> {
    let Some(include) = entity.include.get() else { return vec![]; };
    let Some(include) = include.as_ref() else { return vec![]; };
    let mut v = vec![];
    for (i, (team_ref, _)) in include.iter().enumerate() {
        let sub = ref_exists(team_ref).await;
        v.extend(sub.into_iter().map(|viol| RuleViolation::sub(format!("[{i}]"), viol.message)));
    }
    v
}

// no_include_cycle, no_import_cycle — BFS (stub bodies, real impl in Task 09)
async fn no_include_cycle(entity: &TrackedTeam) -> Vec<RuleViolation> {
    // TODO Task 09: BFS over include keys via EntityServer load
    vec![]
}

async fn no_import_cycle(entity: &TrackedTeam) -> Vec<RuleViolation> {
    // TODO Task 09: BFS over import entries via EntityServer load
    vec![]
}
```

---

## ArtifactKind Schema (`src/validation/artifact_kind.rs`)

Structural only: `entity_ref` (kebab), `name` (non-empty), `description` (opt non-empty), `service` (non-empty), `access` (opt non-empty), `guidance` (opt non-empty), `extensions` (x-prefix).

No semantic, no cross-entity rules.

---

## Workflow Semantic Rules

```rust
// depends_on_valid — for each work step with depends_on,
// every listed id must be a key in steps appearing before this step.
async fn depends_on_valid(entity: &TrackedWorkflow) -> Vec<RuleViolation> {
    let Some(steps) = entity.steps.get() else { return vec![]; };
    let step_keys: Vec<&str> = steps.keys().map(|k| k.as_str()).collect();
    let mut v = vec![];
    for (step_id, step) in steps.iter() {
        let depends_on = match step {
            Step::Task { depends_on, .. } | Step::Relay { depends_on, .. }
            | Step::EmbeddedWorkflow { depends_on, .. } => depends_on.as_deref().unwrap_or(&[]),
            Step::Review { .. } => continue,
        };
        let step_pos = step_keys.iter().position(|k| *k == step_id).unwrap();
        for (i, dep) in depends_on.iter().enumerate() {
            let dep_pos = step_keys.iter().position(|k| *k == dep);
            match dep_pos {
                Some(pos) if pos < step_pos => {} // valid
                _ => v.push(RuleViolation::sub(
                    format!(".{step_id}.depends_on[{i}]"),
                    format!("'{dep}' is not a prior step")
                )),
            }
        }
    }
    v
}

// on_reject_valid — for each Review step, on_reject must be a key in steps.
async fn on_reject_valid(entity: &TrackedWorkflow) -> Vec<RuleViolation> {
    let Some(steps) = entity.steps.get() else { return vec![]; };
    let mut v = vec![];
    for (step_id, step) in steps.iter() {
        let Step::Review { on_reject, .. } = step else { continue; };
        if !steps.contains_key(on_reject) {
            v.push(RuleViolation::sub(
                format!(".{step_id}.on_reject"),
                format!("'{on_reject}' is not a step id")
            ));
        }
    }
    v
}

// reviewing_state_required — if any Review step present, states must have a Reviewing semantic.
async fn reviewing_state_required(entity: &TrackedWorkflow) -> Vec<RuleViolation> {
    let Some(steps) = entity.steps.get() else { return vec![]; };
    let has_review = steps.values().any(|s| matches!(s, Step::Review { .. }));
    if !has_review { return vec![]; }
    let Some(states) = entity.states.get() else { return vec![]; };
    let has_reviewing = states.iter().any(|s| matches!(s.semantic, Some(WorkflowSemantic::Reviewing)));
    if has_reviewing { vec![] }
    else { vec![RuleViolation::field("at least one Reviewing state required when review steps are present")] }
}
```

---

## Workflow Cross-Entity Rules

```rust
// work_step_refs_exist — for each Task/Relay/EmbeddedWorkflow step, entity_ref must exist.
async fn work_step_refs_exist(entity: &TrackedWorkflow) -> Vec<RuleViolation> {
    let Some(steps) = entity.steps.get() else { return vec![]; };
    let mut v = vec![];
    for (step_id, step) in steps.iter() {
        let missing = match step {
            Step::Task { entity_ref, .. } => ref_exists(entity_ref).await,
            Step::Relay { entity_ref, .. } => ref_exists(entity_ref).await,
            Step::EmbeddedWorkflow { entity_ref, .. } => ref_exists(entity_ref).await,
            Step::Review { .. } => vec![],
        };
        v.extend(missing.into_iter().map(|viol|
            RuleViolation::sub(format!(".{step_id}.entity_ref"), viol.message)
        ));
    }
    v
}

// review_approver_roles_exist — for each Review step, all approver role refs must exist.
async fn review_approver_roles_exist(entity: &TrackedWorkflow) -> Vec<RuleViolation> {
    let Some(steps) = entity.steps.get() else { return vec![]; };
    let mut v = vec![];
    for (step_id, step) in steps.iter() {
        let Step::Review { approver, .. } = step else { continue; };
        for (i, role_ref) in approver.iter().enumerate() {
            let sub = ref_exists(role_ref).await;
            v.extend(sub.into_iter().map(|viol|
                RuleViolation::sub(format!(".{step_id}.approver[{i}]"), viol.message)
            ));
        }
    }
    v
}

// no_relay_in_tree — for ReusableWorkflow: BFS; reports single error at "steps".
// Stub for Task 07; real BFS uses EntityServer in Task 09.
async fn no_relay_in_tree(entity: &TrackedReusableWorkflow) -> Vec<RuleViolation> {
    let Some(steps) = entity.steps.get() else { return vec![]; };
    for step in steps.values() {
        if let Step::Relay { .. } = step {
            return vec![RuleViolation::field("Relay steps are not permitted in ReusableWorkflow")];
        }
    }
    // TODO Task 09: recursively check EmbeddedWorkflow steps via EntityServer load
    vec![]
}
```

---

## Relay Schema (`src/validation/relay.rs`)

```rust
fn non_empty_map_state_map(value: &std::collections::HashMap<String, crate::entities::relay::StateMapEntry>) -> Vec<RuleViolation> {
    if value.is_empty() {
        vec![RuleViolation::field("state_map must not be empty")]
    } else {
        vec![]
    }
}

fn camel_case_state_keys(value: &std::collections::HashMap<String, crate::entities::relay::StateMapEntry>) -> Vec<RuleViolation> {
    value.keys()
        .flat_map(|k| camel_case(k).into_iter().map(|viol| RuleViolation::sub(format!(".{k}"), viol.message)))
        .collect()
}

// delegates_to_exists — ref_exists on delegates_to
// maps_to_states_exist — load delegates_to ReusableWorkflow, check each maps_to value
// Stub for Task 07; real load in Task 09.
async fn maps_to_states_exist(entity: &TrackedRelay) -> Vec<RuleViolation> {
    // TODO Task 09: load delegates_to, check StateMapEntry.maps_to vs workflow states
    vec![]
}
```

---

## Wiring `Entity::validation_schema()`

`ValidationSchema<E>` contains `Box<dyn Fn(...)>` closures and cannot be `const`. The `Entity` trait uses a function instead (approved change from Task 02 design):

```rust
pub trait Entity: Sized + 'static {
    const KIND: EntityKind;
    fn validation_schema() -> &'static ValidationSchema<Self>;
    type Parent: ParentKind;
    type Tracked: TrackedEntity<Entity = Self>;
}
```

The `#[derive(Entity)]` macro (Task 03) generates a `OnceLock`-backed static for each entity type. This task populates it with real rules via a `*_validation_schema()` constructor function:

```rust
// Generated by #[derive(Entity)] — update Task 03 macro to emit this form:
impl Entity for Role {
    const KIND: EntityKind = EntityKind::Role;
    type Parent = NoParent;
    type Tracked = TrackedRole;

    fn validation_schema() -> &'static ValidationSchema<Role> {
        static SCHEMA: OnceLock<ValidationSchema<Role>> = OnceLock::new();
        SCHEMA.get_or_init(|| crate::validation::role::role_validation_schema())
    }
}
```

Each `src/validation/<entity>.rs` exposes `pub fn <entity>_validation_schema() -> ValidationSchema<E>` that constructs and returns the schema. The `OnceLock` ensures it's built once.

---

## TDD: Tests to Write First

```rust
// tests/validate_entities.rs
use pari::entities::role::{Role, TrackedRole};
use pari::entity::{Entity, EntityRef};
use pari::validation::{run_validations, ValidationKind, ValidationErrors};
use std::collections::HashMap;

// Helper: fully initialize a TrackedRole from a plain role
fn tracked_role(id: &str, name: &str, purpose: &str) -> TrackedRole {
    let plain = Role {
        entity_ref:  EntityRef::new(id),
        name:        name.to_string(),
        description: None,
        purpose:     purpose.to_string(),
        traits:      None,
        extensions:  HashMap::new(),
    };
    TrackedRole::from(plain)
}

// --- Role structural validation ---

#[tokio::test]
async fn role_valid_passes_all_structural() {
    let tracked = tracked_role("eng-lead", "Engineering Lead", "Leads engineering");
    let errors = run_validations::<Role>(&tracked, &[], &[ValidationKind::Structural]).await;
    assert!(errors.is_empty(), "{:?}", errors.errors);
}

#[tokio::test]
async fn role_invalid_id_fails_structural() {
    let tracked = tracked_role("EngLead", "Engineering Lead", "Leads engineering");
    let errors = run_validations::<Role>(&tracked, &["entity_ref"], &[ValidationKind::Structural]).await;
    assert!(!errors.is_empty());
    assert!(errors.errors.iter().any(|e| e.path.contains("entity_ref")));
}

#[tokio::test]
async fn role_empty_name_fails_structural() {
    let tracked = tracked_role("eng-lead", "", "Leads engineering");
    let errors = run_validations::<Role>(&tracked, &["name"], &[ValidationKind::Structural]).await;
    assert!(!errors.is_empty());
    assert!(errors.errors.iter().any(|e| e.path == "name"));
}

#[tokio::test]
async fn role_empty_purpose_fails_structural() {
    let tracked = tracked_role("eng-lead", "Eng Lead", "");
    let errors = run_validations::<Role>(&tracked, &["purpose"], &[ValidationKind::Structural]).await;
    assert!(!errors.is_empty());
}

#[tokio::test]
async fn role_non_x_extension_fails_structural() {
    use serde_json::json;
    let mut ext = HashMap::new();
    ext.insert("owner".to_string(), json!("alice"));
    let plain = Role {
        entity_ref:  EntityRef::new("eng-lead"),
        name:        "Eng Lead".to_string(),
        description: None,
        purpose:     "Purpose".to_string(),
        traits:      None,
        extensions:  ext,
    };
    let tracked = TrackedRole::from(plain);
    let errors = run_validations::<Role>(&tracked, &["extensions"], &[ValidationKind::Structural]).await;
    assert!(!errors.is_empty());
}

// --- Hook structural validation ---

#[tokio::test]
async fn hook_valid_passes_structural() {
    use pari::entities::hook::{Hook, TrackedHook, HookInput};
    let plain = Hook {
        entity_ref:   EntityRef::new("notify-slack"),
        name:         "Notify Slack".to_string(),
        description:  None,
        instructions: vec!["Send a message".to_string()],
        inputs:       Some(vec![HookInput { name: "channel".to_string(), description: None, required: true }]),
        extensions:   HashMap::new(),
    };
    let tracked = TrackedHook::from(plain);
    let errors = run_validations::<Hook>(&tracked, &[], &[ValidationKind::Structural]).await;
    assert!(errors.is_empty(), "{:?}", errors.errors);
}

#[tokio::test]
async fn hook_duplicate_input_names_fails_structural() {
    use pari::entities::hook::{Hook, TrackedHook, HookInput};
    let plain = Hook {
        entity_ref:   EntityRef::new("notify-slack"),
        name:         "Notify Slack".to_string(),
        description:  None,
        instructions: vec!["Send a message".to_string()],
        inputs:       Some(vec![
            HookInput { name: "channel".to_string(), description: None, required: true },
            HookInput { name: "channel".to_string(), description: None, required: false }, // duplicate
        ]),
        extensions:   HashMap::new(),
    };
    let tracked = TrackedHook::from(plain);
    let errors = run_validations::<Hook>(&tracked, &["inputs"], &[ValidationKind::Structural]).await;
    assert!(!errors.is_empty());
}

// --- run_validations field filtering ---

#[tokio::test]
async fn run_validations_only_runs_requested_fields() {
    // Make name invalid but only run "purpose" field — no errors for name
    let tracked = tracked_role("eng-lead", "", "Valid purpose");
    let errors = run_validations::<Role>(&tracked, &["purpose"], &[ValidationKind::Structural]).await;
    assert!(errors.is_empty(), "should not have checked 'name'");
}

#[tokio::test]
async fn run_validations_empty_fields_runs_all() {
    let tracked = tracked_role("eng-lead", "", "Valid purpose");
    let errors = run_validations::<Role>(&tracked, &[], &[ValidationKind::Structural]).await;
    assert!(!errors.is_empty(), "should have found empty name");
}

// --- Workflow semantic validation ---

#[tokio::test]
async fn workflow_on_reject_valid_passes_when_target_exists() {
    // Build a workflow with a Review step whose on_reject points to a prior step
    // ... (construct TrackedWorkflow from plain)
    // Assertion: no semantic violations for "steps"
}

#[tokio::test]
async fn workflow_reviewing_state_required_when_review_steps_present() {
    // Build a workflow with a Review step but no Reviewing state
    // Assertion: reviewing_state_required fires
}

// --- Relay structural validation ---

#[tokio::test]
async fn relay_empty_state_map_fails_structural() {
    use pari::entities::relay::{Relay, TrackedRelay};
    use pari::entity::EntityRef;
    use pari::entities::workflow::ReusableWorkflow;
    let plain = Relay {
        entity_ref:   EntityRef::new_embedded("HandoffToReview", "InitiativeWorkflow"),
        name:         "Handoff".to_string(),
        description:  None,
        purpose:      "Delegate to review workflow".to_string(),
        raci:         None,
        delegates_to: EntityRef::new("review-workflow"),
        briefing:     None,
        debriefing:   None,
        state_map:    HashMap::new(), // empty — invalid
        intercepts:   None,
        guidance:     None,
        extensions:   HashMap::new(),
    };
    let tracked = TrackedRelay::from(plain);
    let errors = run_validations::<Relay>(&tracked, &["state_map"], &[ValidationKind::Structural]).await;
    assert!(!errors.is_empty());
}

#[tokio::test]
async fn relay_non_camel_case_state_key_fails_structural() {
    use pari::entities::relay::{Relay, TrackedRelay, StateMapEntry};
    use pari::entity::EntityRef;
    let mut state_map = HashMap::new();
    state_map.insert("in-progress".to_string(), StateMapEntry { // kebab — invalid
        maps_to: "Active".to_string(),
        description: None,
        semantic: None,
    });
    let plain = Relay {
        entity_ref:   EntityRef::new_embedded("R1", "WF1"),
        name:         "R".to_string(),
        description:  None,
        purpose:      "P".to_string(),
        raci:         None,
        delegates_to: EntityRef::new("rwf"),
        briefing:     None,
        debriefing:   None,
        state_map,
        intercepts:   None,
        guidance:     None,
        extensions:   HashMap::new(),
    };
    let tracked = TrackedRelay::from(plain);
    let errors = run_validations::<Relay>(&tracked, &["state_map"], &[ValidationKind::Structural]).await;
    assert!(!errors.is_empty());
}
```

---

## Implementation Notes

### `fn validation_schema()` — approved design

The `Entity` trait uses `fn validation_schema() -> &'static ValidationSchema<Self>` (approved change from Task 02 design; `Box<dyn Fn>` makes `const` impossible). This task updates:
1. `#[derive(Entity)]` macro in `pari-macros/src/lib.rs` (Task 03 update) — emit `OnceLock`-backed stub calling `*_validation_schema()`
2. All entity impls — wired via the macro

The macro generates `fn validation_schema()` that initializes a `OnceLock<ValidationSchema<Self>>` by calling the entity's schema constructor from `src/validation/<entity>.rs`.

### Uninitialized Field Behavior

All rule closures guard with `field.get().map(|v| rule(v)).unwrap_or_default()`. An uninitialized field produces no violations — by design. Rules only run on fields that have been loaded.

### `entity_ref` in Structural Closures

`entity_ref` is a sync field, not a `TrackedField`. The closure accesses it directly:
```rust
Box::new(|e: &TrackedRole| kebab_case(e.entity_ref().id()))
```

### Async Rule Closures

Semantic and cross-entity rules are stored as `Box<dyn Fn(&TrackedX) -> Pin<Box<dyn Future<...>>>>`. Each named async function (e.g. `async fn depends_on_valid`) is wrapped:
```rust
Box::new(|e: &TrackedWorkflow| Box::pin(depends_on_valid(e)) as Pin<Box<dyn Future<...>>>)
```

The schema construction function registers these wrapped closures.

---

## Acceptance Criteria

- `cargo test validate_entities` passes
- `run_validations::<Role>` correctly reports structural violations for invalid id, empty name, empty purpose, non-x extension keys
- `run_validations::<Hook>` correctly reports duplicate input names
- `run_validations` with `fields: &["name"]` only runs name rules; other fields not checked
- `reviewing_state_required` fires when review steps present but no reviewing state
- `depends_on_valid` fires when a step's depends_on references a non-existent or non-prior step
- `relay` state_map: empty map and non-CamelCase keys fail structural
- Cross-entity stubs compile and return empty violations (full impl deferred to Task 09)
- Task 02-06 tests still pass
