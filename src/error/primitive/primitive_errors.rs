//! `PrimitiveError` — the Primitive tier of Pari's error chain.
//!
//! Primitives are the leaf-most evidence in Pari's three-tier error model.
//! Where an `ActivityError` says *what subsystem outcome* occurred and how a
//! caller should react, a `PrimitiveError` says *exactly what broke* and where.
//! A primitive is always the tail of a `source()` chain — nothing is wrapped
//! below it.
//!
//! # Contract
//!
//! Every primitive variant shares the same fixed diagnostic shape, held in
//! [`PrimitiveContext`]:
//!
//! | Field | Who provides it | Purpose |
//! |---|---|---|
//! | `message: String` | Caller, at construction | Human-readable explanation of the leaf failure. |
//! | `location: ErrorLocation` | Auto-captured (or overridden) | The source location the failure points at. |
//! | `span_trace: SpanTrace` | Auto-captured at construction | Tracing context the moment the primitive was built. |
//! | `backtrace: Backtrace` | Auto-captured at construction | Stack backtrace the moment the primitive was built. |
//!
//! Beyond the fixed diagnostics, each variant declares its own **typed detail
//! fields** — `path: String`, `line: usize`, `raw_snippet: String`, and so on.
//! These are the primitive-specific evidence, and they are the only thing an
//! author writes when adding a new primitive. Everything else — `thiserror`
//! plumbing, `Display`, the diagnostic fields, the OTel emission, the
//! constructors, the accessors — is generated.
//!
//! # Why capture is at construction
//!
//! `SpanTrace` and `Backtrace` are only useful when they reflect the moment
//! the failure was observed. Capturing them at higher tiers (Activity, Job)
//! would point at wrapping code instead of the real leaf. So the primitive
//! captures them once, at construction, and no other tier re-captures — the
//! chain carries that original evidence up to the caller untouched.
//!
//! # Construction
//!
//! Each variant has two generated constructors:
//!
//! - `new(message, ...fields)` — auto-captures `location` at the call site
//!   via `#[track_caller]`. Use this when the construction site *is* the
//!   meaningful location.
//! - `new_with_location(location, message, ...fields)` — caller supplies the
//!   location. Use this when the meaningful location is elsewhere (a line
//!   inside a parsed document, a failing asset path) and the construction
//!   site would mislead.
//!
//! # Why one centralized enum
//!
//! Primitives are reused across layers — the same `AssetWrite` variant can be
//! the leaf of a persist activity or a cleanup activity, for instance. Keeping
//! them in one enum avoids duplicating identity-check, transport, codec, and
//! I/O leaf types per subsystem. It also makes `err.as_error::<PrimitiveError>()`
//! a single, stable typed entry point for callers that want leaf-level
//! inspection.
//!
//! # Usage
//!
//! Pure `lib/` components return `PrimitiveError` directly. Orchestration code
//! wraps those into `ActivityError` variants at the layer boundary. Tests can
//! assert against concrete primitive variants via `err.as_error::<...>()` or
//! by matching on the enum.
//!
//! Generation mechanics live in
//! [`pari-macros::primitive_error_enum`](../../../../pari-macros/src/primitive_error_enum.rs).

use std::collections::HashMap;

use pari_macros::primitive_errors;

use crate::error::{ErrorLayer, ErrorLocation, OTelEmit, PrimitiveDetail};

/// The fixed diagnostic payload carried by every primitive variant.
///
/// Authors never construct this directly — primitive generation stores and
/// populates it automatically. It is exposed so callers can read the common
/// diagnostics uniformly regardless of which specific primitive they have.
#[derive(Debug)]
pub struct PrimitiveContext {
    pub(crate) message: String,
    pub(crate) location: ErrorLocation,
    pub(crate) span_trace: tracing_error::SpanTrace,
    pub(crate) backtrace: std::backtrace::Backtrace,
}

