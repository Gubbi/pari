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

pub(crate) fn insert_path_value(
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

#[cfg(test)]
mod tests {
    //! Unit coverage for the dot-path helpers. The codec layer trusts
    //! these to thread nested keys correctly — a regression here
    //! breaks every dot-pathed field (Task's `artifact.kind`, RACI
    //! children, etc.). One earlier production bug in
    //! `project_to_fields` traced back to dot-path handling at this
    //! layer; pinning the contract directly.
    //!
    //! Both helpers split on the literal `.` character. There is no
    //! escape syntax — keys with dots in them are not supported by
    //! design.
    use serde_json::json;

    use super::*;

    // -----------------------------------------------------------------------
    // value_at_path
    // -----------------------------------------------------------------------

    #[test]
    fn value_at_path_top_level_key() {
        let v = json!({"name": "eng-lead", "purpose": "test"});
        assert_eq!(value_at_path(&v, "name"), Some(&json!("eng-lead")));
    }

    #[test]
    fn value_at_path_dot_path_walks_nested() {
        let v = json!({"artifact": {"kind": "doc", "template": "x"}});
        assert_eq!(value_at_path(&v, "artifact.kind"), Some(&json!("doc")));
    }

    #[test]
    fn value_at_path_deep_dot_path() {
        let v = json!({"a": {"b": {"c": {"d": 7}}}});
        assert_eq!(value_at_path(&v, "a.b.c.d"), Some(&json!(7)));
    }

    #[test]
    fn value_at_path_missing_top_level_returns_none() {
        let v = json!({"name": "x"});
        assert_eq!(value_at_path(&v, "absent"), None);
    }

    #[test]
    fn value_at_path_missing_intermediate_returns_none() {
        let v = json!({"artifact": {"kind": "doc"}});
        assert_eq!(value_at_path(&v, "artifact.template"), None);
    }

    #[test]
    fn value_at_path_intermediate_not_object_returns_none() {
        // A scalar where an object was expected aborts the walk.
        let v = json!({"artifact": "not-an-object"});
        assert_eq!(value_at_path(&v, "artifact.kind"), None);
    }

    #[test]
    fn value_at_path_empty_path_returns_value_unchanged() {
        // Splitting "" yields a single empty segment; `.get("")` on
        // an object lookups the empty string. If absent (the usual
        // case) → None.
        let v = json!({"name": "x"});
        assert_eq!(value_at_path(&v, ""), None);
    }

    #[test]
    fn value_at_path_null_value_at_path() {
        // null is a valid JSON value; the path resolves to `&Null`
        // rather than None.
        let v = json!({"description": null});
        assert_eq!(value_at_path(&v, "description"), Some(&json!(null)));
    }

    // -----------------------------------------------------------------------
    // insert_path_value
    // -----------------------------------------------------------------------

    fn insert(path: &str, value: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
        let mut target = serde_json::Map::new();
        insert_path_value(&mut target, path, value);
        target
    }

    #[test]
    fn insert_top_level_key() {
        let m = insert("name", json!("eng-lead"));
        assert_eq!(m.get("name"), Some(&json!("eng-lead")));
    }

    #[test]
    fn insert_dot_path_creates_nested_object() {
        let m = insert("artifact.kind", json!("doc"));
        assert_eq!(
            serde_json::Value::Object(m),
            json!({"artifact": {"kind": "doc"}})
        );
    }

    #[test]
    fn insert_deep_dot_path_creates_chain_of_objects() {
        let m = insert("a.b.c.d", json!(7));
        assert_eq!(
            serde_json::Value::Object(m),
            json!({"a": {"b": {"c": {"d": 7}}}})
        );
    }

    #[test]
    fn insert_two_dot_paths_under_same_head_share_object() {
        let mut target = serde_json::Map::new();
        insert_path_value(&mut target, "artifact.kind", json!("doc"));
        insert_path_value(&mut target, "artifact.template", json!("body"));
        assert_eq!(
            serde_json::Value::Object(target),
            json!({"artifact": {"kind": "doc", "template": "body"}})
        );
    }

    #[test]
    fn insert_overwrites_existing_top_level_key() {
        let mut target = serde_json::Map::new();
        insert_path_value(&mut target, "name", json!("first"));
        insert_path_value(&mut target, "name", json!("second"));
        assert_eq!(target.get("name"), Some(&json!("second")));
    }

    #[test]
    fn insert_replaces_non_object_intermediate_with_fresh_object() {
        // If a previous insert put a scalar at "artifact" and a later
        // insert wants "artifact.kind", the helper replaces the
        // scalar with a fresh object — the dot-path insert wins, the
        // scalar is lost. This is the documented (if implicit)
        // contract; pin it explicitly here.
        let mut target = serde_json::Map::new();
        insert_path_value(&mut target, "artifact", json!("scalar"));
        insert_path_value(&mut target, "artifact.kind", json!("doc"));
        assert_eq!(
            serde_json::Value::Object(target),
            json!({"artifact": {"kind": "doc"}})
        );
    }

    #[test]
    fn insert_null_value_at_dot_path() {
        let m = insert("description.full", json!(null));
        assert_eq!(
            serde_json::Value::Object(m),
            json!({"description": {"full": null}})
        );
    }

    #[test]
    fn insert_nested_object_value() {
        // The value itself can be an object — it lands as-is at the
        // leaf path.
        let m = insert("artifact", json!({"kind": "doc", "template": null}));
        assert_eq!(
            serde_json::Value::Object(m),
            json!({"artifact": {"kind": "doc", "template": null}})
        );
    }
}
