use crate::{
    entity::{AnyEntityRef, EntityKind, TrackedEntity},
    error::primitive::PrimitiveError,
    store::EntityChange,
    substrate::{
        pipeline::{self},
        Substrate, SubstrateError,
    },
};

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

    fn load_strategy(_: EntityKind, _: &str) -> Result<pipeline::LoadStrategy, SubstrateError> {
        Ok(pipeline::LoadStrategy {
            prerequisites: vec![],
            mutable_without_load: true,
        })
    }

    async fn exists(&self, refs: &[AnyEntityRef]) -> Result<Vec<bool>, SubstrateError> {
        Ok(vec![false; refs.len()])
    }

    async fn load(
        &self,
        entity: &TrackedEntity,
        _: &[&str],
    ) -> Result<TrackedEntity, SubstrateError> {
        let entity_ref = entity.any_ref().id().to_string();
        Err(SubstrateError::corrupt_persistence_state(
            PrimitiveError::unsupported_load("unsupported load", entity_ref, "void_substrate"),
        ))
    }

    async fn persist(
        &self,
        _: impl Iterator<Item = EntityChange<'_>> + Send,
    ) -> Result<(), Vec<SubstrateError>> {
        Ok(())
    }
}