primitive_errors! {
    /// A batch was created even though the caller promised at least one failure item.
    EmptyBatch { batch_kind: String }
    /// A batch combined failures from incompatible operation contexts.
    HeterogeneousBatch { batch_kind: String, conflict: String }
    /// Multiple primitive errors aggregated from a single batch operation.
    BatchedErrors { batch_kind: String, errors: Vec<PrimitiveError> }

    /// The selected schema slots cannot be encoded together into one valid asset body.
    UnsupportedSlotComposition { slot: String, field: String }
    /// Frontmatter content could not be serialized into the encoded document format.
    FrontmatterSerialization { field: String, reason: String }
    /// A section payload could not be rendered into the target document body.
    SectionRendering { section: String, field: String }
    /// An encoded frontmatter block could not be parsed as a valid frontmatter structure.
    MalformedFrontmatter { raw_snippet: String }
    /// A frontmatter block was present but was not valid YAML.
    InvalidYamlFrontmatter { raw_snippet: String }
    /// Parsed YAML frontmatter could not be converted into the expected JSON shape.
    InvalidYamlJsonConversion { raw_snippet: String }
    /// A decoded section body did not match the shape required by the schema slot.
    UnsupportedSectionBodyShape { section: String, body_shape: String }
    /// Multiple headings or sections collapsed into the same reconstructed field mapping.
    DuplicateHeadingCollision { heading: String }
    /// The document body did not contain enough valid structure to reconstruct a schema slot.
    UnreconstructableSchemaSlot { slot: String, field: String }

    /// A top-level entity identifier did not satisfy the required identifier format.
    InvalidTopLevelIdentifierFormat { id: String }
    /// An identifier was syntactically valid but belongs to a reserved identity space.
    ReservedIdentifierValue { id: String }
    /// An identifier could not be normalized into the canonical stored form.
    IdentifierCanonicalization { id: String, reason: String }
    /// An embedded child identifier did not satisfy the required identifier format.
    InvalidEmbeddedIdentifierFormat { id: String, child_kind: String }
    /// A parent identity was incompatible with the requested child entity kind.
    ParentChildKindMismatch { parent_kind: String, child_kind: String }
    /// Required parent identity data was missing from a child reference.
    MissingParentIdentityComponent { child_kind: String, component: String }
    /// A parent chain described an impossible or cyclic entity hierarchy.
    ImpossibleParentChain { child_kind: String, parent_path: String }
    /// A serialized reference omitted one or more fields required to reconstruct identity.
    MissingRequiredReferenceField { field: String }
    /// A serialized reference contained an entity kind tag that is not recognized.
    UnknownEntityKindTag { entity_kind: String }
    /// Parent identity data was present but could not be parsed into a valid parent reference.
    MalformedParentPayload { parent_kind: String, reason: String }
    /// A reference payload combined a valid child kind with an incompatible parent kind.
    ReferenceParentKindMismatch { parent_kind: String, child_kind: String }
    /// A serialized identifier field existed but used the wrong scalar or structured representation.
    IdentifierPayloadTypeMismatch { field: String, actual_type: String }
    /// A serialized reference payload contained overlapping or contradictory identity data.
    ConflictingReferenceFields { field: String, conflict: String }
    /// Parent identity was mandatory but the required parent object was missing.
    MissingRequiredParentObject { child_kind: String }
    /// A top-level entity payload incorrectly included parent identity data.
    UnexpectedParentOnTopLevelEntity { entity_kind: String }
    /// A nested parent reference was present but structurally invalid.
    MalformedNestedParentReference { parent_ref: String, reason: String }

    /// A storage existence check could not be completed for the requested asset path.
    ExistenceCheck { asset_path: String, operation: String }
    /// A storage read failed while trying to fetch an asset.
    AssetRead { asset_path: String, operation: String }
    /// A storage write failed while trying to persist an asset.
    AssetWrite { asset_path: String, operation: String }
    /// A storage delete failed while trying to remove an asset.
    AssetDelete { asset_path: String }
    /// The substrate root directory could not be created or initialized.
    RootDirectoryCreation { root: String }
    /// Traversal of stale substrate artifacts failed before cleanup could complete.
    StaleCleanupTraversal { path: String }
    /// A stale substrate artifact was identified but could not be deleted.
    StaleCleanupDeletion { path: String }
    /// A required directory could not be read during substrate or executor work.
    DirectoryRead { path: String }
    /// A directory entry could not be enumerated or inspected during traversal.
    DirectoryEntryRead { path: String }
    /// A file could not be read from the backing storage.
    FileRead { asset_path: String }
    /// A parent directory required for a write could not be created.
    ParentDirectoryCreation { directory_path: String }
    /// A file could not be written to the backing storage.
    FileWrite { asset_path: String }
    /// A file could not be deleted from the backing storage.
    FileDelete { asset_path: String }
    /// The executor received an operation kind that it does not implement.
    UnsupportedExecutorOperation { operation: String, asset_path: String }
    /// The backing storage rejected access to the requested asset path.
    PathPermissionDenied { asset_path: String, operation: String }
    /// A requested asset was not present in the backing storage.
    MissingAsset { asset_path: String }

    /// A path template could not be resolved into a concrete asset location.
    PathResolution { path_template: String, reason: String }
    /// A configured root path was syntactically or semantically unusable.
    InvalidRootPath { root: String }
    /// A path template referenced a placeholder that was not available in the input projection.
    UnresolvedTemplatePlaceholder { path_template: String, placeholder: String }
    /// Parent-derived path data required by a template was missing.
    MissingParentBaseData { path_template: String, parent_kind: String }
    /// A configured path template was invalid before resolution could succeed.
    InvalidPathTemplate { path_template: String }
    /// Path resolution would have produced a location outside the permitted substrate root.
    PathEscapesSubstrateRoot { root: String, resolved_path: String }

    /// A returned payload shape did not match the operation's expected response contract.
    ResponseShapeMismatch { operation: String, expected: String, actual: String }
    /// A tracked entity could not be projected into the payload shape required by a lower boundary.
    EntityProjection { entity_ref: String, reason: String }
    /// Encoded asset content could not be decoded into field values.
    AssetDecode { asset_kind: String, field: String }
    /// A decoded field map could not be merged back into tracked entity state.
    FieldMapMerge { field: String, reason: String }
    /// A change payload could not be serialized into the required persistence boundary shape.
    ChangePayloadSerialization { entity_ref: String, change_kind: String }
    /// A nested entity reference could not be serialized while building a payload.
    InvalidNestedReferenceSerialization { field: String, entity_ref: String }
    /// A decoded value did not match the type expected for the target field.
    IncompatibleDecodedFieldType { field: String, expected_type: String, actual_type: String }
    /// Dot-path reconstruction collided with an incompatible intermediate path segment.
    DecodedPathSegmentCollision { path: String, segment: String }
    /// A partial payload could not be deserialized into a coherent tracked entity shape.
    PartialPayloadDeserialization { entity_ref: String, reason: String }
    /// Flattened extension keys conflicted with explicit payload keys.
    ConflictingExtensionKeys { key: String }
    /// A payload identified an entity kind different from the expected target kind.
    EntityKindPayloadMismatch { entity_ref: String, expected_kind: String, actual_kind: String }
    /// A payload omitted a field that is required for reconstruction.
    MissingRequiredPayloadField { field: String }
    /// A nested entity reference in an incoming payload failed its identity contract.
    InvalidNestedEntityReference { field: String, entity_ref: String }
    /// The overall payload shape could not be reconciled with the tracked entity definition.
    IncompatibleTrackedEntityShape { entity_ref: String, reason: String }
    /// A scalar value was required but a different payload shape was supplied.
    ExpectedScalarValue { field: String, actual_type: String }
    /// An object value was required but a different payload shape was supplied.
    ExpectedObjectValue { field: String, actual_type: String }
    /// An array value was required but a different payload shape was supplied.
    ExpectedArrayValue { field: String, actual_type: String }
    /// A field payload could not be encoded into JSON for an in-memory or external boundary.
    JsonEncoding { field: String, reason: String }
    /// Extracted field data did not match the shape expected by the codec.
    FieldExtractionShapeMismatch { field: String, expected_shape: String }
    /// A raw JSON payload was malformed and could not be parsed safely.
    MalformedJsonPayload { raw_snippet: String }
    /// A decoded field map did not match the expected field-to-value mapping shape.
    IncompatibleFieldMapShape { field: String, expected_shape: String }

    /// A requested field does not exist in the schema-backed surface.
    UnknownSchemaField { field: String }
    /// More than one asset mapping claimed ownership of the same field.
    AmbiguousFieldMapping { field: String, asset_count: usize }
    /// Declared asset dependencies did not form a coherent dependency graph.
    InvalidAssetDependencyGraph { field: String, dependency: String }
    /// Field prerequisite expansion encountered a dependency cycle.
    CyclicPrerequisiteChain { field: String, cycle: Vec<String> }
    /// The schema could not select a coherent asset set for the requested field operation.
    AssetSelection { operation: String, field: String }
    /// Cached field-index metadata no longer matched the schema definition.
    CorruptedFieldIndexCache { field: String }
    /// A prerequisite declaration referenced fields or assets that do not form a valid plan.
    InvalidPrerequisiteDeclaration { field: String, prerequisite: String }
    /// A field was classified as mutable-without-load even though its asset shape forbids that.
    InvalidMutableWithoutLoadClassification { field: String }
    /// Asset selection produced conflicting or duplicate candidates for a field.
    ConflictingAssetSelection { field: String, asset_count: usize }
    /// Schema metadata was internally contradictory for the referenced component.
    InconsistentSchemaMetadata { schema_component: String, reason: String }
    /// The selected asset kind cannot express the requested partial write behavior.
    UnsupportedPartialWriteMapping { field: String, asset_kind: String }
    /// Shared state or bookkeeping was corrupted in a way that violates expected invariants.
    SharedStateCorrupted { }

    /// Only a subset of the requested persistence operations completed successfully.
    PartialPersist { failed_operation_count: usize }
    /// A change set described a state transition that the persistence contract cannot honor.
    ImpossibleChangeSet { entity_ref: String, change_kind: String }
    /// A field that must already be loaded or initialized was still unavailable.
    UnloadedRequiredField { field: String }
    /// An injected shared storage handle did not satisfy the substrate's expected contract.
    InconsistentInjectedStorageHandle { storage_kind: String }
    /// Preloaded in-memory asset state could not support coherent substrate operations.
    IncompatiblePreloadedAssetState { asset_path: String, reason: String }
    /// A no-load substrate was asked to materialize entity data.
    UnsupportedLoad { entity_ref: String }

    /// The schema registry has no entry for the requested entity kind.
    UnsupportedEntityKind { entity_kind: String }

    /// A request could not be sent across the intended boundary.
    RequestTransportUnavailable { operation: String, boundary: String }
    /// A request channel rejected or lost an outbound request.
    RequestChannelSend { operation: String, boundary: String }
    /// A reply was never delivered on the expected response channel.
    ReplyChannelDropped { operation: String, boundary: String }
    /// The target actor terminated before it could complete the request.
    ActorTerminatedMidRequest { operation: String, actor: String }
    /// A request payload reaching the boundary did not match the expected operation contract.
    MalformedRequestPayload { operation: String, reason: String }
    /// A response shape did not match the request/response protocol for the operation.
    RequestResponseProtocolMismatch { operation: String, expected: String, actual: String }
    /// Internal actor state could not support dispatch or the requested transition.
    ActorStateTransitionInvariantViolation { operation: String, reason: String }

    /// Validation was asked to evaluate a field that is not represented in the schema.
    InvalidValidationFieldSelection { field: String }
    /// The tracked entity and validation schema did not correspond to the same entity type.
    TrackedEntitySchemaMismatch { tracked_kind: String, schema_kind: String }
    /// Validation dispatch could not route the requested rule set coherently.
    ValidationDispatch { tracked_kind: String, reason: String }
    /// A type-erased tracked wrapper could not dispatch to the correct validation path.
    TrackedWrapperDispatch { tracked_kind: String }
    /// A tracked wrapper resolved to the wrong tracked subtype for the underlying entity kind.
    WrongTrackedSubtype { expected_kind: String, actual_kind: String }
    /// Validation schema maps did not describe a coherent field inventory.
    ValidationSchemaInconsistency { schema_component: String, reason: String }
    /// Validation rule maps named fields in ways that cannot be reconciled.
    ConflictingValidationFieldNames { field: String, conflict: String }
    /// Validation error aggregation could not preserve a coherent combined error set.
    ValidationAggregation { reason: String }
    /// Validation aggregation produced a mix of validation kinds that the contract cannot group.
    IncompatibleValidationKindMix { expected_kind: String, actual_kind: String }
    /// Field-level validation errors keyed by field path.
    FieldValidationError { errors: HashMap<String, Vec<PrimitiveError>> }

    /// A scalar value did not satisfy the primitive validation format required by the rule.
    MalformedScalarValue { sub_path: Option<String>, rule_kind: String }
    /// A collection did not satisfy the minimum size or shape required by the rule.
    MalformedCollectionValue { rule_kind: String }
    /// A value violated a required naming convention or lexical contract.
    NamingFormatViolation { sub_path: Option<String>, rule_kind: String }
    /// A collection contained entries that were required to be unique.
    DuplicateEntryViolation { sub_path: String, rule_kind: String }
    /// A required field or collection entry was empty when non-empty content was required.
    EmptyRequiredValue { sub_path: Option<String>, rule_kind: String }
    /// A workflow or dependency graph inside the entity was not semantically coherent.
    WorkflowGraphInconsistency { reason: String }
    /// A declared dependency pointed to a step, task, or relationship that is not valid.
    IllegalDependencyReference { sub_path: String, reference: String }
    /// A referenced state transition was not allowed by the workflow semantics.
    IllegalStateTransitionReference { sub_path: String, reference: String }
    /// Rejection-handling configuration pointed to an invalid or impossible target.
    InvalidOnRejectTarget { sub_path: String, target: String }
    /// The same semantic relationship was declared more than once in a conflicting way.
    DuplicateSemanticRelationship { sub_path: String, relationship: String }
    /// A required companion concept or state was absent.
    MissingRequiredCompanionState { required_state: String }
    /// A required referenced entity did not exist.
    ReferencedEntityAbsent { sub_path: String, entity_ref: String }
    /// A reference resolved to an entity kind incompatible with the field semantics.
    ReferencedEntityKindMismatch { sub_path: String, entity_ref: String, actual_kind: String }
    /// A required set of related references was incomplete.
    IncompleteReferenceSet { sub_path: String, missing_reference: String }
    /// A referenced definition existed but did not match the consuming entity's expectations.
    ReferencedDefinitionMismatch { sub_path: String, entity_ref: String, reason: String }

    /// The requested entity was not found in the store or substrate.
    EntityNotFound { entity_ref: String }
    /// An insert was issued against a ref that already exists in the store.
    EntityAlreadyExists { entity_ref: String }
    /// The entity is already checked out by another operation.
    AlreadyCheckedOut { entity_ref: String }
    /// An undo-checkout was requested but the entity was not checked out.
    EntityNotCheckedOut { entity_ref: String }
    /// An operation was blocked because the entity is still checked out.
    EntityStillCheckedOut { entity_ref: String }
    /// An undo-commit was requested but the entity has no uncommitted changes.
    NoUncommittedChanges { entity_ref: String }
    /// An unload was blocked because the entity has unsaved adds or modifications.
    EntityHasUnsavedChanges { entity_ref: String }
    /// Persist was blocked because one or more entities are still checked out.
    PendingCheckouts { count: usize }
}

