use crate::entity::{types::Extensions, EntityKind, EntityRef};

/// A category of deliverable that a task can produce.
///
/// Tasks declare an [`Artifact`](crate::entity::types::Artifact) referencing
/// an `ArtifactKind` (e.g. `design-doc`, `pull-request`) rather than encoding
/// service details inline. This keeps the *what to produce* on the task and
/// the *where/how it lives* — service, access rules, authoring guidance — on
/// a reusable definition shared across workflows.
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
