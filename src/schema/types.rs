use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// --- RACI ---

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Raci {
    pub responsible: String,
    pub accountable: String,
    pub consulted: Vec<String>,
    pub informed: Vec<String>,
}

// --- HookInvocation and HooksMap ---

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum HookInvocation {
    Bare(String),
    Object {
        hook: String,
        with: Option<HashMap<String, String>>,
    },
}

impl HookInvocation {
    pub fn hook_id(&self) -> &str {
        match self {
            Self::Bare(id) => id,
            Self::Object { hook, .. } => hook,
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum HookInvocationValue {
    Single(HookInvocation),
    List(Vec<HookInvocation>),
}

impl HookInvocationValue {
    pub fn invocations(&self) -> Vec<&HookInvocation> {
        match self {
            Self::Single(inv) => vec![inv],
            Self::List(invs) => invs.iter().collect(),
        }
    }
}

pub type HooksMap = HashMap<String, HookInvocationValue>;

// --- Step types ---

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct WorkStep {
    #[schemars(regex(pattern = r"^[A-Z][A-Za-z0-9]*$"))]
    pub id: String,
    pub depends_on: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ReviewStep {
    #[schemars(regex(pattern = r"^[A-Z][A-Za-z0-9]*$"))]
    pub id: String,
    pub approver: String,
    pub on_reject: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Step {
    Work(WorkStep),
    Review(ReviewStep),
}

impl Step {
    pub fn id(&self) -> &str {
        match self {
            Self::Work(s) => &s.id,
            Self::Review(s) => &s.id,
        }
    }
}

// --- State entry types ---

#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowSemantic {
    Reviewing,
    Complete,
    Blocked,
    Failed,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct WorkflowStateEntry {
    #[schemars(regex(pattern = r"^[A-Z][A-Za-z0-9]*$"))]
    pub id: String,
    pub description: String,
    pub semantic: Option<WorkflowSemantic>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskSemantic {
    Complete,
    Blocked,
    Failed,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct TaskStateEntry {
    #[schemars(regex(pattern = r"^[A-Z][A-Za-z0-9]*$"))]
    pub id: String,
    pub description: String,
    pub semantic: Option<TaskSemantic>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RelayStateSemantic {
    Complete,
    Blocked,
    Failed,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct StateMapEntry {
    pub maps_to: String,
    pub semantic: Option<RelayStateSemantic>,
}

// --- Artifact ---

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Artifact {
    pub name: String,
    pub template: Option<String>,
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    // --- 6.1: Raci struct tests ---

    #[test]
    fn raci_with_empty_lists_is_valid() {
        let r = Raci {
            responsible: "eng-lead".to_string(),
            accountable: "pm".to_string(),
            consulted: vec![],
            informed: vec![],
        };
        assert_eq!(r.responsible, "eng-lead");
        assert_eq!(r.accountable, "pm");
        assert!(r.consulted.is_empty());
        assert!(r.informed.is_empty());
    }

    #[test]
    fn raci_with_lists() {
        let r = Raci {
            responsible: "eng-lead".to_string(),
            accountable: "pm".to_string(),
            consulted: vec!["designer".to_string()],
            informed: vec!["sre-lead".to_string()],
        };
        assert_eq!(r.consulted.len(), 1);
        assert_eq!(r.informed.len(), 1);
    }

    // --- 6.3: HookInvocation and HooksMap tests ---

    #[test]
    fn hook_invocation_bare_string() {
        let inv = HookInvocation::Bare("NotifySlack".to_string());
        assert_eq!(inv.hook_id(), "NotifySlack");
    }

    #[test]
    fn hook_invocation_object_with_inputs() {
        let mut with = HashMap::new();
        with.insert("status".to_string(), "Done".to_string());
        let inv = HookInvocation::Object {
            hook: "UpdateJiraStatus".to_string(),
            with: Some(with),
        };
        assert_eq!(inv.hook_id(), "UpdateJiraStatus");
    }

    #[test]
    fn hook_invocation_object_without_with() {
        let inv = HookInvocation::Object {
            hook: "UpdateJiraStatus".to_string(),
            with: None,
        };
        assert_eq!(inv.hook_id(), "UpdateJiraStatus");
        if let HookInvocation::Object { with, .. } = &inv {
            assert!(with.is_none());
        }
    }

    #[test]
    fn hooks_map_single_invocation() {
        let mut map: HooksMap = HashMap::new();
        map.insert(
            "after".to_string(),
            HookInvocationValue::Single(HookInvocation::Bare("NotifySlack".to_string())),
        );
        let val = map.get("after").unwrap();
        assert_eq!(val.invocations().len(), 1);
    }

    #[test]
    fn hooks_map_list_invocation() {
        let mut map: HooksMap = HashMap::new();
        map.insert(
            "after".to_string(),
            HookInvocationValue::List(vec![
                HookInvocation::Bare("NotifySlack".to_string()),
                HookInvocation::Object {
                    hook: "UpdateJiraStatus".to_string(),
                    with: None,
                },
            ]),
        );
        let val = map.get("after").unwrap();
        assert_eq!(val.invocations().len(), 2);
    }

    // --- 7.1: WorkStep and ReviewStep tests ---

    #[test]
    fn work_step_with_depends_on() {
        let s = WorkStep {
            id: "Proposal".to_string(),
            depends_on: Some(vec!["Shape".to_string()]),
        };
        assert_eq!(s.id, "Proposal");
        assert_eq!(s.depends_on.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn work_step_without_depends_on() {
        let s = WorkStep {
            id: "Shape".to_string(),
            depends_on: None,
        };
        assert!(s.depends_on.is_none());
    }

    #[test]
    fn review_step_all_fields_required() {
        let s = ReviewStep {
            id: "LegalApproval".to_string(),
            approver: "legal-counsel".to_string(),
            on_reject: "Shape".to_string(),
        };
        assert_eq!(s.id, "LegalApproval");
        assert_eq!(s.approver, "legal-counsel");
        assert_eq!(s.on_reject, "Shape");
    }

    #[test]
    fn step_id_returns_correct_value() {
        let ws = Step::Work(WorkStep {
            id: "Proposal".to_string(),
            depends_on: None,
        });
        assert_eq!(ws.id(), "Proposal");

        let rs = Step::Review(ReviewStep {
            id: "LegalApproval".to_string(),
            approver: "legal-counsel".to_string(),
            on_reject: "Proposal".to_string(),
        });
        assert_eq!(rs.id(), "LegalApproval");
    }

    // --- 8.1: State entry type tests ---

    #[test]
    fn workflow_state_entry_with_reviewing_semantic() {
        let e = WorkflowStateEntry {
            id: "UnderReview".to_string(),
            description: "Awaiting gate decision".to_string(),
            semantic: Some(WorkflowSemantic::Reviewing),
        };
        assert_eq!(e.semantic, Some(WorkflowSemantic::Reviewing));
    }

    #[test]
    fn workflow_state_entry_without_semantic() {
        let e = WorkflowStateEntry {
            id: "Active".to_string(),
            description: "Work underway".to_string(),
            semantic: None,
        };
        assert!(e.semantic.is_none());
    }

    #[test]
    fn workflow_state_entry_complete_semantic() {
        let e = WorkflowStateEntry {
            id: "Done".to_string(),
            description: "Completed".to_string(),
            semantic: Some(WorkflowSemantic::Complete),
        };
        assert_eq!(e.semantic, Some(WorkflowSemantic::Complete));
    }

    #[test]
    fn task_state_entry_with_complete_semantic() {
        let e = TaskStateEntry {
            id: "Done".to_string(),
            description: "Completed".to_string(),
            semantic: Some(TaskSemantic::Complete),
        };
        assert_eq!(e.semantic, Some(TaskSemantic::Complete));
    }

    #[test]
    fn task_state_entry_without_semantic() {
        let e = TaskStateEntry {
            id: "Draft".to_string(),
            description: "Being written".to_string(),
            semantic: None,
        };
        assert!(e.semantic.is_none());
    }

    #[test]
    fn task_state_entry_reviewing_not_available() {
        // TaskSemantic enum does not have Reviewing variant — enforced at type level
        // This test documents the constraint via exhaustive match
        let e = TaskStateEntry {
            id: "Done".to_string(),
            description: "Completed".to_string(),
            semantic: Some(TaskSemantic::Complete),
        };
        match e.semantic.unwrap() {
            TaskSemantic::Complete => {}
            TaskSemantic::Blocked => {}
            TaskSemantic::Failed => {}
            // No Reviewing variant — compiler enforces this
        }
    }

    #[test]
    fn state_map_entry_with_semantic() {
        let e = StateMapEntry {
            maps_to: "Complete".to_string(),
            semantic: Some(RelayStateSemantic::Complete),
        };
        assert_eq!(e.maps_to, "Complete");
        assert_eq!(e.semantic, Some(RelayStateSemantic::Complete));
    }

    #[test]
    fn state_map_entry_without_semantic() {
        let e = StateMapEntry {
            maps_to: "Active".to_string(),
            semantic: None,
        };
        assert!(e.semantic.is_none());
    }
}
