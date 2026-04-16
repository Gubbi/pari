# primitive-errors

**Cross-Cutting**

---

## Purpose

This document defines the intended design for Pari's Primitive Errors.

It refines the Primitive layer described in [error-handling](error-handling.md) and focuses on one goal:

- make all Primitive Errors uniform without forcing every Primitive Error struct to manually repeat the same boilerplate fields, constructor logic, trait derives, and observability setup

---

## Design Direction

Primitive Errors should not rely on a hand-written per-error trait implementation.

Instead, Pari should provide a single derive-driven macro contract, along the lines of:

```rust
#[derive(PariError)]
#[error_layer = "primitive"]
```

The exact macro name and attribute syntax can evolve, but the design intent is:

- one declarative entry point for Primitive Errors
- common diagnostic fields provided automatically
- constructor behavior provided automatically
- `thiserror` / `Display` support standardized
- OTel behavior standardized

If `#[error_type = "..."]` is omitted, the macro should derive a default value by converting the Rust type name from `CamelCase` to `snake_case`.

Examples:

- `MalformedFrontmatter` -> `malformed_frontmatter`
- `RenameFailed` -> `rename_failed`

`#[error_type = "..."]` remains available as an override when a specific emitted value is desired.

---

## Mandatory Common Diagnostics

Every Primitive Error must carry the same fixed set of diagnostic details:

- `message`
- `location`
- `span_trace`
- `backtrace`

These are not optional design choices per Primitive Error. They are part of the Primitive layer contract.

### Meaning of each diagnostic

- `message`
  Human-readable explanation of the concrete leaf failure.
- `location`
  The most relevant concrete location for the failure.
  By default this is the error creation site captured by the generated constructor.
- `span_trace`
  Tracing context captured at the point the Primitive Error is created.
- `backtrace`
  Backtrace captured at the point the Primitive Error is created.

---

## Constructor Contract

Primitive Errors should not manually declare and populate these common diagnostic fields in each error struct constructor.

The derive-driven Primitive Error contract should provide an auto-capturing constructor pattern so that:

- `span_trace` is always captured at construction time
- `backtrace` is always captured at construction time
- `location` is auto-captured through a standardized constructor contract
- `message` is accepted through a standardized constructor contract
- the error author only supplies the error-specific details for that Primitive Error

Illustrative direction:

```rust
#[derive(PariError)]
#[error_layer = "primitive"]
#[error_type = "malformed_frontmatter"]
pub struct MalformedFrontmatter {
    pub line: usize,
    pub raw_snippet: String,
}
```

And then the macro-generated constructor shape would conceptually behave like:

```rust
impl MalformedFrontmatter {
    pub fn new(message: impl Into<String>, line: usize, raw_snippet: String) -> Self {
        // message stored
        // location auto-captured
        // span_trace auto-captured
        // backtrace auto-captured
        // detail fields stored
    }
}
```

The exact constructor signature may vary by implementation, but the capture semantics should be fixed.

### Auto-captured location

The intended default is that `location` is auto-captured, not manually passed for every Primitive Error construction.

That auto-captured location represents the error creation site by default.

### Location capture mechanics

The generated constructor captures `location` directly using a standardized caller-location mechanism such as `#[track_caller]` with `std::panic::Location::caller()`.

### Location override

Override is supported for the cases where the creation site is not the most useful domain location.

The design is:

- default constructor: auto-captures location
- explicit override constructor: accepts a caller-supplied location

Illustrative shape:

```rust
impl MalformedFrontmatter {
    pub fn new(message: impl Into<String>, line: usize, raw_snippet: String) -> Self;
    pub fn new_with_location(
        location: ErrorLocation,
        message: impl Into<String>,
        line: usize,
        raw_snippet: String,
    ) -> Self;
}
```

Creation-site location means the source-code location where the Primitive Error is constructed.

Examples where override is useful:

- a parser detects an error while iterating through a decoded document and wants the location to point to the offending document line and column, not the helper function that creates the error
- a repository loader detects a problem in a referenced asset and wants the location to point to that asset path rather than the generic loader call site
- a validator builds a Primitive Error from precomputed source metadata and wants to preserve that source metadata as the error location

Override should be done through `new_with_location(...)`, not by directly setting a managed common field.

---

## What Primitive Error Authors Should Declare

When defining a Primitive Error, authors should only need to declare:

- the error-specific detail fields
- the error type / observability annotation data
- optionally the desired display template if the macro does not infer one

They should not need to repeatedly declare:

- `message`
- `location`
- `span_trace`
- `backtrace`
- standard derive stack for error formatting and emission

### Primitive detail fields

Fields like `line` and `raw_snippet` are the Primitive Error's structured detail fields.

They are:

- not part of the fixed common diagnostics
- specific to that Primitive Error type
- expected to be available to both tracing and error logging through the same derive-driven emission path

They should be treated as structured detail fields by default unless explicitly marked otherwise.

---

## Boilerplate The Macro Should Standardize

For Primitive Errors, the catch-all derive should standardize all of the following:

- `thiserror::Error`
- `Display`
- common diagnostic storage
- constructor-time location capture
- constructor-time `SpanTrace` capture
- constructor-time `Backtrace` capture
- OTel integration
- generated helper/accessor methods used by higher layers and emitters

### Required generated helpers / accessors

The Primitive Error derive should generate the following helpers/accessors:

- `fn error_layer(&self) -> ErrorLayer`
  Always returns `ErrorLayer::Primitive`.
- `fn error_type(&self) -> &'static str`
  Returns the explicit `error_type` or the default snake_case type-name-derived value.
