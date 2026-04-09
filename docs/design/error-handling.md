# error-handling

**Cross-Cutting**

---

## Why This Document Exists

Error handling in a library is a contract with the library's users. A poor contract forces callers to either swallow errors blindly or write fragile string-matching code. A good contract gives callers structured, actionable information — enough to retry intelligently, surface the right message to their own users, alert operators, and diagnose failures after the fact.

Pari is a Rust library. Its errors cross multiple abstraction layers: a client initiates an operation, which may trigger internal ops, which eventually reach a leaf-level failure. Each layer knows something the layers above don't. The error system must preserve that information all the way to the surface without losing structure.

This document defines how errors are structured, named, composed, and observed in `pari`. It is written for engineers who need to understand the system — whether to implement new error types, consume errors in application code, or extend the design.

---

## Goals

1. **Actionable for callers** — every error carries enough information to decide: retry, surface to user, alert, or abort.
2. **Diagnosable for operators** — every error carries enough context to find the failing component, file, or entity without reading source code.
3. **Uniform observability** — OTel emission is driven by derive macros, not hand-written per error type.
4. **Principled composition** — classification properties (recoverability, fix domain, severity) propagate automatically through composed error chains.
5. **No information loss through layers** — structured fields from the deepest error remain accessible at the surface.

---

## Design Principles

**1. Outcome-based naming**
Error type names describe what went wrong or what state was left behind — not which code component failed. `BadDefinitionError` tells a reader what happened. `CodecParseError` tells a reader which class failed. The module path already identifies the component.

**2. Module path reflects ownership**
The path to an error type reflects who defines and owns its semantics. `pari::substrate::repo::codec::error::MalformedFrontmatter` — the `codec` component within the `repo` substrate implementation owns this error. No extra annotation needed to understand who is responsible.

**3. Three mandatory layers, variable middle**
Every error chain has exactly three mandatory layers: Job → Activity → Primitive. Between Job and Activity there may be zero or more Intermediary Op layers, depending on how many internal operations were involved. The chain is never longer than it needs to be.

**4. Properties declared once, propagated automatically**
Classification properties (`fix`, `recoverability`) are declared at the Activity layer — the layer that contextualises the failure. Intermediary and Job layers delegate automatically. Nothing is re-declared at upper layers.

**5. Severity is derived, not declared**
Severity follows deterministically from `FixDomain` and `Recoverability`. It is never an annotation. This prevents mismatches between declared severity and actual semantics.

**6. Composition and observability are separate concerns**
`#[derive(ErrorCompose)]` handles structural concerns: property propagation, delegation, downcasting. `#[derive(OTelEmit)]` handles observability: emitting structured events via OTel semantic conventions. They are separate derives with separate annotations, though `OTelEmit` reads from `ErrorCompose`-generated methods.

**7. SpanTrace and Backtrace are captured once, at origin**
Captured at the Primitive layer — the earliest point where a concrete failure is known. Never re-captured at higher layers. Propagated through the `source()` chain.

**8. Correlation IDs are not embedded in errors**
Correlation flows through the active tracing span context, not through error structs. OTel subscribers inject `trace_id` and `span_id` into emitted records automatically.

**9. Channel failure is explicit, never unwrapped**
When a public API operation depends on the `EntityServer` channel, channel failure is surfaced as `StoreError::Unavailable` and then mapped into the operation-specific `StoreUnavailable(...)` variant. Message transport failure is never handled with panic or `unwrap()` at the public boundary.

---

## The Error Layer Model

Every error chain in Pari follows this structure:

```
┌──────────────────────────────────────────────────────────────────────┐
│  JOB LAYER                                            (mandatory)   │
│                                                                      │
│  What the client asked Pari to do.                                   │
│  Single top-level enum: PariError.                                   │
│  Exposes: recoverability(), fix_domain(), severity(), emit()         │
│  Module: pari::error                                                 │
└─────────────────────────────┬────────────────────────────────────────┘
                              │ #[source]
                              ▼
┌──────────────────────────────────────────────────────────────────────┐
│  INTERMEDIARY OP LAYER(S)                              (optional)    │
│                                                                      │
│  What Pari was internally doing to serve the job.                    │
│  Zero or more layers, depending on operations involved.              │
│  Delegates all classification properties — adds operation context.   │
│  Module: <owning-module>::error::                                    │
└─────────────────────────────┬────────────────────────────────────────┘
                              │ #[source]
                              ▼
┌──────────────────────────────────────────────────────────────────────┐
│  ACTIVITY LAYER                                       (mandatory)    │
│                                                                      │
│  What a system component was doing when failure was encountered.     │
│  Outcome-based name: what went wrong / what state was left.          │
│  Declares: fix domain, recoverability (via #[compose(...)]).         │
│  Carries: hint — corrective guidance for the caller or operator.     │
│  Module: <owning-module>::error::                                    │
└─────────────────────────────┬────────────────────────────────────────┘
                              │ #[source]
                              ▼
┌──────────────────────────────────────────────────────────────────────┐
│  PRIMITIVE LAYER                                      (mandatory)    │
│                                                                      │
│  The most specific, atomic failure.                                  │
│  Carries: component — which leaf component produced the failure.     │
│  Carries: SpanTrace + Backtrace — captured at construction.          │
│  Carries: error-specific structured fields (path, line, id, etc.)   │
│  Module: <owning-module>::<component>::error::                       │
└──────────────────────────────────────────────────────────────────────┘
```

### Activity vs Intermediary Op — behavioral distinction

Both Activity and Intermediary Op errors live at `<owning-module>::error::`. The distinction is **behavioral, not positional**:

| | Activity Layer | Intermediary Op Layer |
|---|---|---|
| Declares `fix` + `recoverability` | Yes — `#[compose(fix=..., recoverability=...)]` | No |
| Carries `hint` | Yes | No |
| Wraps | A Primitive error | Another error (Activity or Intermediary Op) |
| Compose annotation | Declares | `#[compose(delegate)]` |

### Chain depth varies

The chain is only as deep as what actually ran. A simple input validation failure has no internal operations — its chain is the minimum three layers. A persist operation failing mid-swap may traverse five layers.

```
Minimum (no internal ops):    Job → Activity → Primitive
Deeper (internal ops involved): Job → [Op] → ... → [Op] → Activity → Primitive
```

---

## Module Path and Naming Conventions

### Path pattern per layer

| Layer | Module path pattern | Naming style |
|---|---|---|
| Job | `pari::error::` | Outcome / client operation |
| Intermediary Op | `<owner>::error::` | Outcome / internal operation |
| Activity | `<owner>::error::` | Outcome — what went wrong / what state |
| Primitive | `<owner>::<component>::error::` | Specific failure, technical naming ok |

The `<component>` sub-module appears **only at the Primitive layer** — that is where ownership is granular enough to belong to a specific component (e.g., codec, executor, validator). Activity and Intermediary Op errors are owned by the module as a whole.

### Concrete path examples

```
pari::error::PariError                                    ← job layer
pari::store::error::ExpansionFailed                       ← intermediary op
pari::substrate::error::LoadFailed                        ← intermediary op
pari::substrate::repo::error::BadDefinitionError          ← activity
pari::substrate::repo::error::CorruptPersistenceState     ← activity
pari::substrate::repo::codec::error::MalformedFrontmatter ← primitive, codec component
pari::substrate::repo::executor::error::RenameFailed      ← primitive, executor component
pari::validator::error::SchemaViolated                    ← activity (different owner)
pari::validator::id::error::InvalidIdentifierFormat       ← primitive, id-validator component
```

---

## Classification Dimensions

Every error in Pari carries three classification properties. Declared once at the Activity layer, propagated automatically up through composition.

### FixDomain — where the fix lives

Describes which domain owns the resolution. Not accusatory — describes where someone needs to act.

```rust
pub enum FixDomain {
    Client,  // fix is in the caller's input or usage
    Data,    // fix requires repairing stored content (corrupt or malformed)
    Infra,   // fix is in the underlying infrastructure (permissions, disk, network)
    Pari,    // fix is in Pari's code (invariant violated, logic bug)
}
```

### Recoverability — what the caller should do

```rust
pub enum Recoverability {
    Retryable,        // transient failure — retry automatically after backoff
    UserAction,       // caller must fix their input or definition, then retry
    OperatorAction,   // operator must fix infrastructure or data, then retry
    NotRecoverable,   // code invariant violated — do not retry, escalate to developer
}
```

