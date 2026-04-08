# entity-change-iterator

**Store Layer → `store_layer/change-tracking/`**

---

## Purpose

At persist time the store exposes its pending changes as a lazy iterator rather than a pre-materialised list. The substrate's `persist` default implementation consumes this iterator — pulling one `EntityChange` at a time and mapping it to `AssetRequest`s — without requiring the full change set to be built upfront.

---

## EntityChange

```rust
enum EntityChange<'a> {
    Added(&'a TrackedEntity),
    Modified(&'a TrackedEntity, &'a [String]),  // dirty_fields
    Removed(AnyEntityRef),
}
```

- `Added` — entity was inserted since last persist; all fields are written.
- `Modified` — entity's own fields were mutated since last persist; only the dirty fields are passed so the write path can select the minimal set of assets to rewrite.
- `Removed` — entity was deleted; only the ref is needed to compute which asset paths to delete.

---

## Store::changes()

```rust
impl<S: Substrate> Store<S> {
    fn changes(&self) -> impl Iterator<Item = EntityChange<'_>> {
        self.added.iter()
            .map(|r| EntityChange::Added(self.entries.get(r).unwrap()))
            .chain(self.modified.iter().map(|r| {
                let entity = self.entries.get(r).unwrap();
                EntityChange::Modified(entity, entity.dirty_fields())
            }))
            .chain(self.removed.iter().map(|r| EntityChange::Removed(r.clone())))
    }
}
```

Yields added entities first, then modified, then removed. Traversal order within each group is unspecified — the substrate's executor is responsible for any ordering required for atomicity (e.g., LCA computation for RepoSubstrate).

---

## Why a Generator Instead of a Visitor

The previous visitor (push) model had the store driving callbacks into the substrate. The generator (pull) model inverts this: the substrate consumes changes at its own pace. This is the natural Rust iterator pattern, composes cleanly with the `persist` default implementation on the `Substrate` trait, and eliminates the need for a separate `DirtyEntityVisitor` trait.

---

## Added vs Modified

The `EntityChange` enum makes the distinction explicit so the substrate can select the correct write op per AssetKind (`Post` vs `Put` for `distinguishes_create: true` substrates). The Store layer does not conflate the two.
