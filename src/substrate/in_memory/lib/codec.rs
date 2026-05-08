use std::collections::HashSet;

use crate::{
    error::primitive::PrimitiveError,
    substrate::{
        lib::serde::value_at_path,
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
        // wire key in entity_json and writes that entry into the slice.
        for field in schema {
            match field.slot {
                ValueSlot::Value => {
                    if let Some(value) = value_at_path(entity_json, field.key) {
                        slice.insert(field.key.to_string(), value.clone());
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
