# validate-shared

**Validation → `validation/`**

---

## Purpose

Establishes the `ValidationSchema` structure used by every entity kind and the shared primitive rules that entity schemas reference.

---

## ValidationSchema

Each entity kind declares a static `ValidationSchema` — three maps from field name to a list of rules for that field:

```
ValidationSchema {
    structural:    { field → [StructuralRule] }
    semantic:      { field → [SemanticRule] }
    cross_entity:  { field → [CrossEntityRule] }
}
```

Rules for a field are listed in the map only if that field has rules of that kind. A field absent from a map has no rules of that kind.

---

## Rule Return Type

Rules are atomic — they validate their scope and return their own result. They have no knowledge of `ValidationErrors`, `FieldValidationError`, or `ValidationKind`. The runner owns all aggregation.

```rust
struct RuleViolation {
    sub_path: Option<String>,  // None = the field itself; Some("[0].role") = sub-field path
    message: String,
}
```

## Rule Function Signatures

```rust
// Structural — sync; receives field value only; no entity or store context
type StructuralRule<T> = fn(value: &T) -> Vec<RuleViolation>;

// Semantic — async; receives the tracked entity; sibling fields load transparently
type SemanticRule<T>   = async fn(entity: &T) -> Vec<RuleViolation>;

// CrossEntity — async; receives the tracked entity; may query store via EntityServer::sender()
type CrossEntityRule<T> = async fn(entity: &T) -> Vec<RuleViolation>;
```

`T` is the tracked entity type (e.g. `TrackedRole`). Validators are tracked-entity-specific — plain entities are valid by construction and do not pass through runtime validators.

Each rule is specific to one field and constructs `sub_path` relative to that field. For a simple field violation, `sub_path` is `None`. For a nested violation (e.g. checking `members[0].role`), `sub_path` is `Some("[0].role")`.

---

## Runner

A single async runner. It selects rules from the schema by field name and requested kinds, executes them, and aggregates `RuleViolation`s into `ValidationErrors`. It owns path construction (`"{field_name}{sub_path}"`) and kind labelling.

```rust
async fn run_validations<T: Entity>(
    entity: &T::Tracked,
    fields: &[&str],          // &[] = all fields in the schema
    kinds: &[ValidationKind], // which kinds to run
) -> ValidationErrors
```

The runner reads `T::VALIDATION_SCHEMA` to resolve the schema. Semantic and cross-entity rules receive the tracked entity directly — fields not yet loaded are transparently fetched on first access.

Callers compose exactly the kinds they need:

| Trigger | `kinds` | `fields` |
|---|---|---|
| External API | `[Structural]` | requested fields |
| Setter | `[Structural, Semantic]` | the field being set |
| Load path | `[Structural, Semantic, CrossEntity]` | newly loaded fields |
| Check-in (new) | `[Structural, Semantic, CrossEntity]` | `&[]` (all) |
| Check-in (modified) | `[CrossEntity]` | dirty fields |

---

## Shared Primitive Rules

Utility functions used internally by rule implementations. They return `Vec<RuleViolation>` — callers embed the results (adjusting `sub_path` as needed) into their own return value.

### Structural primitives

```rust
fn kebab_case(value: &str)        -> Vec<RuleViolation>  // [a-z0-9]+(-[a-z0-9]+)*
fn camel_case(value: &str)        -> Vec<RuleViolation>  // [A-Z][a-zA-Z0-9]*
fn kebab_case_id<T: Entity>(entity_ref: &EntityRef<T>) -> Vec<RuleViolation>  // extracts id, delegates to kebab_case
fn camel_case_id<T: Entity>(entity_ref: &EntityRef<T>) -> Vec<RuleViolation>  // extracts id, delegates to camel_case
fn non_empty_str(value: &str)     -> Vec<RuleViolation>  // not empty or whitespace-only
fn non_empty_list<T>(value: &[T]) -> Vec<RuleViolation>  // at least one element
fn min_length<T>(value: &[T], min: usize) -> Vec<RuleViolation>
fn unique_by<T, K: Eq + Hash>(value: &[T], key_fn: fn(&T) -> K) -> Vec<RuleViolation>
fn x_prefix_keys(value: &Extensions) -> Vec<RuleViolation>  // all keys start with "x-"
fn states_valid<S: StateEntry>(value: &[S]) -> Vec<RuleViolation>
// CamelCase ids; unique ids; min 2; at least one Done; at least one non-Done
```

### Raci primitives

Used by Task, Relay, and Workflow validators for the `raci` field.

```rust
fn raci_structural(value: &Raci) -> Vec<RuleViolation>
// responsible must be non-empty

async fn raci_roles_exist(raci: &Raci) -> Vec<RuleViolation>
// checks all role refs in responsible, accountable, consulted, informed via ref_exists
// sub_path set to the nested field path (e.g. "responsible[0]") for each missing ref
```

---

### Cross-entity primitives

These are building blocks called internally by named cross-entity rules in entity validator modules. They do not appear directly in `ValidationSchema` maps.

```rust
async fn ref_exists<T: Entity>(entity_ref: &EntityRef<T>) -> Vec<RuleViolation>
// entity must exist in store via has_ref

async fn all_refs_exist<T: Entity>(refs: &[EntityRef<T>]) -> Vec<RuleViolation>
// each ref must exist; sub_path set to "[{i}]" for each missing ref

async fn hook_call_inputs_valid(hook_call: &HookCall) -> Vec<RuleViolation>
// loads the referenced hook; checks every key in `with` matches a declared HookInput.name
// (no unknown keys) and every declared input has a binding (no missing keys);
// sub_path set to "with.{key}" for each violation
```

More complex cross-entity rules (cycle detection, state map validation) are defined in their respective entity validator modules.
