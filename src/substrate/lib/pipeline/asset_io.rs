//! Executor input/output vocabulary.
//!
//! The executor is the only pipeline component that does I/O; it
//! consumes a batch of [`AssetRequest`]s and produces a matching batch
//! of [`AssetResponse`]s.

/// Per-asset operation the executor performs at the resolved location.
///
/// `Put` is a full write; `Post` is a create when the asset kind
/// distinguishes creates; `Patch` is a partial write when the asset
/// kind supports it. `Get` / `Head` / `Delete` are self-explanatory.
pub enum AssetOp<E> {
    Put(E),
    Post(E),
    Patch(E),
    Delete,
    Get,
    Head,
}

/// One executor task: where to act (`location`) and what to do (`op`).
pub struct AssetRequest<L, E> {
    pub location: L,
    pub op: AssetOp<E>,
}

/// Executor output aligned with the input batch. `Done` acknowledges
/// writes and deletes; `Data` carries a fetched payload; `Exists`
/// carries the result of a `Head`.
pub enum AssetResponse<E> {
    Done,
    Data(E),
    Exists(bool),
}
