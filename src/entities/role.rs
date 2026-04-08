use crate::entity::{EntityKind, EntityRef};
use crate::types::Extensions;

#[derive(pari_macros::Entity)]
#[entity(kind = EntityKind::Role, schema = crate::validation::role::role_validation_schema)]
pub struct Role {
    pub entity_ref: EntityRef<Role>,
    pub name: String,
    pub description: Option<String>,
    pub purpose: String,
    pub traits: Option<Vec<String>>,
    pub extensions: Extensions,
}
