use super::primitives::{non_empty_list, unique_by};
use crate::{
    entity::{
        entities::{
            role::Role,
            team::{Team, TeamMember},
        },
        EntityRef,
    },
    error::primitive::PrimitiveError,
};

fn valid_handle(handle: &str) -> bool {
    if !handle.starts_with('@') || handle.len() < 2 {
        return false;
    }
    handle[1..]
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_' || c == '-')
}

pub fn include_structural(
    value: &Option<Vec<(EntityRef<Team>, EntityRef<Role>)>>,
) -> Vec<PrimitiveError> {
    match value {
        None => vec![],
        Some(includes) => {
            let mut v = non_empty_list(includes.as_slice());
            v.extend(unique_by(includes, |(team, _role)| team.id().to_string()));
            v
        }
    }
}

pub fn members_structural(value: &Option<Vec<TeamMember>>) -> Vec<PrimitiveError> {
    match value {
        None => vec![],
        Some(members) => {
            let mut v = vec![];
            for (i, m) in members.iter().enumerate() {
                if !valid_handle(&m.handle) {
                    v.push(PrimitiveError::naming_format_violation(
                        format!(
                            "'{}' is not a valid handle (must match ^@[a-z0-9._-]+$)",
                            m.handle
                        ),
                        Some(format!("[{i}].handle")),
                        "handle_format",
                    ));
                }
            }
            v.extend(unique_by(members, |m| m.handle.clone()));
            v
        }
    }
}
