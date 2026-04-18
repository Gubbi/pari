//! Validation-rule and validation-dispatch primitive errors.

use pari_macros::primitive_with_fields;

/// Validation was asked to evaluate a field that is not represented in the schema.
#[primitive_with_fields]
pub struct InvalidValidationFieldSelection {
    field: String,
}

/// The tracked entity and validation schema did not correspond to the same entity type.
#[primitive_with_fields]
pub struct TrackedEntitySchemaMismatch {
    tracked_kind: String,
    schema_kind: String,
}

/// A structural validation rule rejected the candidate state.
#[primitive_with_fields]
pub struct StructuralRuleViolation {
    field_path: String,
    rule_kind: String,
}

/// A semantic validation rule rejected the candidate state.
#[primitive_with_fields]
pub struct SemanticRuleViolation {
    field_path: String,
    rule_kind: String,
}

/// A cross-entity validation rule rejected the candidate state.
#[primitive_with_fields]
pub struct CrossEntityRuleViolation {
    field_path: String,
    rule_kind: String,
}

/// Validation dispatch could not route the requested rule set coherently.
#[primitive_with_fields]
pub struct ValidationDispatchFailed {
    tracked_kind: String,
    reason: String,
}

/// A type-erased tracked wrapper could not dispatch to the correct validation path.
#[primitive_with_fields]
pub struct TrackedWrapperDispatchFailed {
    tracked_kind: String,
}

/// A tracked wrapper resolved to the wrong tracked subtype for the underlying entity kind.
#[primitive_with_fields]
pub struct WrongTrackedSubtype {
    expected_kind: String,
    actual_kind: String,
}

/// Validation schema maps did not describe a coherent field inventory.
#[primitive_with_fields]
pub struct ValidationSchemaInconsistency {
    schema_component: String,
    reason: String,
}

/// Validation rule maps named fields in ways that cannot be reconciled.
#[primitive_with_fields]
pub struct ConflictingValidationFieldNames {
    field: String,
    conflict: String,
}

/// A reported nested violation path could not be normalized safely.
#[primitive_with_fields]
pub struct InvalidValidationSubPath {
    sub_path: String,
}

/// Concatenating a field path and nested path produced an impossible output path.
#[primitive_with_fields]
pub struct ImpossibleValidationPath {
    field: String,
    sub_path: String,
}

/// A scalar value did not satisfy the primitive validation format required by the rule.
#[primitive_with_fields]
pub struct MalformedScalarValue {
    field_path: String,
    rule_kind: String,
}

/// A collection value did not satisfy the shape required by the rule.
#[primitive_with_fields]
pub struct MalformedCollectionValue {
    field_path: String,
    rule_kind: String,
}

/// A value violated a required naming convention or lexical contract.
#[primitive_with_fields]
pub struct NamingFormatViolation {
    field_path: String,
    rule_kind: String,
}

/// A collection contained entries that were required to be unique.
#[primitive_with_fields]
pub struct DuplicateEntryViolation {
    field_path: String,
    rule_kind: String,
}

/// A required field or collection entry was empty when non-empty content was required.
#[primitive_with_fields]
pub struct EmptyRequiredValue {
    field_path: String,
    rule_kind: String,
}

/// A workflow or dependency graph inside the entity was not semantically coherent.
#[primitive_with_fields]
pub struct WorkflowGraphInconsistency {
    field_path: String,
    reason: String,
}

/// A declared dependency pointed to a step, task, or relationship that is not valid.
#[primitive_with_fields]
pub struct IllegalDependencyReference {
    field_path: String,
    reference: String,
}

/// A referenced state transition was not allowed by the workflow semantics.
#[primitive_with_fields]
pub struct IllegalStateTransitionReference {
    field_path: String,
    reference: String,
}

/// Rejection-handling configuration pointed to an invalid or impossible target.
#[primitive_with_fields]
pub struct InvalidOnRejectTarget {
    field_path: String,
    target: String,
}

/// The same semantic relationship was declared more than once in a conflicting way.
#[primitive_with_fields]
pub struct DuplicateSemanticRelationship {
    field_path: String,
    relationship: String,
}

/// A required companion concept or state was absent.
#[primitive_with_fields]
pub struct MissingRequiredCompanionState {
    field_path: String,
    required_state: String,
}

/// A required referenced entity did not exist.
#[primitive_with_fields]
pub struct ReferencedEntityAbsent {
    field_path: String,
    entity_ref: String,
}

/// A reference resolved to an entity kind incompatible with the field semantics.
#[primitive_with_fields]
pub struct ReferencedEntityKindMismatch {
    field_path: String,
    entity_ref: String,
    actual_kind: String,
}

/// A required set of related references was incomplete.
#[primitive_with_fields]
pub struct IncompleteReferenceSet {
    field_path: String,
    missing_reference: String,
}

/// A referenced definition existed but did not match the consuming entity's expectations.
#[primitive_with_fields]
pub struct ReferencedDefinitionMismatch {
    field_path: String,
    entity_ref: String,
    reason: String,
}

/// Validation error aggregation could not preserve a coherent combined error set.
#[primitive_with_fields]
pub struct ValidationAggregationFailed {
    reason: String,
}

/// Validation aggregation produced a mix of validation kinds that the contract cannot group.
#[primitive_with_fields]
pub struct IncompatibleValidationKindMix {
    expected_kind: String,
    actual_kind: String,
}