impl PrimitiveError {
    pub fn missing_required_reference_field(
        message: impl Into<String>,
        field: impl Into<String>,
    ) -> Self {
        Self::MissingRequiredReferenceField {
            context: Self::context(message),
            field: field.into(),
        }
    }

    pub fn unknown_entity_kind_tag(
        message: impl Into<String>,
        entity_kind: impl Into<String>,
    ) -> Self {
        Self::UnknownEntityKindTag {
            context: Self::context(message),
            entity_kind: entity_kind.into(),
        }
    }

    pub fn missing_required_parent_object(
        message: impl Into<String>,
        child_kind: impl Into<String>,
    ) -> Self {
        Self::MissingRequiredParentObject {
            context: Self::context(message),
            child_kind: child_kind.into(),
        }
    }

    pub fn unexpected_parent_on_top_level_entity(
        message: impl Into<String>,
        entity_kind: impl Into<String>,
    ) -> Self {
        Self::UnexpectedParentOnTopLevelEntity {
            context: Self::context(message),
            entity_kind: entity_kind.into(),
        }
    }

    pub fn malformed_nested_parent_reference(
        message: impl Into<String>,
        parent_ref: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::MalformedNestedParentReference {
            context: Self::context(message),
            parent_ref: parent_ref.into(),
            reason: reason.into(),
        }
    }

