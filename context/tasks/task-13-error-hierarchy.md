# Task 13 — Concrete Error Hierarchy

## Scope

Define all concrete error types across every layer, wired with `#[derive(ErrorCompose, OTelEmit)]`
from Task 12. Replace all stub error types introduced in Tasks 06, 09, 10, and 11.

The error type names and variant structures come directly from the design docs:
- `substrate_layer/substrate-trait/error-types.md` — store-boundary errors
- `validation_layer/validation-api.md` — validation error types
- `substrate_layer/pipeline/codec.md` — `CodecError`
- `substrate_layer/pipeline/executor.md` — `ExecutorError`
- `store_layer/entity-server/store-server.md` — `StoreError`

Classification (`fix`, `recoverability`) is applied to each variant or type according to its
failure semantics.

---

## Files

**New:**
- `src/substrate/pipeline/codec/error.rs` — `CodecError` primitive
- `src/substrate/pipeline/executor/error.rs` — `ExecutorError` primitive
- `src/substrate/error.rs` — `SubstrateError`
- `src/store/error.rs` — `StoreError`, `CheckoutError`, `CommitError`, `LoadError`, `UndoError`, `PersistError`, `ResolveError`
- `src/validation/error.rs` — `SetterError`, `ValidationErrors`, `FieldValidationError`, `ValidationKind`
- `src/error/pari_error.rs` — `PariError` job-layer enum

**Updated (replacing stubs):**
- `src/substrate/pipeline/mod.rs` — replace `CodecError { field, message }` and `ExecutorError { location, message }` stubs with imports from the new error modules
- `src/substrate/mod.rs` — replace `SubstrateError { path, message }` stub on trait methods
- `src/validation/mod.rs` — replace `SubstrateError`, `SetterError`, `LoadError` stubs
- `src/store/mod.rs` — replace `StoreError`, `CheckoutError`, `CommitError`, `UndoError`, `PersistError`, `ResolveError` stubs
- `src/lib.rs` — `pub use error::pari_error::PariError;`

---

## Dependencies

- Task 12: `ErrorCompose`, `OTelEmit`, `BatchError`, `FixDomain`, `Recoverability`
- Tasks 06, 09, 10, 11: stubs to replace

---

## Layer Overview

```
PariError                                  job layer    (src/error/pari_error.rs)
  │
  ├─ ResolveError                          op/activity  (src/store/error.rs)
  ├─ CheckoutError                         op/activity  (src/store/error.rs)
  ├─ CommitError                           op/activity  (src/store/error.rs)
  ├─ PersistError                          op/activity  (src/store/error.rs)
  │    └─ BatchError<SubstrateError>
  ├─ SetterError                           op/activity  (src/validation/error.rs)
  ├─ LoadError                             op/activity  (src/store/error.rs)
  │
  └─ SubstrateError                        op/activity  (src/substrate/error.rs)
       ├─ CodecError                       primitive    (src/substrate/pipeline/codec/error.rs)
       └─ ExecutorError                    primitive    (src/substrate/pipeline/executor/error.rs)
```

`ValidationErrors` / `FieldValidationError` / `ValidationKind` are **plain data** — not
`ErrorCompose`. They are carried inside the `ValidationFailed` variant of operation errors.

---

## Primitive Layer

Primitives are the deepest concrete failures. They carry `span_trace` and `backtrace`.

### `CodecError` (`src/substrate/pipeline/codec/error.rs`)

From `codec.md`: encode/decode failure scoped to a named field.

```rust
#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
#[error("codec error on field '{field}': {message}")]
#[compose(fix = Data, recoverability = OperatorAction)]
#[otel(error_type = "codec_error")]
pub struct CodecError {
    #[otel(field = "error.field")]
    pub field:   String,
    #[otel(field = "error.message")]
    pub message: String,
    pub span_trace: tracing_error::SpanTrace,
    pub backtrace:  std::backtrace::Backtrace,
}

impl CodecError {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field:      field.into(),
            message:    message.into(),
            span_trace: tracing_error::SpanTrace::capture(),
            backtrace:  std::backtrace::Backtrace::capture(),
        }
    }
}
```

### `ExecutorError` (`src/substrate/pipeline/executor/error.rs`)

