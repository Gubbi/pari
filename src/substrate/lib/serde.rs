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
    entity.to_json_value().map_err(|e| {
        PrimitiveError::entity_projection(
            "entity projection failed",
            entity.any_ref().id().to_string(),
            e.to_string(),
        )
    })
}

/// Merge a codec-decoded slice into an in-progress JSON accumulator
/// that already carries `entity_ref`. Dot-notation keys (e.g.
/// `"raci.accountable"`) become nested objects.
///
/// `extensions` (if the codec emits it as a nested envelope) is
/// flattened so its keys land at the entity-root namespace. Codecs
/// rewritten to emit wire-flat slices directly will not surface an
/// `extensions` key here; the special case is harmless in that
/// scenario and slated for removal once both backends produce
/// wire-flat output.
pub(crate) fn merge_field_map_into_json(
    accumulator: &mut serde_json::Map<String, serde_json::Value>,
    field_map: serde_json::Value,
) {
    let serde_json::Value::Object(mut obj) = field_map else {
        return;
    };

    if let Some(ext) = obj.remove("extensions") {
        if let serde_json::Value::Object(inner) = ext {
            for (k, v) in inner {
                obj.insert(k, v);
            }
        }
    }

    for (key, value) in obj {
        insert_path_value(accumulator, &key, value);
    }
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

pub(crate) fn any_ref_json(any_ref: &AnyEntityRef) -> serde_json::Value {
    any_ref
        .to_json_value()
        .expect("entity refs should always serialize")
}
