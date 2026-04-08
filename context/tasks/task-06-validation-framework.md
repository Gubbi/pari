# Task 06 — Validation Framework

## Scope

Implement the core validation infrastructure:

1. `ValidationSchema` — replaces the placeholder from Task 02; three maps of field → rules
2. `RuleViolation`, `ValidationErrors`, `FieldValidationError`, `ValidationKind` — error types
3. `StructuralRule<T>`, `SemanticRule<T>`, `CrossEntityRule<T>` — rule function type aliases
4. `run_validations` — the async runner that dispatches schema rules and aggregates errors
5. Full suite of shared structural primitive rule functions
6. `raci_structural` — shared structural primitive for Raci
7. `SetterError` and `LoadError` — replace stubs from Task 03

**Cross-entity primitives (`ref_exists`, `all_refs_exist`, `hook_call_inputs_valid`, `raci_roles_exist`) are in Task 07**, because they require the EntityServer channel for store access.

---

## Files

- `src/validation.rs` — new file; all types and functions listed above
- `src/entity.rs` — replace `ValidationSchema` stub and error type stubs
- `src/lib.rs` — `pub mod validation;`

---

## Dependencies

- Task 02: `Entity`, `EntityRef`, `AnyEntityRef`, `EntityKind`
- Task 05: `Extensions`, `Raci`, `WorkflowStateEntry`, `TaskStateEntry` (needed for structural primitives)

---

## Types and Signatures

### `src/validation.rs`

