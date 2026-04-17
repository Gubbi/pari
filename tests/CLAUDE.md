# tests — Test Layer Coverage

## Ownership

This directory belongs to the formal `test` layer.

Tests may exercise any production layer, but production code must not depend on test helpers or test-only assumptions.

## Current Active Files

- [tests/store_operations.rs](/Users/vinuth/code/pari/tests/store_operations.rs): store/workspace integration against `InMemorySubstrate`
- [tests/entity_definitions.rs](/Users/vinuth/code/pari/tests/entity_definitions.rs): entity identity and parent-shape coverage
- [tests/error_compose.rs](/Users/vinuth/code/pari/tests/error_compose.rs): error-layer derive behavior
- [tests/error_hierarchy.rs](/Users/vinuth/code/pari/tests/error_hierarchy.rs): `PariError` and downcast behavior
- [tests/tracked_serde.rs](/Users/vinuth/code/pari/tests/tracked_serde.rs): tracked/entity serde behavior
- [tests/validate_entities.rs](/Users/vinuth/code/pari/tests/validate_entities.rs): validation coverage

## Intentionally Disabled Files

These files are currently compiled out with `#![cfg(any())]` and should stay documented as intentionally disabled rather than treated as active coverage:

- [tests/core_jobs.rs](/Users/vinuth/code/pari/tests/core_jobs.rs)
- [tests/derive_entity.rs](/Users/vinuth/code/pari/tests/derive_entity.rs)
- [tests/substrate_pipeline.rs](/Users/vinuth/code/pari/tests/substrate_pipeline.rs)

If you touch one of those files, preserve or update its file-level `TODO:` note to explain what blocks re-enabling it.

## Test Style

- Prefer public API coverage over reaching into store internals.
- Use `EntityServer::with(...)` for isolated actor sessions.
- Use `InMemorySubstrate` when filesystem behavior is not part of the test's purpose.
- Keep tests aligned with the current layer model and current API names.

Do not add new tests that defend removed architecture or stale helper APIs.
