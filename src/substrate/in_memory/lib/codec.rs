use std::collections::HashSet;

use crate::{
    error::primitive::PrimitiveError,
    substrate::{
        lib::serde::{insert_path_value, value_at_path},
        pipeline::{Codec, FieldMapping, ValueSlot},
    },
};

pub struct InMemoryCodec;

impl Codec for InMemoryCodec {
    type Slot = ValueSlot;
    type Encoded = String;

    fn encode(
        &self,
        entity_json: &serde_json::Value,
        schema: &[FieldMapping<Self::Slot>],
    ) -> Result<Self::Encoded, PrimitiveError> {
        let mut slice = serde_json::Map::new();

        // Phase 1: literal-key slots. Each `Value` field looks up its
        // wire key in entity_json and writes that entry into the slice
        // at the same dot-path so the stored blob mirrors wire shape
        // (nested objects, not literal dot-keys).
        for field in schema {
            match field.slot {
                ValueSlot::Value => {
                    if let Some(value) = value_at_path(entity_json, field.key) {
                        insert_path_value(&mut slice, field.key, value.clone());
                    }
                }
                ValueSlot::Flattened(_) => {}
            }
        }

        // Phase 2: route entity_json's unclaimed top-level wire keys
        // through the asset's flatten slots by longest-prefix-match.
        // A key with no matching rule is a codec-level error — this
        // asset doesn't know how to persist it.
        if let Some(obj) = entity_json.as_object() {
            let claimed = claimed_top_level_keys(schema);
            for (key, value) in obj {
                if claimed.contains(key.as_str()) {
                    continue;
                }
                if best_flatten_match(schema, key).is_none() {
                    return Err(PrimitiveError::unsupported_slot_composition(
                        "no flatten slot accepts this wire key",
                        "flattened",
                        key,
                    ));
                }
                slice.insert(key.clone(), value.clone());
            }
        }

        serde_json::to_string(&serde_json::Value::Object(slice)).map_err(|e| {
            PrimitiveError::JsonEncoding {
                context: PrimitiveError::context("json encoding failed"),
                field: "in_memory".to_string(),
                reason: e.to_string(),
            }
        })
    }

    fn decode(
        &self,
        raw: &Self::Encoded,
        schema: &[FieldMapping<Self::Slot>],
    ) -> Result<serde_json::Value, PrimitiveError> {
        let value: serde_json::Value =
            serde_json::from_str(raw).map_err(|_| PrimitiveError::MalformedJsonPayload {
                context: PrimitiveError::context("malformed json payload"),
                raw_snippet: raw.clone(),
            })?;
        let serde_json::Value::Object(obj) = value else {
            return Err(PrimitiveError::MalformedJsonPayload {
                context: PrimitiveError::context("expected json object at slice root"),
                raw_snippet: raw.clone(),
            });
        };

        // The stored blob is already wire-shaped; the only check is
        // that every top-level key is either claimed by a literal-key
        // slot or absorbed by a flatten slot. An unclaimed unmatched
        // key means the asset's schema can't represent something on
        // disk — surface as a codec rejection so loaders fail fast.
        let claimed = claimed_top_level_keys(schema);
        for key in obj.keys() {
            if claimed.contains(key.as_str()) {
                continue;
            }
            if best_flatten_match(schema, key).is_none() {
                return Err(PrimitiveError::unsupported_slot_composition(
                    "no flatten slot accepts this stored key",
                    "flattened",
                    key,
                ));
            }
        }

        Ok(serde_json::Value::Object(obj))
    }
}

fn claimed_top_level_keys(schema: &[FieldMapping<ValueSlot>]) -> HashSet<&'static str> {
    let mut claimed: HashSet<&'static str> = HashSet::new();
    claimed.insert("entity_ref");
    for field in schema {
        if matches!(field.slot, ValueSlot::Flattened(_)) {
            continue;
        }
        let head = field
            .key
            .split_once('.')
            .map(|(head, _)| head)
            .unwrap_or(field.key);
        claimed.insert(head);
    }
    claimed
}

