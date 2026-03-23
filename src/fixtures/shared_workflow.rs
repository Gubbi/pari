use crate::{
    fixtures::workflow::raci,
    schema::{
        entities::workflow::{ReviewStep, SharedWorkflow, Step},
        types::Extensions,
    },
};

pub fn minimal_shared_workflow(id: &str, state_ids: &[&str]) -> SharedWorkflow {
    use crate::schema::types::{WorkflowSemantic, WorkflowStateEntry};

    let n = state_ids.len();
    let states = state_ids
        .iter()
        .enumerate()
        .map(|(i, sid)| WorkflowStateEntry {
            id: sid.to_string(),
            description: "desc".to_string(),
            semantic: if i == n - 1 {
                Some(WorkflowSemantic::Complete)
            } else {
                None
            },
        })
        .collect();

    SharedWorkflow {
        id: id.into(),
        name: id.to_string(),
        description: None,
        purpose: "test purpose".to_string(),
        accountability: raci(),
        steps: vec![Step::Review(ReviewStep {
            id: "Approve".to_string(),
            approver: "eng-lead".to_string(),
            on_reject: "Approve".to_string(),
        })],
        states,
        hooks: None,
        guidance: None,
        extensions: Extensions::default(),
    }
}
