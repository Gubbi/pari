# known-issues

Acknowledged gaps in the design — not open questions, but explicitly deferred. Captured here to prevent the information from being lost.

---

## KI-1 · Removal without reverse-reference queries

Removing a `Role` referenced in a `Task`'s `raci` leaves the store in an invalid state — the Task now references a non-existent Role. The store has no reverse-reference index to prevent or detect this.

**Current status:** `Store::removed: HashSet<AnyEntityRef>` (see [store-structure](store_layer/entity-store/store-structure.md)) records the removal, but there is no mechanism to check what other entities reference the removed entity before or after removal.

**Deferred to:** A future "reverse index" or "safe remove" proposal.

---

## KI-2 · Staleness from external edits (OnceLock write-once consequence)

When a field is accessed, its `OnceLock` is initialized on first load. If the underlying substrate file is then edited externally, a subsequent accessor call finds the `OnceLock` already initialized and returns the stale value silently.

This is a **design consequence of the write-once load model** — the guarantee that loaded values never change is what makes accessor code safe and avoids re-validation. The tradeoff is that external mutations are invisible without explicit invalidation.

**Deferred to:** A future file-watcher integration proposal. Resolution requires:
- A reverse mapping from filesystem path → `AnyEntityRef` (inverse of LocationResolver)
- A mechanism to invalidate specific `TrackedField` values (clear the `OnceLock`) without replacing the entity
- Re-validation of the invalidated entity after reload

---

## KI-3 · Concurrent edits / MVCC

If two processes modify the same entity via the substrate concurrently, the last writer wins. For `RepoSubstrate`, the atomic swap (LCA + `.part/` + `fs::rename`) prevents torn writes within a single process, but does not prevent a second process from overwriting a commit.

A database substrate could use optimistic locking (version columns) or MVCC. No conflict resolution is implemented or planned for `RepoSubstrate`.

**Deferred to:** Multi-process collaboration is an acknowledged out-of-scope limitation.
