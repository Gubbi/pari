//! `RepoExecutor` — execute batched asset requests with LCA-based atomic swap.

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use crate::substrate::pipeline::{
    AssetOp, AssetRequest, AssetResponse, Executor, ExecutorError,
};

#[derive(Clone)]
pub struct RepoExecutor {
    root: PathBuf,
}

impl RepoExecutor {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl Executor for RepoExecutor {
    type Location = PathBuf;
    type Encoded = String;

    fn execute(
        &self,
        ops: Vec<AssetRequest<PathBuf, String>>,
    ) -> Result<Vec<AssetResponse<String>>, Vec<ExecutorError>> {
        let mut responses: Vec<(usize, AssetResponse<String>)> = Vec::new();
        let mut write_ops: Vec<(usize, PathBuf, Option<String>)> = Vec::new(); // (idx, path, content or None=delete)
        let mut errors: Vec<ExecutorError> = Vec::new();

        for (idx, req) in ops.into_iter().enumerate() {
            match req.op {
                AssetOp::Get => {
                    match fs::read_to_string(&req.location) {
                        Ok(content) => responses.push((idx, AssetResponse::Data(content))),
                        Err(e) => errors.push(ExecutorError::new(
                            req.location.to_string_lossy(),
                            e.to_string(),
                        )),
                    }
                }
                AssetOp::Head => {
                    responses.push((idx, AssetResponse::Exists(req.location.exists())));
                }
                AssetOp::Put(content) | AssetOp::Post(content) | AssetOp::Patch(content) => {
                    write_ops.push((idx, req.location, Some(content)));
                }
                AssetOp::Delete => {
                    write_ops.push((idx, req.location, None));
                }
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        if !write_ops.is_empty() {
            let write_paths: Vec<&Path> = write_ops.iter().map(|(_, p, _)| p.as_path()).collect();
            let lca = compute_lca(&write_paths);
            let part_dir = part_path(&lca);
            let old_dir = old_path(&lca);

            // Clean up any stale dirs
            for stale in [&part_dir, &old_dir] {
                if stale.exists() {
                    if let Err(e) = fs::remove_dir_all(stale) {
                        errors.push(err_path(stale, &e));
                        return Err(errors);
                    }
                }
            }

            // Create staging dir
            if let Err(e) = fs::create_dir_all(&part_dir) {
                return Err(vec![err_path(&part_dir, &e)]);
            }

            // Hard-link unchanged files from lca into part_dir
            let paths_to_overwrite: HashSet<&Path> = write_ops
                .iter()
                .filter(|(_, _, c)| c.is_some())
                .map(|(_, p, _)| p.as_path())
                .collect();
            let paths_to_delete: HashSet<&Path> = write_ops
                .iter()
                .filter(|(_, _, c)| c.is_none())
                .map(|(_, p, _)| p.as_path())
                .collect();

            if lca.exists() {
                if let Err(e) = hard_link_dir(
                    &lca,
                    &part_dir,
                    &paths_to_overwrite,
                    &paths_to_delete,
                ) {
                    let _ = fs::remove_dir_all(&part_dir);
                    return Err(vec![err_path(&lca, &e)]);
                }
            }

            // Write new/updated files into part_dir
            for (idx, path, content_opt) in &write_ops {
                let Some(content) = content_opt else { continue };
                // Compute path relative to lca
                let rel = path.strip_prefix(&lca).unwrap_or(path);
                let dest = part_dir.join(rel);
                if let Some(parent) = dest.parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        let _ = fs::remove_dir_all(&part_dir);
                        return Err(vec![err_path(parent, &e)]);
                    }
                }
                if let Err(e) = fs::write(&dest, content) {
                    let _ = fs::remove_dir_all(&part_dir);
                    return Err(vec![err_path(&dest, &e)]);
                }
                responses.push((*idx, AssetResponse::Done));
            }

            // Mark deletes as Done (omission = deletion already handled above)
            for (idx, _, content_opt) in &write_ops {
                if content_opt.is_none() {
                    responses.push((*idx, AssetResponse::Done));
                }
            }

            // Atomic swap
            if lca.exists() {
                if let Err(e) = fs::rename(&lca, &old_dir) {
                    let _ = fs::remove_dir_all(&part_dir);
                    return Err(vec![err_path(&lca, &e)]);
                }
            }
            if let Err(e) = fs::rename(&part_dir, &lca) {
                // Try to restore
                let _ = fs::rename(&old_dir, &lca);
                return Err(vec![err_path(&part_dir, &e)]);
            }
            if old_dir.exists() {
                let _ = fs::remove_dir_all(&old_dir);
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        // Sort responses by original index
        responses.sort_by_key(|(idx, _)| *idx);
        Ok(responses.into_iter().map(|(_, r)| r).collect())
    }
}

// ---------------------------------------------------------------------------
// LCA computation
// ---------------------------------------------------------------------------

/// Find the lowest common ancestor directory of a set of file paths.
/// Each path is treated as a file; the LCA is computed over parent directories.
pub fn compute_lca(paths: &[&Path]) -> PathBuf {
    if paths.is_empty() {
        return PathBuf::new();
    }

    // Get parent directories
    let dirs: Vec<PathBuf> = paths
        .iter()
        .map(|p| p.parent().map(|d| d.to_path_buf()).unwrap_or_default())
        .collect();

    // Split into components
    let components: Vec<Vec<std::ffi::OsString>> = dirs
        .iter()
        .map(|d| {
            d.components()
                .map(|c| c.as_os_str().to_os_string())
                .collect()
        })
        .collect();

    if components.is_empty() {
        return PathBuf::new();
    }

    let first = &components[0];
    let mut common_len = first.len();

    for other in &components[1..] {
        let max = common_len.min(other.len());
        let mut new_len = 0;
        for i in 0..max {
            if first[i] == other[i] {
                new_len = i + 1;
            } else {
                break;
            }
        }
        common_len = new_len;
    }

    first[..common_len].iter().collect()
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

fn part_path(lca: &Path) -> PathBuf {
    let mut s = lca.as_os_str().to_os_string();
    s.push(".part");
    PathBuf::from(s)
}

fn old_path(lca: &Path) -> PathBuf {
    let mut s = lca.as_os_str().to_os_string();
    s.push(".old");
    PathBuf::from(s)
}

fn err_path(path: &Path, e: &impl std::fmt::Display) -> ExecutorError {
    ExecutorError::new(path.to_string_lossy(), e.to_string())
}

// ---------------------------------------------------------------------------
// Hard-link helper
// ---------------------------------------------------------------------------

/// Recursively hard-link all files from `src_dir` into `dst_dir`, skipping
/// files in `skip` (will be overwritten) and `delete` (will be omitted).
fn hard_link_dir(
    src_dir: &Path,
    dst_dir: &Path,
    skip: &HashSet<&Path>,
    delete: &HashSet<&Path>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(src_dir)? {
        let entry = entry?;
        let src_path = entry.path();
        let rel = src_path.strip_prefix(src_dir).unwrap_or(&src_path);
        let dst_path = dst_dir.join(rel);

        if src_path.is_dir() {
            fs::create_dir_all(&dst_path)?;
            hard_link_dir(&src_path, &dst_path, skip, delete)?;
        } else {
            if skip.contains(src_path.as_path()) || delete.contains(src_path.as_path()) {
                continue;
            }
            if let Some(parent) = dst_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::hard_link(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
