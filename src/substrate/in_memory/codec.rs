use std::collections::HashMap;

use crate::substrate::{
    pipeline::{Codec, CodecError, FieldMapping, ValueSlot},
    serde::value_at_path,
};

pub struct InMemoryCodec;

impl Codec for InMemoryCodec {
    type Slot = ValueSlot;
    type Encoded = String;

    fn encode(
        &self,
        entity_json: &serde_json::Value,
        schema: &[FieldMapping<Self::Slot>],
    ) -> Result<Self::Encoded, CodecError> {
        let mut fields = HashMap::new();
        for field in schema {
            if let Some(value) = value_at_path(entity_json, field.key) {
                fields.insert(field.key, value.clone());
            }
        }
        serde_json::to_string(&fields).map_err(|e| CodecError::new("in_memory", e.to_string()))
    }

    fn decode(
        &self,
        raw: &Self::Encoded,
        _schema: &[FieldMapping<Self::Slot>],
    ) -> Result<HashMap<String, serde_json::Value>, CodecError> {
        serde_json::from_str(raw).map_err(|e| CodecError::new("in_memory", e.to_string()))
    }
}