    pub fn invalid_validation_field_selection(
        message: impl Into<String>,
        field: impl Into<String>,
    ) -> Self {
        Self::InvalidValidationFieldSelection {
            context: Self::context(message),
            field: field.into(),
        }
    }

    pub fn malformed_scalar_value(
        message: impl Into<String>,
        sub_path: Option<impl Into<String>>,
        rule_kind: impl Into<String>,
    ) -> Self {
        Self::MalformedScalarValue {
            context: Self::context(message),
            sub_path: sub_path.map(Into::into),
            rule_kind: rule_kind.into(),
        }
    }

    pub fn malformed_collection_value(
        message: impl Into<String>,
        rule_kind: impl Into<String>,
    ) -> Self {
        Self::MalformedCollectionValue {
            context: Self::context(message),
            rule_kind: rule_kind.into(),
        }
    }

    pub fn naming_format_violation(
        message: impl Into<String>,
        sub_path: Option<impl Into<String>>,
        rule_kind: impl Into<String>,
    ) -> Self {
        Self::NamingFormatViolation {
            context: Self::context(message),
            sub_path: sub_path.map(Into::into),
            rule_kind: rule_kind.into(),
        }
    }

    pub fn duplicate_entry_violation(
        message: impl Into<String>,
        sub_path: impl Into<String>,
        rule_kind: impl Into<String>,
    ) -> Self {
        Self::DuplicateEntryViolation {
            context: Self::context(message),
            sub_path: sub_path.into(),
            rule_kind: rule_kind.into(),
        }
    }

