# Pending Tasks — Code Fix Session

## Status

In the middle of fixing code to align with the design. Build was failing. Last action: changed `substrate::Substrate` trait methods from `async fn` to `fn -> impl Future + Send` pattern in `src/substrate/mod.rs`.

---

## What Was Done

### Design Docs Updated (complete)
All 11 design docs updated to reflect agreed design:
- `tracked-field.md` — AtomicBool, three constructors (new/loaded/mutated)
- `cow-field-convention.md` — async setter, TrackedField::mutated, is_dirty()
- `store-load-internal.md` — enrich→validate→initialize-in-place merge sequence, accessor uses EntityClient::load
- `ensure-mutable.md` — full rewrite: setter sends StoreRequest::EnsureMutable to EntityServer
- `async-accessor-variants.md` — accessor uses EntityClient::load + Arc sharing; no get_or_load
- `entity-registry.md` — invocation uses `=> ParentType`
- `entity-client-api.md` — added load/ensure_mutable methods; removed StoreUnavailable from checkout/persist
- `store-resolve.md` — batch exists, updated error variants
- `store-checkout.md` — updated CheckoutError variants with fields
- `store-commit.md` — updated CommitError variants, undo_checkout returns UndoError
- `store-entity-lifecycle.md` — updated UndoError variants

### Code Changes Applied (partial — build failing)

#### `src/tracked.rs` ✓
- `with_value` renamed → `mutated`
- `new_initialized` renamed → `loaded`
- `get_or_load` removed
- Inline tests updated

#### `src/substrate/mod.rs` — in progress
- `exists`: changed to batch (`&[AnyEntityRef]` → `Vec<bool>`) — done
- `load`: changed to take `&StoreEntity` instead of `&AnyEntityRef` — done
- `atomic_persist` renamed → `persist`, takes `impl Iterator<Item = EntityChange>` — done
- **LAST ACTION**: Changing `async fn` → `fn -> impl Future + Send` pattern to fix Send bounds — in progress

#### `src/substrate/repo/mod.rs` — partial
- Removed `store::StoreEntityChange` import — done
- Removed `store::Substrate` impl block entirely — done
- Updated `substrate::Substrate` impl: `exists` batch, `load` takes entity, `persist` with iterator — done
- **STILL NEED**: Change `async fn` to `fn -> impl Future + Send` in the trait impl

#### `src/store/mod.rs` — partial
- Removed `store::Substrate` trait — done
- Removed `StoreEntityChange` enum — done
- `InMemorySubstrate` now implements `substrate::Substrate` — done
- `Store<S>` uses `substrate::Substrate` bound — done
- `StoreRequest::EnsureMutable` added — done
- `StoreResponse::LoadErr(LoadError)` added — done
- `Store::resolve()` uses batch `exists` — done
- `load_from_substrate` replaced by `load_field` + `ensure_mutable` — done
- `Store::persist()` uses `EntityChange` iterator — done
- `EntityClient::load` and `EntityClient::ensure_mutable` added — done
- Checkout/persist `StoreUnavailable` wrapping removed — done
- **STILL NEED**: Fix `InMemorySubstrate` impl to use `fn -> impl Future + Send` (not `async fn`)
- **STILL NEED**: Remove now-unused imports (`ExecutorError`, `VoidSlot`, etc. if not needed)

#### `pari-macros/src/lib.rs` — done
- Accessor generation: replaced `get_or_load().await` with `EntityClient::load()` pattern
- Setter generation: replaced `self.ensure_mutable()` with `EntityClient::ensure_mutable()`, uses `TrackedField::mutated`
- `reset_stmts`: now calls `self.#fname.reset_dirty()` in-place (not replacing Arc)
- `ensure_mutable()` generated method removed
- `TrackedField::loaded()` used in `From<PlainEntity>` impl

---

## Remaining Code Fixes

### 1. `src/substrate/mod.rs` — complete the `fn -> impl Future + Send` change

The VoidSubstrate impl still uses `async fn`. Needs to change to match the new trait signatures:

```rust
// VoidSubstrate::exists
fn exists<'a>(&'a self, refs: &'a [AnyEntityRef]) -> impl Future<Output = Result<Vec<bool>, SubstrateError>> + Send + 'a {
    let n = refs.len();
    async move { Ok(vec![false; n]) }
}

// VoidSubstrate::load
fn load<'a>(&'a self, entity: &'a StoreEntity, _: &'a [&'a str]) -> impl Future<Output = Result<StoreEntity, SubstrateError>> + Send + 'a {
    let id = entity.any_ref().id().to_owned();
    async move { Err(SubstrateError::Executor(ExecutorError::new(id, "VoidSubstrate: no load"))) }
}

// VoidSubstrate::persist
fn persist<'a>(&'a self, _: impl Iterator<Item = EntityChange<'a>> + Send + 'a) -> impl Future<Output = Result<(), Vec<SubstrateError>>> + Send + 'a {
    async { Ok(()) }
}
```

