# Task 03 — `#[derive(Entity)]` Proc Macro

## Scope

Rewrite `pari-macros/src/lib.rs` to implement the `#[derive(Entity)]` proc macro. Given a plain entity struct, the macro generates:

1. `TrackedX` struct — each non-identity field wrapped in `Arc<TrackedField<T>>`
2. `From<X> for TrackedX` — plain entity conversion; all fields initialized, all clean
3. Async accessors on `TrackedX` — `async fn name(&self) -> Result<&str, LoadError>`
4. Async setters on `TrackedX` — `async fn set_name(&mut self, v: T) -> Result<(), SetterError>`
5. Dirty operations on `TrackedX` — `has_dirty_fields`, `dirty_fields`, `merge_dirty_into`, `reset_dirty`
6. `impl Entity for X` — `KIND`, `validation_schema()` (stub returning empty schema), `Parent`, `Tracked` associated items
7. `impl TrackedEntity for TrackedX` — `type Entity = X`
8. `impl Resolvable for X` — `to_any_ref` and `extract`

**Serde impls are not generated here** — they are in Task 08.

---

## Files

- `pari-macros/src/lib.rs` — proc macro crate; full rewrite
- `pari-macros/Cargo.toml` — ensure `syn`, `quote`, `proc-macro2` deps present
- `src/tracked.rs` — add `get_or_load` stub method to `TrackedField<T>`
- `src/entity.rs` — add `LoadError` and `SetterError` stubs (placeholder types for this task; filled in later)

---

## Dependencies

- Task 01: `TrackedField<T>` — used as the field wrapper type
- Task 02: `Entity`, `TrackedEntity`, `EntityKind`, `AnyEntityRef`, `StoreEntity`, `Resolvable`, `EntityRef`, `ParentKind`, `NoParent`, `WorkflowParent`, `ValidationSchema`

---

## Error Type Stubs (add to `src/entity.rs`)

These are placeholder types to allow the macro output to compile. They will be replaced by full types in Task 06 and Task 09.

```rust
/// Placeholder. Task 09 fills this out (EntityServer load failures).
#[derive(Debug)]
pub struct LoadError;

/// Placeholder. Task 09 fills this out (setter precondition failures).
#[derive(Debug)]
pub enum SetterError {
    Substrate,
    Validation,
}
```

---

## `get_or_load` Stub (add to `src/tracked.rs`)

Add to `impl<T> TrackedField<T>`:

```rust
/// Returns the value if initialized, or an error if not yet loaded.
/// The real load-path implementation (Task 09) replaces this body with an
/// EntityServer channel call. This stub is sufficient for tests that
/// pre-initialize all fields.
pub async fn get_or_load(&self) -> Result<&T, crate::entity::LoadError> {
    self.get().ok_or(crate::entity::LoadError)
}
```

---

## Macro Input Convention

```rust
#[derive(Entity)]
pub struct Role {
    pub entity_ref: EntityRef<Role>,      // identity field — not wrapped in TrackedField
    pub name:        String,
    pub description: Option<String>,
    pub purpose:     String,
    pub traits:      Option<Vec<String>>,
    pub extensions:  Extensions,
}
```

The macro identifies `entity_ref` by field name. All other fields are domain fields.

### Required Companion Attribute on each `#[derive(Entity)]` type

To supply the `Entity::KIND` discriminant and optional parent override, a companion attribute is required:

```rust
#[entity(kind = EntityKind::Role)]
#[derive(Entity)]
pub struct Role { ... }

// Embedded entity with WorkflowParent:
#[entity(kind = EntityKind::Task, parent = WorkflowParent)]
#[derive(Entity)]
pub struct Task { ... }
```

`parent` defaults to `NoParent` when omitted.

---

## Generated Code — Full Specification

For a struct `Role` with fields `entity_ref`, `name: String`, `description: Option<String>`:

### 1. `TrackedRole` struct

