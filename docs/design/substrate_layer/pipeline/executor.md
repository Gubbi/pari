# executor

**Substrate Layer → `substrate_layer/pipeline/`**

---

## Purpose

`Executor` executes a batch of `AssetRequest`s against the substrate's backing store. It is the boundary between the framework's pipeline logic and the substrate's I/O. One batch interface — write batches are all-or-nothing.

---

## Trait

```rust
trait Executor {
    type Location;
    type Encoded;

    fn execute(
        &self,
        ops: Vec<AssetRequest<Self::Location, Self::Encoded>>,
    ) -> Result<Vec<AssetResponse<Self::Encoded>>, Vec<ExecutorError>>;
}
```

The batch is submitted as a `Vec<AssetRequest>`. Responses are returned in the same order. Errors are collected — not short-circuited — so the caller gets a complete picture of all failures.

---

## AssetResponse

```rust
enum AssetResponse<E> {
    Done,           // write completed (Put, Post, Patch, Delete)
    Data(E),        // read result (Get)
    Exists(bool),   // existence result (Head)
}
```

---

## ExecutorError

```rust
struct ExecutorError {
    location: String,
    message: String,
}
```

---

## Batch Semantics

**Write batches** (from `persist`): the executor is responsible for atomicity. If any write fails, no partial state should be visible. For RepoSubstrate this is enforced via the LCA-based `.part/` staging + `fs::rename` pattern (see [repo-executor](../repo-substrate/repo-executor.md)).

**Read batches** (from `exists` and load operations): not required to be atomic. Individual failures are surfaced as errors in the response vec.

---

## Mixed Batches

A single batch may contain a mix of op types (Get + Head + Put). The executor dispatches each request by `AssetOp` variant. The `persist` default impl assembles the batch; the executor executes it without needing to understand entity types or schema structure.