From `executor.md`: a single asset request failed at the I/O layer.

```rust
#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
#[error("executor error at '{location}': {message}")]
#[compose(fix = Infra, recoverability = OperatorAction)]
#[otel(error_type = "executor_error")]
pub struct ExecutorError {
    #[otel(field = "fs.path")]
    pub location: String,
    #[otel(field = "error.message")]
    pub message:  String,
    pub span_trace: tracing_error::SpanTrace,
    pub backtrace:  std::backtrace::Backtrace,
}

impl ExecutorError {
    pub fn new(location: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            location:   location.into(),
            message:    message.into(),
            span_trace: tracing_error::SpanTrace::capture(),
            backtrace:  std::backtrace::Backtrace::capture(),
        }
    }
}
```

---

## Substrate Boundary: `SubstrateError` (`src/substrate/error.rs`)

From `error-types.md`: the error type returned by `Substrate::persist`, `::load`, `::exists`.
It is an Intermediary Op delegating to either a codec or executor primitive.

```rust
#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
pub enum SubstrateError {
    #[error(transparent)]
    #[compose(delegate)]
    #[otel(delegate)]
    Codec(#[from] CodecError),

    #[error(transparent)]
    #[compose(delegate)]
    #[otel(delegate)]
    Executor(#[from] ExecutorError),
}
```

The `Substrate` trait methods keep the same signatures as in Task 10, now using `SubstrateError`
from this module instead of the stub:

```rust
async fn persist(&self, changes: impl Iterator<Item = EntityChange<'_>>)
    -> Result<(), Vec<SubstrateError>>;

async fn load(&self, entity: &TrackedEntity, fields: &[&str])
    -> Result<TrackedEntity, SubstrateError>;

async fn exists(&self, refs: &[AnyEntityRef])
    -> Result<Vec<bool>, SubstrateError>;
```

---

## Validation Data Types (`src/validation/error.rs`)

From `validation-api.md`. Plain data — no `ErrorCompose`.

```rust
#[derive(Debug, Clone)]
pub struct ValidationErrors {
    pub errors: Vec<FieldValidationError>,
}

impl ValidationErrors {
    pub fn new() -> Self { Self { errors: vec![] } }
    pub fn is_empty(&self) -> bool { self.errors.is_empty() }
    pub fn extend(&mut self, other: ValidationErrors) { self.errors.extend(other.errors); }
}

#[derive(Debug, Clone)]
pub struct FieldValidationError {
    pub path:    String,   // dot-notation: "id", "steps.WriteProposal.depends_on"
    pub message: String,
    pub kind:    ValidationKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationKind { Structural, Semantic, CrossEntity }
```

---

## `SetterError` (`src/validation/error.rs`)

From `validation-api.md`. Replaces the stub from Task 06.

```rust
#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
pub enum SetterError {
    /// ensure_mutable triggered a substrate load which failed.
    #[error(transparent)]
    #[compose(delegate)]
    #[otel(delegate)]
    Substrate(#[from] SubstrateError),

    /// Structural or semantic validation rejected the incoming value.
    #[error("validation failed: {error_count} error(s)")]
    #[compose(fix = Client, recoverability = UserAction)]
    #[otel(error_type = "setter_validation_failed")]
    Validation {
        #[otel(field = "validation.error_count")]
        error_count: usize,
        // Carried for caller inspection; not ErrorCompose.
        pub errors: ValidationErrors,
    },
}
```

---

## Store Operation Errors (`src/store/error.rs`)

From `error-types.md`, `store-checkout.md`, `store-commit.md`.

### `StoreError` — channel-level failure

Returned when the EntityServer actor channel is closed. Invariant violation (should never happen).

```rust
#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
pub enum StoreError {
    #[error("entity server unavailable")]
    #[compose(fix = Pari, recoverability = NotRecoverable)]
    #[otel(error_type = "store_unavailable")]
    Unavailable,
}
```

### `CheckoutError`

From `store-checkout.md` and `error-types.md`. Returned by `EntityClient::checkout()`.

