# Task 12 — Error Handling Infrastructure

## Scope

Implement the cross-cutting error handling infrastructure:

1. `FixDomain`, `Recoverability`, `Severity` — classification enums
2. `ErrorCompose` trait — sealed, with classification methods and `as_error<E>()` downcasting
3. `OTelEmit` trait — structured OTel event emission
4. `BatchError<E>` — collection error with worst-case aggregation
5. `#[derive(ErrorCompose)]` proc macro — `#[compose(...)]` annotation support
6. `#[derive(OTelEmit)]` proc macro — `#[otel(...)]` annotation support

This task produces no concrete error types. Those are in Task 13. This task provides only the derive macros and the infrastructure types that Task 13 consumes.

---

## Files

- `src/error/mod.rs` — `FixDomain`, `Recoverability`, `Severity`, `ErrorCompose` trait, `OTelEmit` trait, `BatchError<E>`
- `pari-macros/src/error_compose.rs` — `#[derive(ErrorCompose)]` implementation
- `pari-macros/src/otel_emit.rs` — `#[derive(OTelEmit)]` implementation
- `pari-macros/src/lib.rs` — add `pub use` for `ErrorCompose` and `OTelEmit` derives
- `Cargo.toml` — add `thiserror`, `tracing`, `tracing-error`, `opentelemetry_semantic_conventions`
- `src/lib.rs` — `pub mod error;`

---

## Dependencies

- Tasks 01–11: no code dependency; this task is consumed by Task 13

---

## Classification Types (`src/error/mod.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixDomain {
    /// Fix is in the caller's input or usage.
    Client,
    /// Fix requires repairing stored content (corrupt or malformed).
    Data,
    /// Fix is in the underlying infrastructure (permissions, disk, network).
    Infra,
    /// Fix is in Pari's code (invariant violated, logic bug).
    Pari,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recoverability {
    /// Transient failure — retry automatically after backoff.
    Retryable,
    /// Caller must fix their input or definition, then retry.
    UserAction,
    /// Operator must fix infrastructure or data, then retry.
    OperatorAction,
    /// Code invariant violated — do not retry, escalate to developer.
    NotRecoverable,
}

/// Severity is derived, never declared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Warn,
    Error,
}

impl Severity {
    pub fn from_classification(fix: FixDomain, recoverability: Recoverability) -> Self {
        match (fix, recoverability) {
            (FixDomain::Pari,   Recoverability::NotRecoverable) => Severity::Error,
            (FixDomain::Data,   Recoverability::OperatorAction) => Severity::Error,
            (FixDomain::Infra,  Recoverability::OperatorAction) => Severity::Error,
            (FixDomain::Infra,  Recoverability::Retryable)      => Severity::Warn,
            (FixDomain::Client, Recoverability::UserAction)     => Severity::Warn,
            _                                                   => Severity::Error,
        }
    }
}
```

---

## `ErrorCompose` Trait (`src/error/mod.rs`)

```rust
pub trait ErrorCompose: sealed::AsAny + std::error::Error + Send + Sync + 'static {
    fn fix_domain(&self)     -> FixDomain;
    fn recoverability(&self) -> Recoverability;
    fn severity(&self)       -> Severity {
        Severity::from_classification(self.fix_domain(), self.recoverability())
    }
}

impl dyn ErrorCompose {
    /// Downcast through the composition chain to find a concrete error type.
    /// Searches only the current node — callers walk `source()` manually if needed.
    pub fn as_error<E: 'static>(&self) -> Option<&E> {
        self.as_any().downcast_ref::<E>()
    }
}

mod sealed {
    pub trait AsAny: 'static {
        fn as_any(&self) -> &dyn std::any::Any;
    }
    impl<T: 'static> AsAny for T {
        fn as_any(&self) -> &dyn std::any::Any { self }
    }
}
```

---

## `OTelEmit` Trait (`src/error/mod.rs`)

```rust
pub trait OTelEmit {
    /// Emit a structured OTel event. Cascades to inner errors via `source()`.
    /// Call once at the job layer — cascades automatically.
    fn emit(&self);
}
```

---

## `BatchError<E>` (`src/error/mod.rs`)

```rust
/// Wraps a collection of failures from a single operation.
/// Sits as an Intermediary Op node in the source() chain.
/// Classification properties are aggregated worst-case across all inner errors.
#[derive(Debug)]
pub struct BatchError<E: ErrorCompose + std::fmt::Debug> {
    pub errors: Vec<E>,
}

