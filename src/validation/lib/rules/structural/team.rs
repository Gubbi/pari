use super::primitives::unique_by;
use crate::{entity::entities::team::TeamMember, error::primitive::PrimitiveError};

fn valid_handle(handle: &str) -> bool {
    if !handle.starts_with('@') || handle.len() < 2 {
        return false;
    }
    handle[1..]
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_' || c == '-')
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
