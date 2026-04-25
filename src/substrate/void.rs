use crate::{
    entity::{AnyEntityRef, EntityKind, TrackedEntity},
    error::{primitive::PrimitiveError, ActivityError},
    store::EntityChange,
    substrate::{
        pipeline::{self},
        Substrate,
    },
};

/// No-op backend for tests that only need the `Substrate` contract
/// surface. Overrides the schema-driven defaults with trivial behaviors:
/// `exists` always returns `false`, `persist` is a no-op, and `load`
/// reports an unsupported-load error.
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

    fn base_of(&self, _: &String) -> String {
        String::new()
    }
}

pub struct VoidCodec;

impl pipeline::Codec for VoidCodec {
    type Slot = VoidSlot;
    type Encoded = String;

    fn encode(
        &self,
        _: &serde_json::Value,
        _: &[pipeline::FieldMapping<VoidSlot>],
    ) -> Result<String, PrimitiveError> {
        Ok(String::new())
    }

    fn decode(
        &self,
        _: &String,
        _: &[pipeline::FieldMapping<VoidSlot>],
    ) -> Result<std::collections::HashMap<String, serde_json::Value>, PrimitiveError> {
        Ok(std::collections::HashMap::new())
    }
}

pub struct VoidExecutor;

impl pipeline::Executor for VoidExecutor {
    type Location = String;
    type Encoded = String;

    fn execute<I>(&self, _: I) -> Result<Vec<pipeline::AssetResponse<String>>, Vec<PrimitiveError>>
    where
        I: IntoIterator<Item = pipeline::AssetRequest<String, String>>,
    {
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

    fn resolver(&self) -> &VoidResolver {
        &VoidResolver
    }

    fn codec(&self) -> &VoidCodec {
        &VoidCodec
    }

    fn executor(&self) -> &VoidExecutor {
        &VoidExecutor
    }

    fn substrate_name() -> &'static str {
        "void_substrate"
    }

    fn load_strategy(_: EntityKind, _: &str) -> Result<pipeline::LoadStrategy, ActivityError> {
        Ok(pipeline::LoadStrategy {
            prerequisites: vec![],
            mutable_without_load: true,
        })
    }

    async fn exists(&self, refs: &[AnyEntityRef]) -> Result<Vec<bool>, ActivityError> {
        Ok(vec![false; refs.len()])
    }

    async fn load(
        &self,
        entity: &TrackedEntity,
        _: &[&str],
    ) -> Result<TrackedEntity, ActivityError> {
        let entity_ref = entity.any_ref().id().to_string();
        Err(ActivityError::corrupt_persistence_state(
            Self::substrate_name(),
            PrimitiveError::unsupported_load("unsupported load", entity_ref),
        ))
    }

    async fn persist<'a>(
        &'a self,
        _: impl Iterator<Item = EntityChange> + Send + 'a,
    ) -> Result<(), ActivityError> {
        Ok(())
    }
}