- `fn message(&self) -> &str`
- `fn location(&self) -> &ErrorLocation`
- `fn span_trace(&self) -> &SpanTrace`
- `fn backtrace(&self) -> &Backtrace`
- `fn details(&self) -> &[PrimitiveDetail]`
  Exposes the primitive-specific structured detail fields in emitted form.
- `fn emit(&self)`
  Provided through the standardized `OTelEmit` integration.

### Required generated constructors

The Primitive Error derive should generate:

- `fn new(...) -> Self`
  Uses auto-captured location.
- `fn new_with_location(location: ErrorLocation, ...) -> Self`
  Uses explicit override location.

---

## OTel / Observability Contract

Primitive Errors are the origin point for the most concrete diagnostic evidence in the chain.

The derive-driven Primitive Error contract should therefore also standardize observability requirements:

- Primitive Errors should carry a stable error type identifier for OTel emission.
- `message`, `location`, `span_trace`, and `backtrace` should be consistently available to the OTel emission path.
- error-specific detail fields should also be available for structured emission.
- Primitive Errors should not require hand-written `emit()` implementations in normal cases.

### How detail fields should flow into emission

Primitive detail fields such as `line` and `raw_snippet` are serialized into structured observability fields by the same derive-driven machinery.

### Field-name standardization

Field naming should be finalized as follows:

- Use `opentelemetry_semantic_conventions` wherever a standard field exists.
- For the common exception fields:
  - `error_type` -> OTel exception type
  - `message` -> OTel exception message
  - `backtrace` -> OTel exception stacktrace
- For captured source location:
  - `location.file` -> OTel code file path field
  - `location.line` -> OTel code line number field
  - `location.column` -> OTel code column number field when available
- For primitive-specific detail fields:
  - emit them under the namespaced pattern `error.<error_type>.<snake_case_field_name>`

Examples:

- `line` on `MalformedFrontmatter` -> `error.malformed_frontmatter.line`
- `raw_snippet` on `MalformedFrontmatter` -> `error.malformed_frontmatter.raw_snippet`
- `from` on `RenameFailed` -> `error.rename_failed.from`
- `to` on `RenameFailed` -> `error.rename_failed.to`

This keeps:

- standard fields mapped to OTel semantic conventions
- primitive-specific details stable and machine-readable
- emission deterministic across tracing and logging paths

### Composition with higher-layer fields

Error-specific structured fields are namespaced by concrete `error_type`, not by layer.

The composition rule is:

- standard OTel semantic-convention fields remain at their standard names
- shared semantic fields such as `error.component` and `error.hint` keep their shared names
- every error-specific structured field uses the namespace:
  - `error.<error_type>.<field_name>`

Examples in one emitted chain:

- shared field: `error.component`
- shared field: `error.hint`
- Primitive field: `error.rename_failed.from`
- Primitive field: `error.rename_failed.to`
- Activity field: `error.bad_definition.definition_kind`
- Intermediary field: `error.cross_reference_check_failed.entity_ref`

This keeps higher-layer context and lower-layer evidence from colliding while allowing multiple higher-layer errors to contribute their own structured fields in the same emitted chain.

---

## Relationship to Activity Errors

Primitive Errors carry the concrete leaf failure evidence.

They do **not** carry the higher-level contextual meaning of the subsystem activity. That belongs to the Activity layer.

So the split remains:

- Primitive Error
  Carries concrete failure diagnostics and primitive-specific detail fields.
- Activity Error
  Communicates the outcome, classification (`fix`, `recoverability`), component identity, and recovery hint.

This separation is important because it keeps the Primitive layer focused on leaf evidence while the Activity layer carries the semantic interpretation of that evidence.

---

## Primitive Error Shape

Conceptually, every Primitive Error instance should behave as if it has this shape:

```rust
struct PrimitiveErrorShape<D> {
    message: String,
    location: ErrorLocation,
    span_trace: SpanTrace,
    backtrace: Backtrace,
    details: D,
}
```

This is a conceptual shape, not necessarily the literal implementation shape.

The important part is:

- the common diagnostics are fixed and universal
- the detail payload varies by concrete Primitive Error type

---

## Example

Illustrative direction:

```rust
#[derive(PariError)]
#[error_layer = "primitive"]
#[error_display = "rename failed: {from} -> {to}: {message}"]
pub struct RenameFailed {
    pub from: String,
    pub to: String,
}
```

Conceptually, this definition should be enough for the macro to provide:

- `thiserror::Error`
- `Display`
- stored `message`
- auto-captured `location`
- captured `span_trace`
- captured `backtrace`
- structured OTel emission

without redefining those common fields inside the struct body.

---

## `emit()` Invocation

Primitive Errors should still derive whatever is needed for structured emission, but the normal call site for `emit()` is the top-level Job Error.

So the intended runtime model is:

- Primitive Errors participate in the OTel emission contract
- Activity and Intermediary errors delegate
- `emit()` is normally invoked at the Job layer
- the cascade then reaches the Primitive Error and includes its common diagnostics and detail fields

So yes: in normal usage, `emit()` is called at the Job layer, while Primitive Errors still need standardized emission support so they can contribute their fields during that cascade.

The mechanics are:

- every layer derives or implements `OTelEmit`
- Job-layer `emit()` starts the cascade
- each delegating layer forwards emission through its `source`
- the Primitive layer contributes:
  - OTel exception type
  - OTel exception message
  - OTel exception stacktrace
  - captured location fields
  - all primitive detail fields

This keeps the call site simple while preserving full leaf-level structured evidence.