```rust
use std::collections::HashMap;
use std::hash::Hash;
use crate::entity::Entity;

// ---------------------------------------------------------------------------
// Rule violation — single violation from one rule
// ---------------------------------------------------------------------------

/// A single violation returned by a rule function.
/// `sub_path = None` means the violation is at the field itself.
/// `sub_path = Some("[0].role")` means a nested sub-field of the field.
pub struct RuleViolation {
    pub sub_path: Option<String>,
    pub message: String,
}

impl RuleViolation {
    pub fn field(message: impl Into<String>) -> Self {
        Self { sub_path: None, message: message.into() }
    }

    pub fn sub(sub_path: impl Into<String>, message: impl Into<String>) -> Self {
        Self { sub_path: Some(sub_path.into()), message: message.into() }
    }
}

// ---------------------------------------------------------------------------
// ValidationKind
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationKind {
    Structural,
    Semantic,
    CrossEntity,
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

pub struct FieldValidationError {
    /// Dot-notation path: "name", "steps.WriteProposal.depends_on[0]"
    pub path: String,
    pub message: String,
    pub kind: ValidationKind,
}

pub struct ValidationErrors {
    pub errors: Vec<FieldValidationError>,
}

impl ValidationErrors {
    pub fn new() -> Self { Self { errors: Vec::new() } }
    pub fn is_empty(&self) -> bool { self.errors.is_empty() }
    pub fn extend(&mut self, other: ValidationErrors) { self.errors.extend(other.errors); }
}

// ---------------------------------------------------------------------------
// SetterError (replaces stub from Task 03)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum SetterError {
    Substrate(SubstrateError),
    Validation(ValidationErrors),
}

/// Re-exported here so entity.rs can refer to it; SubstrateError full type in Task 11.
#[derive(Debug)]
pub struct SubstrateError {
    pub path: String,
    pub message: String,
}

// ---------------------------------------------------------------------------
// LoadError (replaces stub from Task 03)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum LoadError {
    NotLoaded,
    Substrate(SubstrateError),
    ValidationFailed(ValidationErrors),
}

// ---------------------------------------------------------------------------
// Rule function type aliases
// ---------------------------------------------------------------------------

/// Structural rule: sync, receives field value only.
pub type StructuralRule<T> = fn(&T) -> Vec<RuleViolation>;

/// Semantic rule: async, receives tracked entity; sibling fields load transparently.
/// Represented as a function pointer to an async fn.
pub type SemanticRule<T> = for<'a> fn(&'a T) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<RuleViolation>> + Send + 'a>>;

/// Cross-entity rule: async, receives tracked entity; may query store.
pub type CrossEntityRule<T> = for<'a> fn(&'a T) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<RuleViolation>> + Send + 'a>>;

// ---------------------------------------------------------------------------
// ValidationSchema — replaces the placeholder stub from Task 02
// ---------------------------------------------------------------------------

/// Per-entity validation schema.
/// Three maps from field name to lists of rules.
/// A field absent from a map has no rules of that kind — not an error.
pub struct ValidationSchema<T> {
    pub structural:   HashMap<&'static str, Vec<StructuralRule<T>>>,
    pub semantic:     HashMap<&'static str, Vec<SemanticRule<T>>>,
    pub cross_entity: HashMap<&'static str, Vec<CrossEntityRule<T>>>,
}

impl<T> ValidationSchema<T> {
    pub fn empty() -> Self {
        Self {
            structural:   HashMap::new(),
            semantic:     HashMap::new(),
            cross_entity: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// run_validations
// ---------------------------------------------------------------------------

/// Runs validation rules from the entity's schema.
///
/// `fields: &[]` means all fields present in the schema.
/// `fields: &["name", "purpose"]` runs only those fields.
/// `kinds` selects which rule kinds to run.
///
/// Errors accumulate — all failing rules are collected before returning.
pub async fn run_validations<T: Entity>(
    entity: &T::Tracked,
    fields: &[&str],
    kinds: &[ValidationKind],
) -> ValidationErrors
where
    T::Tracked: ValidatableTracked<T>,
{
    let schema = T::validation_schema();
    let mut result = ValidationErrors::new();

    // Determine which fields to check
    let all_fields: Vec<&str> = schema.all_field_names();
    let target_fields: Vec<&&str> = if fields.is_empty() {
        all_fields.iter().collect()
    } else {
        fields.iter().collect()
    };

    for field_name in target_fields {
        // Structural rules
        if kinds.contains(&ValidationKind::Structural) {
            if let Some(rules) = schema.structural.get(field_name) {
                let violations = entity.run_structural_rules(field_name, rules);
                for v in violations {
                    let path = build_path(field_name, &v.sub_path);
                    result.errors.push(FieldValidationError {
                        path,
                        message: v.message,
                        kind: ValidationKind::Structural,
                    });
                }
            }
        }

        // Semantic rules
        if kinds.contains(&ValidationKind::Semantic) {
            if let Some(rules) = schema.semantic.get(field_name) {
                for rule in rules {
                    let violations = rule(entity).await;
                    for v in violations {
                        let path = build_path(field_name, &v.sub_path);
                        result.errors.push(FieldValidationError {
                            path,
                            message: v.message,
                            kind: ValidationKind::Semantic,
                        });
                    }
                }
            }
        }

        // Cross-entity rules
        if kinds.contains(&ValidationKind::CrossEntity) {
            if let Some(rules) = schema.cross_entity.get(field_name) {
                for rule in rules {
                    let violations = rule(entity).await;
                    for v in violations {
                        let path = build_path(field_name, &v.sub_path);
                        result.errors.push(FieldValidationError {
                            path,
                            message: v.message,
                            kind: ValidationKind::CrossEntity,
                        });
                    }
                }
            }
        }
    }

    result
}

fn build_path(field: &str, sub_path: &Option<String>) -> String {
    match sub_path {
        None       => field.to_string(),
        Some(sub)  => format!("{field}{sub}"),
    }
}
```

**Note on `ValidationSchema<E>` and the `Entity` trait**: `ValidationSchema<E>` contains `Box<dyn Fn(...)>` closures and cannot be `const`. The `Entity` trait already declares this as a function (approved deviation from Task 02 design):