### Severity — derived, never declared

Severity follows deterministically from `FixDomain` and `Recoverability`. No annotation needed.

```
FixDomain   + Recoverability        → Severity
──────────────────────────────────────────────
Pari        + NotRecoverable        → Error
Data        + OperatorAction        → Error
Infra       + OperatorAction        → Error
Infra       + Retryable             → Warn
Client      + UserAction            → Warn
```

Computed by `#[derive(ErrorCompose)]` — no annotation required.

---

## Information Carried by Errors

### At every layer

| Dimension | Mechanism |
|---|---|
| Developer-facing detail | `Debug` via `#[derive(thiserror::Error)]` |
| User-facing message | `Display` via `#[derive(thiserror::Error)]` |
| Cause chain | `#[source]` on fields — traversable via `std::error::Error::source()` |
| Structured fields | Typed fields on each error struct/variant |
| Fix domain | Generated by `ErrorCompose` — `fix_domain() -> FixDomain` |
| Recoverability | Generated by `ErrorCompose` — `recoverability() -> Recoverability` |
| Severity | Generated by `ErrorCompose` — `severity() -> Severity` (derived) |

### At the Activity layer

| Dimension | Field |
|---|---|
| Corrective guidance | `hint: Option<String>` |

### At the Primitive layer

| Dimension | Field / Mechanism |
|---|---|
| Which component produced this failure | `component: <ComponentEnum>` (always present) |
| Execution context at failure | `span_trace: SpanTrace` — captured at construction |
| Code location at failure | `backtrace: Backtrace` — captured at construction |
| Error-specific structured fields | Typed fields per error — defined during implementation |

### Not in errors

| Dimension | Where it lives instead |
|---|---|
| Correlation ID / trace ID | Active tracing span — injected by OTel subscriber into log records |
| Error codes | Variant names are the stable identifiers (`PariError::NotFound`, etc.) |

---

## `#[derive(ErrorCompose)]` — Composition and Property Propagation

`ErrorCompose` is a proc macro derive that handles all structural error concerns. It generates property accessor methods and wires downcasting. Observability is not its concern.

### What it generates

On every type it is applied to:
- `recoverability() -> Recoverability`
- `fix_domain() -> FixDomain`
- `severity() -> Severity` (derived — no annotation needed)
- `as_error<E: 'static>() -> Option<&E>` — downcast through composition chain

### At the Activity layer — declare

The Activity layer owns the classification. Declare `fix` and `recoverability` here:

```rust
#[derive(Error, Debug, ErrorCompose, OTelEmit)]
#[compose(fix = Data, recoverability = OperatorAction)]
#[otel(error_type = "bad_definition")]
pub struct BadDefinitionError {
    pub hint: Option<String>,
    #[source]
    #[otel(delegate)]    // OTel cascade to primitive
    pub cause: MalformedFrontmatterError,
}

#[derive(Error, Debug, ErrorCompose, OTelEmit)]
#[compose(fix = Infra, recoverability = OperatorAction)]
#[otel(error_type = "corrupt_persistence_state")]
pub struct CorruptPersistenceState {
    pub hint: Option<String>,
    #[source]
    #[otel(delegate)]
    pub cause: RenameFailed,
}
```

### At the Intermediary Op and Job layers — delegate

Upper layers do not re-declare properties. They delegate to their inner error:

```rust
// Intermediary op layer
#[derive(Error, Debug, ErrorCompose, OTelEmit)]
pub enum ExpansionFailed {
    #[error(transparent)]
    #[compose(delegate)]
    #[otel(delegate)]
    LoadFailed(LoadError),
}

// Job layer — mix of delegating and directly-declared variants
#[derive(Error, Debug, ErrorCompose, OTelEmit)]
pub enum PariError {
    // Delegates — properties come from the inner type
    #[error(transparent)]
    #[compose(delegate)]
    #[otel(delegate)]
    DefinitionRejected(DefineError),

    #[error(transparent)]
    #[compose(delegate)]
    #[otel(delegate)]
    SaveFailed(SaveError),

    // Declares directly — true leaf at job layer, no inner composed error
    #[error("invariant violated: {message}")]
    #[otel(error_type = "invariant_violated")]
    #[compose(fix = Pari, recoverability = NotRecoverable)]
    InvariantViolated { message: String },
}
```

