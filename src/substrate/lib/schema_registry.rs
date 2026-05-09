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
///
/// Field keys may be dot-paths (e.g. `"artifact.kind"`) when an asset
/// covers a sub-tree of a nested entity property. The projection
/// retains the top-level property (`"artifact"`) whenever any field's
/// first dot-segment matches it; finer-grained narrowing of nested
/// shapes would over-constrain the slice and is left to the entity
/// schema author.
fn project_to_fields(full: &serde_json::Value, fields: &[&'static str]) -> serde_json::Value {
    let mut projected = full.clone();
    let obj = projected
        .as_object_mut()
        .expect("entity schema is a JSON object");

    let top_level_fields: std::collections::HashSet<&str> = fields
        .iter()
        .map(|f| f.split_once('.').map(|(head, _)| head).unwrap_or(*f))
        .collect();

    if let Some(serde_json::Value::Object(props)) = obj.get_mut("properties") {
        props.retain(|k, _| top_level_fields.contains(k.as_str()));
    }
    if let Some(serde_json::Value::Array(req)) = obj.get_mut("required") {
        req.retain(|v| {
            v.as_str()
                .map(|s| top_level_fields.contains(s))
                .unwrap_or(false)
        });
    }

    projected
}

#[cfg(test)]
mod tests {
    //! Unit coverage for `project_to_fields`. The dot-path handling
    //! in particular caught a regression earlier — a Task slice's
    //! `artifact: {kind: ...}` was rejected because `artifact.kind`
    //! didn't match the entity-level property name literally. These
    //! tests pin the projection contract directly so the next change
    //! flags any drift.

    use serde_json::json;

    use super::*;

    fn full_role_like_schema() -> serde_json::Value {
        // Shape mirrors what `schemars` emits for a Role-ish entity:
        // top-level properties, `required`, plus the boundary
        // constraints (`patternProperties`, `additionalProperties`)
        // that the projection must carry over verbatim.
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "purpose": {"type": "string"},
                "description": {"type": ["string", "null"]},
                "traits": {"type": ["array", "null"]},
            },
            "required": ["name", "purpose"],
            "patternProperties": {"^x-": true},
            "additionalProperties": false,
        })
    }

    #[test]
    fn project_retains_only_fields_in_set() {
        let projected = project_to_fields(&full_role_like_schema(), &["name", "purpose"]);
        let props = projected
            .get("properties")
            .and_then(|v| v.as_object())
            .unwrap();
        let mut keys: Vec<&str> = props.keys().map(String::as_str).collect();
        keys.sort();
        assert_eq!(keys, vec!["name", "purpose"]);
    }

    #[test]
    fn project_filters_required_to_fields_in_set() {
        let projected = project_to_fields(&full_role_like_schema(), &["name"]);
        let req = projected
            .get("required")
            .and_then(|v| v.as_array())
            .unwrap();
        assert_eq!(req, &vec![json!("name")]);
    }

    #[test]
    fn project_retains_pattern_properties_verbatim() {
        let projected = project_to_fields(&full_role_like_schema(), &["name"]);
        assert_eq!(
            projected.get("patternProperties"),
            Some(&json!({"^x-": true}))
        );
    }

    #[test]
    fn project_retains_additional_properties_verbatim() {
        let projected = project_to_fields(&full_role_like_schema(), &["name"]);
        assert_eq!(projected.get("additionalProperties"), Some(&json!(false)));
    }

    #[test]
    fn project_empty_field_list_yields_empty_properties_and_required() {
        let projected = project_to_fields(&full_role_like_schema(), &[]);
        let props = projected
            .get("properties")
            .and_then(|v| v.as_object())
            .unwrap();
        assert!(props.is_empty());
        let req = projected
            .get("required")
            .and_then(|v| v.as_array())
            .unwrap();
        assert!(req.is_empty());
    }

    #[test]
    fn project_dot_path_field_retains_top_level_property() {
        // Task's `artifact.kind` field references the top-level
        // `artifact` property — the projection must keep it.
        let full = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "artifact": {
                    "type": "object",
                    "properties": {"kind": {"type": "object"}, "template": {"type": ["string", "null"]}},
                    "required": ["kind"],
                },
            },
            "required": ["name", "artifact"],
            "additionalProperties": false,
        });
        let projected = project_to_fields(&full, &["artifact.kind"]);
        let props = projected
            .get("properties")
            .and_then(|v| v.as_object())
            .unwrap();
        assert!(
            props.contains_key("artifact"),
            "expected `artifact`, got {props:?}"
        );
        assert!(!props.contains_key("name"));
        // Required is filtered the same way — `artifact` survives,
        // `name` does not.
        let req = projected
            .get("required")
            .and_then(|v| v.as_array())
            .unwrap();
        assert_eq!(req, &vec![json!("artifact")]);
    }

    #[test]
    fn project_multiple_dot_paths_under_one_head_collapse_to_one_property() {
        // Both `raci.accountable` and `raci.responsible` map to the
        // same top-level `raci` property; it appears once.
        let full = json!({
            "type": "object",
            "properties": {
                "raci": {"type": "object"},
                "name": {"type": "string"},
            },
            "required": ["raci"],
            "additionalProperties": false,
        });
        let projected = project_to_fields(&full, &["raci.accountable", "raci.responsible"]);
        let props = projected
            .get("properties")
            .and_then(|v| v.as_object())
            .unwrap();
        assert_eq!(props.len(), 1);
        assert!(props.contains_key("raci"));
    }

    #[test]
    fn project_field_not_in_entity_schema_drops_silently() {
        // If the asset declares a field key the entity schema doesn't
        // mention (a schema-author bug), the projection just drops
        // it from the retained set — no panic, no error. The schema
        // gate downstream catches the resulting validation oddities.
        let projected = project_to_fields(&full_role_like_schema(), &["bogus"]);
        let props = projected
            .get("properties")
            .and_then(|v| v.as_object())
            .unwrap();
        assert!(props.is_empty());
    }

    #[test]
    fn project_does_not_mutate_input() {
        // The function clones internally; the caller's view is
        // untouched. Pin so future "optimisation" doesn't break this.
        let full = full_role_like_schema();
        let _ = project_to_fields(&full, &["name"]);
        assert_eq!(
            full.get("properties")
                .and_then(|v| v.as_object())
                .map(|o| o.len()),
            Some(4),
            "input schema must not be mutated"
        );
    }
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