```rust
pub trait Entity: Sized + 'static {
    const KIND: EntityKind;
    fn validation_schema() -> &'static ValidationSchema<Self>;
    type Parent: ParentKind;
    type Tracked: TrackedEntity<Entity = Self>;
}
```

`run_validations` calls `T::validation_schema()`. The `ValidationSchema` stub from Task 02 (`pub struct ValidationSchema<E>(PhantomData<E>)`) is replaced entirely by the real generic type defined here. The `#[derive(Entity)]` macro generates a `OnceLock`-backed static for each entity type; Task 07 populates it with real rules.

### `ValidatableTracked` helper trait

The runner needs to call structural rules with the correct field value. Since `StructuralRule<T>` takes `&T` (the field value), the tracked entity must extract the field value by name at runtime. This is handled via a generated trait:

```rust
/// Implemented by #[derive(Entity)] on TrackedX.
/// Provides runtime dispatch from field name → structural rule execution.
pub trait ValidatableTracked<E: Entity> {
    /// Run structural rules for `field_name`, extracting the field value from self.
    /// Returns empty vec if the field is not initialized.
    fn run_structural_rules(
        &self,
        field_name: &str,
        rules: &[StructuralRule</* field type — dynamic dispatch */dyn std::any::Any>],
    ) -> Vec<RuleViolation>;
}
```

**Implementation deviation note**: The `StructuralRule<T>` type alias is generic over the field type `T`. A runner that dispatches over field names cannot hold a homogeneous `Vec<StructuralRule<T>>` for different field types.

Resolution: use `Box<dyn Fn(&dyn std::any::Any) -> Vec<RuleViolation>>` as the actual stored rule type, with a wrapper generated by `#[derive(Entity)]` that downcasts to the concrete field type before calling the rule. This is an internal detail — callers still write `fn camel_case(value: &str) -> Vec<RuleViolation>` as typed functions; the macro wraps them at schema construction time.

Alternatively, use a simpler approach: each entity's `ValidationSchema` stores `Box<dyn Fn(&TrackedX) -> Vec<RuleViolation>>` closures that capture both the rule and the field extraction. The macro generates these closures.

**For this task**: define the schema types with this dynamic dispatch approach:

```rust
pub type AnyStructuralRule<E> = Box<dyn Fn(&<E as Entity>::Tracked) -> Vec<RuleViolation> + Send + Sync>;
pub type AnySemanticRule<E>   = Box<dyn for<'a> Fn(&'a <E as Entity>::Tracked)
    -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<RuleViolation>> + Send + 'a>>
    + Send + Sync>;
pub type AnyCrossEntityRule<E> = AnySemanticRule<E>;

pub struct ValidationSchema<E: Entity> {
    pub structural:   HashMap<&'static str, Vec<AnyStructuralRule<E>>>,
    pub semantic:     HashMap<&'static str, Vec<AnySemanticRule<E>>>,
    pub cross_entity: HashMap<&'static str, Vec<AnyCrossEntityRule<E>>>,
}
```

The macro generates a `validation_schema()` function on each `TrackedX` (or as a method on the entity type) that returns a `&'static ValidationSchema<E>`. Rule registration closures capture field extraction (e.g. `|e: &TrackedRole| { let v = e.name.get()?; camel_case(v) }`).

---

## Shared Structural Primitives (`src/validation.rs`)

