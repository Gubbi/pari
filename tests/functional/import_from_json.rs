//! User job: import a raw-JSON entity into a workspace via
//! [`Workspace::import_json`].
//!
//! Sectioned by concern: happy path (valid wire JSON imports cleanly),
//! schema rejections (malformed JSON is rejected at the schema gate),
//! validation ordering (the schema gate runs first; structural and
//! semantic tiers still fire when the user calls `validate` on the
//! returned viewer), and `Extensions` `x-` prefix round-trip.
//!
//! `import_json` is a pure caller-facing operation that does not touch
//! the store unless the caller subsequently inserts. Scenarios that do
//! not insert run against `InMemorySubstrate` only — substrate is
//! incidental.

use pari::{
    entities::{role::Role, team::Team, workflow::Workflow},
    error::{primitive::PrimitiveError, ActivityError},
    types::Extensions,
    workspace::XViewer,
};
use serde_json::json;

use crate::{
    common::substrate::{run_with, SubstrateKind},
    fixtures::{
        role::{a_minimal_role, a_role_with_optional_fields},
        team::a_minimal_team,
        workflow::a_workflow_with_empty_steps,
    },
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn assert_schema_rejection<T: std::fmt::Debug>(
    result: Result<XViewer<'_, T>, ActivityError>,
    needle: &str,
) where
    T: pari::entity::Entity,
{
    let err = result.err().expect("expected ValidationFailed");
    let cause = match &err {
        ActivityError::ValidationFailed { cause, .. } => cause,
        _ => panic!("expected ValidationFailed, got: {err:?}"),
    };
    let reason = match cause {
        PrimitiveError::PartialPayloadDeserialization { reason, .. } => reason,
        _ => panic!("expected PartialPayloadDeserialization, got: {cause:?}"),
    };
    assert!(
        reason.to_lowercase().contains(&needle.to_lowercase()),
        "expected schema-rejection reason to mention '{needle}', got: {reason}"
    );
}

fn assert_field_validation_error(
    result: Result<(), ActivityError>,
    field: &str,
    matches: impl Fn(&PrimitiveError) -> bool,
) {
    let err = result.expect_err("expected ValidationFailed");
    let cause = match &err {
        ActivityError::ValidationFailed { cause, .. } => cause,
        _ => panic!("expected ValidationFailed, got: {err:?}"),
    };
    let errors = match cause {
        PrimitiveError::FieldValidationError { errors, .. } => errors,
        _ => panic!("expected FieldValidationError, got: {cause:?}"),
    };
    let field_errors = errors
        .get(field)
        .unwrap_or_else(|| panic!("expected errors at field '{field}', got: {errors:?}"));
    assert!(
        field_errors.iter().any(matches),
        "expected matching PrimitiveError at '{field}', got: {field_errors:?}"
    );
}

// ===========================================================================
// Happy path
// ===========================================================================

#[tokio::test]
async fn import_json_role_round_trips_through_persist() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        let role = a_role_with_optional_fields("eng-lead");
        let value = serde_json::to_value(&role).unwrap();

        let viewer = workspace
            .import_json::<Role>(value)
            .expect("valid role JSON should pass schema validation");

        assert_eq!(viewer.name().await.unwrap(), "Engineering Lead");
        assert_eq!(viewer.purpose().await.unwrap(), "test purpose");
        assert_eq!(
            viewer.description().await.unwrap(),
            Some("Owns delivery of the engineering roadmap."),
        );
    })
    .await;
}

#[tokio::test]
async fn import_json_team_round_trips_through_persist() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        let team = a_minimal_team("eng");
        let value = serde_json::to_value(&team).unwrap();

        let viewer = workspace
            .import_json::<Team>(value)
            .expect("valid team JSON should pass schema validation");

        assert_eq!(viewer.name().await.unwrap(), "Minimal Team");
    })
    .await;
}

