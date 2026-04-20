use std::hash::Hash;

use crate::entity::{
    types::{Extensions, Raci, TaskStateEntry, WorkflowStateEntry},
    Entity, EntityRef, ParentKind,
};

use super::rule_violation::RuleViolation;

/// Id must match `[a-z0-9]+(-[a-z0-9]+)*`
pub fn kebab_case(value: &str) -> Vec<RuleViolation> {
    let valid = !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
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
            .map(|viol| RuleViolation {
                sub_path: viol.sub_path.map(|p| p + ".id"),
                ..viol
            }),
    );
    let has_done = value.iter().any(|s| {
        matches!(
            s.semantic,
            Some(crate::entity::types::WorkflowSemantic::Done)
        )
    });
    let has_non_done = value.iter().any(|s| {
        !matches!(
            s.semantic,
            Some(crate::entity::types::WorkflowSemantic::Done)
        )
    });
    if !has_done {
        v.push(RuleViolation::field("must include at least one Done state"));
    }
    if !has_non_done {
        v.push(RuleViolation::field(
            "must include at least one non-Done state",
        ));
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
            .map(|viol| RuleViolation {
                sub_path: viol.sub_path.map(|p| p + ".id"),
                ..viol
            }),
    );
    let has_done = value
        .iter()
        .any(|s| matches!(s.semantic, Some(crate::entity::types::TaskSemantic::Done)));
    let has_non_done = value
        .iter()
        .any(|s| !matches!(s.semantic, Some(crate::entity::types::TaskSemantic::Done)));
    if !has_done {
        v.push(RuleViolation::field("must include at least one Done state"));
    }
    if !has_non_done {
        v.push(RuleViolation::field(
            "must include at least one non-Done state",
        ));
    }
    v
}

/// `Raci.responsible` must be non-empty.
pub fn raci_structural(value: &Raci) -> Vec<RuleViolation> {
    if value.responsible.is_empty() {
        vec![RuleViolation::sub(".responsible", "must not be empty")]
    } else {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut ext = Extensions::new();
        ext.insert("x-owner".to_string(), serde_json::json!("alice"));
        assert!(x_prefix_keys(&ext).is_empty());
    }

    #[test]
    fn x_prefix_keys_rejects_non_x_keys() {
        let mut ext = Extensions::new();
        ext.insert("owner".to_string(), serde_json::json!("alice"));
        let v = x_prefix_keys(&ext);
        assert_eq!(v.len(), 1);
        assert!(v[0].sub_path.as_ref().unwrap().contains("owner"));
    }

    // --- states_valid_workflow ---

    fn make_workflow_state(
        id: &str,
        semantic: Option<crate::entity::types::WorkflowSemantic>,
    ) -> crate::entity::types::WorkflowStateEntry {
        crate::entity::types::WorkflowStateEntry {
            id: id.to_string(),
            description: "d".to_string(),
            semantic,
        }
    }

    #[test]
    fn states_valid_workflow_valid_states() {
        let states = vec![
            make_workflow_state("Draft", None),
            make_workflow_state("Done", Some(crate::entity::types::WorkflowSemantic::Done)),
        ];
        assert!(states_valid_workflow(&states).is_empty());
    }

    #[test]
    fn states_valid_workflow_requires_min_2() {
        let states = vec![make_workflow_state(
            "Done",
            Some(crate::entity::types::WorkflowSemantic::Done),
        )];
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
            make_workflow_state("Draft", Some(crate::entity::types::WorkflowSemantic::Done)),
        ];
        let v = states_valid_workflow(&states);
        assert!(v.iter().any(|e| e.message.contains("duplicate")));
    }

    #[test]
    fn states_valid_workflow_rejects_lowercase_id() {
        let states = vec![
            make_workflow_state("draft", None),
            make_workflow_state("Done", Some(crate::entity::types::WorkflowSemantic::Done)),
        ];
        let v = states_valid_workflow(&states);
        assert!(v.iter().any(|e| e
            .sub_path
            .as_ref()
            .map(|p| p.contains("id"))
            .unwrap_or(false)));
    }

    // --- raci_structural ---

    #[test]
    fn raci_structural_valid_when_responsible_non_empty() {
        use crate::{entities::role::Role, entity::EntityRef};
        let raci = crate::entity::types::Raci {
            responsible: vec![EntityRef::<Role>::new("eng-lead")],
            accountable: EntityRef::new("pm"),
            consulted: None,
            informed: None,
        };
        assert!(raci_structural(&raci).is_empty());
    }

    #[test]
    fn raci_structural_rejects_empty_responsible() {
        use crate::{entities::role::Role, entity::EntityRef};
        let raci = crate::entity::types::Raci {
            responsible: vec![],
            accountable: EntityRef::<Role>::new("pm"),
            consulted: None,
            informed: None,
        };
        let v = raci_structural(&raci);
        assert!(!v.is_empty());
        assert!(v[0]
            .sub_path
            .as_ref()
            .map(|p| p.contains("responsible"))
            .unwrap_or(false));
    }
}
