use crate::schema::{
    entities::task::Task,
    types::{Artifact, Extensions, TaskSemantic, TaskStateEntry},
};

pub fn minimal_task(id: &str) -> Task {
    Task {
        id: id.into(),
        name: format!("{id} Name"),
        description: None,
        purpose: "test purpose".to_string(),
        instructions: vec!["do it".to_string()],
        criteria: vec!["done".to_string()],
        accountability: None,
        artifact: Artifact {
            name: "output".to_string(),
            template: None,
        },
        states: vec![
            TaskStateEntry {
                id: "Draft".to_string(),
                description: "In progress.".to_string(),
                semantic: None,
            },
            TaskStateEntry {
                id: "Done".to_string(),
                description: "Complete.".to_string(),
                semantic: Some(TaskSemantic::Complete),
            },
        ],
        hooks: None,
        guidance: None,
        extensions: Extensions::default(),
    }
}