```rust
#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
pub enum CheckoutError {
    #[error("entity already checked out: {entity_ref}")]
    #[compose(fix = Client, recoverability = UserAction)]
    #[otel(error_type = "already_checked_out")]
    AlreadyCheckedOut {
        #[otel(field = "entity.ref")]
        entity_ref: String,
    },

    #[error("entity not found: {entity_ref}")]
    #[compose(fix = Client, recoverability = UserAction)]
    #[otel(error_type = "entity_not_found")]
    EntityNotFound {
        #[otel(field = "entity.ref")]
        entity_ref: String,
    },

    /// Substrate I/O failed while checking existence.
    #[error(transparent)]
    #[compose(delegate)]
    #[otel(delegate)]
    Substrate(#[from] SubstrateError),
}
```

### `CommitError`

From `error-types.md` and `store-commit.md`. Returned by `TrackedEntity::commit()`.

```rust
#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
pub enum CommitError {
    /// One or more validation rules failed; entity remains checked out.
    #[error("commit validation failed: {error_count} error(s)")]
    #[compose(fix = Client, recoverability = UserAction)]
    #[otel(error_type = "commit_validation_failed")]
    ValidationFailed {
        #[otel(field = "validation.error_count")]
        error_count: usize,
        pub errors: ValidationErrors,
    },

    /// A substrate I/O error occurred while verifying a cross-entity ref.
    /// The ref's validity is unknown — not determined to be absent. Caller may retry.
    #[error(transparent)]
    #[compose(delegate)]
    #[otel(delegate)]
    CrossReferenceCheckFailed(SubstrateError),

    /// EntityServer actor channel closed.
    #[error(transparent)]
    #[compose(delegate)]
    #[otel(delegate)]
    StoreUnavailable(#[from] StoreError),
}
```

### `LoadError`

From `error-types.md`. Returned by internal load operations (field accessors).

```rust
#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
pub enum LoadError {
    /// Entity does not exist in store or substrate.
    #[error("entity not found: {entity_ref}")]
    #[compose(fix = Client, recoverability = UserAction)]
    #[otel(error_type = "load_entity_not_found")]
    NotFound {
        #[otel(field = "entity.ref")]
        entity_ref: String,
    },

    /// Substrate I/O failed during the load.
    #[error(transparent)]
    #[compose(delegate)]
    #[otel(delegate)]
    Substrate(#[from] SubstrateError),

    /// Validation of the newly loaded fields failed before merge.
    #[error("load validation failed: {error_count} error(s)")]
    #[compose(fix = Data, recoverability = OperatorAction)]
    #[otel(error_type = "load_validation_failed")]
    ValidationFailed {
        #[otel(field = "validation.error_count")]
        error_count: usize,
        pub errors: ValidationErrors,
    },
}
```

### `UndoError`

From `error-types.md`. Returned by `EntityClient::undo_commit()` and `EntityClient::unload()`.

```rust
#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
pub enum UndoError {
    /// Entity is not in the required state for this undo operation.
    #[error("wrong state for undo operation")]
    #[compose(fix = Pari, recoverability = NotRecoverable)]
    #[otel(error_type = "wrong_state_for_undo")]
    WrongState,

    /// EntityServer actor channel closed.
    #[error(transparent)]
    #[compose(delegate)]
    #[otel(delegate)]
    StoreUnavailable(#[from] StoreError),
}
```

### `PersistError`

From `error-types.md`. Returned by `EntityClient::persist()`.

```rust
#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
pub enum PersistError {
    /// One or more entities are still checked out; persist is blocked.
    #[error("persist blocked: {checked_out_count} checkout(s) pending")]
    #[compose(fix = Client, recoverability = UserAction)]
    #[otel(error_type = "pending_checkouts")]
    PendingCheckouts {
        #[otel(field = "store.checked_out_count")]
        checked_out_count: usize,
    },

    /// One or more substrate write operations failed; errors collected.
    #[error("{0}")]
    #[compose(delegate)]
    #[otel(delegate)]
    SubstrateErrors(BatchError<SubstrateError>),
}
```

### `ResolveError`

From `store-resolve.md`. Returned by `EntityClient::resolve()`.

