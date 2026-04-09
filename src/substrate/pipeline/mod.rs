//! Substrate pipeline vocabulary types.
//!
//! Provides the declarative mapping from entity fields to substrate assets,
//! plus the `LocationResolver`, `Codec`, and `Executor` traits consumed by
//! the `Substrate` trait's default implementations.

pub mod codec;
pub mod executor;
pub use codec::CodecError;
pub use executor::ExecutorError;

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Slot — substrate-specific encoding target marker
// ---------------------------------------------------------------------------

/// Marker trait for substrate-specific encoding targets (slots within an asset).
pub trait Slot: Copy + 'static {}

// ---------------------------------------------------------------------------
// AssetOp / AssetRequest / AssetResponse
// ---------------------------------------------------------------------------

pub enum AssetOp<E> {
    Put(E),
    Post(E),
    Patch(E),
    Delete,
    Get,
    Head,
}

pub struct AssetRequest<L, E> {
    pub location: L,
    pub op: AssetOp<E>,
}

pub enum AssetResponse<E> {
    Done,
    Data(E),
    Exists(bool),
}

// ---------------------------------------------------------------------------
// AssetKind
// ---------------------------------------------------------------------------

pub struct AssetKind {
    pub distinguishes_create: bool,
    pub supports_partial: bool,
}

pub const MARKDOWN_FILE: AssetKind =
    AssetKind { distinguishes_create: false, supports_partial: false };
pub const RAW_FILE: AssetKind =
    AssetKind { distinguishes_create: false, supports_partial: false };

// ---------------------------------------------------------------------------
// FieldMapping
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub struct FieldMapping<S: Slot> {
    pub key: &'static str,
    pub slot: S,
}

// ---------------------------------------------------------------------------
// RefAssetDef / AssetDef / EntitySchema
// ---------------------------------------------------------------------------

pub struct RefAssetDef<S: Slot> {
    pub path_template: &'static str,
    pub kind: &'static AssetKind,
    pub fields: &'static [FieldMapping<S>],
}

impl<S: Slot> RefAssetDef<S> {
    fn path_deps(&self) -> Vec<&'static str> {
        vec![]
    }
}

pub struct AssetDef<S: Slot> {
    pub path_template: &'static str,
    pub kind: &'static AssetKind,
    pub fields: &'static [FieldMapping<S>],
    /// Fields from the ref_asset whose values are needed to resolve this asset's path template.
    pub path_deps: &'static [&'static str],
}

pub struct EntitySchema<S: Slot> {
    pub ref_asset: RefAssetDef<S>,
    pub assets: &'static [AssetDef<S>],
}

impl<S: Slot> EntitySchema<S> {
    /// Derive the `LoadStrategy` for a given field name.
    pub fn load_strategy_for(&self, field: &str) -> LoadStrategy {
        if self.ref_asset.fields.iter().any(|f| f.key == field) {
            return LoadStrategy {
                prerequisites: self.ref_asset.path_deps(),
                mutable_without_load: false,
            };
        }
        for asset in self.assets {
            if asset.fields.iter().any(|f| f.key == field) {
                return LoadStrategy {
                    prerequisites: asset.path_deps.to_vec(),
                    mutable_without_load: asset.kind.supports_partial || asset.fields.len() == 1,
                };
            }
        }
        LoadStrategy { prerequisites: vec![], mutable_without_load: true }
    }
}

// ---------------------------------------------------------------------------
// LoadStrategy
// ---------------------------------------------------------------------------

pub struct LoadStrategy {
    pub prerequisites: Vec<&'static str>,
    pub mutable_without_load: bool,
}

// ---------------------------------------------------------------------------
// SubstrateSchema trait
// ---------------------------------------------------------------------------

/// Implemented per (entity type, substrate) pair; provides the `SCHEMA` const.
pub trait SubstrateSchema<Sub: super::Substrate>: crate::entity::Entity {
    const SCHEMA: EntitySchema<<Sub as super::Substrate>::Slot>;
}

// ---------------------------------------------------------------------------
// AssetLike — object-safe common interface over RefAssetDef / AssetDef
// ---------------------------------------------------------------------------

pub trait AssetLike<S: Slot> {
    fn path_template(&self) -> &str;
    fn kind(&self) -> &AssetKind;
    fn fields(&self) -> &[FieldMapping<S>];
}

impl<S: Slot> AssetLike<S> for RefAssetDef<S> {
    fn path_template(&self) -> &str { self.path_template }
    fn kind(&self) -> &AssetKind { self.kind }
    fn fields(&self) -> &[FieldMapping<S>] { self.fields }
}

impl<S: Slot> AssetLike<S> for AssetDef<S> {
    fn path_template(&self) -> &str { self.path_template }
    fn kind(&self) -> &AssetKind { self.kind }
    fn fields(&self) -> &[FieldMapping<S>] { self.fields }
}

// ---------------------------------------------------------------------------
// AssetMapper
// ---------------------------------------------------------------------------

pub struct AssetMapper;

impl AssetMapper {
    /// For `Added` entities: return all assets.
    /// For `Modified` entities with `dirty_fields`: return only assets containing ≥1 dirty field.
    pub fn select_for_write<'a, S: Slot>(
        _schema: &'a EntitySchema<S>,
        _dirty_fields: Option<&'a [&'a str]>,
    ) -> Vec<&'a dyn AssetLike<S>> {
        todo!("AssetMapper::select_for_write — implemented in Task 11")
    }
}

// ---------------------------------------------------------------------------
// LocationResolver
// ---------------------------------------------------------------------------

pub trait LocationResolver {
    type Location;
    fn resolve(&self, path_template: &str, entity_json: &serde_json::Value) -> Self::Location;
}

// ---------------------------------------------------------------------------
// Codec
// ---------------------------------------------------------------------------

pub trait Codec {
    type Slot: Slot;
    type Encoded;

    fn encode(
        &self,
        fields: &HashMap<&str, serde_json::Value>,
        schema: &[FieldMapping<Self::Slot>],
    ) -> Result<Self::Encoded, CodecError>;

    fn decode(
        &self,
        raw: &Self::Encoded,
        schema: &[FieldMapping<Self::Slot>],
    ) -> Result<HashMap<String, serde_json::Value>, CodecError>;
}

// ---------------------------------------------------------------------------
// Executor
// ---------------------------------------------------------------------------

pub trait Executor {
    type Location;
    type Encoded;

    fn execute(
        &self,
        ops: Vec<AssetRequest<Self::Location, Self::Encoded>>,
    ) -> Result<Vec<AssetResponse<Self::Encoded>>, Vec<ExecutorError>>;
}