#[tokio::test]
async fn import_json_workflow_round_trips_through_persist() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        let wf = a_workflow_with_empty_steps("DesignFlow", "eng-lead");
        let value = serde_json::to_value(&wf).unwrap();

        let viewer = workspace
            .import_json::<Workflow>(value)
            .expect("valid workflow JSON should pass schema validation");

        assert_eq!(viewer.name().await.unwrap(), "Design Workflow");
        assert_eq!(
            viewer.purpose().await.unwrap(),
            "Drive a single design through review.",
        );
    })
    .await;
}

// ===========================================================================
// Schema rejections
// ===========================================================================

#[tokio::test]
async fn import_json_rejects_missing_required_field() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        // `purpose` is required by the Role schema.
        let value = json!({
            "entity_ref": {"id": "eng-lead", "kind": "Role"},
            "name": "Engineering Lead",
        });

        assert_schema_rejection(workspace.import_json::<Role>(value), "purpose");
    })
    .await;
}

#[tokio::test]
async fn import_json_rejects_wrong_json_type() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        // `name` must be a string; supply a number.
        let value = json!({
            "entity_ref": {"id": "eng-lead", "kind": "Role"},
            "name": 42,
            "purpose": "test",
        });

        assert_schema_rejection(workspace.import_json::<Role>(value), "string");
    })
    .await;
}

#[tokio::test]
async fn import_json_rejects_unknown_top_level_field() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        // `additionalProperties: false` together with `patternProperties: ^x-`
        // means a bare (non-`x-`) unknown key is rejected by the schema gate.
        // This also covers the "bare extension key" case: at the wire level
        // an extension key without the `x-` prefix is indistinguishable from
        // any other unknown top-level field.
        let value = json!({
            "entity_ref": {"id": "eng-lead", "kind": "Role"},
            "name": "Engineering Lead",
            "purpose": "test",
            "rogue": "value",
        });

        assert_schema_rejection(workspace.import_json::<Role>(value), "rogue");
    })
    .await;
}

// ===========================================================================
// Validation ordering — schema gate runs first; later tiers still fire
// when the caller invokes `validate` on the returned viewer.
// ===========================================================================

#[tokio::test]
async fn import_json_schema_valid_then_structural_validate_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        // Empty `name` is allowed by the wire schema (string type, no
        // minLength) but rejected by the `non_empty_str` structural rule.
        let value = json!({
            "entity_ref": {"id": "eng-lead", "kind": "Role"},
            "name": "",
            "purpose": "test",
        });

        let viewer = workspace
            .import_json::<Role>(value)
            .expect("schema accepts empty string for `name`");

        assert_field_validation_error(viewer.validate().await, "name", |e| {
            matches!(e, PrimitiveError::EmptyRequiredValue { .. })
        });
    })
    .await;
}

#[tokio::test]
async fn import_json_schema_and_structural_valid_then_cross_entity_validate_fails() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        // The Role itself is structurally valid but its full validation
        // tier set runs cross-entity rules too. A standalone Role has no
        // cross-entity refs, so use a Workflow whose RACI references a role
        // that does not exist in the store — schema and structural pass;
        // cross-entity fails on the missing role ref.
        let wf = a_workflow_with_empty_steps("DesignFlow", "eng-lead");
        let value = serde_json::to_value(&wf).unwrap();

        let viewer = workspace
            .import_json::<Workflow>(value)
            .expect("schema accepts well-formed workflow");

        assert_field_validation_error(viewer.validate().await, "raci", |e| {
            matches!(e, PrimitiveError::ReferencedEntityAbsent { .. })
        });
    })
    .await;
}

// ===========================================================================
// Extensions round-trip
// ===========================================================================

