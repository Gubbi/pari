## Context

`schemas/*.json` and `src/schema/` Rust types are currently maintained in parallel with no automated link. Both suites of tests (JSON schema tests and Rust unit tests) exercise the same domain independently. Any change to one side can silently diverge from the other.

The fix is to make Rust types the single source of truth and generate JSON schemas from them via `schemars`.

## Goals / Non-Goals

**Goals:**
- Rust types encode all structural constraints (field presence, id patterns, minItems)
- JSON schemas are generated artifacts — always in sync with types by construction
- Validation layer handles only what types cannot express (referential integrity, cross-field semantics)
- CI enforces committed schemas are not stale

**Non-Goals:**
- Parser implementation — deferred; `RepoContext` stub unchanged
- Changing validation function signatures
- Changing the shape of `ValidationError` or `RepoContext`

## Decisions

**serde + schemars as production dependencies**

`#[derive(JsonSchema, Serialize, Deserialize)]` proc-macros must be co-located with type definitions in production code. A `#[cfg_attr(test, derive(...))]` workaround also requires every constrained field to carry a conditional `#[cfg_attr(test, schemars(...))]` annotation — the type definitions become hard to read. Since serde is zero-cost when unused and effectively a standard Rust dependency, accepting it as a production dep is the right call.

*Alternatives considered:* dev-dependency only via `cfg_attr` — rejected due to annotation noise and schema generation being test-only.

**Field-level annotations over newtype wrappers**

`#[schemars(regex(pattern = r"..."))]` and `#[schemars(length(min = N))]` work directly on `String` and `Vec<T>` fields. Newtypes would require `.parse().unwrap()` at every test construction site, change the public API, and add boilerplate for each constrained type. Validation of patterns and lengths remains in the validator functions; the annotations serve schema generation only.

*Alternatives considered:* `KebabId`/`CamelId` newtypes — rejected; ergonomic cost outweighs the type-safety gain given the validator already enforces constraints.

**xtask crate for schema generation**

`build.rs` cannot import from the lib crate — build scripts are separate programs. An `xtask` crate (standard Rust pattern) provides a `generate-schemas` binary that imports the lib types, calls `schemars::schema_for!` for each entity, and writes to `schemas/*.json`. Run explicitly (`cargo xtask generate-schemas`) and in CI.

*Alternatives considered:* test-based generation — schemas only generated during `cargo test`, awkward to run standalone.

**CI diff check on committed schemas**

CI runs `cargo xtask generate-schemas` and diffs against committed `schemas/*.json`. Fails if stale. This is the enforcement mechanism — ensures developers who change types remember to regenerate.

**Remove `tests/json_schema_validation.rs`**

Once Rust types encode constraints via schemars, the JSON schema tests validate schemars itself — not Pari logic. Removing them eliminates a maintenance surface with no loss of coverage.

## Risks / Trade-offs

- **Wrong annotation string** (e.g., typo in regex pattern) → not caught at compile time. Mitigation: a schema coherence test generates schemas at test time and spot-checks key constraints (pattern present on id fields, minItems on constrained arrays).
- **schemars version upgrade changes generated output** → pin schemars version; review generated diff on upgrade as part of the PR.
- **Oneof types** (`Step`, `HookInvocation`) need `#[serde(untagged)]` to serialize correctly for schemars to generate the right `oneOf` shape. Must verify generated output for these types.
