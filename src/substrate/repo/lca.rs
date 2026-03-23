//! LCA (lowest common ancestor) utility for filesystem path sets.
//!
//! Used by [`super::storage::RepoSubstrate`] to find the deepest directory
//! that covers all changed files, so that only that subtree needs to be
//! staged and atomically swapped.

/// Return the lowest common ancestor directory of a set of file paths.
///
/// Each element of `paths` is treated as a `/`-separated file path.  The
/// return value is the deepest directory that is a parent of every path.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(lca(&["roles/eng-lead.md"]), "roles");
/// assert_eq!(lca(&["roles/eng-lead.md", "roles/pm.md"]), "roles");
/// assert_eq!(lca(&["roles/eng-lead.md", "teams/platform.md"]), "");
/// ```
pub fn lca(paths: &[&str]) -> String {
    if paths.is_empty() {
        return String::new();
    }
    let split: Vec<Vec<&str>> = paths.iter().map(|p| p.split('/').collect()).collect();
    let min_len = split.iter().map(|v| v.len()).min().unwrap_or(0);
    let mut common = 0;
    // Walk directory components only (stop before the last component, which is
    // the file name or the entity directory itself).
    'outer: for i in 0..min_len.saturating_sub(1) {
        let segment = split[0][i];
        for components in &split[1..] {
            if components[i] != segment {
                break 'outer;
            }
        }
        common = i + 1;
    }
    split[0][..common].join("/")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- 8.1: LCA computation ---

    #[test]
    fn single_path_returns_parent_directory() {
        assert_eq!(lca(&["roles/eng-lead.md"]), "roles");
    }

    #[test]
    fn single_nested_path_returns_parent_directory() {
        assert_eq!(lca(&["workflows/Initiative/README.md"]), "workflows/Initiative");
    }

    #[test]
    fn sibling_paths_return_common_parent() {
        assert_eq!(lca(&["roles/eng-lead.md", "roles/pm.md"]), "roles");
    }

    #[test]
    fn nested_sibling_paths_return_common_parent() {
        assert_eq!(
            lca(&[
                "workflows/Initiative/WriteProposal/README.md",
                "workflows/Initiative/LegalReview/README.md",
            ]),
            "workflows/Initiative"
        );
    }

    #[test]
    fn ancestor_and_descendant_paths_return_ancestor() {
        assert_eq!(
            lca(&[
                "workflows/Initiative/README.md",
                "workflows/Initiative/WriteProposal/README.md",
            ]),
            "workflows/Initiative"
        );
    }

    #[test]
    fn cross_top_level_paths_return_root() {
        assert_eq!(lca(&["roles/eng-lead.md", "teams/platform.md"]), "");
    }

    #[test]
    fn cross_top_level_nested_paths_return_root() {
        assert_eq!(
            lca(&["roles/eng-lead.md", "workflows/Initiative/README.md"]),
            ""
        );
    }

    #[test]
    fn single_top_level_file_returns_empty_root() {
        // A file directly under root has no parent directory component.
        assert_eq!(lca(&["README.md"]), "");
    }

    #[test]
    fn empty_paths_returns_root() {
        assert_eq!(lca(&[]), "");
    }

    #[test]
    fn identical_paths_return_parent() {
        assert_eq!(
            lca(&["roles/eng-lead.md", "roles/eng-lead.md"]),
            "roles"
        );
    }
}
