# src/substrate — Substrate Layer Persistence Contracts

## Ownership

This directory belongs to the formal `substrate` layer.

It owns:

- the persistence contract trait
- schema-backed default load/persist behavior
- asset pipeline traits and vocabulary
- concrete backends such as `RepoSubstrate`, `InMemorySubstrate`, and `VoidSubstrate`

The authoritative design docs for this area live under [docs/design/substrate_layer/](/Users/vinuth/code/pari/docs/design/substrate_layer/).

## Primary Entry Points

- [src/substrate/contract.rs](/Users/vinuth/code/pari/src/substrate/contract.rs): `Substrate` trait
- [src/substrate/defaults.rs](/Users/vinuth/code/pari/src/substrate/defaults.rs): default schema-driven `load_strategy`, `exists`, `load`, and `persist`
- [src/substrate/schema_registry.rs](/Users/vinuth/code/pari/src/substrate/schema_registry.rs): schema lookup across entity kinds
- [src/substrate/pipeline/](/Users/vinuth/code/pari/src/substrate/pipeline): pipeline traits and schema vocabulary
- [src/substrate/repo/](/Users/vinuth/code/pari/src/substrate/repo): filesystem-backed backend
- [src/substrate/in_memory/](/Users/vinuth/code/pari/src/substrate/in_memory): in-memory backend
- [src/substrate/void.rs](/Users/vinuth/code/pari/src/substrate/void.rs): no-op backend

## Current Contract

The crate-wide substrate trait is `crate::substrate::Substrate`.

Key points:

- `load_strategy(entity_kind, field)` returns `Result<LoadStrategy, SubstrateError>`
- `exists(&[AnyEntityRef])` is batched
- `load(&TrackedEntity, &[&str])` returns a tracked entity payload
- `persist(iterator_of_EntityChange)` consumes the explicit store-owned handoff type

The substrate layer may depend on `EntityChange` from `store`, but not on store actor internals.

## Boundary Rules

- Do not move actor flow, request handling, or checkout lifecycle into this layer.
- Do not add caller-facing async API helpers here; that belongs to `workspace`.
- Do not author validation policy here.
- Keep storage layout, schema mapping, codec behavior, resolver behavior, and executor behavior here.

## Concrete Backends

- `RepoSubstrate`: schema-driven filesystem backend in `src/substrate/repo/`
- `InMemorySubstrate`: schema-driven in-memory backend in `src/substrate/in_memory/`
- `VoidSubstrate`: minimal no-op backend for tests that only need the contract surface

Avoid documenting removed legacy storage modules or schema-era backend structure.