    pub fn empty_required_value(
        message: impl Into<String>,
        sub_path: Option<impl Into<String>>,
        rule_kind: impl Into<String>,
    ) -> Self {
        Self::EmptyRequiredValue {
            context: Self::context(message),
            sub_path: sub_path.map(Into::into),
            rule_kind: rule_kind.into(),
        }
    }

    pub fn workflow_graph_inconsistency(
        message: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::WorkflowGraphInconsistency {
            context: Self::context(message),
            reason: reason.into(),
        }
    }

    pub fn illegal_dependency_reference(
        message: impl Into<String>,
        sub_path: impl Into<String>,
        reference: impl Into<String>,
    ) -> Self {
        Self::IllegalDependencyReference {
            context: Self::context(message),
            sub_path: sub_path.into(),
            reference: reference.into(),
        }
    }

    pub fn illegal_state_transition_reference(
        message: impl Into<String>,
        sub_path: impl Into<String>,
        reference: impl Into<String>,
    ) -> Self {
        Self::IllegalStateTransitionReference {
            context: Self::context(message),
            sub_path: sub_path.into(),
            reference: reference.into(),
        }
    }

    pub fn invalid_on_reject_target(
        message: impl Into<String>,
        sub_path: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        Self::InvalidOnRejectTarget {
            context: Self::context(message),
            sub_path: sub_path.into(),
            target: target.into(),
        }
    }