impl<E: ErrorCompose + std::fmt::Debug> BatchError<E> {
    pub fn new(errors: Vec<E>) -> Self { Self { errors } }
}

impl<E: ErrorCompose + std::fmt::Debug + std::fmt::Display> std::fmt::Display for BatchError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} error(s)", self.errors.len())?;
        for (i, e) in self.errors.iter().enumerate() {
            write!(f, "; [{i}] {e}")?;
        }
        Ok(())
    }
}

impl<E: ErrorCompose + std::fmt::Debug + std::fmt::Display> std::error::Error for BatchError<E> {}

impl<E: ErrorCompose + std::fmt::Debug + std::fmt::Display> ErrorCompose for BatchError<E> {
    fn fix_domain(&self) -> FixDomain {
        self.errors.iter()
            .map(|e| e.fix_domain())
            .max_by_key(|d| match d {
                FixDomain::Pari   => 3,
                FixDomain::Data   => 2,
                FixDomain::Infra  => 1,
                FixDomain::Client => 0,
            })
            .unwrap_or(FixDomain::Pari)
    }

    fn recoverability(&self) -> Recoverability {
        self.errors.iter()
            .map(|e| e.recoverability())
            .max_by_key(|r| match r {
                Recoverability::NotRecoverable => 3,
                Recoverability::OperatorAction => 2,
                Recoverability::UserAction     => 1,
                Recoverability::Retryable      => 0,
            })
            .unwrap_or(Recoverability::NotRecoverable)
    }
}
```

---

## `#[derive(ErrorCompose)]` Proc Macro

Added to `pari-macros/src/lib.rs`; implemented in `pari-macros/src/error_compose.rs`.

### Annotation forms

The macro accepts one of two annotation forms on the type:

**Activity layer — declares classification:**
```rust
#[derive(thiserror::Error, Debug, ErrorCompose)]
#[error("bad definition in {path}")]
#[compose(fix = Data, recoverability = OperatorAction)]
pub struct BadDefinitionError {
    pub path: String,
    pub hint: Option<String>,
    #[source]
    pub cause: SomePrimitiveError,
}
```

**Intermediary Op / Job layer — delegates to a single inner error type:**
```rust
#[derive(thiserror::Error, Debug, ErrorCompose)]
pub enum OpError {
    #[error(transparent)]
    #[compose(delegate)]
    Load(#[from] LoadError),

    #[error(transparent)]
    #[compose(delegate)]
    Persist(PersistError),
}
```

Every variant/field on a delegating type must carry `#[compose(delegate)]`. Missing annotation is a compile error.

### What the macro generates

**Declaring type:**
```rust
impl ErrorCompose for BadDefinitionError {
    fn fix_domain(&self)     -> FixDomain     { FixDomain::Data }
    fn recoverability(&self) -> Recoverability { Recoverability::OperatorAction }
}
```

**Delegating enum:**
```rust
impl ErrorCompose for OpError {
    fn fix_domain(&self) -> FixDomain {
        match self {
            Self::Load(inner)    => inner.fix_domain(),
            Self::Persist(inner) => inner.fix_domain(),
        }
    }
    fn recoverability(&self) -> Recoverability {
        match self {
            Self::Load(inner)    => inner.recoverability(),
            Self::Persist(inner) => inner.recoverability(),
        }
    }
}
```

---

## `#[derive(OTelEmit)]` Proc Macro

Implemented in `pari-macros/src/otel_emit.rs`.

### Annotation forms

- `#[otel(error_type = "snake_case_name")]` — on the struct/enum: the `exception.type` OTel attribute value
- `#[otel(field = "attr.name")]` — on a field: emitted as a named attribute
- `#[otel(delegate)]` — on a `#[source]` field or `#[compose(delegate)]` variant: cascades `emit()` to the inner error
- Fields named `span_trace` (type `SpanTrace`) and `backtrace` (type `std::backtrace::Backtrace`) are always included when present — recognized by field name

### What the macro generates

