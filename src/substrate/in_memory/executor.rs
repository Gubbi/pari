use crate::substrate::pipeline::{AssetOp, AssetRequest, AssetResponse, Executor, ExecutorError};
use crate::substrate::in_memory::storage::InMemoryStorage;
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

    fn execute<I>(
        &self,
        ops: I,
    ) -> Result<Vec<AssetResponse<Self::Encoded>>, Vec<ExecutorError>>
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
                    None => errors.push(ExecutorError::new(req.location, "not found")),
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