    pub fn duplicate_semantic_relationship(
        message: impl Into<String>,
        sub_path: impl Into<String>,
        relationship: impl Into<String>,
    ) -> Self {
        Self::DuplicateSemanticRelationship {
            context: Self::context(message),
            sub_path: sub_path.into(),
            relationship: relationship.into(),
        }
    }

    pub fn missing_required_companion_state(
        message: impl Into<String>,
        required_state: impl Into<String>,
    ) -> Self {
        Self::MissingRequiredCompanionState {
            context: Self::context(message),
            required_state: required_state.into(),
        }
    }

    pub fn referenced_entity_absent(
        message: impl Into<String>,
        sub_path: impl Into<String>,
        entity_ref: impl Into<String>,
    ) -> Self {
        Self::ReferencedEntityAbsent {
            context: Self::context(message),
            sub_path: sub_path.into(),
            entity_ref: entity_ref.into(),
        }
    }

    pub fn referenced_entity_kind_mismatch(
        message: impl Into<String>,
        sub_path: impl Into<String>,
        entity_ref: impl Into<String>,
        actual_kind: impl Into<String>,
    ) -> Self {
        Self::ReferencedEntityKindMismatch {
            context: Self::context(message),
            sub_path: sub_path.into(),
            entity_ref: entity_ref.into(),
            actual_kind: actual_kind.into(),
        }
    }

    pub fn incomplete_reference_set(
        message: impl Into<String>,
        sub_path: impl Into<String>,
        missing_reference: impl Into<String>,
    ) -> Self {
        Self::IncompleteReferenceSet {
            context: Self::context(message),
            sub_path: sub_path.into(),
            missing_reference: missing_reference.into(),
        }
    }

    pub fn referenced_definition_mismatch(
        message: impl Into<String>,
        sub_path: impl Into<String>,
        entity_ref: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::ReferencedDefinitionMismatch {
            context: Self::context(message),
            sub_path: sub_path.into(),
            entity_ref: entity_ref.into(),
            reason: reason.into(),
        }
    }

