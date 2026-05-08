use super::{FieldMapping, Slot};
use crate::error::primitive::PrimitiveError;

/// Encodes entity JSON into the backend's on-the-wire shape and
/// decodes responses into a wire-shaped `serde_json::Value` carrying
/// the asset's slice. Schema drives the set of fields in each direction.
///
/// The decoded value is a JSON object whose keys mirror the wire form
/// the entity's `Tracked::Deserialize` expects — flatten-style fields
/// (e.g. `Extensions`) appear as top-level entries, not under a nested
/// envelope. Translating between on-disk layout and wire shape is the
/// codec's job; the substrate layer above just merges the slice into
/// the entity accumulator.
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
    ) -> Result<serde_json::Value, PrimitiveError>;
}
