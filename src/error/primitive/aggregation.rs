//! Batch-shape primitive errors.

use pari_macros::primitive_with_fields;

/// A batch was created even though the caller promised at least one failure item.
#[primitive_with_fields]
pub struct EmptyBatch {
    batch_kind: String,
}

/// A batch combined failures from incompatible operation contexts.
#[primitive_with_fields]
pub struct HeterogeneousBatch {
    batch_kind: String,
    conflict: String,
}