```rust
pub struct TrackedRole {
    pub entity_ref:  EntityRef<Role, NoParent>,
    pub name:        Arc<TrackedField<String>>,
    pub description: Arc<TrackedField<Option<String>>>,
    // ... all domain fields
}
```

### 2. `From<Role> for TrackedRole`

```rust
impl From<Role> for TrackedRole {
    fn from(plain: Role) -> Self {
        TrackedRole {
            entity_ref:  plain.entity_ref,
            name:        Arc::new(TrackedField::new_initialized(plain.name)),
            description: Arc::new(TrackedField::new_initialized(plain.description)),
            // ...
        }
    }
}
```

`TrackedField::new_initialized` — add this to `TrackedField` alongside `with_value`:

```rust
/// Create a field pre-seeded with `value` and marked clean (dirty = false).
/// Used by From<PlainEntity> — the entity is "loaded" from the plain type.
pub fn new_initialized(value: T) -> Self {
    let f = Self::new();
    f.initialize(value);
    f
}
```

### 3. Async Accessors

Return type mapping rules (applied to the domain field type `T`):
- `String` → `&str`
- `Option<String>` → `Option<&str>`
- `Vec<U>` → `&[U]`
- `Option<Vec<U>>` → `Option<&[U]>`
- Any other `T` → `&T`

```rust
impl TrackedRole {
    pub async fn name(&self) -> Result<&str, LoadError> {
        self.name.get_or_load().await.map(|v| v.as_str())
    }

    pub async fn description(&self) -> Result<Option<&str>, LoadError> {
        self.description.get_or_load().await
            .map(|opt| opt.as_deref())
    }

    pub async fn purpose(&self) -> Result<&str, LoadError> {
        self.purpose.get_or_load().await.map(|v| v.as_str())
    }

    // ... one per domain field
}
```

**Note**: `entity_ref` gets a trivial sync accessor, not an async one:
```rust
pub fn entity_ref(&self) -> &EntityRef<Role, NoParent> {
    &self.entity_ref
}
```

### 4. Async Setters

```rust
impl TrackedRole {
    pub async fn set_name(&mut self, value: String) -> Result<(), SetterError> {
        // Step 1: ensure_mutable (stub call; Task 09 fills in real logic)
        self.ensure_mutable().await?;
        // Step 2: COW-replace the Arc with a new dirty TrackedField
        self.name = Arc::new(TrackedField::with_value(value));
        Ok(())
    }

    pub async fn set_description(&mut self, value: Option<String>) -> Result<(), SetterError> {
        self.ensure_mutable().await?;
        self.description = Arc::new(TrackedField::with_value(value));
        Ok(())
    }

    // ... one per domain field

    async fn ensure_mutable(&mut self) -> Result<(), SetterError> {
        // Stub: no-op. Task 09 replaces with EntityServer load-all-fields call.
        Ok(())
    }
}
```

### 5. Dirty Operations

```rust
impl TrackedRole {
    /// Returns true if any domain field has dirty = true.
    pub fn has_dirty_fields(&self) -> bool {
        self.name.is_dirty()
            || self.description.is_dirty()
            // ... all domain fields
    }

    /// Returns the names of all dirty domain fields.
    pub fn dirty_fields(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.name.is_dirty()        { out.push("name"); }
        if self.description.is_dirty() { out.push("description"); }
        // ...
        out
    }

    /// For each dirty field on `self`, replace the corresponding Arc on `target`.
    /// Non-dirty fields on self are not copied — target keeps its own state.
    pub fn merge_dirty_into(&self, target: &mut TrackedRole) {
        if self.name.is_dirty()        { target.name        = Arc::clone(&self.name); }
        if self.description.is_dirty() { target.description = Arc::clone(&self.description); }
        // ...
    }

    /// Replace all dirty field Arcs with clean versions (dirty = false, value preserved).
    pub fn reset_dirty(&mut self) {
        if self.name.is_dirty() {
            if let Some(v) = self.name.get() {
                self.name = Arc::new(TrackedField::new_initialized(v.clone()));
            }
        }
        // ... all domain fields
    }
}
```

### 6. `Entity` impl

