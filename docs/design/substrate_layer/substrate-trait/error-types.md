# error-types

**Substrate Layer → `substrate_layer/substrate-trait/`**

---

## Purpose

Error types across the Store Layer and substrate boundary. Each error is scoped to its operation level. No panics — all failure paths return `Result`.

---

## SubstrateError

Primitive-level error from substrate I/O operations.

```rust
struct SubstrateError {
    path: String,
    message: String,
}
```

Returned by `exists` and `load`, and collected (not short-circuited) by `persist`.

---

## CheckoutError

Returned by `EntityClient::checkout()`.

```rust
enum CheckoutError {
    AlreadyCheckedOut,
    NotFound,
    SubstrateError(SubstrateError),
}
```

- `AlreadyCheckedOut` — the entity is already checked out; single-checkout rule (see [44 · single-checkout-rule](../../store_layer/checkout/single-checkout-rule.md))
- `NotFound` — the ref does not resolve to a known entity in the store or substrate
- `SubstrateError` — the existence check against the substrate failed

---

## CommitError

Returned by `entity.commit()`.

```rust
enum CommitError {
    ValidationFailed(Vec<ValidationError>),
    CrossReferenceCheckFailed(SubstrateError),
    StoreUnavailable,
}
```

- `ValidationFailed` — one or more validation rules failed; the commit is rejected and the entity remains checked out
- `CrossReferenceCheckFailed` — a substrate IO error occurred while verifying a cross-entity ref during check-in validation; distinct from `ValidationFailed` because the validity of the ref is unknown, not determined to be absent. The entity remains checked out; the caller may retry.
- `StoreUnavailable` — the EntityServer channel is closed; should not occur in normal operation

---

## LoadError

Returned by internal load operations.

```rust
enum LoadError {
    NotFound,
    SubstrateError(SubstrateError),
    ValidationFailed(Vec<ValidationError>),
}
```

Load validates after each round (see [60 · progressive-loading-loop](../../workspace_layer/load/progressive-loading-loop.md)). A validation failure during load surfaces as `LoadError::ValidationFailed`.

---

## UndoError

Returned by `EntityClient::undo_commit()` and `EntityClient::unload()`.

```rust
enum UndoError {
    WrongState,
    StoreUnavailable,
}
```

- `WrongState` — entity is not in the required state for the operation (see [48b · store-entity-lifecycle](../../store_layer/entity-store/store-entity-lifecycle.md))
- `StoreUnavailable` — the EntityServer channel is closed; should not occur in normal operation

---

## PersistError

Returned by `EntityClient::persist()`.

```rust
enum PersistError {
    PendingCheckouts,
    SubstrateError(Vec<SubstrateError>),
}
```

- `PendingCheckouts` — one or more entities are currently checked out; persist is blocked until all checkouts are resolved via `commit()` or `undo_checkout()` (see [48 · store-persist-phases](../../store_layer/entity-store/store-persist-phases.md))
- `SubstrateError` — one or more write operations failed; errors are collected, not short-circuited; change lists are preserved for retry
