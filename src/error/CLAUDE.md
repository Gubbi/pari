# src/error — Error Classification Infrastructure

## ErrorCompose trait

Every error type in the codebase derives `#[derive(pari_macros::ErrorCompose)]` which generates an impl of:

```rust
pub trait ErrorCompose: std::error::Error {
    fn fix_domain(&self)     -> FixDomain;
    fn recoverability(&self) -> Recoverability;
    fn severity(&self)       -> Severity;            // derived: Error if NotRecoverable, else Warn
    fn as_error(&self)       -> &dyn std::error::Error;
}
```

**`#[compose(...)]` attribute on variants:**
- `#[compose(fix = X, recoverability = Y)]` — declares values for this variant
- `#[compose(delegate)]` — delegates to the inner error's `ErrorCompose` impl

---

## OTelEmit trait

`#[derive(pari_macros::OTelEmit)]` generates:

```rust
pub trait OTelEmit: ErrorCompose {
    fn emit(&self);  // structured tracing event with OTel semantic attributes
}
```

**`#[otel(...)]` attribute on variants:**
- `#[otel(error_type = "snake_case")]` — sets `error.type` attribute
- `#[otel(field = "attr.name")]` on struct fields — emits field as OTel attribute

---

## FixDomain enum

```rust
pub enum FixDomain {
    Client,    // caller passed bad input
    Data,      // data on disk / in substrate is bad
    Infra,     // infrastructure failure (I/O, network)
    Pari,      // internal library bug
}
```

## Recoverability enum

```rust
pub enum Recoverability {
    Retryable,
    UserAction,
    OperatorAction,
    NotRecoverable,
}
```

## Severity enum

```rust
pub enum Severity { Warn, Error }
// Derived: NotRecoverable → Error; everything else → Warn
```

---

## BatchError<E>

Aggregates multiple errors; classification uses worst-case across all contained errors.

```rust
pub struct BatchError<E: ErrorCompose> { errors: Vec<E> }
impl<E: ErrorCompose> BatchError<E> {
    pub fn new(errors: Vec<E>) -> Self;
    pub fn errors(&self) -> &[E];
}
// Implements ErrorCompose: fix_domain/recoverability aggregate worst-case
// Implements Display: lists all errors
```

Used by `PersistError::SubstrateErrors(BatchError<SubstrateError>)`.

---

## PariError (`src/error/pari_error.rs`)

Top-level error enum for job-layer APIs:
```rust
pub enum PariError {
    Store(StoreError),
    Checkout(CheckoutError),
    Commit(CommitError),
    Load(LoadError),
    Persist(PersistError),
    Resolve(ResolveError),
    Undo(UndoError),
    Substrate(SubstrateError),
    Validation(ValidationErrors),
}
```

---

## Derive Gotcha — No duplicate delegation

The `ErrorCompose` macro reads both `otel` and `compose` attributes (via `darling`). Never put both `#[compose(delegate)]` and `#[otel(delegate)]` on the same variant — darling will error with "Duplicate field 'delegate'". Only `#[compose(delegate)]` is needed; `OTelEmit` reads it too.
