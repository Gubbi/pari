use std::collections::HashMap;

use super::{FieldMapping, Slot};
use crate::error::primitive::PrimitiveError;

pub trait Codec {
    type Slot: Slot;
    type Encoded;

    fn encode(
        &self,
        entity_json: &serde_json::Value,
        schema: &[FieldMapping<Self::Slot>],
    ) -> Result<Self::Encoded, PrimitiveError>;

    fn decode(
        &self,
        raw: &Self::Encoded,
        schema: &[FieldMapping<Self::Slot>],
    ) -> Result<HashMap<String, serde_json::Value>, PrimitiveError>;
}