```rust
impl Entity for Role {
    const KIND: EntityKind = EntityKind::Role;
    fn validation_schema() -> &'static ValidationSchema<Self> {
        // Stub — Task 07 replaces with OnceLock-backed static holding real rules
        static S: std::sync::OnceLock<ValidationSchema<Role>> = std::sync::OnceLock::new();
        S.get_or_init(|| ValidationSchema::empty())
    }
    type Parent = NoParent;
    type Tracked = TrackedRole;
}
```

### 7. `TrackedEntity` impl

```rust
impl TrackedEntity for TrackedRole {
    type Entity = Role;
}
```

### 8. `Resolvable` impl

```rust
impl Resolvable for Role {
    fn to_any_ref(entity_ref: &EntityRef<Self, Self::Parent>) -> AnyEntityRef {
        AnyEntityRef::Role(entity_ref.clone())
    }

    fn extract(entity: &StoreEntity) -> Option<&TrackedRole> {
        match entity {
            StoreEntity::Role(r) => Some(r),
            _ => None,
        }
    }
}
```

---

## `pari-macros/Cargo.toml` — Required Dependencies

```toml
[dependencies]
syn  = { version = "2", features = ["full"] }
quote = "1"
proc-macro2 = "1"

[lib]
proc-macro = true
```

---

## TDD: Tests to Write First

Add a `#[cfg(test)]` module in `src/entity.rs` (or a new `tests/derive_entity.rs` integration test file). Tests use a minimal inline struct to exercise the macro output.