```rust
use crate::types::{Extensions, Raci, WorkflowStateEntry, TaskStateEntry};
use crate::entity::EntityRef;

// ---------------------------------------------------------------------------
// Structural primitives
// ---------------------------------------------------------------------------

/// Id must match [a-z0-9]+(-[a-z0-9]+)*
pub fn kebab_case(value: &str) -> Vec<RuleViolation> {
    let valid = !value.is_empty()
        && value.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--");
    if valid { vec![] }
    else { vec![RuleViolation::field(format!("'{value}' is not kebab-case"))] }
}

/// Id must match [A-Z][a-zA-Z0-9]*
pub fn camel_case(value: &str) -> Vec<RuleViolation> {
    let valid = value.starts_with(|c: char| c.is_ascii_uppercase())
        && value.chars().all(|c| c.is_ascii_alphanumeric());
    if valid { vec![] }
    else { vec![RuleViolation::field(format!("'{value}' is not CamelCase"))] }
}

/// EntityRef id must be kebab-case
pub fn kebab_case_id<T: Entity>(entity_ref: &EntityRef<T>) -> Vec<RuleViolation> {
    kebab_case(entity_ref.id())
}

/// EntityRef id must be CamelCase
pub fn camel_case_id<T: Entity>(entity_ref: &EntityRef<T>) -> Vec<RuleViolation> {
    camel_case(entity_ref.id())
}

/// String must not be empty or whitespace-only
pub fn non_empty_str(value: &str) -> Vec<RuleViolation> {
    if value.trim().is_empty() {
        vec![RuleViolation::field("must not be empty")]
    } else {
        vec![]
    }
}

/// Slice must have at least one element
pub fn non_empty_list<T>(value: &[T]) -> Vec<RuleViolation> {
    if value.is_empty() {
        vec![RuleViolation::field("must not be empty")]
    } else {
        vec![]
    }
}

/// Slice must have at least `min` elements
pub fn min_length<T>(value: &[T], min: usize) -> Vec<RuleViolation> {
    if value.len() < min {
        vec![RuleViolation::field(format!("must have at least {min} elements, got {}", value.len()))]
    } else {
        vec![]
    }
}

/// All elements must produce distinct keys via `key_fn`
pub fn unique_by<T, K: Eq + Hash>(value: &[T], key_fn: fn(&T) -> K) -> Vec<RuleViolation> {
    let mut seen = std::collections::HashSet::new();
    let mut violations = vec![];
    for (i, item) in value.iter().enumerate() {
        let key = key_fn(item);
        if !seen.insert(key) {
            violations.push(RuleViolation::sub(format!("[{i}]"), "duplicate entry"));
        }
    }
    violations
}

/// All keys in extensions must start with "x-"
pub fn x_prefix_keys(value: &Extensions) -> Vec<RuleViolation> {
    value.keys()
        .filter(|k| !k.starts_with("x-"))
        .map(|k| RuleViolation::sub(format!(".{k}"), "extension key must start with 'x-'"))
        .collect()
}

/// State list validation:
/// - All ids are CamelCase
/// - All ids are unique
/// - At least 2 states
/// - At least one Done semantic
/// - At least one non-Done state
pub fn states_valid_workflow(value: &[WorkflowStateEntry]) -> Vec<RuleViolation> {
    let mut v = vec![];
    v.extend(min_length(value, 2));
    for (i, s) in value.iter().enumerate() {
        v.extend(camel_case(&s.id).into_iter().map(|viol| RuleViolation::sub(format!("[{i}].id"), viol.message)));
    }
    v.extend(unique_by(value, |s| s.id.clone())
        .into_iter().map(|viol| RuleViolation { sub_path: viol.sub_path.map(|p| p + ".id"), ..viol }));
    let has_done = value.iter().any(|s| matches!(s.semantic, Some(crate::types::WorkflowSemantic::Done)));
    let has_non_done = value.iter().any(|s| !matches!(s.semantic, Some(crate::types::WorkflowSemantic::Done)));
    if !has_done     { v.push(RuleViolation::field("must include at least one Done state")); }
    if !has_non_done { v.push(RuleViolation::field("must include at least one non-Done state")); }
    v
}

pub fn states_valid_task(value: &[TaskStateEntry]) -> Vec<RuleViolation> {
    let mut v = vec![];
    v.extend(min_length(value, 2));
    for (i, s) in value.iter().enumerate() {
        v.extend(camel_case(&s.id).into_iter().map(|viol| RuleViolation::sub(format!("[{i}].id"), viol.message)));
    }
    v.extend(unique_by(value, |s| s.id.clone())
        .into_iter().map(|viol| RuleViolation { sub_path: viol.sub_path.map(|p| p + ".id"), ..viol }));
    let has_done = value.iter().any(|s| matches!(s.semantic, Some(crate::types::TaskSemantic::Done)));
    let has_non_done = value.iter().any(|s| !matches!(s.semantic, Some(crate::types::TaskSemantic::Done)));
    if !has_done     { v.push(RuleViolation::field("must include at least one Done state")); }
    if !has_non_done { v.push(RuleViolation::field("must include at least one non-Done state")); }
    v
}

// ---------------------------------------------------------------------------
// Raci structural primitive
// ---------------------------------------------------------------------------

/// Raci.responsible must be non-empty.
pub fn raci_structural(value: &Raci) -> Vec<RuleViolation> {
    if value.responsible.is_empty() {
        vec![RuleViolation::sub(".responsible", "must not be empty")]
    } else {
        vec![]
    }
}
```

