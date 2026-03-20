## Why

`RepoContext` and `EntityStore` are parallel representations of the same validated entity set — `RepoContext` as an impoverished projection (id sets, derived caches) and `EntityStore` as the full typed collection. This duplication requires maintaining both in sync and introduces a class of intermediate types (`HookDefinition`, `HookInputInfo`, `team_direct_refs`) that exist only to bridge the gap. The `storage` module name also collides confusingly with `store` (as in `EntityStore`). This change eliminates the duplication, restructures `EntityStore` for efficient validation lookups, and renames the persistence layer to `substrate` for clarity.

## What Changes

- **BREAKING** `RepoContext` is replaced by `EntityStore` as the validation context — all `validate(entity, ctx: &RepoContext)` signatures become `validate(entity, ctx: &EntityStore)`
- `EntityStore` moves from `src/storage/` to `src/schema/store.rs` and is restructured from `Vec<Entity>` to `HashMap<id, Entity>` for O(1) lookups
- `context.rs` and all types within it (`RepoContext`, `HookDefinition`, `HookInputInfo`) are removed
- `src/storage/` module is renamed to `src/substrate/`; all types cascade: `Storage` → `Substrate`, `StorageError` → `SubstrateError`, `RepoStorage` → `RepoSubstrate`
- `team_direct_refs` derived cache dropped — BFS cycle detection reads from `HashMap<String, Team>` directly
- `HookDefinition`/`HookInputInfo` projection types dropped — hook input validation uses `&Hook` directly

## Capabilities

### New Capabilities

- `entity-store`: The unified, HashMap-keyed collection of validated entities serving as both validation context and persistence input

### Modified Capabilities

- `storage-layer`: Renamed to substrate layer; type names cascade; `EntityStore` imported from `src/schema/` rather than defined here

## Impact

- All entity validator files (`role.rs`, `hook.rs`, `team.rs`, `workflow.rs`, `task.rs`, `relay.rs`) — context type changes
- `src/schema/context.rs` — deleted
- `src/schema/store.rs` — new file
- `src/storage/` → `src/substrate/` — directory rename, all internal type references update
- All tests that construct `RepoContext` manually — update to `EntityStore`
- `src/lib.rs` — module declarations update