### 2. `src/store/mod.rs` — fix InMemorySubstrate impl

Change `async fn` to `fn -> impl Future + Send` to match new substrate::Substrate signatures:

```rust
fn exists<'a>(&'a self, refs: &'a [AnyEntityRef]) -> impl Future<Output = Result<Vec<bool>, SubstrateError>> + Send + 'a {
    let results: Vec<bool> = {
        let guard = self.entities.lock().unwrap();
        refs.iter().map(|r| guard.contains_key(r)).collect()
    };
    async move { Ok(results) }
}

fn load<'a>(&'a self, entity: &'a StoreEntity, _fields: &'a [&'a str]) -> impl Future<Output = Result<StoreEntity, SubstrateError>> + Send + 'a {
    let any_ref = entity.any_ref();
    let result = {
        let guard = self.entities.lock().unwrap();
        guard.get(&any_ref).cloned()
            .ok_or_else(|| SubstrateError::from(ExecutorError::new(any_ref.id(), "not found")))
    };
    async move { result }
}

fn persist<'a>(&'a self, _changes: impl Iterator<Item = EntityChange<'a>> + Send + 'a) -> impl Future<Output = Result<(), Vec<SubstrateError>>> + Send + 'a {
    async { Ok(()) }
}
```

Also remove now-unused `std::future::Future` import if it was only needed for `store::Substrate`.

### 3. `src/substrate/repo/mod.rs` — fix substrate::Substrate impl methods

Change the three trait impl methods from `async fn` to `fn -> impl Future + Send`:

```rust
fn exists<'a>(&'a self, refs: &'a [AnyEntityRef]) -> impl Future<Output = Result<Vec<bool>, SubstrateError>> + Send + 'a {
    let results: Vec<bool> = refs.iter().map(|any_ref| {
        let entity_json = Self::any_ref_to_json(any_ref);
        let schema = Self::schema_for(any_ref.kind());
        let path = self.resolver.resolve(schema.ref_asset.path_template, &entity_json);
        path.exists()
    }).collect();
    async move { Ok(results) }
}

fn load<'a>(&'a self, entity: &'a StoreEntity, _fields: &'a [&'a str]) -> impl Future<Output = Result<StoreEntity, SubstrateError>> + Send + 'a {
    let any_ref = entity.any_ref();
    let entity_json = Self::any_ref_to_json(&any_ref);
    let schema = Self::schema_for(any_ref.kind());
    let path = self.resolver.resolve(schema.ref_asset.path_template, &entity_json);
    let result = fs::read_to_string(&path)
        .map_err(|e| SubstrateError::Executor(ExecutorError::new(path.to_string_lossy(), e.to_string())))
        .and_then(|content| Self::decode_to_store_entity(&any_ref, &content, schema));
    async move { result }
}

fn persist<'a>(&'a self, changes: impl Iterator<Item = EntityChange<'a>> + Send + 'a) -> impl Future<Output = Result<(), Vec<SubstrateError>>> + Send + 'a {
    use crate::substrate::pipeline::{AssetOp, AssetRequest};
    // Materialize ops synchronously, then wrap in async move
    let mut ops = Vec::new();
    let mut errors = Vec::new();
    for change in changes {
        match change {
            EntityChange::Added(entity) | EntityChange::Modified(entity, _) => {
                let json = Self::entity_to_json(entity);
                let schema = Self::schema_for(entity.any_ref().kind());
                let path = self.resolver.resolve(schema.ref_asset.path_template, &json);
                let field_map: std::collections::HashMap<&str, serde_json::Value> =
                    schema.ref_asset.fields.iter()
                        .filter_map(|fm| json.get(fm.key).map(|v| (fm.key, v.clone())))
                        .collect();
                match self.codec.encode(&field_map, schema.ref_asset.fields) {
                    Ok(encoded) => ops.push(AssetRequest { location: path, op: AssetOp::Put(encoded) }),
                    Err(e) => errors.push(SubstrateError::Codec(e)),
                }
            }
            EntityChange::Removed(any_ref) => {
                let entity_json = Self::any_ref_to_json(any_ref);
                let schema = Self::schema_for(any_ref.kind());
                let path = self.resolver.resolve(schema.ref_asset.path_template, &entity_json);
                ops.push(AssetRequest { location: path, op: AssetOp::Delete });
            }
        }
    }
    let executor = self.executor.clone();
    async move {
        if !errors.is_empty() { return Err(errors); }
        if ops.is_empty() { return Ok(()); }
        executor.execute(ops).map(|_| ()).map_err(|errs| errs.into_iter().map(SubstrateError::Executor).collect())
    }
}
```