### Key rule

```
Activity layer  →  #[compose(fix = ..., recoverability = ...)]   declares
Intermediary Op →  #[compose(delegate)]                           delegates
Job layer       →  #[compose(delegate)] or declares if true leaf
```

---

## `as_error<E>()` — Downcasting Through the Chain

When a caller has a `PariError` and needs to inspect a concrete error type deep in the chain, `as_error<E>()` traverses the `source()` chain to find it.

### How it works internally

The mechanism uses a sealed supertrait to make `Any`-based downcasting invisible to both implementors and callers:

```rust
// Internal — not part of the public API
mod sealed {
    pub trait AsAny: 'static {
        fn as_any(&self) -> &dyn std::any::Any;
    }
    // Blanket impl — every concrete type gets this automatically
    impl<T: 'static> AsAny for T {
        fn as_any(&self) -> &dyn std::any::Any { self }
    }
}
```

`ErrorCompose` has `sealed::AsAny` as a supertrait. Every type that derives `ErrorCompose` automatically satisfies `AsAny` via the blanket impl — without writing any code. The public `as_error<E>()` method lives on `dyn ErrorCompose`:

```rust
impl dyn ErrorCompose {
    pub fn as_error<E: 'static>(&self) -> Option<&E> {
        self.as_any().downcast_ref::<E>()
    }
}
```

### Implementors see nothing

No extra code required. The macro and blanket impl handle everything.

### Caller usage

```rust
// Inspect a specific primitive type deep in the chain:
if let Some(rename_err) = err.as_error::<RenameFailed>() {
    eprintln!("rename failed: {} → {}", rename_err.from, rename_err.to);
}
```

---

## `#[derive(OTelEmit)]` — Structured Observability

`OTelEmit` handles OTel event emission only. It reads from `ErrorCompose`-generated methods. It has no concern with composition or property propagation.

### The trait

```rust
pub trait OTelEmit {
    fn emit(&self);
}
```

### Cascade down the source chain

`emit()` called at the job layer cascades down the entire `source()` chain. Every layer emits its own structured fields. All fields from all layers land in one OTel event at the call site.

```
PariError::emit()
  → emits job-layer fields
  → source().emit()                          ← Intermediary Op (if any)
       → emits op-level fields
       → source().emit()                     ← Activity layer
            → emits hint
            → source().emit()                ← Primitive layer
                 → emits component, path, line, span_trace, backtrace
```

### Annotation syntax

```rust
// Activity layer — declares compose, uses otel(delegate) for cascade
#[derive(Error, Debug, ErrorCompose, OTelEmit)]
#[compose(fix = Data, recoverability = OperatorAction)]
#[otel(error_type = "bad_definition")]
pub struct BadDefinitionError {
    #[otel(field = "error.hint")]
    pub hint: Option<String>,
    #[source]
    #[otel(delegate)]            // cascades emit() to primitive
    pub cause: MalformedFrontmatterError,
}

// Primitive layer — component, SpanTrace, Backtrace, specific fields
#[derive(Error, Debug, ErrorCompose, OTelEmit)]
#[compose(fix = Data, recoverability = OperatorAction)]
#[otel(error_type = "malformed_frontmatter")]
pub struct MalformedFrontmatterError {
    #[otel(field = "error.component")]
    pub component: CodecComponent,
    #[otel(field = "file.path")]
    pub path: String,
    #[otel(field = "file.line")]
    pub line: usize,
    pub span_trace: SpanTrace,   // auto-included by macro in emit()
    pub backtrace: Backtrace,    // auto-included by macro in emit()
}
```

### What the macro generates (illustrative)

For a static variant, the macro generates a `tracing::warn!` or `tracing::error!` call with compile-time field names, using `opentelemetry_semantic_conventions` constants:

```rust
// Generated for MalformedFrontmatterError::emit():
tracing::error!(
    { otel::EXCEPTION_TYPE }       = "malformed_frontmatter",
    { otel::EXCEPTION_MESSAGE }    = %self,
    { otel::EXCEPTION_STACKTRACE } = %self.backtrace,
    span_trace                     = %self.span_trace,
    error.component                = %self.component,
    file.path                      = %self.path,
    file.line                      = self.line,
);
```

