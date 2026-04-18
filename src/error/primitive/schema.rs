//! Schema and field-selection primitive errors.

use pari_macros::{primitive_message_only, primitive_with_fields};

/// A requested field does not exist in the schema-backed surface.
#[primitive_with_fields]
pub struct UnknownSchemaField {
    field: String,
}

/// More than one asset mapping claimed ownership of the same field.
#[primitive_with_fields]
pub struct AmbiguousFieldMapping {
    field: String,
    asset_count: usize,
}

/// Declared asset dependencies did not form a coherent dependency graph.
#[primitive_with_fields]
pub struct InvalidAssetDependencyGraph {
    field: String,
    dependency: String,
}

/// Field prerequisite expansion encountered a dependency cycle.
#[primitive_with_fields]
pub struct CyclicPrerequisiteChain {
    field: String,
    cycle: Vec<String>,
}

/// The schema could not select a coherent asset set for the requested field operation.
#[primitive_with_fields]
pub struct AssetSelectionFailed {
    operation: String,
    field: String,
}

/// Cached field-index metadata no longer matched the schema definition.
#[primitive_with_fields]
pub struct CorruptedFieldIndexCache {
    field: String,
}

/// A prerequisite declaration referenced fields or assets that do not form a valid plan.
#[primitive_with_fields]
pub struct InvalidPrerequisiteDeclaration {
    field: String,
    prerequisite: String,
}

/// A field was classified as mutable-without-load even though its asset shape forbids that.
#[primitive_with_fields]
pub struct InvalidMutableWithoutLoadClassification {
    field: String,
}

/// Asset selection produced conflicting or duplicate candidates for a field.
#[primitive_with_fields]
pub struct ConflictingAssetSelection {
    field: String,
    asset_count: usize,
}

/// Schema metadata was internally contradictory for the referenced component.
#[primitive_with_fields]
pub struct InconsistentSchemaMetadata {
    schema_component: String,
    reason: String,
}

/// The selected asset kind cannot express the requested partial write behavior.
#[primitive_with_fields]
pub struct UnsupportedPartialWriteMapping {
    field: String,
    asset_kind: String,
}

/// Shared state or bookkeeping was corrupted in a way that violates expected invariants.
#[primitive_message_only]
pub struct SharedStateCorrupted;
