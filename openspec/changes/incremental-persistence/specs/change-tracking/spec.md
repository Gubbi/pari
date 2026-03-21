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
- `insert(key, value)` — inserts or replaces; marks the key as dirty
- `remove(key)` — removes the entry; records the key in the removed set
- `get(key) -> Option<&V>` — read access
- `get_mut(key) -> Option<&mut V>` — mutable access; marks the key as dirty
- `values()` — iterates values in insertion order
- `keys()` — iterates keys in insertion order

#### Scenario: Insert marks key as dirty
- **WHEN** a new key `"eng-lead"` is inserted into a `TrackedMap`
- **THEN** the key appears in the dirty set and `get("eng-lead")` returns the value

#### Scenario: Remove records key in removed set
- **WHEN** an existing key `"eng-lead"` is removed from a `TrackedMap`
- **THEN** `get("eng-lead")` returns `None` and the key appears in the removed set

#### Scenario: get_mut marks key as dirty
- **WHEN** `get_mut("eng-lead")` is called on a `TrackedMap` whose `"eng-lead"` entry was previously clean
- **THEN** the key appears in the dirty set

#### Scenario: Insertion order is preserved
- **WHEN** keys `"a"`, `"b"`, `"c"` are inserted in that order
- **THEN** `keys()` yields `"a"`, `"b"`, `"c"` in that order

#### Scenario: shift_remove preserves order of remaining entries
- **WHEN** key `"b"` is removed from a TrackedMap with keys `["a", "b", "c"]`
- **THEN** `keys()` yields `"a"`, `"c"` in that order

---

### Requirement: TrackedMap supports drain_changes for change detection
The system SHALL provide `drain_changes(&mut self)` on `TrackedMap` that returns the dirty set and removed set, then clears both sets and resets all per-value dirty flags. This is the primary interface for collecting changes.

#### Scenario: drain_changes returns dirty and removed sets
- **WHEN** a `TrackedMap` has dirty keys `["a", "b"]` and removed keys `["c"]`
- **THEN** `drain_changes()` returns dirty: `["a", "b"]`, removed: `["c"]`

#### Scenario: drain_changes resets tracking state
- **WHEN** `drain_changes()` is called on a `TrackedMap`
- **THEN** subsequent `drain_changes()` returns empty dirty and removed sets (assuming no further mutations)

---

### Requirement: TrackedMap::from_vec converts ordered plain collections
The system SHALL provide `TrackedMap::from_vec(items, key_fn)` that takes a `Vec<T>`, extracts a key from each item using `key_fn`, and builds a `TrackedMap` with entries in the original Vec order. All entries SHALL start in the dirty state.

#### Scenario: from_vec preserves order and extracts keys
- **WHEN** `TrackedMap::from_vec(vec![step_a, step_b], |s| s.id.clone())` is called
- **THEN** the TrackedMap contains both steps in `[a, b]` order, keyed by their ids

---

### Requirement: Derive macro generates tracked struct variants
The system SHALL provide a `#[derive(Tracked)]` proc macro in the `pari-macros` crate that, given a plain struct, generates:
1. A tracked struct (named `Tracked<StructName>`) with each field wrapped in `Tracked<T>`
2. A `From<Plain> for Tracked` impl that converts each field via `Tracked::new(field)`

Fields annotated with `#[tracked(map_key = "id")]` on a `Vec<T>` field SHALL generate a `TrackedMap<String, TrackedT>` field in the tracked variant, with conversion via `TrackedMap::from_vec()`.

#### Scenario: Derive generates tracked variant with Tracked fields
- **WHEN** `#[derive(Tracked)]` is applied to a struct with fields `id: String` and `name: String`
- **THEN** a tracked struct is generated with fields `id: Tracked<String>` and `name: Tracked<String>`

#### Scenario: Derive generates From impl
- **WHEN** a plain `Role` is converted via `TrackedRole::from(role)`
- **THEN** every field in the resulting `TrackedRole` is dirty and holds the original value

#### Scenario: Vec field with tracked map_key annotation becomes TrackedMap
- **WHEN** a field `steps: Vec<Step>` is annotated with `#[tracked(map_key = "id")]`
- **THEN** the tracked variant has `steps: TrackedMap<String, TrackedStep>` and conversion uses `from_vec()`
