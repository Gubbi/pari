use pari::entities::role::{Role, TrackedRole};
use pari::entity::EntityRef;
use pari::tracked::TrackedField;
use std::sync::Arc;
use std::collections::HashMap;

// Helper: build a fully-populated TrackedRole
fn full_tracked_role() -> TrackedRole {
    let plain = Role {
        entity_ref:  EntityRef::new("eng-lead"),
        name:        "Engineering Lead".to_string(),
        description: Some("Senior tech lead".to_string()),
        purpose:     "Leads engineering".to_string(),
        traits:      Some(vec!["reviewer".to_string()]),
        extensions:  {
            let mut m = HashMap::new();
            m.insert("x-owner".to_string(), serde_json::json!("alice"));
            m
        },
    };
    TrackedRole::from(plain)
}

// Helper: build a partially-populated TrackedRole (only name initialized)
fn partial_tracked_role() -> TrackedRole {
    TrackedRole {
        entity_ref:  EntityRef::new("eng-lead"),
        name:        Arc::new(TrackedField::new_initialized("Engineering Lead".to_string())),
        description: Arc::new(TrackedField::new()),  // uninitialized
        purpose:     Arc::new(TrackedField::new()),  // uninitialized
        traits:      Arc::new(TrackedField::new()),  // uninitialized
        extensions:  Arc::new(TrackedField::new()),  // uninitialized
    }
}

// --- Serialize ---

#[test]
fn full_tracked_role_serializes_all_fields() {
    let tracked = full_tracked_role();
    let json = serde_json::to_value(&tracked).unwrap();
    assert!(json.get("name").is_some());
    assert!(json.get("description").is_some());
    assert!(json.get("purpose").is_some());
    assert!(json.get("traits").is_some());
    assert!(json.get("x-owner").is_some()); // extensions are flattened
}

#[test]
fn partial_tracked_role_skips_uninitialized_fields() {
    let tracked = partial_tracked_role();
    let json = serde_json::to_value(&tracked).unwrap();
    assert!(json.get("name").is_some());
    assert!(json.get("description").is_none(), "uninitialized field must be absent");
    assert!(json.get("purpose").is_none());
}

#[test]
fn extensions_are_flattened_in_serialized_output() {
    let tracked = full_tracked_role();
    let json = serde_json::to_value(&tracked).unwrap();
    // Extension key appears at top level, not under "extensions"
    assert!(json.get("x-owner").is_some());
    assert!(json.get("extensions").is_none());
}

// --- Deserialize ---

#[test]
fn deserialize_full_json_initializes_all_fields() {
    let json = serde_json::json!({
        "entity_ref": { "id": "eng-lead", "kind": "Role" },
        "name": "Engineering Lead",
        "description": "Senior tech lead",
        "purpose": "Leads engineering",
        "traits": ["reviewer"],
        "x-owner": "alice"
    });
    let tracked: TrackedRole = serde_json::from_value(json).unwrap();
    assert_eq!(tracked.name.get(), Some(&"Engineering Lead".to_string()));
    assert_eq!(tracked.description.get(), Some(&Some("Senior tech lead".to_string())));
    assert_eq!(tracked.purpose.get(), Some(&"Leads engineering".to_string()));
    assert!(tracked.extensions.get().is_some());
    assert_eq!(
        tracked.extensions.get().unwrap().get("x-owner"),
        Some(&serde_json::json!("alice"))
    );
}

#[test]
fn deserialize_partial_json_leaves_missing_fields_uninitialized() {
    let json = serde_json::json!({
        "entity_ref": { "id": "eng-lead", "kind": "Role" },
        "name": "Engineering Lead"
    });
    let tracked: TrackedRole = serde_json::from_value(json).unwrap();
    assert_eq!(tracked.name.get(), Some(&"Engineering Lead".to_string()));
    assert!(tracked.purpose.get().is_none(), "absent field must remain uninitialized");
    assert!(tracked.description.get().is_none());
}

#[test]
fn deserialize_missing_entity_ref_returns_error() {
    let json = serde_json::json!({ "name": "Engineering Lead" });
    let result: Result<TrackedRole, _> = serde_json::from_value(json);
    assert!(result.is_err(), "entity_ref is required");
}

#[test]
fn deserialized_fields_are_not_dirty() {
    let json = serde_json::json!({
        "entity_ref": { "id": "eng-lead", "kind": "Role" },
        "name": "Engineering Lead",
        "purpose": "Leads"
    });
    let tracked: TrackedRole = serde_json::from_value(json).unwrap();
    // initialize() does not set dirty
    assert!(!tracked.name.is_dirty());
    assert!(!tracked.purpose.is_dirty());
}

// --- Roundtrip ---

#[test]
fn full_serialize_deserialize_roundtrip() {
    let original = full_tracked_role();
    let json = serde_json::to_value(&original).unwrap();
    let restored: TrackedRole = serde_json::from_value(json).unwrap();

    assert_eq!(restored.name.get(), original.name.get());
    assert_eq!(restored.purpose.get(), original.purpose.get());
    assert_eq!(restored.description.get(), original.description.get());
    assert_eq!(restored.traits.get(), original.traits.get());
    assert_eq!(restored.entity_ref().id(), original.entity_ref().id());
}

#[test]
fn partial_serialize_deserialize_roundtrip_preserves_partial_state() {
    let original = partial_tracked_role();
    let json = serde_json::to_value(&original).unwrap();
    let restored: TrackedRole = serde_json::from_value(json).unwrap();

    assert_eq!(restored.name.get(), Some(&"Engineering Lead".to_string()));
    assert!(restored.purpose.get().is_none(), "uninitialized field stays uninitialized");
}

// --- EntityRef serde ---

#[test]
fn entity_ref_serializes_with_id_and_kind() {
    let r: EntityRef<Role> = EntityRef::new("eng-lead");
    let json = serde_json::to_value(&r).unwrap();
    assert_eq!(json.get("id").and_then(|v| v.as_str()), Some("eng-lead"));
    assert_eq!(json.get("kind").and_then(|v| v.as_str()), Some("Role"));
}

#[test]
fn entity_ref_deserializes_from_id_and_kind() {
    let json = serde_json::json!({ "id": "eng-lead", "kind": "Role" });
    let r: EntityRef<Role> = serde_json::from_value(json).unwrap();
    assert_eq!(r.id(), "eng-lead");
}
