# repo-asset-kinds

**Substrate Layer → `substrate_layer/repo-substrate/`**

---

## Purpose

RepoSubstrate defines two `AssetKind` constants covering all its asset types. Both declare the same capabilities: no create/upsert distinction, no partial update. Every write is a full asset rewrite.

---

## Constants

```rust
pub const MARKDOWN_FILE: AssetKind = AssetKind {
    distinguishes_create: false,
    supports_partial: false,
};

pub const RAW_FILE: AssetKind = AssetKind {
    distinguishes_create: false,
    supports_partial: false,
};
```

---

## Distinction

| Constant | Used for |
|---|---|
| `MARKDOWN_FILE` | Entity definition files: `roles/{id}.md`, `workflows/{id}/README.md`, etc. Structured markdown with YAML frontmatter. |
| `RAW_FILE` | Content files without structural slots: task template files, hook scripts, etc. Entire file content maps to `RepoSlot::FileContent`. |

Both have identical capabilities — the distinction is semantic (for schema readability and future differentiation if needed).

---

## Implications

- `supports_partial: false` → `mutable_without_load: false` for all fields on all entity types under RepoSubstrate. Every setter triggers a load of the containing asset before mutation.
- `distinguishes_create: false` → all writes use `Put` (create-or-replace). No `Post` ops.
- Every write is a full file rewrite — the write path always has all fields in the asset available (enforced by `ensure_mutable`).