---

## TDD: Tests to Write First

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // --- RuleViolation constructors ---

    #[test]
    fn rule_violation_field_has_no_sub_path() {
        let v = RuleViolation::field("bad value");
        assert!(v.sub_path.is_none());
        assert_eq!(v.message, "bad value");
    }

    #[test]
    fn rule_violation_sub_has_sub_path() {
        let v = RuleViolation::sub("[0].role", "not found");
        assert_eq!(v.sub_path.as_deref(), Some("[0].role"));
    }

    // --- kebab_case ---

    #[test]
    fn kebab_case_valid_ids() {
        assert!(kebab_case("eng-lead").is_empty());
        assert!(kebab_case("abc").is_empty());
        assert!(kebab_case("a-b-c").is_empty());
        assert!(kebab_case("abc123").is_empty());
    }

    #[test]
    fn kebab_case_rejects_uppercase() {
        assert!(!kebab_case("EngLead").is_empty());
    }

    #[test]
    fn kebab_case_rejects_leading_dash() {
        assert!(!kebab_case("-eng").is_empty());
    }

    #[test]
    fn kebab_case_rejects_trailing_dash() {
        assert!(!kebab_case("eng-").is_empty());
    }

    #[test]
    fn kebab_case_rejects_double_dash() {
        assert!(!kebab_case("eng--lead").is_empty());
    }

    #[test]
    fn kebab_case_rejects_empty() {
        assert!(!kebab_case("").is_empty());
    }

    // --- camel_case ---

    #[test]
    fn camel_case_valid_ids() {
        assert!(camel_case("WriteProposal").is_empty());
        assert!(camel_case("Done").is_empty());
        assert!(camel_case("InitiativeWorkflow").is_empty());
    }

    #[test]
    fn camel_case_rejects_lowercase_start() {
        assert!(!camel_case("writeProposal").is_empty());
    }

    #[test]
    fn camel_case_rejects_hyphens() {
        assert!(!camel_case("Write-Proposal").is_empty());
    }

    #[test]
    fn camel_case_rejects_empty() {
        assert!(!camel_case("").is_empty());
    }

    // --- non_empty_str ---

    #[test]
    fn non_empty_str_valid() {
        assert!(non_empty_str("hello").is_empty());
    }

    #[test]
    fn non_empty_str_rejects_empty() {
        assert!(!non_empty_str("").is_empty());
    }

    #[test]
    fn non_empty_str_rejects_whitespace_only() {
        assert!(!non_empty_str("   ").is_empty());
    }

    // --- non_empty_list ---

    #[test]
    fn non_empty_list_valid() {
        assert!(non_empty_list(&[1u32, 2]).is_empty());
    }

    #[test]
    fn non_empty_list_rejects_empty() {
        assert!(!non_empty_list::<u32>(&[]).is_empty());
    }

    // --- min_length ---

    #[test]
    fn min_length_passes_when_met() {
        assert!(min_length(&[1u32, 2], 2).is_empty());
    }

    #[test]
    fn min_length_fails_when_short() {
        assert!(!min_length(&[1u32], 2).is_empty());
    }

    // --- unique_by ---

    #[test]
    fn unique_by_no_duplicates() {
        let v = vec!["a", "b", "c"];
        assert!(unique_by(&v, |s| s.to_string()).is_empty());
    }

    #[test]
    fn unique_by_reports_duplicate_index() {
        let v = vec!["a", "b", "a"];
        let violations = unique_by(&v, |s| s.to_string());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].sub_path.as_deref(), Some("[2]"));
    }

    // --- x_prefix_keys ---

    #[test]
    fn x_prefix_keys_valid() {
        let mut ext = std::collections::HashMap::new();
        ext.insert("x-owner".to_string(), serde_json::json!("alice"));
        assert!(x_prefix_keys(&ext).is_empty());
    }

    #[test]
    fn x_prefix_keys_rejects_non_x_keys() {
        let mut ext = std::collections::HashMap::new();
        ext.insert("owner".to_string(), serde_json::json!("alice"));
        let v = x_prefix_keys(&ext);
        assert_eq!(v.len(), 1);
        assert!(v[0].sub_path.as_ref().unwrap().contains("owner"));
    }

    // --- states_valid_workflow ---

    fn make_workflow_state(id: &str, semantic: Option<crate::types::WorkflowSemantic>) -> crate::types::WorkflowStateEntry {
        crate::types::WorkflowStateEntry { id: id.to_string(), description: "d".to_string(), semantic }
    }

    #[test]
    fn states_valid_workflow_valid_states() {
        let states = vec![
            make_workflow_state("Draft", None),
            make_workflow_state("Done", Some(crate::types::WorkflowSemantic::Done)),
        ];
        assert!(states_valid_workflow(&states).is_empty());
    }

    #[test]
    fn states_valid_workflow_requires_min_2() {
        let states = vec![
            make_workflow_state("Done", Some(crate::types::WorkflowSemantic::Done)),
        ];
        let v = states_valid_workflow(&states);
        assert!(!v.is_empty());
    }

    #[test]
    fn states_valid_workflow_requires_done_semantic() {
        let states = vec![
            make_workflow_state("Draft", None),
            make_workflow_state("Active", None),
        ];
        let v = states_valid_workflow(&states);
        assert!(v.iter().any(|e| e.message.contains("Done")));
    }

    #[test]
    fn states_valid_workflow_rejects_duplicate_ids() {
        let states = vec![
            make_workflow_state("Draft", None),
            make_workflow_state("Draft", Some(crate::types::WorkflowSemantic::Done)),
        ];
        let v = states_valid_workflow(&states);
        assert!(v.iter().any(|e| e.message.contains("duplicate")));
    }

    #[test]
    fn states_valid_workflow_rejects_lowercase_id() {
        let states = vec![
            make_workflow_state("draft", None),
            make_workflow_state("Done", Some(crate::types::WorkflowSemantic::Done)),
        ];
        let v = states_valid_workflow(&states);
        assert!(v.iter().any(|e| e.sub_path.as_ref().map(|p| p.contains("id")).unwrap_or(false)));
    }

    // --- raci_structural ---

    #[test]
    fn raci_structural_valid_when_responsible_non_empty() {
        use crate::entity::EntityRef;
        use crate::entities::role::Role;
        let raci = crate::types::Raci {
            responsible: vec![EntityRef::<Role>::new("eng-lead")],
            accountable: EntityRef::new("pm"),
            consulted:   None,
            informed:    None,
        };
        assert!(raci_structural(&raci).is_empty());
    }

    #[test]
    fn raci_structural_rejects_empty_responsible() {
        use crate::entity::EntityRef;
        use crate::entities::role::Role;
        let raci = crate::types::Raci {
            responsible: vec![],
            accountable: EntityRef::<Role>::new("pm"),
            consulted:   None,
            informed:    None,
        };
        let v = raci_structural(&raci);
        assert!(!v.is_empty());
        assert!(v[0].sub_path.as_ref().map(|p| p.contains("responsible")).unwrap_or(false));
    }

    // --- ValidationErrors ---

    #[test]
    fn validation_errors_starts_empty() {
        let e = ValidationErrors::new();
        assert!(e.is_empty());
    }

    #[test]
    fn validation_errors_extend_combines_errors() {
        let mut e1 = ValidationErrors::new();
        e1.errors.push(FieldValidationError {
            path: "name".to_string(),
            message: "bad".to_string(),
            kind: ValidationKind::Structural,
        });
        let e2 = ValidationErrors::new();
        e1.extend(e2);
        assert_eq!(e1.errors.len(), 1);
    }

    // --- build_path ---

    #[test]
    fn build_path_no_sub_path() {
        assert_eq!(build_path("name", &None), "name");
    }

    #[test]
    fn build_path_with_sub_path() {
        assert_eq!(build_path("steps", &Some(".WriteProposal.depends_on".to_string())),
                   "steps.WriteProposal.depends_on");
    }

    // --- run_validations (basic smoke test) ---

    #[tokio::test]
    async fn run_validations_runs_structural_rules() {
        // Requires an entity with a ValidationSchema — use TrackedRole once Task 05 wires it.
        // This test is a placeholder; full validation integration tests are in Task 07.
    }
}
```

---

## Implementation Notes

### `ValidationSchema<E>` vs `ValidationSchema` stub

The Task 02 stub was `pub struct ValidationSchema<E>(PhantomData<E>)`. This task replaces it with the real generic struct with three rule maps. `ValidationSchema<E>` is parameterized over the plain entity type `E` and uses `E::Tracked` for rule function signatures. Task 07 populates the `OnceLock` statics on each entity type with real rules.

### Rule function storage: `Box<dyn Fn(...)>` approach

Since structural rules operate on specific field types and the runner dispatches by field name, rules are stored as `Box<dyn Fn(&TrackedX) -> Vec<RuleViolation>>` closures. The macro wraps typed rule functions in these closures at schema construction time. For example:

```rust
// Macro-generated, for field `name: String` with rule `non_empty_str`:
Box::new(|entity: &TrackedRole| {
    entity.name.get()
        .map(|v| non_empty_str(v.as_str()))
        .unwrap_or_default()  // field not loaded → skip (no violation)
})
```

This uninitialized-field behavior (`unwrap_or_default()`) is intentional: if a field hasn't been loaded, it is not checked. The load path ensures fields are validated when they are loaded.

### Async rule storage

Semantic and cross-entity rules are stored as `Box<dyn Fn(&TrackedX) -> Pin<Box<dyn Future<...>>>>`. The `async fn` is wrapped in a `Box::pin(async move { ... })` closure. This is the same pattern as `#[async_trait]` boxing.

### `states_valid` split

The design doc shows a single `states_valid<S: StateEntry>` generic function. In practice, `WorkflowStateEntry` and `TaskStateEntry` are distinct types with distinct semantic enums. Two concrete functions (`states_valid_workflow`, `states_valid_task`) are cleaner than a trait. No `StateEntry` trait needed — keep it simple.

---

## Acceptance Criteria

- `cargo test validation` passes — all tests in `src/validation.rs` green
- `ValidationErrors::is_empty()` returns true for a fresh instance
- All structural primitives pass their tests
- `raci_structural` correctly validates `responsible` non-empty
- `states_valid_workflow` enforces: min 2, CamelCase ids, unique ids, at least one Done, at least one non-Done
- `run_validations` function signature compiles
- `SetterError` and `LoadError` stubs from Task 03 are replaced with the full types
- Task 01, 02, 03, 04, 05 tests still pass after `ValidationSchema` type is updated
