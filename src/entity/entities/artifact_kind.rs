use crate::entity::{types::Extensions, EntityKind, EntityRef};

#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, pari_macros::Entity,
)]
#[schemars(deny_unknown_fields)]
#[entity(kind = EntityKind::ArtifactKind, schema = crate::validation::artifact_kind::artifact_kind_validation_schema)]
pub struct ArtifactKind {
    pub entity_ref: EntityRef<ArtifactKind>,
    pub name: String,
    pub description: Option<String>,
    pub service: String,
    pub access: Option<String>,
    pub guidance: Option<String>,
    #[serde(flatten)]
    pub extensions: Extensions,
}