```rust
#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
pub enum ResolveError {
    /// Entity does not exist in store or substrate.
    #[error("entity not found: {entity_ref}")]
    #[compose(fix = Client, recoverability = UserAction)]
    #[otel(error_type = "resolve_entity_not_found")]
    NotFound {
        #[otel(field = "entity.ref")]
        entity_ref: String,
    },

    /// Substrate existence check failed.
    #[error(transparent)]
    #[compose(delegate)]
    #[otel(delegate)]
    Substrate(#[from] SubstrateError),
}
```

---

## Job Layer: `PariError` (`src/error/pari_error.rs`)

Single top-level enum. One variant per client-visible operation. Delegates to store operation
errors and maps them to outcomes the caller can act on.

```rust
#[derive(thiserror::Error, Debug, ErrorCompose, OTelEmit)]
pub enum PariError {
    /// A new entity definition was rejected (validation or store error).
    #[error(transparent)]
    #[compose(delegate)]
    #[otel(delegate)]
    DefinitionRejected(#[from] CommitError),

    /// An entity mutation was rejected (checkout, commit, or setter error).
    #[error(transparent)]
    #[compose(delegate)]
    #[otel(delegate)]
    MutationFailed(CommitError),

    /// Checkout for mutation failed.
    #[error(transparent)]
    #[compose(delegate)]
    #[otel(delegate)]
    CheckoutFailed(#[from] CheckoutError),

    /// Entity or field load failed.
    #[error(transparent)]
    #[compose(delegate)]
    #[otel(delegate)]
    LoadFailed(#[from] LoadError),

    /// Resolving an entity reference failed.
    #[error(transparent)]
    #[compose(delegate)]
    #[otel(delegate)]
    ResolveFailed(#[from] ResolveError),

    /// Persisting changes to the substrate failed.
    #[error(transparent)]
    #[compose(delegate)]
    #[otel(delegate)]
    SaveFailed(#[from] PersistError),

    /// A setter was rejected.
    #[error(transparent)]
    #[compose(delegate)]
    #[otel(delegate)]
    SetterRejected(#[from] SetterError),
}
```

---

## Stub Replacements

| Task | Stub type | Replaced by (same name, new module) |
|------|-----------|--------------------------------------|
| 10 | `CodecError { field, message }` | `src/substrate/pipeline/codec/error.rs::CodecError` |
| 10 | `ExecutorError { location, message }` | `src/substrate/pipeline/executor/error.rs::ExecutorError` |
| 10, 11 | `SubstrateError { path, message }` | `src/substrate/error.rs::SubstrateError` (enum) |
| 06 | `SetterError { Substrate, Validation }` | `src/validation/error.rs::SetterError` |
| 06 | `LoadError { NotLoaded, Substrate, ValidationFailed }` | `src/store/error.rs::LoadError` |
| 06 | `ValidationErrors`, `FieldValidationError`, `ValidationKind` | `src/validation/error.rs` (same types, same fields) |
| 09 | `StoreError` | `src/store/error.rs::StoreError` |
| 09 | `CheckoutError` | `src/store/error.rs::CheckoutError` |
| 09 | `CommitError` | `src/store/error.rs::CommitError` |
| 09 | `UndoError` | `src/store/error.rs::UndoError` |
| 09 | `PersistError` | `src/store/error.rs::PersistError` |
| 09 | `ResolveError` | `src/store/error.rs::ResolveError` |

`ValidationErrors`, `FieldValidationError`, and `ValidationKind` have the same structure as
the Task 06 stubs — no structural change, only moved to `src/validation/error.rs`.

`LoadError` drops the `NotLoaded` variant. The design (`error-types.md`) does not include it —
a "field not loaded" state is an internal invariant in the accessor code, not a user-visible
error from the load operation.

---

## TDD: Tests to Write First