For an `#[otel(delegate)]` field, the macro generates a call to the inner type's `emit()`:

```rust
// Generated for BadDefinitionError::emit():
tracing::error!(
    { otel::EXCEPTION_TYPE }    = "bad_definition",
    { otel::EXCEPTION_MESSAGE } = %self,
    error.hint                  = ?self.hint,
);
self.cause.emit();    // cascade to primitive
```

All field names follow OTel semantic conventions from the `opentelemetry_semantic_conventions` crate. No free-form field naming.

### OTel vs non-OTel paths

```
OTel path:
  err.emit() called at call site
  → structured fields emitted into current OTel span
  → OTel subscriber attaches trace_id / span_id automatically
  → full distributed trace context available to Datadog, Jaeger, etc.

Non-OTel path (structured logging):
  client receives PariError
  → calls ExtractSpanTrace::span_trace(&err)
  → walks source() chain → finds SpanTrace in primitive error
  → logs span context alongside the error message
```

`SpanTrace` primarily serves the non-OTel path. OTel clients get richer context from the active span itself.

---

## SpanTrace and Backtrace

Both are captured at the **Primitive layer only**, at construction time. Never re-captured at higher layers.

```rust
// Primitive layer constructor — always capture both
impl MalformedFrontmatterError {
    pub fn new(component: CodecComponent, path: String, line: usize) -> Self {
        Self {
            component,
            path,
            line,
            span_trace: SpanTrace::capture(),   // tracing_error
            backtrace:  Backtrace::capture(),    // std::backtrace
        }
    }
}
```

`ExtractSpanTrace` (from `tracing_error`) walks the `source()` chain to find the `SpanTrace` embedded in the primitive error. Callers at the job layer do not need to know at which layer it was captured.

---

## Batch Errors

When a single operation produces multiple failures (e.g. persist failing on several entities), the collection is wrapped in `BatchError<E>`. This is a first-class error type that sits as an Intermediary Op node in the `source()` chain.

```rust
pub struct BatchError<E: ErrorCompose> {
    pub errors: Vec<E>,
}
```

### Property aggregation

`BatchError<E>` derives `ErrorCompose` and aggregates across its inner errors:
- `recoverability()` — worst case across the batch
- `fix_domain()` — worst case across the batch

### SpanTrace handling

`BatchError` handles `SpanTrace` itself — it does not propagate to individual inner errors via `source()`. It captures a summary span trace at construction and surfaces that via `ExtractSpanTrace`. Callers needing individual primitive span traces iterate `.errors` directly.

This is deliberate: defaulting `ExtractSpanTrace` to the first inner error would silently misrepresent the batch.

### Structure

```
PariError::SaveFailed                     (job)
  └─ BatchError<PersistActivityError>     (intermediary op — wraps collection)
       │  recoverability: worst-case across batch
       │  SpanTrace: aggregated summary
       .errors: [
           PersistActivityError           (activity — individually source()-traversable)
             └─ RenameFailed             (primitive)
           PersistActivityError
             └─ RenameFailed
       ]
```

---

## Example Error Chains

These are illustrative. Actual type names and fields are determined during implementation.

### Shallow — input validation, no internal ops

The minimum three-layer chain. No internal operations were triggered.

```
PariError::DefinitionRejected                            (job)
  └─ SchemaViolated                                      (activity)
       │  hint: "workflow id must be CamelCase, got 'my-workflow'"
       │  fix: Client, recoverability: UserAction
       └─ pari::validator::id::error::InvalidIdentifierFormat  (primitive)
            │  component: IdentifierValidator
            │  provided: "my-workflow"
            │  expected_pattern: "CamelCase"
            SpanTrace + Backtrace captured here
```

### Medium — internal op involved, entity absent

```
PariError::DefinitionRejected                            (job)
  └─ CrossReferenceCheckFailed                           (intermediary op)
       └─ ReferencedEntityAbsent                         (activity)
            │  hint: "ensure 'eng-lead' role exists before referencing it"
            │  fix: Client, recoverability: UserAction
            └─ pari::resolver::entity::error::NotFound   (primitive)
                 │  component: EntityResolver
                 │  entity_ref: "roles/eng-lead"
                 SpanTrace + Backtrace captured here
```