### 4. `src/store/mod.rs` — fix `Store::persist()` borrow issue

The current `Store::persist()` builds a `Vec<EntityChange<'_>>` borrowing from `self`. There may be a borrow checker issue since we also need `self.substrate` for the call. May need to restructure — collect the changes data into owned values first, then call substrate.

The `EntityChange<'a>` uses borrowed references:
- `Added(&'a StoreEntity)` — borrows from `self.entities`
- `Modified(&'a StoreEntity, &'a [&'a str])` — borrows from `self.entities`; dirty field strs are `&'static str`
- `Removed(&'a AnyEntityRef)` — borrows from `self.removed`

Since `substrate.persist(changes)` also borrows `&self.substrate`, and changes borrow `self.entities` and `self.removed`, we have a split-borrow issue. May need to split `Store` or use indices.

**Option**: Collect the AnyEntityRefs, clone the entities needed, build owned EntityChange variants:
```rust
let added_data: Vec<StoreEntity> = self.added.iter()
    .filter_map(|r| self.entities.get(r).cloned())
    .collect();
// etc.
// Then EntityChange::Added(&entity) from the local Vec
```

Or restructure to pass the substrate by reference separately from the store state.

### 5. `src/store/mod.rs` — fix `load_field` borrow issue

Similarly, `load_field` calls `self.substrate.load(&current, ...)` and then mutates `self.entities`. Borrow checker may complain since `current` borrows from `self.entities`.

The current code clones `current` before the substrate call, which should be fine:
```rust
let current = self.entities.entry(...).or_insert_with(...).clone(); // owned clone
let loaded = self.substrate.load(&current, &[field]).await...;     // borrow current (not self.entities)
loaded.initialize_into(self.entities.get_mut(any_ref).unwrap());   // borrow self.entities
```

This should compile since `current` is an owned clone, not a borrow of `self.entities`.

### 6. Tests that need updating

#### `tests/store_operations.rs` line 69
```rust
// Before:
r.name = std::sync::Arc::new(pari::tracked::TrackedField::with_value("New Name".to_string()));
// After:
r.name = std::sync::Arc::new(pari::tracked::TrackedField::mutated("New Name".to_string()));
```

#### `tests/tracked_serde.rs` line 28
```rust
// Before:
name: Arc::new(TrackedField::new_initialized("Engineering Lead".to_string())),
// After:
name: Arc::new(TrackedField::loaded("Engineering Lead".to_string())),
```

#### `tests/derive_entity.rs` lines 57, 70, 82
```rust
// Before:
tracked.name = Arc::new(TrackedField::with_value("New".to_string()));
// After:
tracked.name = Arc::new(TrackedField::mutated("New".to_string()));
```

#### `tests/substrate_pipeline.rs` line 24
```rust
// Before:
sub.atomic_persist(changes).await.unwrap();
// After: (method renamed + signature changed to iterator)
sub.persist(changes.into_iter()).await.unwrap();
```
Note: May also need to update how `changes` is constructed — was `&[EntityChange<'_>]`, now needs an iterator.

### 7. `src/store/CLAUDE.md` — update to remove store::Substrate section

The CLAUDE.md for the store module still documents the old `store::Substrate` trait and `StoreEntityChange`. Update to reflect:
- No `store::Substrate` trait — only `substrate::Substrate`
- No `StoreEntityChange` — uses `EntityChange` from substrate
- New `Load` and `EnsureMutable` message types
- New `EntityClient::load` and `EntityClient::ensure_mutable` methods

---

## Key Architecture Reminders

- **Arc sharing**: EntityServer calls `OnceLock::set()` on store's Arcs directly. Client holds same Arc allocations. No value travels back through channel for Load/EnsureMutable.
- **Single substrate trait**: Only `substrate::Substrate` exists. `store::Substrate` is gone.
- **`TrackedField` constructors**: `new()` (uninitialized clean), `loaded(v)` (initialized clean), `mutated(v)` (initialized dirty).
- **`reset_dirty()`**: Calls `AtomicBool::store(false)` in-place on existing Arc. Does NOT replace the Arc.
- **`ensure_mutable` in EntityServer**: Calls `load_strategy` (internal), loads prerequisites unconditionally, loads field if `mutable_without_load == false`.
- **Accessor pattern**: Check `OnceLock::get().is_none()` → `EntityClient::load(any_ref, field).await?` → read via `get().expect("field not loaded")`.
