# artifact

**Owning layer: `entity`**

---

## Purpose

`Artifact` specifies the expected deliverable for a `Task`. It is system-agnostic — the artifact could be a pull request, a Notion page, a Git branch, a local file, or any other output. What kind of artifact is expected is expressed via a reference to an `ArtifactKind` entity.

---

## Definition

```rust
pub struct Artifact {
    pub kind: EntityRef<ArtifactKind>,
    pub template: Option<String>,
}
```

- `kind` — references the `ArtifactKind` entity describing what type of artifact this is
- `template` — optional template content that guides the structure of the produced artifact; interpretation is system-specific (PR description template, file boilerplate, etc.)

---

## ArtifactKind Entity

`ArtifactKind` is a top-level entity (alongside Role, Hook, Team) that captures system-level details about an artifact type. Its design is covered in the plain-entities tier. `ArtifactKind` is included in the `EntityKind` enum.
