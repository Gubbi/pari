## Why

Pari needs precise, validated Rust types for its definition-layer entities — Team, Role, Workflow, Task, Relay, and Hook. The schema decisions from explore are settled; now we need an authoritative formal spec and a Rust implementation with validation that the rest of the system can build on.

## What Changes

- Define JSON Schema for all six definition-layer entities (`Team`, `Role`, `Workflow`, `Task`, `Relay`, `Hook`) and their embedded types (`RACI`, `WorkStep`, `ReviewStep`, `Artifact`, `StateMap`, `HookInvocation`)
- Implement corresponding Rust types (`struct`/`enum`) from those schemas
- Implement validation logic: structural correctness plus cross-entity referential integrity (role_ids, hook_ids, workflow IDs, state name matching, RACI constraints, ReviewStep `on_reject` referencing earlier steps, etc.)

## Capabilities

### New Capabilities

- `entity-schemas`: JSON Schema definitions for all definition-layer entities and embedded types — serves as the authoritative contract
- `schema-validation`: Rust module (`src/schema/`) with types and validation logic derived from the schemas

### Modified Capabilities

<!-- none -->

## Impact

- New: `schemas/` directory with JSON Schema files
- New: `src/schema/` Rust module (types + validation)
- No YAML/JSON parsing of entity definitions in this change — that is a subsequent proposal
- No runtime layer impact — definition layer only