#[tokio::test]
async fn import_json_strips_x_prefix_on_extension_keys() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        let value = json!({
            "entity_ref": {"id": "eng-lead", "kind": "Role"},
            "name": "Engineering Lead",
            "purpose": "test",
            "x-color": "red",
            "x-priority": 7,
        });

        let viewer = workspace
            .import_json::<Role>(value)
            .expect("schema accepts x- prefixed extensions");

        let extensions = viewer.extensions().await.unwrap();
        assert_eq!(extensions.0.len(), 2);
        assert_eq!(extensions.0.get("color"), Some(&json!("red")));
        assert_eq!(extensions.0.get("priority"), Some(&json!(7)));
    })
    .await;
}

#[tokio::test]
async fn serialize_prepends_x_prefix_on_extension_keys() {
    let mut role = a_minimal_role("eng-lead");
    role.extensions.0.insert("color".to_string(), json!("red"));
    role.extensions.0.insert("priority".to_string(), json!(7));

    let value = serde_json::to_value(&role).unwrap();
    let obj = value.as_object().expect("role serializes to object");

    assert_eq!(obj.get("x-color"), Some(&json!("red")));
    assert_eq!(obj.get("x-priority"), Some(&json!(7)));
    assert!(
        !obj.contains_key("color") && !obj.contains_key("priority"),
        "serialized role should not carry bare extension keys: {obj:?}"
    );
}

#[tokio::test]
async fn extensions_round_trip_preserves_multiple_and_nested_values() {
    run_with(SubstrateKind::InMemory, |workspace| async move {
        let value = json!({
            "entity_ref": {"id": "eng-lead", "kind": "Role"},
            "name": "Engineering Lead",
            "purpose": "test",
            "x-string": "foo",
            "x-number": 42,
            "x-nested": {"deep": {"value": [1, 2, 3]}},
            "x-array": [true, false, null],
        });

        let viewer = workspace.import_json::<Role>(value.clone()).unwrap();
        let extensions = viewer.extensions().await.unwrap();

        assert_eq!(extensions.0.get("string"), Some(&json!("foo")));
        assert_eq!(extensions.0.get("number"), Some(&json!(42)));
        assert_eq!(
            extensions.0.get("nested"),
            Some(&json!({"deep": {"value": [1, 2, 3]}}))
        );
        assert_eq!(extensions.0.get("array"), Some(&json!([true, false, null])));

        // Serialize the tracked entity back out and confirm every key
        // re-acquires the `x-` prefix.
        let wire = serde_json::to_value(viewer.tracked()).unwrap();
        let obj = wire.as_object().unwrap();
        for k in ["x-string", "x-number", "x-nested", "x-array"] {
            assert!(obj.contains_key(k), "missing wire key {k}: {obj:?}");
        }
    })
    .await;
}

#[tokio::test]
async fn extensions_round_trip_handles_empty_map() {
    let role = a_minimal_role("eng-lead");
    let wire = serde_json::to_value(&role).unwrap();

    // Empty extensions: no `x-`-prefixed keys appear on the wire.
    let obj = wire.as_object().unwrap();
    assert!(
        !obj.keys().any(|k| k.starts_with("x-")),
        "empty Extensions should produce no x- keys: {obj:?}"
    );

    // Round-trip through `Extensions` directly — empty in, empty out.
    let empty = Extensions::default();
    let value = serde_json::to_value(&empty).unwrap();
    let back: Extensions = serde_json::from_value(value).unwrap();
    assert!(back.0.is_empty());
}

#[tokio::test]
async fn extensions_double_x_prefix_round_trip() {
    // Wire `x-x-foo` → in-memory `x-foo` (deserialize strips the first
    // `x-`); serializing back prepends `x-`, yielding `x-x-foo` again.
    let value = json!({"x-x-foo": 1});
    let extensions: Extensions = serde_json::from_value(value).unwrap();
    assert_eq!(extensions.0.get("x-foo"), Some(&json!(1)));

    let wire = serde_json::to_value(&extensions).unwrap();
    assert_eq!(wire, json!({"x-x-foo": 1}));
}
