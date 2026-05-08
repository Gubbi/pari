use std::collections::HashMap;

use crate::{
    error::primitive::PrimitiveError,
    substrate::{
        lib::serde::value_at_path,
        pipeline::{Codec, FieldMapping, ValueSlot},
    },
};

// TODO (step 3): rewrite to emit wire-flat slice JSON without relying
// on a nested "extensions" envelope. Until then this codec keeps the
// pre-refactor field-map shape, lifted into a Value::Object at the
// trait boundary.

pub struct InMemoryCodec;

impl Codec for InMemoryCodec {
    type Slot = ValueSlot;
    type Encoded = String;

    fn encode(
        &self,
        entity_json: &serde_json::Value,
        schema: &[FieldMapping<Self::Slot>],
    ) -> Result<Self::Encoded, PrimitiveError> {
        let mut fields = HashMap::new();
        for field in schema {
            if let Some(value) = value_at_path(entity_json, field.key) {
                fields.insert(field.key, value.clone());
            }
        }
        serde_json::to_string(&fields).map_err(|e| PrimitiveError::JsonEncoding {
            context: PrimitiveError::context("json encoding failed"),
            field: "in_memory".to_string(),
            reason: e.to_string(),
        })
    }

    fn decode(
        &self,
        raw: &Self::Encoded,
        _schema: &[FieldMapping<Self::Slot>],
    ) -> Result<serde_json::Value, PrimitiveError> {
        let map: HashMap<String, serde_json::Value> =
            serde_json::from_str(raw).map_err(|_| PrimitiveError::MalformedJsonPayload {
                context: PrimitiveError::context("malformed json payload"),
                raw_snippet: raw.clone(),
            })?;
        Ok(serde_json::Value::Object(map.into_iter().collect()))
    }
}
