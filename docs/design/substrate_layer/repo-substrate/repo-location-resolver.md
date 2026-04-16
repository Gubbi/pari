# repo-location-resolver

**Owning layer: `substrate`**

---

## Purpose

`RepoLocationResolver` expands path templates from `EntitySchema` into concrete filesystem paths. It handles `{id}`, `{parent.base}`, and `{field_name}` variables.

---

## Implementation

```rust
struct RepoLocationResolver {
    root: PathBuf,
}

impl LocationResolver for RepoLocationResolver {
    type Location = PathBuf;

    fn resolve(&self, path_template: &str, entity: &serde_json::Value) -> PathBuf {
        let expanded = expand_template(path_template, entity);
        self.root.join(expanded)
    }

    fn base_of(&self, location: &PathBuf) -> String {
        location
            .parent()
            .unwrap_or(location.as_path())
            .to_string_lossy()
            .into_owned()
    }
}
```

---

## Template Expansion

| Variable | Resolution |
|---|---|
| `{id}` | `entity["entity_ref"]["id"]` from the serialized entity |
| `{parent.base}` | `base_of(parent entity's ref_asset location)` — parent must be in store |
| `{field_name}` | `entity["field_name"]` — nested paths use dot notation (e.g. `{category.name}`) |

---

## Concrete Examples

| Entity | Template | Resolved path (relative to root) |
|---|---|---|
| Role `eng-lead` | `roles/{id}.md` | `roles/eng-lead.md` |
| Hook `on-submit` | `hooks/{id}.md` | `hooks/on-submit.md` |
| Team `core` | `teams/{id}.md` | `teams/core.md` |
| ArtifactKind `document` | `artifact-kinds/{id}.md` | `artifact-kinds/document.md` |
| Workflow `InitiativeWorkflow` | `workflows/{id}/README.md` | `workflows/InitiativeWorkflow/README.md` |
| ReusableWorkflow `ReviewWorkflow` | `reusable-workflows/{id}/README.md` | `reusable-workflows/ReviewWorkflow/README.md` |
| Task `WriteProposal` (in `InitiativeWorkflow`) | `{parent.base}/{id}/README.md` | `workflows/InitiativeWorkflow/WriteProposal/README.md` |
| Relay `HandoffToClient` (in `InitiativeWorkflow`) | `{parent.base}/{id}/README.md` | `workflows/InitiativeWorkflow/HandoffToClient/README.md` |
| Task `WriteProposal` template | `{parent.base}/{id}/template.md` | `workflows/InitiativeWorkflow/WriteProposal/template.md` |

---

## `base_of`

For a workflow at `workflows/InitiativeWorkflow/README.md`, `base_of` returns `workflows/InitiativeWorkflow`. This is then used as `{parent.base}` when resolving paths for that workflow's child entities (Tasks, Relays).
