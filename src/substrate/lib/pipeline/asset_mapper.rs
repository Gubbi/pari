use super::{EntitySchema, SchemaAsset, Slot};
use crate::error::primitive::PrimitiveError;

/// Selects the subset of assets a read or write operation needs to
/// touch. Used by the default pipeline to turn a field list (or dirty
/// set) into a batch of executor requests.
pub struct AssetMapper;

impl AssetMapper {
    pub fn select_for_write<'a, S: Slot>(
        schema: &'a EntitySchema<S>,
        dirty_fields: Option<&[&str]>,
    ) -> Result<Vec<SchemaAsset<'a, S>>, PrimitiveError> {
        schema.assets_for_write(dirty_fields)
    }

    pub fn select_for_read<'a, S: Slot>(
        schema: &'a EntitySchema<S>,
        fields: &[&str],
    ) -> Result<Vec<SchemaAsset<'a, S>>, PrimitiveError> {
        schema.assets_for_read(fields)
    }
}
