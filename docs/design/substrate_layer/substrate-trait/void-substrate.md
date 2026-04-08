# void-substrate

**Substrate Layer → `substrate_layer/substrate-trait/`**

---

## Purpose

`VoidSubstrate` is a minimal `Substrate` implementation for tests and in-memory usage. It returns "not found" for all reads and succeeds silently on writes. There is no special "no substrate" mode — the substrate is always injected. `VoidSubstrate` is the convention for cases where no backing storage is needed.

---

## Behavior

`VoidSubstrate` overrides the default `load_strategy`, `exists`, `load`, and `persist` methods directly. The required component accessors are satisfied with unit types that are never called.

```rust
struct VoidSubstrate;

impl Substrate for VoidSubstrate {
    type Slot     = ();
    type Location = ();
    type Encoded  = ();
    type Resolver = ();
    type Codec    = ();
    type Executor = ();

    fn resolver(&self) -> &() { &() }
    fn codec(&self)    -> &() { &() }
    fn executor(&self) -> &() { &() }

    fn load_strategy(_kind: EntityKind, _field: &str) -> LoadStrategy {
        LoadStrategy { prerequisites: &[], mutable_without_load: true }
    }

    async fn exists(&self, refs: &[AnyEntityRef]) -> Result<Vec<bool>, SubstrateError> {
        Ok(vec![false; refs.len()])
    }

    async fn load(&self, _entity: &TrackedEntity, _fields: &[&str]) -> Result<TrackedEntity, SubstrateError> {
        Err(SubstrateError::NotFound)
    }

    async fn persist(&self, _changes: impl Iterator<Item = EntityChange<'_>>) -> Result<(), Vec<SubstrateError>> {
        Ok(())
    }
}
```

---

## Behavior Table

| Method | Behavior |
|---|---|
| `load_strategy` | No prerequisites; `mutable_without_load: true` |
| `exists` | All `false` — nothing exists |
| `load` | `Err(NotFound)` |
| `persist` | `Ok(())` — silent no-op |

---

## Usage

- **Unit tests** — inject `VoidSubstrate` to test store and entity logic without filesystem I/O
- **Pure in-memory usage** — build and manipulate entities entirely in memory; persist is a no-op
- **`resolve()` behavior** — `exists` returns all-false, so `resolve()` returns `Err(NotFound)` for any ref not already in the store
