use std::collections::HashMap;

use crate::entity::{entities::role::Role, types::Extensions, EntityKind, EntityRef};

#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, pari_macros::Entity,
)]
#[schemars(deny_unknown_fields)]
#[entity(kind = EntityKind::Team, schema = crate::validation::team::team_validation_schema)]
pub struct Team {
    pub entity_ref: EntityRef<Team>,
    pub name: String,
    pub description: Option<String>,
    pub members: Option<Vec<TeamMember>>,
    pub include: Option<HashMap<EntityRef<Team>, EntityRef<Role>>>,
    pub import: Option<Vec<EntityRef<Team>>>,
    #[serde(flatten)]
    pub extensions: Extensions,
}

#[derive(
    Debug,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    pari_macros::CollectRefs,
)]
#[schemars(deny_unknown_fields)]
pub struct TeamMember {
    #[schemars(regex(pattern = r"^@[a-z0-9._-]+$"))]
    pub handle: String,
    pub role: EntityRef<Role>,
}
