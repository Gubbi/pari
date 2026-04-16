# asset-op-vocabulary

**Owning layer: `substrate`**

---

## Purpose

`AssetOp` is the HTTP-inspired vocabulary of operations the pipeline can issue against a substrate. `AssetRequest` wraps an op with its target location. The vocabulary is substrate-agnostic — each substrate implements only the ops its `AssetKind` constants declare as supported.

---

## AssetOp

```rust
enum AssetOp<P> {
    Post(P),    // create new; error if already exists
    Put(P),     // create or replace (idempotent, full asset)
    Patch(P),   // partial update (subset of fields); error if not exists
    Delete,     // remove; no-op if not exists
    Get,        // full read
    Head,       // existence / metadata check only
}
```

Generic over payload type `P` — the substrate's `Encoded` type.

---

## AssetRequest

```rust
struct AssetRequest<L, P> {
    location: L,
    op: AssetOp<P>,
}
```

One request per asset per operation.

---

## Op Matrix

Which ops each substrate uses, derived from `AssetKind` capabilities:

```
                Post   Put   Patch   Delete   Get   Head
RepoSubstrate    —      ✓     —       ✓       ✓     ✓
DynamoDB (ex)    ✓      ✓     ✓       ✓       ✓     ✓
```

RepoSubstrate never uses `Post` (no create/upsert distinction — `Put` always) or `Patch` (full-file rewrite only).

---

## Op Selection by AssetMapper

The Orchestrator's AssetMapper selects the op for each asset based on entity state and `AssetKind`:

| Entity state | `distinguishes_create` | Op |
|---|---|---|
| In `added` | false | `Put` |
| In `added` | true | `Post` |
| In `modified` | false | `Put` |
| In `modified`, `supports_partial: true` | — | `Patch` |
| In `removed` | — | `Delete` |

Read operations use `Get` (load full asset) or `Head` (exists check).
