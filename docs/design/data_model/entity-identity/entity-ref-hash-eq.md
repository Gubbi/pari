# entity-ref-hash-eq

**Data Model → `data_model/entity-identity/`**

---

## Purpose

`EntityRef` implements `Hash` and `Eq` over three components: entity kind, id, and parent chain. All three are required to guarantee no collisions across entity types or parent contexts.

---

## Why All Three Components

**Kind** — prevents collision across entity types:
```
EntityRef<Role>("eng-lead")  ≠  EntityRef<Hook>("eng-lead")
// same id, different kind → must be distinct
```

**Id** — the entity's own identifier within its kind and parent context.

**Parent chain** — prevents collision across parent contexts:
```
EntityRef<Task>("WriteProposal", Workflow("Initiative"))
  ≠
EntityRef<Task>("WriteProposal", Workflow("Delivery"))
// same kind + id, different parent → must be distinct
```

---

## Implementations

```rust
impl<T: Entity, P: ParentKind + Hash> Hash for EntityRef<T, P> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        T::KIND.hash(state);   // kind baked in at compile time
        self.id.hash(state);
        self.parent.hash(state);
    }
}

impl<T: Entity, P: ParentKind + PartialEq> PartialEq for EntityRef<T, P> {
    fn eq(&self, other: &Self) -> bool {
        // T::KIND is the same for both (same type) — no runtime check needed
        self.id == other.id && self.parent == other.parent
    }
}

impl<T: Entity, P: ParentKind + Eq> Eq for EntityRef<T, P> {}
```

`T::KIND` is included in `Hash` but not in `PartialEq` — because `PartialEq` is only defined between `EntityRef<T, P>` values of the *same* T. Two refs of different entity types are different Rust types; `==` between them is a compile error, not a runtime false. Hash must still include KIND because type-erased collections (e.g. `AnyEntityRef`) store refs of different kinds in the same map.

---

## `NoParent` Contribution

`NoParent` is a ZST. Its `Hash` impl contributes nothing to the hash state. Its `PartialEq` is always `true`. This means top-level entity refs hash and compare purely on kind + id.

---

## Consistency Invariant

Rust requires: if `a == b` then `hash(a) == hash(b)`. Both impls use the same fields in the same order — this invariant holds.
