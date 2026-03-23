use crate::{
    fixtures::task::minimal_task,
    schema::{
        entities::workflow::{Step, WorkStep, WorkStepDefinition, Workflow},
        types::{Extensions, Raci, WorkflowSemantic, WorkflowStateEntry},
    },
};

pub fn raci() -> Raci {
    Raci {
        responsible: "eng-lead".to_string(),
        accountable: "pm".to_string(),
        consulted: vec![],
        informed: vec![],
    }
}

pub fn workflow_states() -> Vec<WorkflowStateEntry> {
    vec![
        WorkflowStateEntry {
            id: "Active".to_string(),
            description: "In progress.".to_string(),
            semantic: None,
        },
        WorkflowStateEntry {
            id: "Done".to_string(),
            description: "Complete.".to_string(),
            semantic: Some(WorkflowSemantic::Complete),
        },
    ]
}

pub fn minimal_workflow(wf_id: &str, task_id: &str) -> Workflow {
    Workflow {
        id: wf_id.into(),
        name: wf_id.to_string(),
        description: None,
        purpose: "test purpose".to_string(),
        accountability: raci(),
        steps: vec![Step::Work(WorkStep {
            depends_on: None,
            definition: WorkStepDefinition::Task(minimal_task(task_id)),
        })],
        states: workflow_states(),
        hooks: None,
        guidance: None,
        extensions: Extensions::default(),
    }
}
