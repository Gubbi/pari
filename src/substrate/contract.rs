use crate::{
    entity::{AnyEntityRef, EntityKind, TrackedEntity},
    store::EntityChange,
    substrate::{defaults, pipeline, schema_registry::SchemaBackedSubstrate, SubstrateError},
};

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

    fn load_strategy(
        entity_kind: EntityKind,
        field: &str,
    ) -> Result<pipeline::LoadStrategy, SubstrateError>
    where
        Self: SchemaBackedSubstrate,
    {
        defaults::load_strategy::<Self>(entity_kind, field)
    }

    fn exists<'a>(
        &'a self,
        refs: &'a [AnyEntityRef],
    ) -> impl std::future::Future<Output = Result<Vec<bool>, SubstrateError>> + Send + 'a
    where
        Self: SchemaBackedSubstrate,
    {
        defaults::exists(self, refs)
    }

    fn load<'a>(
        &'a self,
        entity: &'a TrackedEntity,
        fields: &'a [&'a str],
    ) -> impl std::future::Future<Output = Result<TrackedEntity, SubstrateError>> + Send + 'a
    where
        Self: SchemaBackedSubstrate,
    {
        defaults::load(self, entity, fields)
    }

    fn persist<'a>(
        &'a self,
        changes: impl Iterator<Item = EntityChange<'a>> + Send + 'a,
    ) -> impl std::future::Future<Output = Result<(), Vec<SubstrateError>>> + Send + 'a
    where
        Self: SchemaBackedSubstrate,
    {
        defaults::persist(self, changes)
    }
}
