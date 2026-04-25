use crate::{
    entities::{
        artifact_kind::ArtifactKind,
        hook::Hook,
        relay::Relay,
        role::Role,
        task::Task,
        team::Team,
        workflow::{EmbeddedWorkflow, ReusableWorkflow, Workflow},
    },
    entity::EntityKind,
    substrate::{pipeline, Substrate},
};

/// Dispatches `EntityKind` → `&'static EntitySchema` for the receiving
/// backend. Automatically implemented for any [`Substrate`] whose entity
/// set — currently the fixed nine kinds — all have a
/// [`pipeline::SubstrateSchema<Self>`] impl.
pub trait SchemaBackedSubstrate: Substrate {
    fn schema_for(kind: EntityKind) -> &'static pipeline::EntitySchema<Self::Slot>;
}

impl<Sub> SchemaBackedSubstrate for Sub
where
    Sub: Substrate,
    Role: pipeline::SubstrateSchema<Sub>,
    Hook: pipeline::SubstrateSchema<Sub>,
    Team: pipeline::SubstrateSchema<Sub>,
    ArtifactKind: pipeline::SubstrateSchema<Sub>,
    Workflow: pipeline::SubstrateSchema<Sub>,
    ReusableWorkflow: pipeline::SubstrateSchema<Sub>,
    EmbeddedWorkflow: pipeline::SubstrateSchema<Sub>,
    Task: pipeline::SubstrateSchema<Sub>,
    Relay: pipeline::SubstrateSchema<Sub>,
{
    fn schema_for(kind: EntityKind) -> &'static pipeline::EntitySchema<Self::Slot> {
        match kind {
            EntityKind::Role => &<Role as pipeline::SubstrateSchema<Sub>>::SCHEMA,
            EntityKind::Hook => &<Hook as pipeline::SubstrateSchema<Sub>>::SCHEMA,
            EntityKind::Team => &<Team as pipeline::SubstrateSchema<Sub>>::SCHEMA,
            EntityKind::Workflow => &<Workflow as pipeline::SubstrateSchema<Sub>>::SCHEMA,
            EntityKind::ReusableWorkflow => {
                &<ReusableWorkflow as pipeline::SubstrateSchema<Sub>>::SCHEMA
            }
            EntityKind::ArtifactKind => &<ArtifactKind as pipeline::SubstrateSchema<Sub>>::SCHEMA,
            EntityKind::Task => &<Task as pipeline::SubstrateSchema<Sub>>::SCHEMA,
            EntityKind::Relay => &<Relay as pipeline::SubstrateSchema<Sub>>::SCHEMA,
            EntityKind::EmbeddedWorkflow => {
                &<EmbeddedWorkflow as pipeline::SubstrateSchema<Sub>>::SCHEMA
            }
        }
    }
}
