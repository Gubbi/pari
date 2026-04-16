use crate::{
    entity::{EntityKind, EntityRef},
    types::Extensions,
};

#[derive(pari_macros::Entity)]
#[entity(kind = EntityKind::ArtifactKind, schema = crate::validation::artifact_kind::artifact_kind_validation_schema)]
pub struct ArtifactKind {
    pub entity_ref: EntityRef<ArtifactKind>,
    pub name: String,
    pub description: Option<String>,
    pub service: String,
    pub access: Option<String>,
    pub guidance: Option<String>,
    pub extensions: Extensions,
}
