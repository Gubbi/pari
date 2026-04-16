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
pub use codec::{Codec, CodecError};
pub use executor::{Executor, ExecutorError};
pub use resolver::LocationResolver;
pub use schema::{
    AssetDef, EntitySchema, FieldMapping, LoadStrategy, RefAssetDef, SchemaAsset,
};
pub use slot::{Slot, ValueSlot};

pub trait SubstrateSchema<Sub: super::Substrate>: crate::entity::Entity {
    const SCHEMA: EntitySchema<<Sub as super::Substrate>::Slot>;
}