```rust
// tests/error_hierarchy.rs

use pari::error::{FixDomain, Recoverability, Severity, ErrorCompose};
use pari::substrate::pipeline::{codec::error::CodecError, executor::error::ExecutorError};
use pari::substrate::error::SubstrateError;
use pari::store::error::{
    StoreError, CheckoutError, CommitError, LoadError, UndoError, PersistError, ResolveError,
};
use pari::validation::error::{SetterError, ValidationErrors, FieldValidationError, ValidationKind};
use pari::error::pari_error::PariError;

// --- Primitive classifications ---

#[test]
fn codec_error_is_data_operator_action() {
    let e = CodecError::new("name", "expected string");
    assert_eq!(e.fix_domain(),     FixDomain::Data);
    assert_eq!(e.recoverability(), Recoverability::OperatorAction);
}

#[test]
fn executor_error_is_infra_operator_action() {
    let e = ExecutorError::new("roles/eng-lead.md", "permission denied");
    assert_eq!(e.fix_domain(),     FixDomain::Infra);
    assert_eq!(e.recoverability(), Recoverability::OperatorAction);
}

// --- SubstrateError delegates correctly ---

#[test]
fn substrate_error_codec_variant_delegates_fix_domain() {
    let sub = SubstrateError::Codec(CodecError::new("name", "bad"));
    assert_eq!(sub.fix_domain(), FixDomain::Data);
}

#[test]
fn substrate_error_executor_variant_delegates_fix_domain() {
    let sub = SubstrateError::Executor(ExecutorError::new("roles/x.md", "io error"));
    assert_eq!(sub.fix_domain(), FixDomain::Infra);
}

// --- Store operation error classifications ---

#[test]
fn checkout_already_checked_out_is_client_user_action() {
    let e = CheckoutError::AlreadyCheckedOut { entity_ref: "roles/eng-lead".into() };
    assert_eq!(e.fix_domain(),     FixDomain::Client);
    assert_eq!(e.recoverability(), Recoverability::UserAction);
    assert_eq!(e.severity(),       Severity::Warn);
}

#[test]
fn checkout_not_found_is_client_user_action() {
    let e = CheckoutError::EntityNotFound { entity_ref: "roles/eng-lead".into() };
    assert_eq!(e.fix_domain(),     FixDomain::Client);
    assert_eq!(e.recoverability(), Recoverability::UserAction);
}

#[test]
fn checkout_substrate_delegates() {
    let e = CheckoutError::Substrate(SubstrateError::Executor(
        ExecutorError::new("roles/x.md", "io error")
    ));
    assert_eq!(e.fix_domain(), FixDomain::Infra);
}

#[test]
fn commit_validation_failed_is_client_user_action() {
    let e = CommitError::ValidationFailed {
        error_count: 1,
        errors: ValidationErrors::new(),
    };
    assert_eq!(e.fix_domain(),     FixDomain::Client);
    assert_eq!(e.recoverability(), Recoverability::UserAction);
}

#[test]
fn commit_cross_ref_check_failed_delegates_to_substrate() {
    let e = CommitError::CrossReferenceCheckFailed(
        SubstrateError::Executor(ExecutorError::new("roles/x.md", "io"))
    );
    assert_eq!(e.fix_domain(), FixDomain::Infra);
}

#[test]
fn commit_store_unavailable_is_pari_not_recoverable() {
    let e = CommitError::StoreUnavailable(StoreError::Unavailable);
    assert_eq!(e.fix_domain(),     FixDomain::Pari);
    assert_eq!(e.recoverability(), Recoverability::NotRecoverable);
}

#[test]
fn load_validation_failed_is_data_operator_action() {
    let e = LoadError::ValidationFailed {
        error_count: 2,
        errors: ValidationErrors::new(),
    };
    // Data because substrate returned invalid content, not user input
    assert_eq!(e.fix_domain(),     FixDomain::Data);
    assert_eq!(e.recoverability(), Recoverability::OperatorAction);
}

#[test]
fn persist_pending_checkouts_is_client_user_action() {
    let e = PersistError::PendingCheckouts { checked_out_count: 3 };
    assert_eq!(e.fix_domain(),     FixDomain::Client);
    assert_eq!(e.recoverability(), Recoverability::UserAction);
}

#[test]
fn undo_wrong_state_is_pari_not_recoverable() {
    let e = UndoError::WrongState;
    assert_eq!(e.fix_domain(),     FixDomain::Pari);
    assert_eq!(e.recoverability(), Recoverability::NotRecoverable);
}

// --- as_error downcast through PariError chain ---

#[test]
fn pari_error_downcast_reaches_codec_error() {
    let codec = CodecError::new("name", "bad");
    let sub   = SubstrateError::Codec(codec);
    let load  = LoadError::Substrate(sub);
    let pari  = PariError::LoadFailed(load);

    let found = (&pari as &dyn ErrorCompose).as_error::<LoadError>();
    assert!(found.is_some());
}

// --- emit() compiles and is callable ---

#[test]
fn emit_on_pari_error_does_not_panic() {
    use pari::error::OTelEmit;
    let e = PariError::SaveFailed(PersistError::PendingCheckouts { checked_out_count: 1 });
    e.emit();
}

// --- ValidationErrors: plain data, not ErrorCompose ---

#[test]
fn validation_errors_accumulate() {
    let mut errs = ValidationErrors::new();
    errs.errors.push(FieldValidationError {
        path:    "id".into(),
        message: "must be kebab-case".into(),
        kind:    ValidationKind::Structural,
    });
    assert_eq!(errs.errors.len(), 1);
}

// --- Setter error ---

#[test]
fn setter_validation_error_is_client_user_action() {
    let e = SetterError::Validation {
        error_count: 1,
        errors: ValidationErrors::new(),
    };
    assert_eq!(e.fix_domain(),     FixDomain::Client);
    assert_eq!(e.recoverability(), Recoverability::UserAction);
}

// --- All operation errors implement ErrorCompose ---

fn assert_error_compose<E: ErrorCompose>() {}

#[test]
fn all_operation_errors_implement_error_compose() {
    assert_error_compose::<CodecError>();
    assert_error_compose::<ExecutorError>();
    assert_error_compose::<SubstrateError>();
    assert_error_compose::<StoreError>();
    assert_error_compose::<CheckoutError>();
    assert_error_compose::<CommitError>();
    assert_error_compose::<LoadError>();
    assert_error_compose::<UndoError>();
    assert_error_compose::<PersistError>();
    assert_error_compose::<ResolveError>();
    assert_error_compose::<SetterError>();
    assert_error_compose::<PariError>();
}
```

