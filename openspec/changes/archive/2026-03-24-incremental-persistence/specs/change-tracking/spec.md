## ADDED Requirements

### Requirement: Tracked<T> provides transparent field-level change tracking
The system SHALL provide a `Tracked<T>` newtype in `src/tracked.rs` that wraps any value and tracks whether it has been mutated. `Tracked<T>` SHALL implement `Deref<Target = T>` for transparent read access and `DerefMut<Target = T>` that marks the value as dirty on every mutable borrow. A newly constructed `Tracked<T>` via `Tracked::new(value)` SHALL start in the dirty state.

#### Scenario: Read access through Deref is transparent
- **WHEN** a `Tracked<String>` holds the value `"hello"`
- **THEN** `*tracked == "hello"` and `tracked.len() == 5` (Deref delegates to inner String)

#### Scenario: Mutable access marks dirty
- **WHEN** a `Tracked<String>` has its dirty flag reset, then a mutable reference is obtained via `DerefMut`
- **THEN** `tracked.is_dirty()` returns `true`

#### Scenario: New tracked values start dirty
- **WHEN** `Tracked::new("hello".to_string())` is called
- **THEN** `tracked.is_dirty()` returns `true`

---

### Requirement: Tracked<T> supports dirty state inspection and reset
The system SHALL provide `is_dirty(&self) -> bool` to check the dirty state and `reset_dirty(&mut self)` to clear it. Resetting dirty does not change the inner value.

#### Scenario: Reset clears dirty flag
- **WHEN** a dirty `Tracked<String>` has `reset_dirty()` called
- **THEN** `is_dirty()` returns `false` and the inner value is unchanged

#### Scenario: Mutation after reset re-dirties
- **WHEN** a `Tracked<String>` is reset, then mutated via `DerefMut`
- **THEN** `is_dirty()` returns `true` again

---

### Requirement: TrackedMap provides collection-level change tracking
The system SHALL provide `TrackedMap<K, V>` in `src/tracked.rs`, backed by `IndexMap<K, V>`, that tracks inserted, modified, and removed keys. `TrackedMap` SHALL preserve insertion order. It SHALL expose:
- `insert(key, value)` — inserts or replaces; records the key in the `inserted` set, removes it from `removed` if present
- `remove(key)` — removes the entry; retains the full value in the `removed` map until `reset_tracked()` is called
- `get(key) -> Option<&V>` — read access
- `get_mut(key) -> Option<&mut V>` — mutable access; records the key in the `modified` set
- `iter_mut()` — mutable iteration in insertion order
- `values()` — iterates values in insertion order
- `keys()` — iterates keys in insertion order

`TrackedMap` maintains three sets:
- `inserted: IndexMap<K, ()>` — keys added via `insert()`
- `modified: IndexMap<K, ()>` — keys mutated via `get_mut()`
- `removed: IndexMap<K, V>` — keys removed via `remove()`, full value retained

`inserted` takes precedence over `modified` — a key inserted and then mutated before reset is `Added`, not `Modified`.

#### Scenario: Insert records key in inserted set
- **WHEN** a new key `"eng-lead"` is inserted into a `TrackedMap`
- **THEN** the key appears in the `inserted` set and `get("eng-lead")` returns the value

#### Scenario: Remove retains full value in removed map
- **WHEN** an existing key `"eng-lead"` is removed from a `TrackedMap`
- **THEN** `get("eng-lead")` returns `None` and the full value is retained in the `removed` map

#### Scenario: get_mut records key in modified set
- **WHEN** `get_mut("eng-lead")` is called on a `TrackedMap` whose `"eng-lead"` entry exists
- **THEN** the key appears in the `modified` set

#### Scenario: inserted takes precedence over modified
- **WHEN** a key is inserted and then `get_mut` is called on it before reset
- **THEN** the key appears in `inserted` only, not `modified`

#### Scenario: Insertion order is preserved
- **WHEN** keys `"a"`, `"b"`, `"c"` are inserted in that order
- **THEN** `keys()` yields `"a"`, `"b"`, `"c"` in that order

#### Scenario: shift_remove preserves order of remaining entries
- **WHEN** key `"b"` is removed from a TrackedMap with keys `["a", "b", "c"]`
- **THEN** `keys()` yields `"a"`, `"c"` in that order

---

### Requirement: TrackedMap supports read-only change inspection and explicit reset
The system SHALL provide `has_changes(&self) -> bool` on `TrackedMap` that returns true if any inserted, modified, or removed entries exist. Change state SHALL be readable via `&self` — no mutation required to inspect it.