fn best_flatten_match(schema: &[FieldMapping<ValueSlot>], key: &str) -> Option<ValueSlot> {
    schema
        .iter()
        .filter_map(|field| match field.slot {
            ValueSlot::Flattened(rule) => rule.match_len(key).map(|len| (len, field.slot)),
            _ => None,
        })
        .max_by_key(|(len, _)| *len)
        .map(|(_, slot)| slot)
}

#[cfg(test)]
mod tests {
    //! In-memory codec helpers mirror the repo codec's, just over a
    //! simpler slot type (`ValueSlot` has only `Value` and
    //! `Flattened` variants — no positional concept). Tests pin
    //! claimed-key derivation and longest-prefix flatten resolution.

    use super::*;
    use crate::substrate::pipeline::FlattenRule;

    fn fields(entries: &[(&'static str, ValueSlot)]) -> Vec<FieldMapping<ValueSlot>> {
        entries
            .iter()
            .map(|(k, s)| FieldMapping { key: k, slot: *s })
            .collect()
    }

    #[test]
    fn claimed_keys_includes_entity_ref_and_field_heads() {
        let s = fields(&[
            ("name", ValueSlot::Value),
            ("artifact.kind", ValueSlot::Value),
            (
                "extensions",
                ValueSlot::Flattened(FlattenRule::Prefix("x-")),
            ),
        ]);
        let claimed = claimed_top_level_keys(&s);
        assert!(claimed.contains("entity_ref"));
        assert!(claimed.contains("name"));
        assert!(claimed.contains("artifact"));
        assert!(!claimed.contains("artifact.kind"));
        assert!(!claimed.contains("extensions"));
    }

    #[test]
    fn best_flatten_match_picks_only_matching_slot() {
        let s = fields(&[(
            "extensions",
            ValueSlot::Flattened(FlattenRule::Prefix("x-")),
        )]);
        assert!(matches!(
            best_flatten_match(&s, "x-color"),
            Some(ValueSlot::Flattened(_))
        ));
        assert!(best_flatten_match(&s, "rogue").is_none());
    }

    #[test]
    fn best_flatten_match_longest_prefix_wins() {
        // Two flatten slots with overlapping prefixes co-existing in
        // one asset is allowed; longest match wins per key.
        let s = fields(&[
            (
                "extensions",
                ValueSlot::Flattened(FlattenRule::Prefix("x-")),
            ),
            (
                "extensions",
                ValueSlot::Flattened(FlattenRule::Prefix("x-doc-")),
            ),
        ]);
        // `x-doc-rationale` matches both; `x-doc-` is longer.
        let general = ValueSlot::Flattened(FlattenRule::Prefix("x-"));
        let specific = ValueSlot::Flattened(FlattenRule::Prefix("x-doc-"));
        let _ = general; // silence dead-code on the variant references
        let _ = specific;

        // The chosen slot's rule should match `x-doc-rationale` with
        // length 6; the general rule would have produced 2.
        let chosen = best_flatten_match(&s, "x-doc-rationale").expect("matches");
        match chosen {
            ValueSlot::Flattened(rule) => {
                assert_eq!(rule.match_len("x-doc-rationale"), Some(6));
            }
            _ => panic!("expected Flattened slot"),
        }

        // A non-doc key only matches the general rule (length 2).
        let chosen = best_flatten_match(&s, "x-color").expect("matches");
        match chosen {
            ValueSlot::Flattened(rule) => {
                assert_eq!(rule.match_len("x-color"), Some(2));
            }
            _ => panic!("expected Flattened slot"),
        }
    }

    #[test]
    fn best_flatten_match_no_flatten_slots_returns_none() {
        let s = fields(&[("name", ValueSlot::Value)]);
        assert!(best_flatten_match(&s, "x-foo").is_none());
    }
}
