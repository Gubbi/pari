use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};

use jsonschema::Validator;
use schemars::{schema_for, JsonSchema};

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

/// Per-kind cache of the entity's full schemars-derived JSON Schema.
/// Computed once at first access; reused by per-asset projection.
static FULL_SCHEMARS_BY_KIND: LazyLock<HashMap<EntityKind, serde_json::Value>> =
    LazyLock::new(|| {
        fn entry<T: JsonSchema>() -> serde_json::Value {
            serde_json::to_value(schema_for!(T)).expect("schemars schema is always serializable")
        }
        let mut map = HashMap::new();
        map.insert(EntityKind::Role, entry::<Role>());
        map.insert(EntityKind::Hook, entry::<Hook>());
        map.insert(EntityKind::Team, entry::<Team>());
        map.insert(EntityKind::ArtifactKind, entry::<ArtifactKind>());
        map.insert(EntityKind::Workflow, entry::<Workflow>());
        map.insert(EntityKind::ReusableWorkflow, entry::<ReusableWorkflow>());
        map.insert(EntityKind::EmbeddedWorkflow, entry::<EmbeddedWorkflow>());
        map.insert(EntityKind::Task, entry::<Task>());
        map.insert(EntityKind::Relay, entry::<Relay>());
        map
    });

/// Project the full entity schema down to the field set covered by a
/// single asset slice. Removes properties not in the field set, and
/// updates `required` similarly. The slice schema's other top-level
/// constraints (`patternProperties`, `additionalProperties`) carry
/// over verbatim.
fn project_to_fields(full: &serde_json::Value, fields: &[&'static str]) -> serde_json::Value {
    let mut projected = full.clone();
    let obj = projected
        .as_object_mut()
        .expect("entity schema is a JSON object");

    if let Some(serde_json::Value::Object(props)) = obj.get_mut("properties") {
        props.retain(|k, _| fields.contains(&k.as_str()));
    }
    if let Some(serde_json::Value::Array(req)) = obj.get_mut("required") {
        req.retain(|v| v.as_str().map(|s| fields.contains(&s)).unwrap_or(false));
    }

    projected
}

/// Build the per-(kind, asset path_template) projected validator map
/// for a backend. Each backend stamps its own `LazyLock<HashMap>` from
/// this helper.
pub(crate) fn build_validators_for<Sub>() -> HashMap<(EntityKind, &'static str), Arc<Validator>>
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
    let kinds = [
        EntityKind::Role,
        EntityKind::Hook,
        EntityKind::Team,
        EntityKind::ArtifactKind,
        EntityKind::Workflow,
        EntityKind::ReusableWorkflow,
        EntityKind::EmbeddedWorkflow,
        EntityKind::Task,
        EntityKind::Relay,
    ];
    let mut out = HashMap::new();
    for kind in kinds {
        let full = FULL_SCHEMARS_BY_KIND
            .get(&kind)
            .expect("every kind has a full schema");
        let entity_schema = schema_for_kind::<Sub>(kind);
        for asset in entity_schema.all_assets() {
            let fields: Vec<&'static str> = asset.fields().iter().map(|f| f.key).collect();
            let projected = project_to_fields(full, &fields);
            let validator = jsonschema::validator_for(&projected)
                .expect("projected schema compiles into a validator");
            out.insert((kind, asset.path_template()), Arc::new(validator));
        }
    }
    out
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
