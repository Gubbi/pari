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
    entity::{AnyEntityRef, EntityKind},
    substrate::{pipeline, Substrate},
};

/// Dispatches `&AnyEntityRef` → `&'static EntitySchema` for the receiving
/// backend. Automatically implemented for any [`Substrate`] whose entity
/// set — currently the fixed nine kinds — all have a
/// [`pipeline::SubstrateSchema<Self>`] impl.
///
/// The trait surface speaks `&AnyEntityRef`; per-kind dispatch is an
/// internal detail. Substrate-layer code that already has an
/// `EntityKind` in hand (e.g. the repo resolver decoding JSON) reaches
/// the same schemas through the `pub(crate)` [`schema_for_kind`] helper
/// below.
pub trait SchemaBackedSubstrate: Substrate {
    fn schema_for(any_ref: &AnyEntityRef) -> &'static pipeline::EntitySchema<Self::Slot>;
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
    fn schema_for(any_ref: &AnyEntityRef) -> &'static pipeline::EntitySchema<Self::Slot> {
        schema_for_kind::<Sub>(any_ref.kind())
    }
}

/// Substrate-internal kind-keyed dispatch. Used by code paths that
/// already hold an `EntityKind` (e.g. resolvers decoding JSON) and by
/// the trait's public [`SchemaBackedSubstrate::schema_for`] method.
pub(crate) fn schema_for_kind<Sub>(kind: EntityKind) -> &'static pipeline::EntitySchema<Sub::Slot>
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