The system SHALL provide `reset_tracked(&mut self)` on `TrackedMap` that clears the `inserted`, `modified`, and `removed` sets and drops retained removed values. This SHALL only be called after successful persistence.

#### Scenario: has_changes returns true when changes exist
- **WHEN** a `TrackedMap` has inserted keys `["a"]` and removed keys `["c"]`
- **THEN** `has_changes()` returns `true`

#### Scenario: reset_tracked clears all tracking state
- **WHEN** `reset_tracked()` is called on a `TrackedMap` with dirty and removed entries
- **THEN** `has_changes()` returns `false` and all removed values are dropped

---

### Requirement: TrackedMap::from_vec converts ordered plain collections
The system SHALL provide `TrackedMap::from_vec(items, convert_fn)` that takes a `Vec<S>`, converts each item to `V` via `convert_fn: Fn(S) -> V`, extracts a key from each converted item, and builds a `TrackedMap<K, V>` with entries in the original Vec order. All entries SHALL start in the `inserted` set.

The key extraction SHALL be handled by the `convert_fn` result type implementing a `HasId` (or equivalent) trait, OR via a separate `key_fn` argument — implementation to determine the cleaner API at code time.

#### Scenario: from_vec preserves order and marks entries as inserted
- **WHEN** `TrackedMap::from_vec(vec![step_a, step_b], TrackedStep::from)` is called
- **THEN** the TrackedMap contains both steps in `[a, b]` order, keyed by their ids, all in the `inserted` set

---

### Requirement: Derive macro generates tracked variants for structs, enums, and generic types
The system SHALL provide a `#[derive(Tracked)]` proc macro in the `pari-macros` crate. The macro handles three cases:

**Structs**: generates a tracked struct (`TrackedRole` from `Role`) with each field wrapped in `Tracked<T>`, a `From<Plain> for Tracked` impl, and a `dirty_fields() -> Vec<&'static str>` method that returns the names of all dirty fields.

**Enums**: generates a tracked enum (`TrackedWorkStepDefinition` from `WorkStepDefinition`) with `Tracked` prepended to each variant's inner type name, a `From<Plain> for Tracked` impl with match arms, and `dirty_fields()` delegating to the active variant's inner tracked type.

**Generic types**: preserves generic parameters and introduces a `TS: From<S>` bound. `WorkflowDef<S>` generates `TrackedWorkflowDef<TS>` with `impl<S, TS: From<S>> From<WorkflowDef<S>> for TrackedWorkflowDef<TS>`. Concrete type aliases are declared manually.

Fields annotated with `#[tracked(map_key = "id")]` on a `Vec<S>` field SHALL generate a `TrackedMap<String, TS>` field in the tracked variant, with conversion via `TrackedMap::from_vec(plain.steps, TS::from)` and a `S: HasId` bound for key extraction.

#### Scenario: Derive generates tracked struct with Tracked fields and dirty_fields method
- **WHEN** `#[derive(Tracked)]` is applied to a struct with fields `id: String` and `name: String`
- **THEN** a tracked struct is generated with `id: Tracked<String>`, `name: Tracked<String>`, and `dirty_fields()` returns `["name"]` when only `name` is dirty

#### Scenario: Derive generates From impl for struct
- **WHEN** a plain `Role` is converted via `TrackedRole::from(role)`
- **THEN** every field in the resulting `TrackedRole` is dirty and holds the original value

#### Scenario: Derive generates tracked enum with Tracked-prefixed variant types
- **WHEN** `#[derive(Tracked)]` is applied to an enum with variant `Task(Task)`
- **THEN** the tracked enum has variant `Task(TrackedTask)` and `From` impl converts via `TrackedTask::from(t)`

#### Scenario: Derive handles generic structs with TS: From<S> bound
- **WHEN** `#[derive(Tracked)]` is applied to `WorkflowDef<S>`
- **THEN** `TrackedWorkflowDef<TS>` is generated with `impl<S, TS: From<S>> From<WorkflowDef<S>> for TrackedWorkflowDef<TS>`

#### Scenario: Vec field with map_key annotation becomes TrackedMap
- **WHEN** a field `steps: Vec<S>` is annotated with `#[tracked(map_key = "id")]`
- **THEN** the tracked variant has `steps: TrackedMap<String, TS>` and conversion uses `TrackedMap::from_vec(plain.steps, TS::from)`