For a **struct** with annotations:
```rust
// Given:
#[derive(OTelEmit)]
#[otel(error_type = "malformed_frontmatter")]
pub struct MalformedFrontmatterError {
    #[otel(field = "error.component")]
    pub component: CodecComponent,
    #[otel(field = "file.path")]
    pub path: String,
    #[otel(field = "file.line")]
    pub line: Option<usize>,
    pub span_trace: SpanTrace,
    pub backtrace: std::backtrace::Backtrace,
}

// Generated:
impl OTelEmit for MalformedFrontmatterError {
    fn emit(&self) {
        tracing::error!(
            exception.type    = "malformed_frontmatter",
            exception.message = %self,
            exception.stacktrace = %self.backtrace,
            span_trace        = %self.span_trace,
            error.component   = %self.component,
            file.path         = %self.path,
            file.line         = ?self.line,
        );
    }
}
```

For a **struct** with `#[otel(delegate)]` on a `#[source]` field:
```rust
// Generated:
impl OTelEmit for BadDefinitionError {
    fn emit(&self) {
        tracing::error!(
            exception.type    = "bad_definition",
            exception.message = %self,
            error.hint        = ?self.hint,
        );
        self.cause.emit();   // cascade to inner
    }
}
```

For a **delegating enum**:
```rust
impl OTelEmit for OpError {
    fn emit(&self) {
        match self {
            Self::Load(inner)    => inner.emit(),
            Self::Persist(inner) => inner.emit(),
        }
    }
}
```

The macro uses `tracing::warn!` when `severity()` would be `Warn`, `tracing::error!` when `Error`. Since severity is determined at the Activity layer and the macro generates the emit at compile time, the choice between `warn!` and `error!` is driven by the `#[compose(fix = ..., recoverability = ...)]` annotation on the same type. Delegating types always call the inner type's `emit()` without emitting their own tracing event.

---

## TDD: Tests to Write First

