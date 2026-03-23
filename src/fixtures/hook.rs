use crate::schema::{
    entities::hook::{Hook, HookInput},
    types::Extensions,
};

pub fn minimal_hook(id: &str) -> Hook {
    Hook {
        id: id.into(),
        name: id.to_string(),
        description: "test".to_string(),
        instructions: vec!["do it".to_string()],
        inputs: None,
        extensions: Extensions::default(),
    }
}

pub fn hook_with_inputs(id: &str) -> Hook {
    Hook {
        id: id.into(),
        name: id.to_string(),
        description: "test".to_string(),
        instructions: vec!["do it".to_string()],
        inputs: Some(vec![
            HookInput {
                name: "status".to_string(),
                description: "desc".to_string(),
                required: true,
            },
            HookInput {
                name: "comment".to_string(),
                description: "desc".to_string(),
                required: false,
            },
        ]),
        extensions: Extensions::default(),
    }
}
