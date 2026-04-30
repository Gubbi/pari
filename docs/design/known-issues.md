# Known Issues

Acknowledged gaps in the design. These are not open questions — they
are explicitly deferred limitations, captured here so the information
is not lost and so tests do not pin behavior the design has not
promised.

---

## KI-1 · Removal without reverse-reference queries

Removing an entity referenced by another entity leaves the store in a
state where the surviving entity cites a non-existent target. For
example, removing a `Role` cited by a `Workflow` step's RACI leaves the
workflow with a dangling `EntityRef`. The store layer has no
reverse-reference index to detect or prevent this at removal time, and
no scrub at load time.

The removal itself is recorded by `EntityChange::Removed` (see
[src/store/lib/change.rs:15](../../src/store/lib/change.rs)) and
propagated to substrate, but nothing checks what cites the target
before the removal is accepted.

**Deferred to:** a future "reverse index" or "safe remove" proposal.

**Test implication:** do not pin "removing a referenced entity is
rejected" — the design does not promise that. Whatever observable
behavior exists today (silent dangling ref) is incidental, not
contractual.

---

## KI-2 · Staleness from external edits

`TrackedField<T>` uses a write-once load path
([src/entity/tracked/tracked_field.rs:50](../../src/entity/tracked/tracked_field.rs)):
once a field is `initialize`d from substrate, subsequent reads return
the same value. If the underlying substrate artifact (e.g. a
`RepoSubstrate` markdown file) is edited externally after that point,
the staleness is invisible until the entity is explicitly `unload`ed
and re-resolved.

This is a design consequence of the write-once load model. The
guarantee that loaded values do not change underneath a holder is what
makes accessor code safe and removes the need for re-validation on
every read. The tradeoff is that external mutations are not visible
without explicit invalidation.

A full resolution requires:

- A reverse mapping from substrate artifact (e.g. filesystem path) to
  `EntityRef`, the inverse of the `RepoSubstrate` location resolver.
- A mechanism to invalidate a specific `TrackedField` (clear the cell)
  without replacing the entity wholesale.
- Re-validation of the invalidated entity after reload.

**Deferred to:** a future file-watcher integration proposal.

**Test implication:** do not pin "external edit between resolve and
commit is detected." A commit after an external edit will write back
the previously-loaded value; that is the design. Manual `unload`
followed by re-resolve is the contract for picking up external edits,
and the happy path of that flow is covered functionally.

---

## KI-3 · Concurrent edits and MVCC

`RepoSubstrate` performs an atomic swap (LCA + `.part/` directory +
`fs::rename`) so a single process cannot produce a torn write. It does
nothing to coordinate with a second process editing the same artifact
concurrently — the last writer wins, with no version check or conflict
detection.

A different substrate (a database backend with version columns,
optimistic locking, or MVCC) could offer stronger guarantees. None is
planned for `RepoSubstrate`.

**Deferred to:** multi-process collaboration is out of scope.

**Test implication:** no concurrent-process scenarios. Single-process
concurrency (multiple `EntityServer` instances over one
`StoreManager`) is serialized by the singleton actor and is a separate
concern.
