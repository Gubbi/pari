use crate::entity::{types::Extensions, EntityKind, EntityRef};

#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, pari_macros::Entity,
)]
#[schemars(deny_unknown_fields)]
#[entity(kind = EntityKind::Role, schema = crate::validation::role::role_validation_schema)]
pub struct Role {
    pub entity_ref: EntityRef<Role>,
    pub name: String,
    pub description: Option<String>,
    pub purpose: String,
    pub traits: Option<Vec<String>>,
    #[serde(flatten)]
    pub extensions: Extensions,
}
