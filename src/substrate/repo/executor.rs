use std::{fs, path::PathBuf};

use crate::substrate::pipeline::{AssetOp, AssetRequest, AssetResponse, Executor, ExecutorError};

pub struct RepoExecutor;

impl RepoExecutor {
    pub fn new(_root: PathBuf) -> Self {
        Self
    }
}

impl Executor for RepoExecutor {
    type Location = PathBuf;
    type Encoded = String;

    fn execute<I>(&self, ops: I) -> Result<Vec<AssetResponse<Self::Encoded>>, Vec<ExecutorError>>
    where
        I: IntoIterator<Item = AssetRequest<Self::Location, Self::Encoded>>,
    {
        let mut responses = Vec::new();
        let mut errors = Vec::new();

        for req in ops {
            match req.op {
                AssetOp::Head => responses.push(AssetResponse::Exists(req.location.exists())),
                AssetOp::Get => match fs::read_to_string(&req.location) {
                    Ok(raw) => responses.push(AssetResponse::Data(raw)),
                    Err(e) => {
                        errors.push(ExecutorError::new(
                            req.location.display().to_string(),
                            e.to_string(),
                        ));
                    }
                },
                AssetOp::Put(encoded) => {
                    if let Some(parent) = req.location.parent() {
                        if let Err(e) = fs::create_dir_all(parent) {
                            errors.push(ExecutorError::new(
                                parent.display().to_string(),
                                e.to_string(),
                            ));
                            continue;
                        }
                    }
                    if let Err(e) = fs::write(&req.location, encoded) {
                        errors.push(ExecutorError::new(
                            req.location.display().to_string(),
                            e.to_string(),
                        ));
                    } else {
                        responses.push(AssetResponse::Done);
                    }
                }
                AssetOp::Post(_) => {
                    errors.push(ExecutorError::new(
                        req.location.display().to_string(),
                        "repo executor does not support POST asset writes",
                    ));
                }
                AssetOp::Patch(_) => {
                    errors.push(ExecutorError::new(
                        req.location.display().to_string(),
                        "repo executor does not support PATCH asset writes",
                    ));
                }
                AssetOp::Delete => {
                    if req.location.exists() {
                        if let Err(e) = fs::remove_file(&req.location) {
                            errors.push(ExecutorError::new(
                                req.location.display().to_string(),
                                e.to_string(),
                            ));
                            continue;
                        }
                    }
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
