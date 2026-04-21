use super::primitives::unique_by;
use crate::{entity::entities::team::TeamMember, error::primitive::PrimitiveError};

pub fn unique_member_handles(value: &Option<Vec<TeamMember>>) -> Vec<PrimitiveError> {
    match value {
        None => vec![],
        Some(members) => unique_by(members, |m| m.handle.clone()),
    }
}
