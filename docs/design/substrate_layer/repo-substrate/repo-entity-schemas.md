# repo-entity-schemas

**Substrate Layer → `substrate_layer/repo-substrate/`**

---

## Purpose

Static `EntitySchema<RepoSlot>` definitions for all entity types under `RepoSubstrate`. These are the `SubstrateSchema<RepoSlot>` associated constants — one per entity type.

---

## Role

```
path:   roles/{id}.md         kind: MARKDOWN_FILE
fields:
  name        → H1
  description → DescriptionParagraph
  purpose     → FrontmatterKey("purpose")
  traits      → FrontmatterKey("traits")
  extensions  → FrontmatterFlattened
```

No additional assets.

---

## Hook

```
path:   hooks/{id}.md         kind: MARKDOWN_FILE
fields:
  name         → H1
  description  → DescriptionParagraph
  instructions → Section("Instructions", BulletList)
  inputs       → FrontmatterKey("inputs")
  extensions   → FrontmatterFlattened
```

No additional assets.

---

## Team

```
path:   teams/{id}.md         kind: MARKDOWN_FILE
fields:
  name        → H1
  description → DescriptionParagraph
  members     → FrontmatterKey("members")
  include     → FrontmatterKey("include")
  import      → FrontmatterKey("import")
  extensions  → FrontmatterFlattened
```

No additional assets.

---

## ArtifactKind

```
path:   artifact-kinds/{id}.md    kind: MARKDOWN_FILE
fields:
  name        → H1
  description → DescriptionParagraph
  service     → FrontmatterKey("service")
  access      → FrontmatterKey("access")
  guidance    → Section("Guidance", Paragraph)
  extensions  → FrontmatterFlattened
```

No additional assets.

---

## Workflow

```
path:   workflows/{id}/README.md    kind: MARKDOWN_FILE
fields:
  name          → H1
  description   → DescriptionParagraph
  purpose       → Section("Purpose", Paragraph)
  raci          → FrontmatterKey("raci")
  steps         → FrontmatterKey("steps")
  states        → FrontmatterKey("states")
  intercepts    → FrontmatterKey("intercepts")
  guidance      → Section("Guidance", Paragraph)
  extensions    → FrontmatterFlattened
```

No additional assets. Tasks and Relays embedded in steps are separate entities with their own files under `workflows/{id}/`.

---

## ReusableWorkflow

```
path:   reusable-workflows/{id}/README.md    kind: MARKDOWN_FILE
fields: (same structure as Workflow)
```

No additional assets.

---

## Task

```
ref_asset:
  path:   {parent.base}/{id}/README.md    kind: MARKDOWN_FILE
  fields:
    name          → H1
    description   → DescriptionParagraph
    purpose       → Section("Purpose", Paragraph)
    instructions  → Section("Instructions", BulletList)
    criteria      → Section("Criteria", BulletList)
    artifact      → FrontmatterKey("artifact")
    raci          → FrontmatterKey("raci")
    states        → FrontmatterKey("states")
    intercepts    → FrontmatterKey("intercepts")
    guidance      → Section("Guidance", Paragraph)
    extensions    → FrontmatterFlattened

assets:
  path:   {parent.base}/{id}/template.md    kind: RAW_FILE
  fields:
    template_content → FileContent
```

---

## Relay

```
path:   {parent.base}/{id}/README.md    kind: MARKDOWN_FILE
fields:
  name          → H1
  description   → DescriptionParagraph
  purpose       → Section("Purpose", Paragraph)
  raci          → FrontmatterKey("raci")
  delegates_to  → FrontmatterKey("delegates_to")
  briefing      → Section("Briefing", Paragraph)
  debriefing    → Section("Debriefing", Paragraph)
  state_map     → FrontmatterKey("state_map")
  intercepts    → FrontmatterKey("intercepts")
  guidance      → Section("Guidance", Paragraph)
  extensions    → FrontmatterFlattened
```

No additional assets.

---

## Notes

- All paths listed are relative to the RepoSubstrate root
- Task and Relay paths use `{parent.base}` which resolves to the parent Workflow's directory
- The `steps` field on Workflow is serialized as YAML in the README frontmatter (step metadata: type, depends_on, entity_ref). The step entity bodies live in their own files.
- Field names match the plain entity struct field names
