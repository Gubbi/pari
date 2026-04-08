# repo-slot-enum

**Substrate Layer → `substrate_layer/repo-substrate/`**

---

## Purpose

`RepoSlot` is the substrate-defined enum of encoding targets within a markdown+YAML frontmatter file. Each variant names a distinct location within the file format. The Codec uses these to know where to place or read a field value.

---

## Definition

```rust
enum RepoSlot {
    H1,
    FrontmatterKey(&'static str),
    FrontmatterFlattened,
    DescriptionParagraph,
    Section(&'static str, SectionContent),
    FileContent,
}

enum SectionContent {
    Paragraph,
    BulletList,
}
```

---

## Variants

| Variant | File location | Field type |
|---|---|---|
| `H1` | First `#` heading | String (entity name) |
| `FrontmatterKey(k)` | YAML frontmatter key `k` | Any serializable value |
| `FrontmatterFlattened` | All remaining YAML frontmatter keys | Extensions (HashMap) |
| `DescriptionParagraph` | First paragraph after H1 (before frontmatter or sections) | Optional String |
| `Section(heading, content)` | Markdown section with the given heading | String or Vec<String> |
| `FileContent` | Entire file contents (raw file, no structure) | String |

`FrontmatterFlattened` captures all YAML keys not mapped to a named `FrontmatterKey`. Used for the `extensions` field which holds arbitrary `x-*` keys.

`FileContent` is used for raw template files that are not markdown (e.g., a task's template content asset).

---

## One Codec for All Entity Types

One `RepoCodec` implementation handles all entity types by dispatching on `RepoSlot` variants. Per-entity variation lives entirely in the static `EntitySchema<RepoSlot>` definitions — the codec itself has no entity-type knowledge.