```rust
// tests/derive_entity.rs
use pari::entity::{
    Entity, TrackedEntity, EntityKind, EntityRef, NoParent, AnyEntityRef, StoreEntity,
    Resolvable, ValidationSchema, LoadError, SetterError,
};
use pari::tracked::TrackedField;
use std::sync::Arc;

// Minimal test entity with two domain fields
#[pari_macros::entity(kind = EntityKind::Role)]
#[derive(pari_macros::Entity)]
pub struct TestRole {
    pub entity_ref:  EntityRef<TestRole>,
    pub name:        String,
    pub count:       Option<u32>,
}

// --- From conversion ---

#[test]
fn from_plain_initializes_all_fields() {
    let plain = TestRole {
        entity_ref: EntityRef::new("test-role"),
        name:       "Eng Lead".to_string(),
        count:      Some(3),
    };
    let tracked = TrackedTestRole::from(plain);
    assert_eq!(tracked.name.get(), Some(&"Eng Lead".to_string()));
    assert_eq!(tracked.count.get(), Some(&Some(3)));
}

#[test]
fn from_plain_fields_are_clean() {
    let plain = TestRole {
        entity_ref: EntityRef::new("r"),
        name:       "X".into(),
        count:      None,
    };
    let tracked = TrackedTestRole::from(plain);
    assert!(!tracked.name.is_dirty());
    assert!(!tracked.count.is_dirty());
}

// --- Dirty operations ---

#[test]
fn has_dirty_fields_false_after_from_conversion() {
    let plain = TestRole { entity_ref: EntityRef::new("r"), name: "N".into(), count: None };
    let tracked = TrackedTestRole::from(plain);
    assert!(!tracked.has_dirty_fields());
}

#[test]
fn dirty_fields_empty_after_from_conversion() {
    let plain = TestRole { entity_ref: EntityRef::new("r"), name: "N".into(), count: None };
    let tracked = TrackedTestRole::from(plain);
    assert!(tracked.dirty_fields().is_empty());
}

#[test]
fn has_dirty_fields_true_after_cow_replacement() {
    let plain = TestRole { entity_ref: EntityRef::new("r"), name: "Old".into(), count: None };
    let mut tracked = TrackedTestRole::from(plain);
    // Simulate what a setter does: COW-replace the Arc
    tracked.name = Arc::new(TrackedField::with_value("New".to_string()));
    assert!(tracked.has_dirty_fields());
    assert_eq!(tracked.dirty_fields(), vec!["name"]);
}

#[test]
fn merge_dirty_into_copies_only_dirty_fields() {
    let base = TestRole { entity_ref: EntityRef::new("r"), name: "Old".into(), count: Some(1) };
    let mut target = TrackedTestRole::from(base);

    let source_plain = TestRole { entity_ref: EntityRef::new("r"), name: "Old".into(), count: Some(1) };
    let mut source = TrackedTestRole::from(source_plain);
    // Dirty only `name` on source
    source.name = Arc::new(TrackedField::with_value("New".to_string()));

    source.merge_dirty_into(&mut target);

    // name was replaced
    assert_eq!(target.name.get(), Some(&"New".to_string()));
    // count was not touched (not dirty on source) — still holds original value
    assert_eq!(target.count.get(), Some(&Some(1)));
}

#[test]
fn reset_dirty_clears_all_dirty_flags() {
    let plain = TestRole { entity_ref: EntityRef::new("r"), name: "Old".into(), count: None };
    let mut tracked = TrackedTestRole::from(plain);
    tracked.name = Arc::new(TrackedField::with_value("New".to_string()));
    assert!(tracked.has_dirty_fields());

    tracked.reset_dirty();

    assert!(!tracked.has_dirty_fields());
    assert_eq!(tracked.dirty_fields(), vec![] as Vec<&str>);
    // Value must be preserved after reset
    assert_eq!(tracked.name.get(), Some(&"New".to_string()));
}

// --- Entity trait ---

#[test]
fn entity_kind_is_correct() {
    assert_eq!(<TestRole as Entity>::KIND, EntityKind::Role);
}

// --- TrackedEntity companion trait ---

#[test]
fn tracked_entity_roundtrip_compiles() {
    // Compile-time check: TrackedTestRole::Entity = TestRole
    fn _check(_: <TrackedTestRole as TrackedEntity>::Entity) {}
    let _ = |r: TestRole| _check(r);
}

// --- Resolvable ---

#[test]
fn to_any_ref_wraps_in_correct_variant() {
    let r: EntityRef<TestRole> = EntityRef::new("test-role");
    let any = TestRole::to_any_ref(&r);
    match any {
        AnyEntityRef::Role(inner) => assert_eq!(inner.id(), "test-role"),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn extract_returns_some_for_matching_variant() {
    let plain = TestRole { entity_ref: EntityRef::new("r"), name: "N".into(), count: None };
    let tracked = TrackedTestRole::from(plain);
    let store_entity = StoreEntity::Role(tracked);
    assert!(TestRole::extract(&store_entity).is_some());
}

#[test]
fn extract_returns_none_for_non_matching_variant() {
    // Need a different variant — use whatever non-Role variant is available
    // This test only compiles and passes if StoreEntity has at least two variants
    // which it does once entity_registry! is run (Task 04). Skip if only one variant exists.
}

// --- Async accessor (pre-initialized field) ---

#[tokio::test]
async fn accessor_returns_value_when_initialized() {
    let plain = TestRole { entity_ref: EntityRef::new("r"), name: "Eng Lead".into(), count: Some(5) };
    let tracked = TrackedTestRole::from(plain);

    let name = tracked.name().await.unwrap();
    assert_eq!(name, "Eng Lead");

    let count = tracked.count().await.unwrap();
    assert_eq!(count, Some(5));
}

#[tokio::test]
async fn accessor_returns_error_when_uninitialized() {
    let tracked = TrackedTestRole {
        entity_ref: EntityRef::new("r"),
        name:  Arc::new(TrackedField::new()),
        count: Arc::new(TrackedField::new()),
    };
    assert!(tracked.name().await.is_err());
}

// --- Async setter ---

#[tokio::test]
async fn setter_replaces_field_value() {
    let plain = TestRole { entity_ref: EntityRef::new("r"), name: "Old".into(), count: None };
    let mut tracked = TrackedTestRole::from(plain);

    tracked.set_name("New".to_string()).await.unwrap();

    assert_eq!(tracked.name.get(), Some(&"New".to_string()));
    assert!(tracked.name.is_dirty());
}

#[tokio::test]
async fn setter_marks_field_dirty() {
    let plain = TestRole { entity_ref: EntityRef::new("r"), name: "X".into(), count: None };
    let mut tracked = TrackedTestRole::from(plain);
    assert!(!tracked.has_dirty_fields());

    tracked.set_name("Y".to_string()).await.unwrap();
    assert!(tracked.has_dirty_fields());
}
```

