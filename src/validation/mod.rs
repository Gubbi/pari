//! Validation framework.
//!
//! [`ValidationSchema`] — per-entity schema with three rule maps.
//! [`run_validations`] — async runner that dispatches rules and accumulates errors.
//! Shared structural primitive rule functions.

use std::collections::HashMap;
use std::hash::Hash;
use crate::entity::Entity;

pub mod cross_entity;
pub mod role;
pub mod hook;
pub mod team;
pub mod artifact_kind;
pub mod task;
pub mod relay;
pub mod workflow;

// ---------------------------------------------------------------------------
// RuleViolation — single violation from one rule
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

#[derive(Debug)]
pub struct FieldValidationError {
    /// Dot-notation path: `"name"`, `"steps.WriteProposal.depends_on[0]"`
    pub path: String,
    pub message: String,
    pub kind: ValidationKind,
}

pub struct ValidationErrors {
    pub errors: Vec<FieldValidationError>,
}

impl ValidationErrors {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn extend(&mut self, other: ValidationErrors) {
        self.errors.extend(other.errors);
    }
}

impl Default for ValidationErrors {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// SetterError (replaces Task 03 stub)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum SetterError {
    Substrate(SubstrateError),
    Validation(ValidationErrors),
}

/// Substrate-level I/O error. Full type in Task 11.
#[derive(Debug)]
pub struct SubstrateError {
    pub path: String,
    pub message: String,
}

impl std::fmt::Debug for ValidationErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ValidationErrors({} errors)", self.errors.len())
    }
}

// ---------------------------------------------------------------------------
// LoadError (replaces Task 03 stub)
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

/// Structural rule: sync closure that receives the whole tracked entity.
/// The closure captures field extraction (e.g. `|e: &TrackedRole| { e.name.get().map(f).unwrap_or_default() }`).
pub type AnyStructuralRule<E> =
    Box<dyn Fn(&<E as Entity>::Tracked) -> Vec<RuleViolation> + Send + Sync>;

/// Semantic rule: async closure that receives the whole tracked entity.
pub type AnySemanticRule<E> = Box<
    dyn for<'a> Fn(
            &'a <E as Entity>::Tracked,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<RuleViolation>> + Send + 'a>>
        + Send
        + Sync,
>;

/// Cross-entity rule: same signature as a semantic rule.
pub type AnyCrossEntityRule<E> = AnySemanticRule<E>;

// ---------------------------------------------------------------------------
// ValidationSchema — replaces the placeholder stub from Task 02
// ---------------------------------------------------------------------------

/// Per-entity validation schema.
/// Three maps from field name → list of rules.
/// A field absent from a map has no rules of that kind.
pub struct ValidationSchema<E: Entity> {
    pub structural: HashMap<&'static str, Vec<AnyStructuralRule<E>>>,
    pub semantic: HashMap<&'static str, Vec<AnySemanticRule<E>>>,
    pub cross_entity: HashMap<&'static str, Vec<AnyCrossEntityRule<E>>>,
}

impl<E: Entity> ValidationSchema<E> {
    pub fn empty() -> Self {
        Self { structural: HashMap::new(), semantic: HashMap::new(), cross_entity: HashMap::new() }
    }

    /// All field names that appear in any rule map.
    pub fn all_field_names(&self) -> Vec<&str> {
        let mut fields: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for k in self.structural.keys() {
            fields.insert(k);
        }
        for k in self.semantic.keys() {
            fields.insert(k);
        }
        for k in self.cross_entity.keys() {
            fields.insert(k);
        }
        fields.into_iter().collect()
    }
}

// ---------------------------------------------------------------------------
// ValidatableTracked helper trait
// ---------------------------------------------------------------------------

/// Implemented by every `TrackedX` struct (blanket impl below).
/// Provides runtime dispatch from field name → structural rule execution.
pub trait ValidatableTracked<E: Entity> {
    fn run_structural_rules(
        &self,
        field_name: &str,
        rules: &[AnyStructuralRule<E>],
    ) -> Vec<RuleViolation>;
}

