# artifact-kind-plain

**Data Model → `data_model/plain-entities/`**

---

## Purpose

`ArtifactKind` is a top-level reference entity that names and describes a type of deliverable. It is referenced by `Artifact.kind` on Task definitions to identify what kind of output a task produces.

---

## Definition

```rust
pub struct ArtifactKind {
    pub entity_ref: EntityRef<ArtifactKind>,
    pub name: String,
    pub description: Option<String>,
    pub service: String,
    pub access: Option<String>,
    pub guidance: Option<String>,
    pub extensions: Extensions,
}
```

---

## Fields

- `entity_ref` — carries the artifact kind's id (kebab-case) and kind; top-level entity, defaults to `NoParent`
- `name` — human-readable display name
- `description` — optional short summary of what this artifact kind represents
- `service` — base URI of the service that hosts or manages artifacts of this kind
- `access` — optional instructions for how to access or interact with the service
- `guidance` — optional freeform guidance on how to produce artifacts of this kind
- `extensions` — open-ended metadata; only `x-` prefixed keys are permitted (see [13 · extensions](../value-types/extensions.md))

---
