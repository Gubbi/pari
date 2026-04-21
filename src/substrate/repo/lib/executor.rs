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
                            errors.push(PrimitiveError::path_permission_denied(
                                "path permission denied",
                                path,
                                "get",
                            ));
                        } else {
                            errors.push(PrimitiveError::file_read("file read failed", path));
                        }
                    }
                },
                AssetOp::Put(encoded) => {
                    if let Some(parent) = req.location.parent() {
                        if let Err(e) = fs::create_dir_all(parent) {
                            let path = parent.display().to_string();
                            if e.kind() == std::io::ErrorKind::PermissionDenied {
                                errors.push(PrimitiveError::path_permission_denied(
                                    "path permission denied",
                                    path,
                                    "create_dir_all",
                                ));
                            } else {
                                errors.push(PrimitiveError::parent_directory_creation(
                                    "parent directory creation failed",
                                    path,
                                ));
                            }
                            continue;
                        }
                    }
                    if let Err(e) = fs::write(&req.location, encoded) {
                        let path = req.location.display().to_string();
                        if e.kind() == std::io::ErrorKind::PermissionDenied {
                            errors.push(PrimitiveError::path_permission_denied(
                                "path permission denied",
                                path,
                                "put",
                            ));
                        } else {
                            errors.push(PrimitiveError::file_write("file write failed", path));
                        }
                    } else {
                        responses.push(AssetResponse::Done);
                    }
                }
                AssetOp::Post(_) => {
                    errors.push(PrimitiveError::unsupported_executor_operation(
                        "unsupported executor operation",
                        "post",
                        req.location.display().to_string(),
                    ));
                }
                AssetOp::Patch(_) => {
                    errors.push(PrimitiveError::unsupported_executor_operation(
                        "unsupported executor operation",
                        "patch",
                        req.location.display().to_string(),
                    ));
                }
                AssetOp::Delete => {
                    if req.location.exists() {
                        if let Err(e) = fs::remove_file(&req.location) {
                            let path = req.location.display().to_string();
                            if e.kind() == std::io::ErrorKind::PermissionDenied {
                                errors.push(PrimitiveError::path_permission_denied(
                                    "path permission denied",
                                    path,
                                    "delete",
                                ));
                            } else {
                                errors
                                    .push(PrimitiveError::file_delete("file delete failed", path));
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
