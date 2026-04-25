use super::{AssetRequest, AssetResponse};
use crate::error::primitive::PrimitiveError;

/// Runs a batch of [`AssetRequest`]s and returns responses aligned
/// with the inputs, or a vector of per-request primitive errors on
/// batch failure. The defaults layer folds multiple errors into a
/// single `ActivityError`.
pub trait Executor {
    type Location;
    type Encoded;

    fn execute<I>(&self, ops: I) -> Result<Vec<AssetResponse<Self::Encoded>>, Vec<PrimitiveError>>
    where
        I: IntoIterator<Item = AssetRequest<Self::Location, Self::Encoded>>;
}
