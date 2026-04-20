use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    error::primitive::PrimitiveError,
    substrate::{
        repo::{
            codec::RepoCodec, executor::RepoExecutor, resolver::RepoLocationResolver,
            schema::RepoSlot,
        },
        Substrate, SubstrateError,
    },
};

pub struct RepoSubstrate {
    resolver: RepoLocationResolver,
    codec: RepoCodec,
    executor: RepoExecutor,
}

impl RepoSubstrate {
    pub fn new(root: PathBuf) -> Result<Self, SubstrateError> {
        cleanup_stale(&root)?;
        fs::create_dir_all(&root).map_err(|e| {
            let root_path = root.display().to_string();
            let primitive = PrimitiveError::RootDirectoryCreation {
                context: PrimitiveError::context("root directory creation failed"),
                root: root_path.clone(),
            };
            let _message = if e.kind() == std::io::ErrorKind::PermissionDenied {
                format!("filesystem permission denied creating substrate root '{root_path}'")
            } else {
                format!("failed to create substrate root '{root_path}'")
            };
            SubstrateError::corrupt_persistence_state(primitive)
        })?;

        Ok(Self {
            resolver: RepoLocationResolver::new(root.clone()),
            codec: RepoCodec,
            executor: RepoExecutor::new(root),
        })
    }
}

impl Substrate for RepoSubstrate {
    type Slot = RepoSlot;
    type Location = PathBuf;
    type Encoded = String;
    type Resolver = RepoLocationResolver;
    type Codec = RepoCodec;
    type Executor = RepoExecutor;

    fn resolver(&self) -> &Self::Resolver {
        &self.resolver
    }

    fn codec(&self) -> &Self::Codec {
        &self.codec
    }

    fn executor(&self) -> &Self::Executor {
        &self.executor
    }
}

fn cleanup_stale(root: &Path) -> Result<(), SubstrateError> {
    if !root.exists() {
        return Ok(());
    }

    fn walk(dir: &Path) -> Result<(), SubstrateError> {
        for entry in fs::read_dir(dir).map_err(|e| {
            let path = dir.display().to_string();
            let primitive = PrimitiveError::DirectoryRead {
                context: PrimitiveError::context("directory read failed"),
                path: path.clone(),
            };
            let _message = if e.kind() == std::io::ErrorKind::PermissionDenied {
                format!("filesystem permission denied reading directory '{path}'")
            } else {
                format!("failed to read directory '{path}'")
            };
            SubstrateError::corrupt_persistence_state(primitive)
        })? {
            let entry = entry.map_err(|_e| {
                let path = dir.display().to_string();
                let primitive = PrimitiveError::DirectoryEntryRead {
                    context: PrimitiveError::context("directory entry read failed"),
                    path: path.clone(),
                };
                SubstrateError::corrupt_persistence_state(primitive)
            })?;
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.ends_with(".part") || name.ends_with(".old") {
                    fs::remove_dir_all(&path).map_err(|e| {
                        let stale_path = path.display().to_string();
                        let primitive = PrimitiveError::StaleCleanupDeletion {
                            context: PrimitiveError::context("stale cleanup deletion failed"),
                            path: stale_path.clone(),
                        };
                        let _message = if e.kind() == std::io::ErrorKind::PermissionDenied {
                            format!(
                                "filesystem permission denied removing stale substrate directory '{stale_path}'"
                            )
                        } else {
                            format!("failed to remove stale substrate directory '{stale_path}'")
                        };
                        SubstrateError::corrupt_persistence_state(primitive)
                    })?;
                } else {
                    walk(&path)?;
                }
            }
        }
        Ok(())
    }

    walk(root)
}
