pub mod error;
pub use error::CodecError;

use std::collections::HashMap;

use crate::substrate::pipeline::{FieldMapping, Slot};

pub trait Codec {
    type Slot: Slot;
    type Encoded;

    fn encode(
        &self,
        entity_json: &serde_json::Value,
        schema: &[FieldMapping<Self::Slot>],
    ) -> Result<Self::Encoded, CodecError>;

    fn decode(
        &self,
        raw: &Self::Encoded,
        schema: &[FieldMapping<Self::Slot>],
    ) -> Result<HashMap<String, serde_json::Value>, CodecError>;
}