    // -------------------------------------------------------------------------
    // Substrate layer
    // -------------------------------------------------------------------------

    pub fn empty_batch(message: impl Into<String>, batch_kind: impl Into<String>) -> Self {
        Self::EmptyBatch {
            context: Self::context(message),
            batch_kind: batch_kind.into(),
        }
    }

    pub fn batched_errors(
        message: impl Into<String>,
        batch_kind: impl Into<String>,
        errors: Vec<PrimitiveError>,
    ) -> Self {
        Self::BatchedErrors {
            context: Self::context(message),
            batch_kind: batch_kind.into(),
            errors,
        }
    }

    pub fn entity_projection(
        message: impl Into<String>,
        entity_ref: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::EntityProjection {
            context: Self::context(message),
            entity_ref: entity_ref.into(),
            reason: reason.into(),
        }
    }

    pub fn partial_payload_deserialization(
        message: impl Into<String>,
        entity_ref: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::PartialPayloadDeserialization {
            context: Self::context(message),
            entity_ref: entity_ref.into(),
            reason: reason.into(),
        }
    }

    pub fn unknown_schema_field(message: impl Into<String>, field: impl Into<String>) -> Self {
        Self::UnknownSchemaField {
            context: Self::context(message),
            field: field.into(),
        }
    }

    pub fn missing_asset(message: impl Into<String>, asset_path: impl Into<String>) -> Self {
        Self::MissingAsset {
            context: Self::context(message),
            asset_path: asset_path.into(),
        }
    }

    pub fn file_read(message: impl Into<String>, asset_path: impl Into<String>) -> Self {
        Self::FileRead {
            context: Self::context(message),
            asset_path: asset_path.into(),
        }
    }

    pub fn file_write(message: impl Into<String>, asset_path: impl Into<String>) -> Self {
        Self::FileWrite {
            context: Self::context(message),
            asset_path: asset_path.into(),
        }
    }

    pub fn file_delete(message: impl Into<String>, asset_path: impl Into<String>) -> Self {
        Self::FileDelete {
            context: Self::context(message),
            asset_path: asset_path.into(),
        }
    }

    pub fn path_permission_denied(
        message: impl Into<String>,
        asset_path: impl Into<String>,
        operation: impl Into<String>,
    ) -> Self {
        Self::PathPermissionDenied {
            context: Self::context(message),
            asset_path: asset_path.into(),
            operation: operation.into(),
        }
    }

    pub fn parent_directory_creation(
        message: impl Into<String>,
        directory_path: impl Into<String>,
    ) -> Self {
        Self::ParentDirectoryCreation {
            context: Self::context(message),
            directory_path: directory_path.into(),
        }
    }

    pub fn unsupported_executor_operation(
        message: impl Into<String>,
        operation: impl Into<String>,
        asset_path: impl Into<String>,
    ) -> Self {
        Self::UnsupportedExecutorOperation {
            context: Self::context(message),
            operation: operation.into(),
            asset_path: asset_path.into(),
        }
    }

    pub fn expected_scalar_value(
        message: impl Into<String>,
        field: impl Into<String>,
        actual_type: impl Into<String>,
    ) -> Self {
        Self::ExpectedScalarValue {
            context: Self::context(message),
            field: field.into(),
            actual_type: actual_type.into(),
        }
    }

    pub fn expected_object_value(
        message: impl Into<String>,
        field: impl Into<String>,
        actual_type: impl Into<String>,
    ) -> Self {
        Self::ExpectedObjectValue {
            context: Self::context(message),
            field: field.into(),
            actual_type: actual_type.into(),
        }
    }

    pub fn expected_array_value(
        message: impl Into<String>,
        field: impl Into<String>,
        actual_type: impl Into<String>,
    ) -> Self {
        Self::ExpectedArrayValue {
            context: Self::context(message),
            field: field.into(),
            actual_type: actual_type.into(),
        }
    }

    pub fn json_encoding(
        message: impl Into<String>,
        field: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::JsonEncoding {
            context: Self::context(message),
            field: field.into(),
            reason: reason.into(),
        }
    }

    pub fn malformed_frontmatter(
        message: impl Into<String>,
        raw_snippet: impl Into<String>,
    ) -> Self {
        Self::MalformedFrontmatter {
            context: Self::context(message),
            raw_snippet: raw_snippet.into(),
        }
    }

