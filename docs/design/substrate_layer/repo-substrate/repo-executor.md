# repo-executor-lca-atomic-swap

**Owning layer: `substrate`**

---

## Purpose

`RepoExecutor` achieves atomic multi-file writes via an LCA-based `.part/` staging directory and `fs::rename` swap. Hard-links preserve unchanged sibling files. The swap is atomic at the OS level — either the full set of changes is visible or none is.

---

## Algorithm

```
execute(ops: Vec<AssetRequest<PathBuf, String>>):

  1. Compute LCA
     Find the Lowest Common Ancestor directory of all write/delete op paths.
     This is the smallest subtree that contains all changed files.

  2. Stage in <lca>.part/
     Create <lca>.part/ directory.
     For each file under <lca>:
       If the file has a Put/Delete op:
         Write/skip it into <lca>.part/ as the new content.
       Else (unchanged):
         Hard-link the existing file into <lca>.part/.
         Hard-link is O(1) and shares inodes — no file copying.

  3. Atomic swap
     fs::rename(<lca>.part/, <lca>.old/)
       Wait — actually: rename <lca> → <lca>.old/, then rename <lca>.part/ → <lca>
       Both renames are atomic at OS level within the same filesystem.

  4. Cleanup
     Remove <lca>.old/ after successful swap.
```

Read ops (`Get`, `Head`) are executed directly — no staging needed.

---

## LCA Computation

Given paths:
```
workflows/InitiativeWorkflow/WriteProposal/README.md     (modified)
workflows/InitiativeWorkflow/HandoffToClient/README.md   (added)
```

LCA = `workflows/InitiativeWorkflow/`

The `.part/` directory spans the entire workflow directory, including unchanged task directories that are hard-linked in.

---

## Hard-Links

Unchanged siblings under the LCA are hard-linked (not copied) into `.part/`. Hard-links:
- Are O(1) — no data is copied
- Share inodes — on swap, the directory entries change but file data is shared
- Preserve atomicity — if the swap fails, the original directory still has all files

---

## Stale Cleanup on Startup

If a previous process crashed during a swap, stale `.part/` or `.old/` directories may exist. On `RepoSubstrate::new()`, the substrate scans for and removes any stale `.part/` and `.old/` directories before accepting operations.

---

## Error Collection

If any individual file write fails during staging, the executor collects the error and continues staging (best-effort). If any errors were collected, the swap does not proceed and all errors are returned. The `.part/` directory is cleaned up on error.
