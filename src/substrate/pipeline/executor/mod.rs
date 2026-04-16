pub mod error;
pub use error::ExecutorError;

use crate::substrate::pipeline::{AssetRequest, AssetResponse};

pub trait Executor {
    type Location;
    type Encoded;

    fn execute<I>(&self, ops: I) -> Result<Vec<AssetResponse<Self::Encoded>>, Vec<ExecutorError>>
    where
        I: IntoIterator<Item = AssetRequest<Self::Location, Self::Encoded>>;
}
