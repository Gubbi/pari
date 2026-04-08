use std::collections::HashMap;
use crate::entity::{EntityKind, EntityRef};
use crate::types::Extensions;
use crate::entities::role::Role;

#[derive(pari_macros::Entity)]
#[entity(kind = EntityKind::Team, schema = crate::validation::team::team_validation_schema)]
pub struct Team {
    pub entity_ref: EntityRef<Team>,
    pub name: String,
    pub description: Option<String>,
    pub members: Option<Vec<TeamMember>>,
    pub include: Option<HashMap<EntityRef<Team>, EntityRef<Role>>>,
    pub import: Option<Vec<EntityRef<Team>>>,
    pub extensions: Extensions,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TeamMember {
    pub handle: String,
    pub role: EntityRef<Role>,
}
