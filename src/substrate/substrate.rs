use crate::{
    entity::{AnyEntityRef, EntityKind, TrackedEntity},
    error::ActivityError,
    store::EntityChange,
    substrate::{defaults, lib::schema_registry::SchemaBackedSubstrate, pipeline},
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

    /// Short snake_case identifier for this backend, used in component strings.
    /// Format: `"{backend_name}"` e.g. `"repo_substrate"`, `"in_memory_substrate"`.
    fn substrate_name() -> &'static str;

    fn load_strategy(
        entity_kind: EntityKind,
        field: &str,
    ) -> Result<pipeline::LoadStrategy, ActivityError>
    where
        Self: SchemaBackedSubstrate,
    {
        defaults::load_strategy::<Self>(entity_kind, field)
    }

    fn exists<'a>(
        &'a self,
        refs: &'a [AnyEntityRef],
    ) -> impl std::future::Future<Output = Result<Vec<bool>, ActivityError>> + Send + 'a
    where
        Self: SchemaBackedSubstrate,
    {
        defaults::exists(self, refs)
    }

    fn load<'a>(
        &'a self,
        entity: &'a TrackedEntity,
        fields: &'a [&'a str],
    ) -> impl std::future::Future<Output = Result<TrackedEntity, ActivityError>> + Send + 'a
    where
        Self: SchemaBackedSubstrate,
    {
        defaults::load(self, entity, fields)
    }

    fn persist<'a>(
        &'a self,
        changes: impl Iterator<Item = EntityChange<'a>> + Send + 'a,
    ) -> impl std::future::Future<Output = Result<(), Vec<ActivityError>>> + Send + 'a
    where
        Self: SchemaBackedSubstrate,
    {
        defaults::persist(self, changes)
    }
}