```rust
// tests/error_compose.rs

use pari::error::{FixDomain, Recoverability, Severity, ErrorCompose, BatchError};
use pari_macros::{ErrorCompose, OTelEmit};
use thiserror::Error;

// --- Classification types ---

#[test]
fn severity_pari_not_recoverable_is_error() {
    assert_eq!(
        Severity::from_classification(FixDomain::Pari, Recoverability::NotRecoverable),
        Severity::Error
    );
}

#[test]
fn severity_client_user_action_is_warn() {
    assert_eq!(
        Severity::from_classification(FixDomain::Client, Recoverability::UserAction),
        Severity::Warn
    );
}

#[test]
fn severity_infra_retryable_is_warn() {
    assert_eq!(
        Severity::from_classification(FixDomain::Infra, Recoverability::Retryable),
        Severity::Warn
    );
}

#[test]
fn severity_data_operator_action_is_error() {
    assert_eq!(
        Severity::from_classification(FixDomain::Data, Recoverability::OperatorAction),
        Severity::Error
    );
}

// --- ErrorCompose derive: Activity (declaring) ---

#[derive(Error, Debug, ErrorCompose)]
#[error("data error: {message}")]
#[compose(fix = Data, recoverability = OperatorAction)]
struct TestDataError {
    message: String,
    hint: Option<String>,
}

#[test]
fn activity_fix_domain() {
    let e = TestDataError { message: "bad".into(), hint: None };
    assert_eq!(e.fix_domain(), FixDomain::Data);
}

#[test]
fn activity_recoverability() {
    let e = TestDataError { message: "bad".into(), hint: None };
    assert_eq!(e.recoverability(), Recoverability::OperatorAction);
}

#[test]
fn activity_severity_derived() {
    let e = TestDataError { message: "bad".into(), hint: None };
    assert_eq!(e.severity(), Severity::Error);
}

// --- ErrorCompose derive: Intermediary Op (delegating enum) ---

#[derive(Error, Debug, ErrorCompose)]
#[error("client error")]
#[compose(fix = Client, recoverability = UserAction)]
struct TestClientError;

#[derive(Error, Debug, ErrorCompose)]
pub enum TestOpError {
    #[error(transparent)]
    #[compose(delegate)]
    Data(TestDataError),

    #[error(transparent)]
    #[compose(delegate)]
    Client(TestClientError),
}

#[test]
fn delegating_propagates_data_fix_domain() {
    let e = TestOpError::Data(TestDataError { message: "x".into(), hint: None });
    assert_eq!(e.fix_domain(), FixDomain::Data);
}

#[test]
fn delegating_propagates_client_recoverability() {
    let e = TestOpError::Client(TestClientError);
    assert_eq!(e.recoverability(), Recoverability::UserAction);
}

// --- as_error downcasting ---

#[test]
fn as_error_finds_inner_type() {
    let op: &dyn ErrorCompose =
        &TestOpError::Data(TestDataError { message: "oops".into(), hint: Some("fix it".into()) });
    let inner = op.as_error::<TestDataError>();
    assert!(inner.is_some());
    assert_eq!(inner.unwrap().hint.as_deref(), Some("fix it"));
}

#[test]
fn as_error_returns_none_for_wrong_type() {
    let op: &dyn ErrorCompose = &TestOpError::Client(TestClientError);
    assert!(op.as_error::<TestDataError>().is_none());
}

// --- BatchError worst-case aggregation ---

#[test]
fn batch_fix_domain_worst_case() {
    let batch = BatchError::new(vec![
        TestOpError::Client(TestClientError),
        TestOpError::Data(TestDataError { message: "x".into(), hint: None }),
    ]);
    // Data > Client
    assert_eq!(batch.fix_domain(), FixDomain::Data);
}

#[test]
fn batch_recoverability_worst_case() {
    let batch = BatchError::new(vec![
        TestOpError::Client(TestClientError),       // UserAction
        TestOpError::Data(TestDataError { message: "x".into(), hint: None }), // OperatorAction
    ]);
    // OperatorAction > UserAction
    assert_eq!(batch.recoverability(), Recoverability::OperatorAction);
}

#[test]
fn batch_empty_defaults_to_pari_not_recoverable() {
    let batch: BatchError<TestOpError> = BatchError::new(vec![]);
    assert_eq!(batch.fix_domain(), FixDomain::Pari);
    assert_eq!(batch.recoverability(), Recoverability::NotRecoverable);
}

// --- OTelEmit compile test (emit() exists and is callable) ---

#[derive(Error, Debug, ErrorCompose, OTelEmit)]
#[error("test emit")]
#[compose(fix = Client, recoverability = UserAction)]
#[otel(error_type = "test_emit")]
struct TestEmitError {
    #[otel(field = "test.field")]
    pub detail: String,
}

#[test]
fn otel_emit_compiles_and_is_callable() {
    let e = TestEmitError { detail: "hello".into() };
    e.emit();  // must compile; event is a no-op if no tracing subscriber
}
```

---

## Implementation Notes

### Proc macro attribute parsing

Use `darling` crate for attribute parsing in `pari-macros`. `#[compose(...)]` and `#[otel(...)]` are distinct helper attributes; each macro only reads its own.

### `as_error<E>()` on `dyn ErrorCompose`

Defined on `dyn ErrorCompose`, not on the trait itself. Requires `ErrorCompose: 'static` — already enforced by `sealed::AsAny: 'static`. Downcasts only the current node; walking the `source()` chain is the caller's responsibility.

### OTelEmit in non-OTel environments

`tracing` events with no active subscriber are no-ops. No feature flag needed.

### Cargo.toml additions

```toml
[dependencies]
thiserror = "2"
tracing = "0.1"
tracing-error = "0.2"
opentelemetry_semantic_conventions = "0.29"

[dependencies.pari-macros]
path = "pari-macros"

# pari-macros/Cargo.toml additions:
[dependencies]
darling = "0.20"
quote = "1"
proc-macro2 = "1"
syn = { version = "2", features = ["full"] }
```

---

## Acceptance Criteria

- `#[derive(ErrorCompose)]` compiles on struct and enum types with `#[compose(...)]` annotations
- `fix_domain()` and `recoverability()` return declared values at the Activity layer
- Delegation propagates through Intermediary Op and Job layers
- `as_error<E>()` correctly downcasts the current node; returns `None` for the wrong type
- `BatchError` worst-case aggregation is correct for both `fix_domain` and `recoverability`
- `Severity::from_classification` maps all defined pairs correctly
- `#[derive(OTelEmit)]` compiles and `emit()` is callable without panicking
- Tasks 01–11 tests still pass
