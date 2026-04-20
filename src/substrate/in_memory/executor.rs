use crate::{
    error::primitive::PrimitiveError,
    substrate::{
        in_memory::storage::InMemoryStorage,
        pipeline::{AssetOp, AssetRequest, AssetResponse, Executor},
    },
};
pub struct InMemoryExecutor {
    pub(super) assets: InMemoryStorage,
}

impl InMemoryExecutor {
    pub fn new(assets: InMemoryStorage) -> Self {
        Self { assets }
    }
}

impl Executor for InMemoryExecutor {
    type Location = String;
    type Encoded = String;

    fn execute<I>(&self, ops: I) -> Result<Vec<AssetResponse<Self::Encoded>>, Vec<PrimitiveError>>
    where
        I: IntoIterator<Item = AssetRequest<Self::Location, Self::Encoded>>,
    {
        let assets = self.assets.assets();
        let mut assets = assets.lock().unwrap();
        let mut responses = Vec::new();
        let mut errors = Vec::new();

        for req in ops {
            match req.op {
                AssetOp::Head => {
                    responses.push(AssetResponse::Exists(assets.contains_key(&req.location)))
                }
                AssetOp::Get => match assets.get(&req.location) {
                    Some(data) => responses.push(AssetResponse::Data(data.clone())),
                    None => {
                        let asset_path = req.location.clone();
                        errors.push(PrimitiveError::MissingAsset {
                            context: PrimitiveError::context("requested asset missing"),
                            asset_path: asset_path.clone(),
                        });
                    }
                },
                AssetOp::Put(encoded) | AssetOp::Post(encoded) | AssetOp::Patch(encoded) => {
                    assets.insert(req.location, encoded);
                    responses.push(AssetResponse::Done);
                }
                AssetOp::Delete => {
                    assets.remove(&req.location);
                    responses.push(AssetResponse::Done);
                }
            }
        }

        if errors.is_empty() {
            Ok(responses)
        } else {
            Err(errors)
        }
    }
}