    pub fn frontmatter_serialization(
        message: impl Into<String>,
        field: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::FrontmatterSerialization {
            context: Self::context(message),
            field: field.into(),
            reason: reason.into(),
        }
    }

    pub fn unsupported_slot_composition(
        message: impl Into<String>,
        slot: impl Into<String>,
        field: impl Into<String>,
    ) -> Self {
        Self::UnsupportedSlotComposition {
            context: Self::context(message),
            slot: slot.into(),
            field: field.into(),
        }
    }

    pub fn root_directory_creation(message: impl Into<String>, root: impl Into<String>) -> Self {
        Self::RootDirectoryCreation {
            context: Self::context(message),
            root: root.into(),
        }
    }

    pub fn directory_read(message: impl Into<String>, path: impl Into<String>) -> Self {
        Self::DirectoryRead {
            context: Self::context(message),
            path: path.into(),
        }
    }

    pub fn directory_entry_read(message: impl Into<String>, path: impl Into<String>) -> Self {
        Self::DirectoryEntryRead {
            context: Self::context(message),
            path: path.into(),
        }
    }

    pub fn stale_cleanup_deletion(message: impl Into<String>, path: impl Into<String>) -> Self {
        Self::StaleCleanupDeletion {
            context: Self::context(message),
            path: path.into(),
        }
    }

    pub fn unsupported_load(message: impl Into<String>, entity_ref: impl Into<String>) -> Self {
        Self::UnsupportedLoad {
            context: Self::context(message),
            entity_ref: entity_ref.into(),
        }
    }

    // -------------------------------------------------------------------------
    // Workspace layer
    // -------------------------------------------------------------------------

    pub fn store_unavailable(message: impl Into<String>) -> Self {
        Self::RequestTransportUnavailable {
            context: Self::context(message),
            operation: "store_request".into(),
            boundary: "entity_server".into(),
        }
    }

    // -------------------------------------------------------------------------
    // Validation layer
    // -------------------------------------------------------------------------

    pub fn field_validation_error(
        message: impl Into<String>,
        errors: HashMap<String, Vec<PrimitiveError>>,
    ) -> Self {
        Self::FieldValidationError {
            context: Self::context(message),
            errors,
        }
    }

    // -------------------------------------------------------------------------
    // Store layer
    // -------------------------------------------------------------------------

    pub fn entity_not_found(message: impl Into<String>, entity_ref: impl Into<String>) -> Self {
        Self::EntityNotFound {
            context: Self::context(message),
            entity_ref: entity_ref.into(),
        }
    }

    pub fn already_checked_out(message: impl Into<String>, entity_ref: impl Into<String>) -> Self {
        Self::AlreadyCheckedOut {
            context: Self::context(message),
            entity_ref: entity_ref.into(),
        }
    }

    pub fn entity_not_checked_out(
        message: impl Into<String>,
        entity_ref: impl Into<String>,
    ) -> Self {
        Self::EntityNotCheckedOut {
            context: Self::context(message),
            entity_ref: entity_ref.into(),
        }
    }

    pub fn entity_already_exists(
        message: impl Into<String>,
        entity_ref: impl Into<String>,
    ) -> Self {
        Self::EntityAlreadyExists {
            context: Self::context(message),
            entity_ref: entity_ref.into(),
        }
    }

    pub fn entity_still_checked_out(
        message: impl Into<String>,
        entity_ref: impl Into<String>,
    ) -> Self {
        Self::EntityStillCheckedOut {
            context: Self::context(message),
            entity_ref: entity_ref.into(),
        }
    }

    pub fn no_uncommitted_changes(
        message: impl Into<String>,
        entity_ref: impl Into<String>,
    ) -> Self {
        Self::NoUncommittedChanges {
            context: Self::context(message),
            entity_ref: entity_ref.into(),
        }
    }

    pub fn entity_has_unsaved_changes(
        message: impl Into<String>,
        entity_ref: impl Into<String>,
    ) -> Self {
        Self::EntityHasUnsavedChanges {
            context: Self::context(message),
            entity_ref: entity_ref.into(),
        }
    }

    pub fn pending_checkouts(message: impl Into<String>, count: usize) -> Self {
        Self::PendingCheckouts {
            context: Self::context(message),
            count,
        }
    }
}
