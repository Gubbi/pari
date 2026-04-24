use crate::entity::{types::Extensions, EntityKind, EntityRef};

/// A named responsibility within a team.
///
/// Roles are the vocabulary teams use to say *who does what* without naming
/// specific people. They are referenced by [`Raci`](crate::entity::types::Raci)
/// assignments, team rosters, and review approvers — so a workflow can declare
/// "the accountable role is `pm`" and the resolution to an actual handle
/// happens through [`Team`](crate::entity::entities::team::Team).
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
