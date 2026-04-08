//! Substrate layer — persistence backend trait and implementations.

pub mod changeset;
pub mod pipeline;
pub mod repo;

pub use crate::schema::store::EntityStore;

use crate::entity::{AnyEntityRef, EntityKind, StoreEntity};

// ---------------------------------------------------------------------------
// SubstrateError
// ---------------------------------------------------------------------------

/// A filesystem path + human-readable description of what went wrong.
#[derive(Debug, thiserror::Error)]
#[error("{message} at {path}")]
pub struct SubstrateError {
    pub path: String,
    pub message: String,
}

// ---------------------------------------------------------------------------
// EntityChange
// ---------------------------------------------------------------------------

/// A single entity change to be persisted.
pub enum EntityChange<'a> {
    Added(&'a StoreEntity),
    Modified(&'a StoreEntity, &'a [&'a str]),
    Removed(&'a AnyEntityRef),
}

// ---------------------------------------------------------------------------
// Substrate trait
// ---------------------------------------------------------------------------

/// Persistence backend interface.
pub trait Substrate: Sized + Send + Sync + 'static {
    type Slot: pipeline::Slot;
    type Location: Send;
    type Encoded: Send;
    type Resolver: pipeline::LocationResolver<Location = Self::Location>;
    type Codec: pipeline::Codec<Slot = Self::Slot, Encoded = Self::Encoded>;
    type Executor: pipeline::Executor<Location = Self::Location, Encoded = Self::Encoded>;

    fn resolver(&self) -> &Self::Resolver;
    fn codec(&self) -> &Self::Codec;
    fn executor(&self) -> &Self::Executor;

    /// Derive `LoadStrategy` for a (entity_kind, field) pair.
    fn load_strategy(entity_kind: EntityKind, field: &str) -> pipeline::LoadStrategy;

    /// Check existence of a single entity.
    async fn exists(&self, _any_ref: &AnyEntityRef) -> Result<bool, SubstrateError> {
        todo!("default impl: resolver + executor Head")
    }

    /// Load the specified fields of an entity. `fields: &[]` = all fields.
    async fn load(
        &self,
        _any_ref: &AnyEntityRef,
        _fields: &[&str],
    ) -> Result<StoreEntity, SubstrateError> {
        todo!("default impl: resolver + executor Get + codec decode")
    }

    /// Persist a set of entity changes atomically.
    async fn atomic_persist(
        &self,
        _changes: &[EntityChange<'_>],
    ) -> Result<(), Vec<SubstrateError>> {
        todo!("default impl: AssetMapper + resolver + codec + executor")
    }
}

// ---------------------------------------------------------------------------
// VoidSubstrate
// ---------------------------------------------------------------------------

/// No-op substrate for tests that don't need persistence.
pub struct VoidSubstrate;

#[derive(Clone, Copy)]
pub struct VoidSlot;
impl pipeline::Slot for VoidSlot {}

pub struct VoidResolver;
impl pipeline::LocationResolver for VoidResolver {
    type Location = String;
    fn resolve(&self, _: &str, _: &serde_json::Value) -> String {
        String::new()
    }
}

pub struct VoidCodec;
impl pipeline::Codec for VoidCodec {
    type Slot = VoidSlot;
    type Encoded = String;
    fn encode(
        &self,
        _: &std::collections::HashMap<&str, serde_json::Value>,
        _: &[pipeline::FieldMapping<VoidSlot>],
    ) -> Result<String, pipeline::CodecError> {
        Ok(String::new())
    }
    fn decode(
        &self,
        _: &String,
        _: &[pipeline::FieldMapping<VoidSlot>],
    ) -> Result<std::collections::HashMap<String, serde_json::Value>, pipeline::CodecError> {
        Ok(std::collections::HashMap::new())
    }
}

pub struct VoidExecutor;
impl pipeline::Executor for VoidExecutor {
    type Location = String;
    type Encoded = String;
    fn execute(
        &self,
        _: Vec<pipeline::AssetRequest<String, String>>,
    ) -> Result<Vec<pipeline::AssetResponse<String>>, Vec<pipeline::ExecutorError>> {
        Ok(vec![])
    }
}

impl Substrate for VoidSubstrate {
    type Slot = VoidSlot;
    type Location = String;
    type Encoded = String;
    type Resolver = VoidResolver;
    type Codec = VoidCodec;
    type Executor = VoidExecutor;

    fn resolver(&self) -> &VoidResolver { &VoidResolver }
    fn codec(&self) -> &VoidCodec { &VoidCodec }
    fn executor(&self) -> &VoidExecutor { &VoidExecutor }

    fn load_strategy(_: EntityKind, _: &str) -> pipeline::LoadStrategy {
        pipeline::LoadStrategy { prerequisites: vec![], mutable_without_load: true }
    }

    async fn exists(&self, _: &AnyEntityRef) -> Result<bool, SubstrateError> {
        Ok(false)
    }

    async fn load(
        &self,
        any_ref: &AnyEntityRef,
        _: &[&str],
    ) -> Result<StoreEntity, SubstrateError> {
        Err(SubstrateError {
            path: any_ref.id().to_string(),
            message: "VoidSubstrate: no load".to_string(),
        })
    }

    async fn atomic_persist(&self, _: &[EntityChange<'_>]) -> Result<(), Vec<SubstrateError>> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- SubstrateError Display and std::error::Error tests ---

    #[test]
    fn substrate_error_display_format() {
        let e = SubstrateError {
            path: "roles/eng-lead.md".to_string(),
            message: "permission denied".to_string(),
        };
        assert_eq!(format!("{}", e), "permission denied at roles/eng-lead.md");
    }

    #[test]
    fn substrate_error_implements_std_error() {
        let e = SubstrateError {
            path: "roles/eng-lead.md".to_string(),
            message: "permission denied".to_string(),
        };
        let _: &dyn std::error::Error = &e;
    }

    #[test]
    fn substrate_error_has_path_and_message() {
        let e = SubstrateError {
            path: "roles/eng-lead.md".to_string(),
            message: "permission denied".to_string(),
        };
        assert_eq!(e.path, "roles/eng-lead.md");
        assert_eq!(e.message, "permission denied");
    }

    #[test]
    fn entity_store_holds_entity_collections() {
        let store = EntityStore::new();
        assert!(store.roles.is_empty());
        assert!(store.hooks.is_empty());
        assert!(store.teams.is_empty());
        assert!(store.workflows.is_empty());
        assert!(store.shared_workflows.is_empty());
    }
}
