//! The [`Substrate`] trait — persistence contract the store calls into.

use crate::{
    entity::{AnyEntityRef, TrackedEntity},
    error::ActivityError,
    store::EntityChange,
    substrate::{defaults, lib::schema_registry::SchemaBackedSubstrate, pipeline},
};

/// The persistence contract.
///
/// Backends pin a `Resolver` / `Codec` / `Executor` trio through the
/// associated types, expose them via `resolver()` / `codec()` /
/// `executor()`, and — once every entity implements
/// `pipeline::SubstrateSchema<Self>` — inherit default implementations
/// of [`Self::load_strategy`], [`Self::exists`], [`Self::load`], and
/// [`Self::persist`] from the layer's `defaults` module.
///
/// `substrate_name()` identifies the backend in error component strings
/// (`"repo_substrate.codec"`, `"in_memory_substrate.executor"`, etc.).
///
/// Shape queries (`load_strategy`, `schema_for`) take `&AnyEntityRef`,
/// not `EntityKind`. `EntityKind` is substrate-internal vocabulary for
/// per-kind dispatch inside `schema_registry.rs`; it does not appear in
/// the trait surface. Reads return `serde_json::Value` payloads — the
/// store performs the JSON ↔ tracked conversion.
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
        any_ref: &AnyEntityRef,
        field: &str,
    ) -> Result<pipeline::LoadStrategy, ActivityError>
    where
        Self: SchemaBackedSubstrate,
    {
        defaults::load_strategy::<Self>(any_ref, field)
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
    ) -> impl std::future::Future<Output = Result<serde_json::Value, ActivityError>> + Send + 'a
    where
        Self: SchemaBackedSubstrate,
    {
        defaults::load(self, entity, fields)
    }

    fn persist<'a>(
        &'a self,
        changes: impl Iterator<Item = EntityChange> + Send + 'a,
    ) -> impl std::future::Future<Output = Result<(), ActivityError>> + Send + 'a
    where
        Self: SchemaBackedSubstrate,
    {
        defaults::persist(self, changes)
    }
}
