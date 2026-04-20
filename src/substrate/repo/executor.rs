use std::{fs, path::PathBuf};

use crate::{
    error::primitive::PrimitiveError,
    substrate::pipeline::{AssetOp, AssetRequest, AssetResponse, Executor},
};

pub struct RepoExecutor;

impl RepoExecutor {
    pub fn new(_root: PathBuf) -> Self {
        Self
    }
}

impl Executor for RepoExecutor {
    type Location = PathBuf;
    type Encoded = String;

    fn execute<I>(&self, ops: I) -> Result<Vec<AssetResponse<Self::Encoded>>, Vec<PrimitiveError>>
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
                        let path = req.location.display().to_string();
                        if e.kind() == std::io::ErrorKind::PermissionDenied {
                            errors.push(PrimitiveError::PathPermissionDenied {
                                context: PrimitiveError::context("path permission denied"),
                                asset_path: path.clone(),
                                operation: "get".to_string(),
                            });
                        } else {
                            errors.push(PrimitiveError::FileRead {
                                context: PrimitiveError::context("file read failed"),
                                asset_path: path.clone(),
                            });
                        }
                    }
                },
                AssetOp::Put(encoded) => {
                    if let Some(parent) = req.location.parent() {
                        if let Err(e) = fs::create_dir_all(parent) {
                            let path = parent.display().to_string();
                            if e.kind() == std::io::ErrorKind::PermissionDenied {
                                errors.push(PrimitiveError::PathPermissionDenied {
                                    context: PrimitiveError::context("path permission denied"),
                                    asset_path: path.clone(),
                                    operation: "create_dir_all".to_string(),
                                });
                            } else {
                                errors.push(PrimitiveError::ParentDirectoryCreation {
                                    context: PrimitiveError::context(
                                        "parent directory creation failed",
                                    ),
                                    directory_path: path.clone(),
                                });
                            }
                            continue;
                        }
                    }
                    if let Err(e) = fs::write(&req.location, encoded) {
                        let path = req.location.display().to_string();
                        if e.kind() == std::io::ErrorKind::PermissionDenied {
                            errors.push(PrimitiveError::PathPermissionDenied {
                                context: PrimitiveError::context("path permission denied"),
                                asset_path: path.clone(),
                                operation: "put".to_string(),
                            });
                        } else {
                            errors.push(PrimitiveError::FileWrite {
                                context: PrimitiveError::context("file write failed"),
                                asset_path: path.clone(),
                            });
                        }
                    } else {
                        responses.push(AssetResponse::Done);
                    }
                }
                AssetOp::Post(_) => {
                    let asset_path = req.location.display().to_string();
                    errors.push(PrimitiveError::UnsupportedExecutorOperation {
                        context: PrimitiveError::context("unsupported executor operation"),
                        operation: "post".to_string(),
                        asset_path: asset_path.clone(),
                    });
                }
                AssetOp::Patch(_) => {
                    let asset_path = req.location.display().to_string();
                    errors.push(PrimitiveError::UnsupportedExecutorOperation {
                        context: PrimitiveError::context("unsupported executor operation"),
                        operation: "patch".to_string(),
                        asset_path: asset_path.clone(),
                    });
                }
                AssetOp::Delete => {
                    if req.location.exists() {
                        if let Err(e) = fs::remove_file(&req.location) {
                            let path = req.location.display().to_string();
                            if e.kind() == std::io::ErrorKind::PermissionDenied {
                                errors.push(PrimitiveError::PathPermissionDenied {
                                    context: PrimitiveError::context("path permission denied"),
                                    asset_path: path.clone(),
                                    operation: "delete".to_string(),
                                });
                            } else {
                                errors.push(PrimitiveError::FileDelete {
                                    context: PrimitiveError::context("file delete failed"),
                                    asset_path: path.clone(),
                                });
                            }
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
