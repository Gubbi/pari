use crate::schema::{entities::team::Team, types::Extensions};

pub fn minimal_team(id: &str) -> Team {
    Team {
        id: id.into(),
        name: id.to_string(),
        description: None,
        members: None,
        include: None,
        import: None,
        extensions: Extensions::default(),
    }
}