---

## Implementation Notes

### `ValidationErrors` in `ValidationFailed` variants

`CommitError::ValidationFailed` and `LoadError::ValidationFailed` carry a `pub errors: ValidationErrors`
field alongside the `error_count`. This is intentional — callers need structured per-field detail
for display and programmatic handling. The `error_count` field drives the OTel attribute; the
`errors` field is the full detail.

Note: Rust `pub` inside an enum variant body requires a tuple struct or pub struct pattern. Use
a named-field struct variant (`CommitError::ValidationFailed { error_count, errors }`) where
both fields are accessible since the variant is `pub`.

### `BatchError<SubstrateError>` in `PersistError`

`PersistError::SubstrateErrors(BatchError<SubstrateError>)` uses `BatchError` from Task 12.
`BatchError<SubstrateError>` implements `ErrorCompose` (Task 12) and aggregates worst-case
`fix_domain` and `recoverability` across the collected errors.

### `LoadError` vs Task 06 stub

Task 06 stub had a `NotLoaded` variant. The design's `error-types.md` does not include it —
`NotLoaded` is an internal accessor invariant, not a load operation failure. Remove it. Field
accessor code that currently returns `Err(LoadError::NotLoaded)` should use a `panic!` or
the field's `OnceLock` guarantees to avoid the error path entirely.

### `SubstrateError` as enum (not struct)

The stub from Tasks 10/11 was `SubstrateError { path, message }`. The real type is an enum
because codec failures (Data) and executor failures (Infra) have different classifications.
As an enum, `as_error<E>()` can reach the inner primitive type through `ErrorCompose`.

### Primitive constructors

`CodecError::new()` and `ExecutorError::new()` always capture `SpanTrace` and `Backtrace` at
construction. Never re-captured at higher layers.

---

## Acceptance Criteria

- All types in "All operation errors implement ErrorCompose" test compile and pass
- `fix_domain()` and `recoverability()` return correct values for all variants per the tests
- `ValidationErrors` / `FieldValidationError` / `ValidationKind` are plain data (no `ErrorCompose`)
- `emit()` is callable on `PariError` without panicking
- `as_error<E>()` downcast works through `LoadError → SubstrateError → CodecError`
- `BatchError<SubstrateError>` compiles in `PersistError::SubstrateErrors`
- All stub types from Tasks 06, 09, 10, 11 are removed; their callers updated to the new types
- Tasks 01–12 tests still pass
