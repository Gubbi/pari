## Context

Explore produced settled schema decisions for six definition-layer entities: Team, Role, Workflow, Task, Relay, Hook. Those decisions are captured in `context/handoff.md`. This design covers how to represent them formally (JSON Schema) and implement them in Rust with full validation.

The Rust codebase does not yet exist. This is the first module.

## Goals / Non-Goals

**Goals:**
- JSON Schema files as the canonical, authoritative spec for each entity
- Rust types that faithfully mirror the JSON Schema structure
- Validation covering both structural constraints and cross-entity referential integrity
- Validation errors that are structured (not just strings) — path + message

**Non-Goals:**
- Parsing entity definitions from YAML/JSON files on disk (separate proposal)
- Runtime layer entities (Run, WorkItem, Gate, Participant)
- Serialization/deserialization to/from YAML (no serde_yaml in this change)
- Hook execution or Relay invocation logic

## Decisions

### JSON Schema as the canonical spec

**Decision:** Author JSON Schema files (`schemas/*.json`) before writing Rust code. Rust types are derived from the schema — not the other way around.

**Rationale:** JSON Schema is language-neutral, toolable, and reviewable by non-Rust contributors. It also serves as the contract for future parsers and the public-facing definition format.

**Alternative considered:** Define Rust structs first, derive the schema. Rejected — schema becomes a by-product of implementation rather than an intentional contract.

---

### Rust type strategy: plain structs + enums, no derive magic

**Decision:** Use plain `struct` and `enum` types. No `serde` derives in this change (serialization is not in scope). Types live in `src/schema/` with one file per entity.

**Rationale:** Keeps the module focused. Adding `serde` derives later is non-breaking.

**Alternative considered:** Use `serde` from the start. Rejected — adds noise and couples to a serialization format before we know the parsing approach.

---

### Validation as a separate pass

**Decision:** Implement validation as a function `validate(entity: &Entity, context: &RepoContext) -> Vec<ValidationError>` rather than failing fast at construction.

`RepoContext` carries all known role_ids, hook_ids, workflow_ids, and state maps needed for referential integrity checks. It contains only already-validated entities.

**Rationale:** Collecting all errors in one pass produces better error messages for users. Constructor-level panics or `Result` short-circuits lose subsequent errors.

**Alternative considered:** Validate at construction via `TryFrom`. Rejected — returns only the first error.

---

### ValidationError structure

```rust
pub struct ValidationError {
    pub path: String,   // dot-notation, e.g. "steps[2].on_reject"
    pub message: String,
}
```

Simple and sufficient for now. No error codes or severity levels yet.

---

### Cross-entity referential integrity scope

All cross-entity references validated in this change:
- RACI `responsible`/`accountable`/`consulted`/`informed` → role_ids exist in repo
- Workflow `hooks`, Task `hooks`, Relay `hooks` → hook_ids exist in repo
- Relay `delegates_to` → workflow_id exists in `shared/`
- `state_map` keys → match state names in referenced shared workflow
- ReviewStep `on_reject` → references an earlier step name in the same steps list
- Team `includes` and `import` → referenced team_ids exist; new team must not appear in any reachable chain (since `RepoContext` holds only validated, cycle-free data and the new team is not yet in it, this reduces to a reachability check — no existing team can already reference the new team, so only a direct/transitive self-reference from the new team is possible)

---

### Embedded types

`RACI`, `WorkStep`, `ReviewStep`, `Artifact`, `StateMap`, `HookInvocation` are Rust types in `src/schema/types.rs`. They are not independently validated — only validated in the context of their parent entity.

## Risks / Trade-offs

- **Schema drift** — JSON Schema and Rust types diverge as the code evolves. Mitigation: tasks include a verification step to confirm alignment before marking complete.
- **`RepoContext` is a stub** — its shape is defined here; it will be populated by the parser (future proposal). For now, tests construct it manually.
