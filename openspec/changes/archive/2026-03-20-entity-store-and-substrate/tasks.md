## 1. EntityStore in src/schema/store.rs

- [x] 1.1 Write tests for EntityStore construction and all lookup methods (has_role, has_hook, has_team, has_shared_workflow, get_hook, get_team, get_shared_workflow_states)
- [x] 1.2 Implement EntityStore in src/schema/store.rs with HashMap-keyed fields and all lookup methods

## 2. Team::get_refs

- [x] 2.1 Write tests for Team::get_refs (include keys, import entries, both, neither)
- [x] 2.2 Implement Team::get_refs on the Team struct

## 3. Substrate rename

- [x] 3.1 Rename src/storage/ → src/substrate/; cascade all type names (Storage → Substrate, StorageError → SubstrateError, RepoStorage → RepoSubstrate) across all files in the module
- [x] 3.2 Update src/substrate/mod.rs to import EntityStore from src/schema::store and remove its own EntityStore definition
- [x] 3.3 Update src/lib.rs module declarations (storage → substrate)

## 4. Switch validators to EntityStore

- [x] 4.1 Update role.rs: test helper and validate signature to use &EntityStore
- [x] 4.2 Update hook.rs: test helper and validate signature to use &EntityStore
- [x] 4.3 Update team.rs: test helper and validate signature to use &EntityStore; BFS uses team.get_refs()
- [x] 4.4 Update workflow.rs: test helper and validate signature to use &EntityStore
- [x] 4.5 Update task.rs: test helper and validate signature to use &EntityStore
- [x] 4.6 Update relay.rs: test helper and validate signature to use &EntityStore

## 5. Clean up

- [x] 5.1 Delete src/schema/context.rs
- [x] 5.2 Run cargo test — all tests must pass
