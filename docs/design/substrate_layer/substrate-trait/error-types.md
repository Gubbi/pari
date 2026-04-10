# error-types

**Substrate Layer → `substrate_layer/substrate-trait/`**

---

## Purpose

Error types across the Store Layer and substrate boundary. Each error is assigned to its layer in the error chain model (see [error-handling](../../../error-handling/error-handling.md)). No panics — all failure paths return `Result`.

---

## Primitive Layer

### CodecError

Primitive error from encode/decode operations inside the substrate codec.

```rust
// pari::substrate::pipeline::codec::error
struct CodecError {
    component: CodecComponent,
    field: String,
    message: String,
    span_trace: SpanTrace,
    backtrace: Backtrace,
}
```

Constructor: `CodecError::new(field, message)` — captures `SpanTrace` and `Backtrace` at call site.

### ExecutorError

Primitive error from I/O operations inside the substrate executor.

```rust
// pari::substrate::pipeline::executor::error
struct ExecutorError {
    component: ExecutorComponent,
    location: String,
    message: String,
    span_trace: SpanTrace,
    backtrace: Backtrace,
}
```

Constructor: `ExecutorError::new(location, message)` — captures `SpanTrace` and `Backtrace` at call site.

---

## Intermediary Op Layer

### SubstrateError

Intermediary op at the substrate boundary. Wraps the two primitive error kinds — codec failures and executor/IO failures.

```rust
// pari::substrate::error
enum SubstrateError {
    Codec(CodecError),
    Executor(ExecutorError),
}
```

Returned by `exists`, `load`, and collected (not short-circuited) by `persist`.

---

## Activity Layer

Activity layer errors declare `fix` and `recoverability` and carry `hint: Option<String>` for corrective guidance.

### CheckoutError

Returned by `EntityClient::checkout()`.

```rust
enum CheckoutError {
    AlreadyCheckedOut {
        entity_ref: String,
        hint: Option<String>,
    },
    EntityNotFound {
        entity_ref: String,
        hint: Option<String>,
    },
    StoreUnavailable(StoreError),
}
```

- `AlreadyCheckedOut` — the entity is already checked out; single-checkout rule (see [single-checkout-rule](../../store_layer/checkout/single-checkout-rule.md))
- `EntityNotFound` — the ref does not resolve to a known entity in the store; callers must `resolve` before `checkout`
- `StoreUnavailable` — the EntityServer channel is closed before the checkout request completes; carries the underlying `StoreError`

### CommitError

Returned by `entity.commit()`.

```rust
enum CommitError {
    ValidationFailed {
        error_count: usize,
        errors: ValidationErrors,
        hint: Option<String>,
    },
    CrossReferenceCheckFailed(SubstrateError),
    StoreUnavailable(StoreError),
}
```

- `ValidationFailed` — one or more validation rules failed; the commit is rejected and the entity remains checked out
- `CrossReferenceCheckFailed` — a substrate error occurred while verifying a cross-entity ref during commit validation; validity of the ref is unknown (not determined absent); entity remains checked out; caller may retry
- `StoreUnavailable` — the EntityServer channel is closed; carries the underlying `StoreError`; should not occur in normal operation

### LoadError

Returned by internal load operations triggered by field accessors.

```rust
enum LoadError {
    NotFound {
        entity_ref: String,
        hint: Option<String>,
    },
    Substrate(SubstrateError),
    ValidationFailed {
        error_count: usize,
        errors: ValidationErrors,
        hint: Option<String>,
    },
    StoreUnavailable(StoreError),
}
```

Load validates after each round (see [progressive-loading-loop](../../workspace_layer/load/progressive-loading-loop.md)). A validation failure during load surfaces as `LoadError::ValidationFailed`. Ref prefetch during load is only an optimization ahead of validation; if validation cannot validate the fetched fields, the load fails and the fetched data is not merged.
- `StoreUnavailable` — the EntityServer channel is closed before the load request completes; carries the underlying `StoreError`

### UndoError

Returned by `EntityClient::undo_commit()` and `entity.undo_checkout()`.

```rust
enum UndoError {
    WrongState { hint: Option<String> },
    StoreUnavailable(StoreError),
}
```

- `WrongState` — entity is not in the required state for the operation (see [store-entity-lifecycle](../../store_layer/entity-store/store-entity-lifecycle.md))
- `StoreUnavailable` — the EntityServer channel is closed; carries the underlying `StoreError`; should not occur in normal operation

### PersistError

Returned by `EntityClient::persist()`.

```rust
enum PersistError {
    PendingCheckouts {
        checked_out_count: usize,
        hint: Option<String>,
    },
    SubstrateErrors(BatchError<SubstrateError>),
    StoreUnavailable(StoreError),
}
```

- `PendingCheckouts` — one or more entities are currently checked out; persist is blocked until all checkouts are resolved via `commit()` or `undo_checkout()` (see [store-persist-phases](../../store_layer/entity-store/store-persist-phases.md))
- `SubstrateErrors` — one or more write operations failed; errors are collected into a `BatchError`, not short-circuited; change lists are preserved for retry; `BatchError` aggregates worst-case `fix_domain` and `recoverability` across all failures (see [error-handling — Batch Errors](../../../error-handling/error-handling.md))
- `StoreUnavailable` — the EntityServer channel is closed before the persist request completes; carries the underlying `StoreError`

### ResolveError

Returned by `EntityClient::resolve()`.

```rust
enum ResolveError {
    NotFound {
        entity_ref: String,
        hint: Option<String>,
    },
    Substrate(SubstrateError),
    StoreUnavailable(StoreError),
}
```

- `NotFound` — the entity does not exist in the store or substrate
- `Substrate` — the substrate failed during an existence check
- `StoreUnavailable` — the EntityServer channel is closed before the resolve request completes; carries the underlying `StoreError`

---

## Channel-Level Error

### StoreError

Not part of the domain error hierarchy. Signals EntityServer channel failure only — does not carry `fix_domain` or `recoverability` beyond `Pari / NotRecoverable`.

```rust
enum StoreError {
    Unavailable,
}
```

Should not occur in normal operation. Wrapped by `StoreUnavailable` variants in activity layer errors where the store channel is a dependency.
