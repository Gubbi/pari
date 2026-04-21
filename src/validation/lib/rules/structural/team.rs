use crate::entity::entities::team::TeamMember;
use crate::error::primitive::PrimitiveError;
use super::primitives::unique_by;

pub fn unique_member_handles(value: &Option<Vec<TeamMember>>) -> Vec<PrimitiveError> {
    match value {
        None => vec![],
        Some(members) => unique_by(members, |m| m.handle.clone()),
    }
}
