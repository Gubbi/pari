//! Substrate layer — persistence backend trait and implementations.

pub mod error;
pub mod pipeline;

pub use error::SubstrateError;
use pipeline::ExecutorError;

use crate::entity::{AnyEntityRef, EntityKind, StoreEntity};

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

    /// Check existence of a batch of entities. Returns one bool per ref, in order.
    fn exists<'a>(
        &'a self,
        refs: &'a [AnyEntityRef],
    ) -> impl std::future::Future<Output = Result<Vec<bool>, SubstrateError>> + Send + 'a {
        let _ = refs;
        async { todo!("default impl: resolver + executor Head per ref") }
    }

    /// Load the specified fields of an entity. `fields: &[]` = all fields.
    /// The passed entity may already have some fields initialized — used for validation context.
    fn load<'a>(
        &'a self,
        entity: &'a StoreEntity,
        fields: &'a [&'a str],
    ) -> impl std::future::Future<Output = Result<StoreEntity, SubstrateError>> + Send + 'a {
        let _ = (entity, fields);
        async { todo!("default impl: resolver + executor Get + codec decode") }
    }

    /// Persist a set of entity changes. Changes are consumed lazily via iterator.
    fn persist<'a>(
        &'a self,
        changes: impl Iterator<Item = EntityChange<'a>> + Send + 'a,
    ) -> impl std::future::Future<Output = Result<(), Vec<SubstrateError>>> + Send + 'a {
        let _ = changes;
        async { todo!("default impl: AssetMapper + resolver + codec + executor") }
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

    async fn exists(&self, refs: &[AnyEntityRef]) -> Result<Vec<bool>, SubstrateError> {
        Ok(vec![false; refs.len()])
    }

    async fn load(
        &self,
        entity: &StoreEntity,
        _: &[&str],
    ) -> Result<StoreEntity, SubstrateError> {
        Err(SubstrateError::Executor(ExecutorError::new(
            entity.any_ref().id().to_string(),
            "VoidSubstrate: no load",
        )))
    }

    async fn persist(
        &self,
        _: impl Iterator<Item = EntityChange<'_>> + Send,
    ) -> Result<(), Vec<SubstrateError>> {
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
        let e = SubstrateError::Executor(ExecutorError::new("roles/eng-lead.md", "permission denied"));
        let msg = format!("{}", e);
        assert!(msg.contains("permission denied"), "display: {msg}");
        assert!(msg.contains("roles/eng-lead.md"), "display: {msg}");
    }

    #[test]
    fn substrate_error_implements_std_error() {
        let e = SubstrateError::Executor(ExecutorError::new("roles/eng-lead.md", "permission denied"));
        let _: &dyn std::error::Error = &e;
    }
}
