use std::fs;
use std::path::{Path, PathBuf};

use crate::substrate::error::SubstrateError;
use crate::substrate::repo::codec::RepoCodec;
use crate::substrate::repo::executor::RepoExecutor;
use crate::substrate::repo::resolver::RepoLocationResolver;
use crate::substrate::repo::schema::RepoSlot;
use crate::substrate::pipeline::ExecutorError;
use crate::substrate::Substrate;

pub struct RepoSubstrate {
    resolver: RepoLocationResolver,
    codec: RepoCodec,
    executor: RepoExecutor,
}

impl RepoSubstrate {
    pub fn new(root: PathBuf) -> Result<Self, SubstrateError> {
        cleanup_stale(&root)?;
        fs::create_dir_all(&root).map_err(|e| {
            SubstrateError::Executor(ExecutorError::new(
                root.display().to_string(),
                e.to_string(),
            ))
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
            SubstrateError::Executor(ExecutorError::new(
                dir.display().to_string(),
                e.to_string(),
            ))
        })? {
            let entry = entry.map_err(|e| {
                SubstrateError::Executor(ExecutorError::new(
                    dir.display().to_string(),
                    e.to_string(),
                ))
            })?;
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.ends_with(".part") || name.ends_with(".old") {
                    fs::remove_dir_all(&path).map_err(|e| {
                        SubstrateError::Executor(ExecutorError::new(
                            path.display().to_string(),
                            e.to_string(),
                        ))
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
