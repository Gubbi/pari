# pari-macros — Proc Macro Crate

Three derive macros + one declarative macro. All defined in `pari-macros/src/lib.rs` (main body) with helpers in `error_compose.rs` and `otel_emit.rs`.

---

## `#[derive(pari_macros::Entity)]`

Used on plain entity structs alongside `#[entity(...)]` attribute.

```rust
#[derive(pari_macros::Entity)]
#[entity(kind = EntityKind::Role, schema = crate::validation::role::role_validation_schema)]
// For embedded entities add: parent = WorkflowParent
pub struct Role { ... }
```

**Generates:**
- `TrackedRole` struct — all non-`entity_ref` fields replaced with `Arc<TrackedField<T>>`
- `impl From<Role> for TrackedRole` — initializes each field via `TrackedField::with_value(...)`; starts clean
- `impl TrackedFor for TrackedRole { type Entity = Role; }`
- `has_dirty_fields() -> bool`, `dirty_fields() -> Vec<&'static str>`, `reset_dirty()`, `merge_dirty_into(&mut self, target: &mut Self)`
- `entity_ref() -> &EntityRef<Role>` accessor
- For each field `name: String`:
  - `async fn name(&self) -> Result<&str, LoadError>` — calls `self.name.get_or_load().await` + type conversion
  - `async fn set_name(&mut self, v: String) -> Result<(), SetterError>`
- `impl Entity for Role` with `KIND`, `validation_schema()`, `to_any_ref()`, `extract()`

**`no_dispatch` option** — skips `to_any_ref`/`extract` generation (used in test-only structs):
```rust
#[entity(kind = EntityKind::Role, no_dispatch)]
```

**Accessor return types by field type:**

| Field type | Accessor return | Transform |
|------------|----------------|-----------|
| `String` | `Result<&str, LoadError>` | `.as_str()` |
| `Option<String>` | `Result<Option<&str>, LoadError>` | `.as_deref()` |
| `Option<Vec<T>>` | `Result<Option<&[T]>, LoadError>` | `.as_deref()` |
| `Vec<T>` | `Result<&[T], LoadError>` | `.as_slice()` |
| `Option<T>` | `Result<Option<&T>, LoadError>` | `.as_ref()` |
| `T` (other) | `Result<&T, LoadError>` | (direct ref) |

---

## `#[derive(Tracked)]` (pari_macros)

Lower-level macro; `#[derive(Entity)]` calls this internally. Can also be used standalone.

```rust
#[derive(pari_macros::Tracked)]
pub struct Foo {
    pub id: String,
    pub items: Vec<Item>,    // #[tracked(map_key = "id")] → TrackedMap<String, Item>
}
```

**Generates:** `TrackedFoo` struct with `From<Foo>` impl, `dirty_fields()`, `merge_dirty_into()`.

---

## `#[derive(ErrorCompose)]`

See `src/error/CLAUDE.md` for usage. Implementation in `pari-macros/src/error_compose.rs`.

Uses `darling` for attribute parsing. Reads `compose` attribute namespace.

**On structs:**
```rust
#[derive(ErrorCompose)]
#[compose(fix = Client, recoverability = UserAction)]
pub struct MyError { ... }
```

**On enums:**
```rust
#[derive(ErrorCompose)]
pub enum MyError {
    #[compose(fix = Client, recoverability = UserAction)]
    BadInput { ... },
    #[compose(delegate)]
    Substrate(SubstrateError),  // delegates fix_domain/recoverability to inner
}
```

---

## `#[derive(OTelEmit)]`

See `src/error/CLAUDE.md` for usage. Implementation in `pari-macros/src/otel_emit.rs`.

Uses `darling` reading both `compose` and `otel` attribute namespaces. **Never put both `#[compose(delegate)]` and `#[otel(delegate)]` on the same variant** — causes darling "Duplicate field" error. `compose(delegate)` alone is sufficient.

---

## `entity_registry! { ... }`

Declarative macro in `pari-macros/src/lib.rs`. Invoked once in `src/entity.rs`.

```rust
pari_macros::entity_registry! {
    Role             => NoParent,
    Task             => WorkflowParent,
    // ...
}
```

**Generates:**
- `EntityKind` enum with all variants + `as_str()` method + `Copy/Clone/Debug/PartialEq/Eq/Hash`
- `AnyEntityRef` enum — one variant per entity with typed `EntityRef`; methods: `kind()`, `id()`
- `StoreEntity` enum — one variant per entity with `Tracked*` type; methods: `any_ref()`, `is_stub()`, `make_stub()`, `has_dirty_fields()`, `dirty_fields()`, `reset_dirty()`, `merge_dirty_into()`, `initialize_into()`, `all_refs()`, `from_role()` etc.
- `load_strategy(EntityKind) -> LoadStrategy` — dispatches to per-entity `SubstrateSchema::load_strategy()`
