# step-shorthand

**Substrate Layer → `substrate_layer/repo-substrate/`**

---

## Purpose

In YAML, steps can be written in a shorthand form where the step key implicitly identifies the entity. When the step key in the `steps` map matches the entity's id, the `entity_ref.id` can be omitted. The codec resolves the shorthand to the explicit form during decode, before the data reaches the store.

---

## Explicit Form

```yaml
steps:
  WriteProposal:
    type: task
    entity_ref:
      id: WriteProposal
      type: task
    depends_on: [Review]
  Review:
    type: review
    approver: [eng-lead]
    on_reject: WriteProposal
```

---

## Shorthand Form

```yaml
steps:
  WriteProposal:
    type: task
    depends_on: [Review]
  Review:
    type: review
    approver: [eng-lead]
    on_reject: WriteProposal
```

When `entity_ref` is absent, the codec infers `entity_ref.id = step_key`. The `type` field determines the entity kind (`EntityKind::Task`, `EntityKind::Relay`, etc.) and is always required.

---

## Resolution Rule

Resolution happens at the codec layer (decode), not in the store or validation:

1. Parse the `steps` YAML map
2. For each step entry:
   - If `entity_ref` is absent → construct `EntityRef { id: step_key, kind: inferred_from_type }`
   - If `entity_ref` is present → use as-is; `entity_ref.id` need not match the step key
3. Pass the fully resolved `Step` values to the store

---

## Encoding (write path)

On encode, the codec always writes the **shorthand** form when `entity_ref.id == step_key` — keeping the YAML human-friendly. The explicit form is written only when the ids differ.

---

## Why Codec Layer

The shorthand is a presentation convenience, not a semantic distinction. Resolving it in the codec keeps the store and entity types clean — the `Step` struct always has a fully-populated `entity_ref`. No special handling is needed anywhere else.
