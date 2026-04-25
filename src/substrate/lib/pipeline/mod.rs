//! Pipeline vocabulary and traits used by schema-driven substrates.
//!
//! A substrate composes three pipeline components — a resolver, a
//! codec, and an executor — around an [`EntitySchema`]. The traits here
//! are pure (`PrimitiveError` only); orchestration wrapping into
//! `ActivityError` happens in the substrate layer's `defaults` module.

pub mod asset_io;
pub mod asset_kind;
pub mod asset_mapper;
pub mod codec;
pub mod executor;
pub mod resolver;
pub mod schema;
pub mod slot;

pub use asset_io::{AssetOp, AssetRequest, AssetResponse};
pub use asset_kind::AssetKind;
pub use asset_mapper::AssetMapper;
pub use codec::Codec;
pub use executor::Executor;
pub use resolver::LocationResolver;
pub use schema::{
    AssetDef, EntitySchema, FieldMapping, LoadStrategy, RefAssetDef, SchemaAsset, SubstrateSchema,
};
pub use slot::{Slot, ValueSlot};
