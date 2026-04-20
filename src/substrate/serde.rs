use std::collections::HashMap;

use crate::{
    entity::{AnyEntityRef, TrackedEntity},
    error::primitive::PrimitiveError,
};

pub(crate) fn any_ref_to_stub_json(any_ref: &AnyEntityRef) -> serde_json::Value {
    serde_json::json!({
        "entity_ref": any_ref_json(any_ref)
    })
}

pub(crate) fn entity_to_json(entity: &TrackedEntity) -> Result<serde_json::Value, PrimitiveError> {
    entity
        .to_json_value()
        .map_err(|e| PrimitiveError::EntityProjection {
            context: PrimitiveError::context("entity projection failed"),
            entity_ref: entity.any_ref().id().to_string(),
            reason: e.to_string(),
        })
}

pub(crate) fn merge_field_map_into(
    target: &mut TrackedEntity,
    mut field_map: HashMap<String, serde_json::Value>,
) -> Result<(), PrimitiveError> {
    if let Some(ext) = field_map.remove("extensions") {
        if let Some(obj) = ext.as_object() {
            for (k, v) in obj {
                field_map.insert(k.clone(), v.clone());
            }
        }
    }

    let mut map = serde_json::Map::new();
    map.insert("entity_ref".to_string(), any_ref_json(&target.any_ref()));
    for (key, value) in field_map {
        insert_path_value(&mut map, &key, value);
    }

    let partial = deserialize_entity_from_value(&target.any_ref(), serde_json::Value::Object(map))?;
    partial.initialize_into(target);
    Ok(())
}

pub(crate) fn value_at_path<'a>(
    value: &'a serde_json::Value,
    path: &str,
) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

fn insert_path_value(
    target: &mut serde_json::Map<String, serde_json::Value>,
    path: &str,
    value: serde_json::Value,
) {
    let mut segments = path.split('.').peekable();
    let mut current = target;

    while let Some(segment) = segments.next() {
        if segments.peek().is_none() {
            current.insert(segment.to_string(), value);
            return;
        }

        let entry = current
            .entry(segment.to_string())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        if !entry.is_object() {
            *entry = serde_json::Value::Object(serde_json::Map::new());
        }
        current = entry.as_object_mut().expect("object inserted above");
    }
}

fn deserialize_entity_from_value(
    any_ref: &AnyEntityRef,
    value: serde_json::Value,
) -> Result<TrackedEntity, PrimitiveError> {
    TrackedEntity::from_json_value(any_ref, value).map_err(|e| {
        PrimitiveError::PartialPayloadDeserialization {
            context: PrimitiveError::context("partial payload deserialization failed"),
            entity_ref: any_ref.id().to_string(),
            reason: e.to_string(),
        }
    })
}

fn any_ref_json(any_ref: &AnyEntityRef) -> serde_json::Value {
    any_ref
        .to_json_value()
        .expect("entity refs should always serialize")
}
