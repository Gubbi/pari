# store-entity-lifecycle

**Owning layer: `store`**

---

## Purpose

Documents the full entity state model, the valid operations per state, and the two undo operations introduced alongside `unload`: `undo_commit` and `unload`.

---

## State Model

Entities have two independent axes of state:

**Primary state** — mutually exclusive:

| State     | Meaning                                                |
|-----------|--------------------------------------------------------|
| `absent`  | Not in `entities`                                      |
| `added`   | In `entities` + `added` set; never been persisted      |
| `modified`| In `entities` + `modified` set; dirty since last persist |
| `clean`   | In `entities`; no pending changes                      |
| `removed` | Evicted from `entities`; retained in `removed` set     |

**Checkout overlay** — independent boolean:

| Overlay        | Meaning                                              |
|----------------|------------------------------------------------------|
| `checked_out`  | Entity is locked for exclusive mutation by a caller  |

Any primary state except `absent` and `removed` may carry the `checked_out` overlay. Valid combinations: `added`, `added + checked_out`, `modified`, `modified + checked_out`, `clean`, `clean + checked_out`.

---

## Operation Preconditions

```
Operation      Preconditions                             Error if violated
─────────────────────────────────────────────────────────────────────────
insert         absent  OR  removed                       AlreadyExists
remove         added, modified, or clean (not checked    CheckedOut
               out); no-op if already removed
checkout       added, modified, or clean (not checked    AlreadyCheckedOut /
               out); does not trigger a load             EntityNotFound
commit         checked_out (any primary state)           NotCheckedOut
undo_checkout  checked_out (any primary state)           NotCheckedOut
undo_commit    modified (not checked_out)                WrongState
unload         clean (not checked_out)                   WrongState
```

`remove` is a no-op — not an error — when the entity is already in `removed`. All other violations return errors.

---

## Transitions

```
absent   ──insert()──▶  added
removed  ──insert()──▶  added  (transition logic: remove-then-add = net added)

added    ──remove()──▶  absent  (transition logic: add-then-remove = no net change)
modified ──remove()──▶  removed
clean    ──remove()──▶  removed

any      ──checkout()──▶  same primary state + checked_out overlay

added    + checked_out  ──commit()──▶  added   (dirty fields cleared)
modified + checked_out  ──commit()──▶  modified
clean    + checked_out  ──commit()──▶  modified

any + checked_out  ──undo_checkout()──▶  same primary state (overlay removed, changes dropped)

modified  ──undo_commit()──▶  clean  (entity replaced with stub; removed from modified)
added     ──undo_commit()──▶  absent (entity removed entirely; no persisted state to revert to)

clean     ──unload()──▶  clean  (entity replaced with stub; fields will reload lazily)
```

---

## undo_commit

```rust
EntityClient::undo_commit(any_ref: AnyEntityRef) -> Result<(), UndoError>
```

Reverts an entity to its last persisted state. Semantics: "this commit never happened."

Inside `EntityServer`:

1. Verify precondition — entity must be in `modified` or `added`, and not in `checked_out`. Return `Err(WrongState)` otherwise.
2. If entity is in `added`:
   - Evict from `entities`. Remove from `added`. Return `Ok(())`.
3. If entity is in `modified`:
   - Replace entity in `entities` with a fresh stub (all OnceLock fields uninitialized, `dirty = false`).
   - Remove `any_ref` from `modified`.
   - Return `Ok(())`.

After `undo_commit` on `modified`, the entity is in `clean` stub state — subsequent field accesses trigger lazy load from the substrate, producing the last persisted values.

---

## unload

```rust
EntityClient::unload(any_ref: AnyEntityRef) -> Result<(), UndoError>
```

Evicts loaded field data from a clean entity, returning it to stub state. Does not affect change tracking.

Inside `EntityServer`:

1. Verify precondition — entity must be in `clean` state (in `entities`, not in `added`, `modified`, `removed`, or `checked_out`). Return `Err(WrongState)` otherwise.
2. Replace entity in `entities` with a fresh stub (all OnceLock fields uninitialized, `dirty = false`).
3. Return `Ok(())`.

Use `unload` to release memory for entities whose data is no longer needed, without marking them as changed.

---

## UndoError

```rust
enum UndoError {
    WrongState { hint: Option<String> },
    StoreUnavailable(StoreError),
}
```