---

## Implementation Notes

### Proc Macro Structure

```rust
// pari-macros/src/lib.rs
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, Data, Fields};
use quote::quote;

#[proc_macro_derive(Entity, attributes(entity))]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let expanded = generate_entity_impl(input);
    TokenStream::from(expanded)
}

fn generate_entity_impl(input: DeriveInput) -> proc_macro2::TokenStream {
    let struct_name = &input.ident;
    let tracked_name = quote::format_ident!("Tracked{}", struct_name);

    // Parse #[entity(kind = ..., parent = ...)] attribute
    // Extract fields, separate entity_ref from domain fields
    // Generate each piece and combine with quote!
    todo!()
}
```

### Field Parsing

- Iterate `input.data` → `Data::Struct` → `Fields::Named`
- `entity_ref` field: identified by field name `entity_ref`; not wrapped in `TrackedField`
- Domain fields: all others; wrapped as `Arc<TrackedField<{original_type}>>`

### Accessor Return Type Mapping

Implement a helper function that inspects the `syn::Type` for domain fields:

```
String            → "v.as_str()"         return type: &str
Option<String>    → "opt.as_deref()"     return type: Option<&str>
Vec<U>            → "v.as_slice()"       return type: &[U]
Option<Vec<U>>    → "opt.as_deref()"     return type: Option<&[U]>
T (other)         → identity             return type: &T
```

For the fallback case (`&T`), the accessor is:
```rust
pub async fn field_name(&self) -> Result<&FieldType, LoadError> {
    self.field_name.get_or_load().await
}
```

### `new_initialized` helper

Must be added to `TrackedField<T>` in `src/tracked.rs`:
```rust
/// Create a field pre-seeded with `value` and clean (dirty = false).
/// Used by From<PlainEntity>. Distinct from with_value (which marks dirty).
pub fn new_initialized(value: T) -> Self {
    let f = Self::new();
    f.initialize(value);
    f  // dirty remains false because initialize() does not set dirty
}
```

### `entity_ref` accessor

Always generated as a sync method (entity_ref is always present, no lazy loading):
```rust
pub fn entity_ref(&self) -> &EntityRef<X, X::Parent> {
    &self.entity_ref
}
```

### `reset_dirty` implementation detail

`reset_dirty` must clone the value before re-wrapping. The field type `T` must implement `Clone`. The derive macro should add a `T: Clone` bound on the `reset_dirty` impl, or simply call `new_initialized(v.clone())`.

### Forward Declarations for AnyEntityRef / StoreEntity

`Resolvable::to_any_ref` references `AnyEntityRef::Role(...)` and `Resolvable::extract` references `StoreEntity::Role(...)`. These enums are generated by `entity_registry!` in Task 04. During Task 03 testing, use the hand-written stubs from Task 02 (`AnyEntityRef` and `StoreEntity` as empty enums with a single `Role` variant for the test entity).

---

## Acceptance Criteria

- `cargo test derive_entity` passes — all tests in `tests/derive_entity.rs` green
- `cargo build` succeeds — generated code compiles without warnings
- `TrackedX` struct is generated with correct field types
- `From<X> for TrackedX` converts all fields correctly
- Dirty operations work correctly: detect dirty, merge, reset
- `Entity::KIND` matches the `#[entity(kind = ...)]` attribute value
- `TrackedEntity::Entity` roundtrip resolves correctly as a type alias
- Async accessors return `Ok(&value)` for pre-initialized fields, `Err(LoadError)` for uninitialized
- Async setters COW-replace the Arc and mark the field dirty