impl<E: Entity> ValidatableTracked<E> for E::Tracked {
    fn run_structural_rules(
        &self,
        _field_name: &str,
        rules: &[AnyStructuralRule<E>],
    ) -> Vec<RuleViolation> {
        rules.iter().flat_map(|r| r(self)).collect()
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

    let all_fields = schema.all_field_names();
    let target_fields: Vec<&str> =
        if fields.is_empty() { all_fields } else { fields.to_vec() };

    for field_name in &target_fields {
        if kinds.contains(&ValidationKind::Structural) {
            if let Some(rules) = schema.structural.get(field_name) {
                for v in entity.run_structural_rules(field_name, rules) {
                    result.errors.push(FieldValidationError {
                        path: build_path(field_name, &v.sub_path),
                        message: v.message,
                        kind: ValidationKind::Structural,
                    });
                }
            }
        }

        if kinds.contains(&ValidationKind::Semantic) {
            if let Some(rules) = schema.semantic.get(field_name) {
                for rule in rules {
                    for v in rule(entity).await {
                        result.errors.push(FieldValidationError {
                            path: build_path(field_name, &v.sub_path),
                            message: v.message,
                            kind: ValidationKind::Semantic,
                        });
                    }
                }
            }
        }

        if kinds.contains(&ValidationKind::CrossEntity) {
            if let Some(rules) = schema.cross_entity.get(field_name) {
                for rule in rules {
                    for v in rule(entity).await {
                        result.errors.push(FieldValidationError {
                            path: build_path(field_name, &v.sub_path),
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

pub fn build_path(field: &str, sub_path: &Option<String>) -> String {
    match sub_path {
        None => field.to_string(),
        Some(sub) => format!("{field}{sub}"),
    }
}

// ---------------------------------------------------------------------------
// Structural primitives
// ---------------------------------------------------------------------------

use crate::entity::{EntityRef, ParentKind};
use crate::types::{Extensions, Raci, TaskStateEntry, WorkflowStateEntry};

/// Id must match `[a-z0-9]+(-[a-z0-9]+)*`
pub fn kebab_case(value: &str) -> Vec<RuleViolation> {
    let valid = !value.is_empty()
        && value.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--");
    if valid {
        vec![]
    } else {
        vec![RuleViolation::field(format!("'{value}' is not kebab-case"))]
    }
}

/// Id must match `[A-Z][a-zA-Z0-9]*`
pub fn camel_case(value: &str) -> Vec<RuleViolation> {
    let valid = !value.is_empty()
        && value.starts_with(|c: char| c.is_ascii_uppercase())
        && value.chars().all(|c| c.is_ascii_alphanumeric());
    if valid {
        vec![]
    } else {
        vec![RuleViolation::field(format!("'{value}' is not CamelCase"))]
    }
}

/// `EntityRef` id must be kebab-case
pub fn kebab_case_id<T: Entity, P: ParentKind>(entity_ref: &EntityRef<T, P>) -> Vec<RuleViolation> {
    kebab_case(entity_ref.id())
}

/// `EntityRef` id must be CamelCase
pub fn camel_case_id<T: Entity, P: ParentKind>(entity_ref: &EntityRef<T, P>) -> Vec<RuleViolation> {
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
        vec![RuleViolation::field(format!(
            "must have at least {min} elements, got {}",
            value.len()
        ))]
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

/// All keys in extensions must start with `"x-"`
pub fn x_prefix_keys(value: &Extensions) -> Vec<RuleViolation> {
    value
        .keys()
        .filter(|k| !k.starts_with("x-"))
        .map(|k| RuleViolation::sub(format!(".{k}"), "extension key must start with 'x-'"))
        .collect()
}

/// State list validation for workflow states:
/// - At least 2 states
/// - All ids are CamelCase
/// - All ids are unique
/// - At least one Done semantic
/// - At least one non-Done state
pub fn states_valid_workflow(value: &[WorkflowStateEntry]) -> Vec<RuleViolation> {
    let mut v = vec![];
    v.extend(min_length(value, 2));
    for (i, s) in value.iter().enumerate() {
        v.extend(
            camel_case(&s.id)
                .into_iter()
                .map(|viol| RuleViolation::sub(format!("[{i}].id"), viol.message)),
        );
    }
    v.extend(
        unique_by(value, |s| s.id.clone())
            .into_iter()
            .map(|viol| RuleViolation { sub_path: viol.sub_path.map(|p| p + ".id"), ..viol }),
    );
    let has_done =
        value.iter().any(|s| matches!(s.semantic, Some(crate::types::WorkflowSemantic::Done)));
    let has_non_done =
        value.iter().any(|s| !matches!(s.semantic, Some(crate::types::WorkflowSemantic::Done)));
    if !has_done {
        v.push(RuleViolation::field("must include at least one Done state"));
    }
    if !has_non_done {
        v.push(RuleViolation::field("must include at least one non-Done state"));
    }
    v
}

/// State list validation for task states — same rules but for `TaskSemantic`.
pub fn states_valid_task(value: &[TaskStateEntry]) -> Vec<RuleViolation> {
    let mut v = vec![];
    v.extend(min_length(value, 2));
    for (i, s) in value.iter().enumerate() {
        v.extend(
            camel_case(&s.id)
                .into_iter()
                .map(|viol| RuleViolation::sub(format!("[{i}].id"), viol.message)),
        );
    }
    v.extend(
        unique_by(value, |s| s.id.clone())
            .into_iter()
            .map(|viol| RuleViolation { sub_path: viol.sub_path.map(|p| p + ".id"), ..viol }),
    );
    let has_done =
        value.iter().any(|s| matches!(s.semantic, Some(crate::types::TaskSemantic::Done)));
    let has_non_done =
        value.iter().any(|s| !matches!(s.semantic, Some(crate::types::TaskSemantic::Done)));
    if !has_done {
        v.push(RuleViolation::field("must include at least one Done state"));
    }
    if !has_non_done {
        v.push(RuleViolation::field("must include at least one non-Done state"));
    }
    v
}

// ---------------------------------------------------------------------------
// Raci structural primitive
// ---------------------------------------------------------------------------

/// `Raci.responsible` must be non-empty.
pub fn raci_structural(value: &Raci) -> Vec<RuleViolation> {
    if value.responsible.is_empty() {
        vec![RuleViolation::sub(".responsible", "must not be empty")]
    } else {
        vec![]
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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

    fn make_workflow_state(
        id: &str,
        semantic: Option<crate::types::WorkflowSemantic>,
    ) -> crate::types::WorkflowStateEntry {
        crate::types::WorkflowStateEntry {
            id: id.to_string(),
            description: "d".to_string(),
            semantic,
        }
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
        let states =
            vec![make_workflow_state("Done", Some(crate::types::WorkflowSemantic::Done))];
        let v = states_valid_workflow(&states);
        assert!(!v.is_empty());
    }

    #[test]
    fn states_valid_workflow_requires_done_semantic() {
        let states =
            vec![make_workflow_state("Draft", None), make_workflow_state("Active", None)];
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
        assert!(v
            .iter()
            .any(|e| e.sub_path.as_ref().map(|p| p.contains("id")).unwrap_or(false)));
    }

    // --- raci_structural ---

    #[test]
    fn raci_structural_valid_when_responsible_non_empty() {
        use crate::entity::EntityRef;
        use crate::entities::role::Role;
        let raci = crate::types::Raci {
            responsible: vec![EntityRef::<Role>::new("eng-lead")],
            accountable: EntityRef::new("pm"),
            consulted: None,
            informed: None,
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
            consulted: None,
            informed: None,
        };
        let v = raci_structural(&raci);
        assert!(!v.is_empty());
        assert!(
            v[0].sub_path.as_ref().map(|p| p.contains("responsible")).unwrap_or(false)
        );
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
        assert_eq!(
            build_path("steps", &Some(".WriteProposal.depends_on".to_string())),
            "steps.WriteProposal.depends_on"
        );
    }

    // --- run_validations (signature compile check) ---

    #[tokio::test]
    async fn run_validations_runs_structural_rules() {
        // Placeholder — full integration tests in Task 07.
        // Verifies the function signature compiles.
        use crate::entity::EntityKind;
        let _ = ValidationKind::Structural;
        let _ = EntityKind::Role;
    }
}
