use crate::schema::{entities::role::Role, types::Extensions};

pub fn minimal_role(id: &str) -> Role {
    Role {
        id: id.into(),
        name: id.to_string(),
        purpose: "test".to_string(),
        traits: None,
        extensions: Extensions::default(),
    }
}
