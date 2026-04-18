//! State, lifecycle, and in-memory substrate primitive errors.

use pari_macros::primitive_with_fields;

/// Only a subset of the requested persistence operations completed successfully.
#[primitive_with_fields]
pub struct PartialPersistFailed {
    failed_operation_count: usize,
}

/// A change set described a state transition that the persistence contract cannot honor.
#[primitive_with_fields]
pub struct ImpossibleChangeSet {
    entity_ref: String,
    change_kind: String,
}

/// A field that must already be loaded or initialized was still unavailable.
#[primitive_with_fields]
pub struct UnloadedRequiredField {
    field: String,
}

/// An injected shared storage handle did not satisfy the substrate's expected contract.
#[primitive_with_fields]
pub struct InconsistentInjectedStorageHandle {
    storage_kind: String,
}

/// Preloaded in-memory asset state could not support coherent substrate operations.
#[primitive_with_fields]
pub struct IncompatiblePreloadedAssetState {
    asset_path: String,
    reason: String,
}

/// A no-load substrate was asked to materialize entity data.
#[primitive_with_fields]
pub struct UnsupportedLoad {
    entity_ref: String,
    substrate_kind: String,
}
