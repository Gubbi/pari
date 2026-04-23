# Pari — Design Documentation

This directory holds Pari's design documentation. Start with [framework.md](./framework.md) for the bird's-eye view, then drill into the per-layer docs under [layers/](./layers/) as needed.

## Guiding Principle — C4 Model Alignment

Design docs follow the [C4 model](https://c4model.com). Each C4 level has a home:

| C4 Level | What It Covers | Where It Lives |
|---|---|---|
| **L1 System Context** | Larger systems Pari is integrated into | Out of scope — Pari is a container within those systems |
| **L2 Container** | Pari as a framework: extension seams, core, persistence | [framework.md](./framework.md) |
| **L3 Component** | Per-layer design and the generic layer-model framework | [layers/](./layers/) |
| **L4 Code** | Key types, interfaces, message protocols | Co-located with source — rustdoc, in-file comments |

## Table of Contents

### Framework-level

- [framework.md](./framework.md) — L2 Container view: extension seams (client surface, persistence backend), core layer roles, error hierarchy for integrators.
- [layers/layer-model.md](./layers/layer-model.md) — generic layer-model framework: formal ownership, dependency rules, pure/orchestration structure.

### Per-layer design

- [layers/entities.md](./layers/entities.md) — entity layer: identity, macros, tracked versions, schemas.
- [layers/workspace.md](./layers/workspace.md) — workspace layer: uniform access gateway, transparent expansion, automatic validation.
- [layers/store.md](./layers/store.md) — store layer: `EntityServer` + `StoreManager` split, actor model, sparse staging.
- [layers/substrate.md](./layers/substrate.md) — substrate layer: asset pipeline, slot/asset/entity composition, schema-driven load/persist paths.
- [layers/validation.md](./layers/validation.md) — validation layer: three-kind model, `ValidationSchema<T>`, runner flow.
- [layers/error-handling.md](./layers/error-handling.md) — error layer: composition, propagation, OTel emission, `as_error<E>()` downcasting, SpanTrace invariants.