### Deep — internal ops, corrupt file encountered

```
PariError::UpdateFailed                                           (job)
  └─ pari::store::error::ExpansionFailed                         (intermediary op)
       └─ pari::substrate::error::LoadFailed                     (intermediary op)
            └─ pari::substrate::repo::error::BadDefinitionError  (activity)
                 │  hint: "check YAML frontmatter at line 4"
                 │  fix: Data, recoverability: OperatorAction
                 └─ pari::substrate::repo::codec::error::MalformedFrontmatter  (primitive)
                      │  component: FrontmatterCodec
                      │  path: "roles/eng-lead.md"
                      │  line: 4
                      SpanTrace + Backtrace captured here
```

### Deep — persist fails mid atomic swap

```
PariError::SaveFailed                                                  (job)
  └─ pari::store::error::PersistFailed                                (intermediary op)
       └─ pari::substrate::error::PersistFailed                       (intermediary op)
            └─ pari::substrate::repo::error::CorruptPersistenceState  (activity)
                 │  hint: "stale .part/ dir may exist — safe to remove"
                 │  fix: Infra, recoverability: OperatorAction
                 └─ pari::substrate::repo::executor::error::RenameFailed  (primitive)
                      │  component: AtomicSwapExecutor
                      │  from: "workflows/Initiative/.part/..."
                      │  to:   "workflows/Initiative/..."
                      └─ std::io::Error
                           SpanTrace + Backtrace captured here
```

### Batch persist failure

```
PariError::SaveFailed                         (job)
  └─ BatchError<PersistActivityError>         (intermediary op — wraps collection)
       .errors: [
         PersistActivityError                 (activity)
           └─ RenameFailed                   (primitive)
         PersistActivityError                 (activity)
           └─ RenameFailed                   (primitive)
       ]
```

---

## Client Usage

```rust
// Programmatic handling based on recoverability
match err.recoverability() {
    Recoverability::Retryable       => retry_with_backoff(op),
    Recoverability::UserAction      => return Err(err.to_string()),
    Recoverability::OperatorAction  => alert_oncall(&err),
    Recoverability::NotRecoverable  => panic!("pari invariant: {err}"),
}

// Structured OTel event — emits all fields from all layers in one call
err.emit();

// Downcast to a specific concrete error type if needed
if let Some(rename_err) = err.as_error::<RenameFailed>() {
    // inspect rename_err.component, rename_err.from, rename_err.to
}

// Non-OTel: extract span trace for structured logging
if let Some(span_trace) = ExtractSpanTrace::span_trace(&err) {
    log::error!("{err}\nSpan trace:\n{span_trace}");
}
```

---

## Quick Reference

| Concern | Mechanism |
|---|---|
| Debug + Display | `#[derive(thiserror::Error)]` — all layers |
| Cause chain | `#[source]` + `source()` traversal — all layers |
| Fix domain | `FixDomain` enum — declared via `#[compose(fix = ...)]` at Activity layer |
| Recoverability | `Recoverability` enum — declared via `#[compose(recoverability = ...)]` at Activity layer |
| Severity | Derived from FixDomain + Recoverability — no annotation |
| Property propagation | `#[derive(ErrorCompose)]` — `#[compose(delegate)]` at Intermediary Op and Job layers |
| Downcasting | `as_error<E>()` — generated by `ErrorCompose`, sealed supertrait internally |
| OTel emission | `#[derive(OTelEmit)]` — cascades down `source()` chain via `#[otel(delegate)]` |
| OTel field naming | `opentelemetry_semantic_conventions` crate — no free-form field names |
| Corrective hint | `hint: Option<String>` — Activity layer only |
| Component identity | `component` field — Primitive layer only (always present) |
| Execution context | `SpanTrace` — Primitive layer construction |
| Code location | `Backtrace` — Primitive layer construction |
| Correlation ID | Not in error — flows via active tracing span, injected by OTel subscriber |
| Stable error codes | Variant names (`PariError::DefinitionRejected`, etc.) |
| Batch failures | `BatchError<E>` — Intermediary Op node, aggregates recoverability + fix domain, owns SpanTrace |
| Primitive structured fields | Typed fields per error — defined during implementation |
