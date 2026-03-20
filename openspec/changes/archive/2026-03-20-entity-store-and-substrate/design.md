## Context

Two parallel types currently represent the same set of validated entities:

- **`RepoContext`** (`src/schema/context.rs`) — a thin projection: `HashSet` of ids for roles/hooks/teams, pre-computed derived caches (`team_direct_refs`, `hook_definitions`), and `Vec<SharedWorkflow>` / `Vec<Workflow>`. Used as the validation context passed to all entity validators.
- **`EntityStore`** (`src/storage/mod.rs`) — the full typed collections: `Vec<Role>`, `Vec<Hook>`, `Vec<Team>`, `Vec<Workflow>`, `Vec<SharedWorkflow>`. Used as the persistence input passed to `persist()`.

The duplication requires maintaining both in sync and introduces intermediate projection types (`HookDefinition`, `HookInputInfo`) that exist only to bridge from full `Hook` entities to what `RepoContext` needed to expose. It also creates a latent circular import risk: if validators were ever to import `EntityStore` from `src/storage/`, they'd cycle back into a module that already imports from `src/schema/`.

The `storage` module name creates surface confusion with `EntityStore` (the "store"). Renaming to `substrate` removes the collision.

## Goals / Non-Goals

**Goals:**
- Single type (`EntityStore`) serves as both validation context and persistence input
- O(1) id lookups during validation — no separate `HashSet` caches needed
- Eliminate intermediate projection types (`HookDefinition`, `HookInputInfo`, `team_direct_refs`)
- Resolve the circular import by relocating `EntityStore` to `src/schema/`
- Rename `storage` layer to `substrate` with all types cascading

**Non-Goals:**
- No changes to validation logic, entity schemas, or file format output
- No `load()` / parser work — deferred to the parser proposal
- No changes to atomic write / temp-dir strategy in `RepoSubstrate`

## Decisions

### EntityStore uses `HashMap<String, Entity>` instead of `Vec<Entity>`

`Vec` lookup is O(n); `RepoContext` maintained separate `HashSet<id>` caches precisely to avoid this. A `HashMap<String, Entity>` gives O(1) lookup via key and access to the full entity via value — both needs in one structure. The pre-computed caches (`team_direct_refs`, `hook_definitions`) and the id-only sets become unnecessary.

For the substrate layer, `.values()` iterates all entities for persistence — same ergonomics as `.iter()` on a `Vec`.

Alternative considered: keep `Vec` and add auxiliary index maps — more moving parts, same asymptotic cost, no advantage.

### EntityStore moves to `src/schema/store.rs`

`EntityStore` is a schema concept (the collection of validated entities), not infrastructure. Moving it to `src/schema/` means the substrate layer imports it from schema — consistent with how it already imports all entity types from schema. This eliminates the circular import risk cleanly.

Alternative considered: `src/lib.rs` at the crate root — works technically but is less expressive about where the concept belongs.

### Intermediate projection types are dropped; entity methods carry the derived logic

`HookDefinition` and `HookInputInfo` were bridges from the full `Hook` to what `RepoContext` could expose. With `EntityStore` holding `HashMap<String, Hook>`, validators call `store.get_hook(id)` and access `hook.inputs` directly.

`team_direct_refs` is dropped similarly. The BFS cycle check calls `team.get_refs()` — a method on `Team` itself that returns an iterator over all referenced team ids from both `include` and `import`. Derived logic belongs to the entity type, not the store.

### "substrate" as the persistence layer name

"storage" clashes with "store" (EntityStore). "persistence" is semantically accurate but awkward in compound type names (`RepoPersistence`). "substrate" conveys the underlying layer without naming collision and composes cleanly: `RepoSubstrate`, `SubstrateError`, `Substrate` trait.

## Risks / Trade-offs

**Validation invariant needs re-documentation** — `RepoContext` was documented with "the incoming entity being validated is never present in RepoContext." That invariant moves to `EntityStore`. It must be documented there explicitly.
→ Mitigation: doc comment on `EntityStore` carries this invariant forward.

## Migration Plan

1. Add `EntityStore` to `src/schema/store.rs` with `HashMap`-keyed fields and lookup methods
2. Add `get_refs()` method to `Team`
3. Update all entity validators to take `&EntityStore` instead of `&RepoContext`
4. Update `src/substrate/mod.rs` to import `EntityStore` from `src/schema::store`
5. Rename `src/storage/` → `src/substrate/`, cascade all type names
6. Delete `src/schema/context.rs`
7. Update `src/lib.rs` module declarations
8. Update all tests that construct `RepoContext` manually

No data migration — `persist()` behavior is unchanged, only type names differ.
