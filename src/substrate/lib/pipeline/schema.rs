use std::{
    collections::{HashMap, HashSet},
    sync::{LazyLock, Mutex},
};

use super::{AssetKind, Slot};
use crate::error::primitive::PrimitiveError;

/// One domain field's placement inside an asset. `slot` is
/// backend-specific — the default `ValueSlot::Value` covers backends
/// whose assets hold a single value slot per field.
#[derive(Clone, Copy)]
pub struct FieldMapping<S: Slot> {
    pub key: &'static str,
    pub slot: S,
}

/// Declares the always-present identity asset for an entity kind. Its
/// `path_template` must resolve from stub JSON — the ref asset is the
/// only asset the store can locate from an `AnyEntityRef` alone.
pub struct RefAssetDef<S: Slot> {
    pub path_template: &'static str,
    pub kind: &'static AssetKind,
    pub fields: &'static [FieldMapping<S>],
}

/// Declares a secondary asset. `path_deps` lists ref-asset fields the
/// `path_template` refers to; those fields become prerequisites of any
/// operation on this asset.
pub struct AssetDef<S: Slot> {
    pub path_template: &'static str,
    pub kind: &'static AssetKind,
    pub fields: &'static [FieldMapping<S>],
    pub path_deps: &'static [&'static str],
}

/// Per-entity-kind schema: one ref asset plus zero or more secondary
/// assets. Each field appears in exactly one asset — duplicate
/// registrations panic at first lookup.
pub struct EntitySchema<S: Slot> {
    pub ref_asset: RefAssetDef<S>,
    pub assets: &'static [AssetDef<S>],
}

/// What the store needs to do before touching `field`.
///
/// `prerequisites` must be loaded first (usually so the asset's
/// `path_template` can be resolved). `mutable_without_load` is `true`
/// when the field's asset can be written without reading first — either
/// the asset kind supports partial writes, or the asset has exactly one
/// field so a full overwrite is equivalent.
pub struct LoadStrategy {
    pub prerequisites: Vec<&'static str>,
    pub mutable_without_load: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum AssetSelector {
    RefAsset,
    Asset(usize),
}

pub enum SchemaAsset<'a, S: Slot> {
    RefAsset(&'a RefAssetDef<S>),
    Asset(&'a AssetDef<S>),
}

static FIELD_INDEX_CACHE: LazyLock<Mutex<HashMap<usize, HashMap<&'static str, AssetSelector>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

impl<'a, S: Slot> SchemaAsset<'a, S> {
    pub fn path_template(&self) -> &'static str {
        match self {
            Self::RefAsset(asset) => asset.path_template,
            Self::Asset(asset) => asset.path_template,
        }
    }

    pub fn kind(&self) -> &'static AssetKind {
        match self {
            Self::RefAsset(asset) => asset.kind,
            Self::Asset(asset) => asset.kind,
        }
    }

    pub fn fields(&self) -> &'a [FieldMapping<S>] {
        match self {
            Self::RefAsset(asset) => asset.fields,
            Self::Asset(asset) => asset.fields,
        }
    }
}

impl<S: Slot> EntitySchema<S> {
    pub const fn new(ref_asset: RefAssetDef<S>, assets: &'static [AssetDef<S>]) -> Self {
        Self { ref_asset, assets }
    }

    pub fn load_strategy_for(&self, field: &str) -> Result<LoadStrategy, PrimitiveError> {
        let selector = self.lookup(field)?;
        let (asset, prerequisites) = self.asset_parts(selector);
        Ok(LoadStrategy {
            prerequisites: prerequisites.to_vec(),
            mutable_without_load: Self::can_mutate_without_load(asset),
        })
    }

    pub fn assets_for_write<'a>(
        &'a self,
        dirty_fields: Option<&[&str]>,
    ) -> Result<Vec<SchemaAsset<'a, S>>, PrimitiveError> {
        match dirty_fields {
            None => Ok(self.all_assets().collect()),
            Some(fields) => self.assets_for_fields(fields),
        }
    }

    pub fn assets_for_read<'a>(
        &'a self,
        fields: &[&str],
    ) -> Result<Vec<SchemaAsset<'a, S>>, PrimitiveError> {
        if fields.is_empty() {
            Ok(self.all_assets().collect())
        } else {
            self.assets_for_fields(fields)
        }
    }

    pub fn all_assets<'a>(&'a self) -> impl Iterator<Item = SchemaAsset<'a, S>> + 'a {
        std::iter::once(SchemaAsset::RefAsset(&self.ref_asset))
            .chain(self.assets.iter().map(SchemaAsset::Asset))
    }

    fn assets_for_fields<'a>(
        &'a self,
        fields: &[&str],
    ) -> Result<Vec<SchemaAsset<'a, S>>, PrimitiveError> {
        let mut resolved = Vec::new();
        let mut seen = HashSet::new();
        for field in fields {
            let selector = self.lookup(field)?;
            if !seen.insert(selector) {
                continue;
            }
            let (asset, _) = self.asset_parts(selector);
            resolved.push(asset);
        }
        Ok(resolved)
    }

    fn asset_parts<'a>(
        &'a self,
        selector: AssetSelector,
    ) -> (SchemaAsset<'a, S>, &'static [&'static str]) {
        match selector {
            AssetSelector::RefAsset => (SchemaAsset::RefAsset(&self.ref_asset), &[]),
            AssetSelector::Asset(index) => {
                let asset = &self.assets[index];
                (SchemaAsset::Asset(asset), asset.path_deps)
            }
        }
    }

    fn can_mutate_without_load(asset: SchemaAsset<'_, S>) -> bool {
        asset.kind().supports_partial || asset.fields().len() == 1
    }

    fn lookup(&self, field: &str) -> Result<AssetSelector, PrimitiveError> {
        let schema_id = self as *const Self as usize;
        let mut cache = FIELD_INDEX_CACHE.lock().unwrap();
        let index = cache.entry(schema_id).or_insert_with(|| {
            let mut index: HashMap<&'static str, AssetSelector> = HashMap::new();
            for mapping in self.ref_asset.fields {
                let previous = index.insert(mapping.key, AssetSelector::RefAsset);
                assert!(
                    previous.is_none(),
                    "field '{}' mapped more than once in schema",
                    mapping.key
                );
            }
            for (asset_index, asset) in self.assets.iter().enumerate() {
                for mapping in asset.fields {
                    let previous = index.insert(mapping.key, AssetSelector::Asset(asset_index));
                    assert!(
                        previous.is_none(),
                        "field '{}' mapped more than once in schema",
                        mapping.key
                    );
                }
            }
            index
        });

        index
            .get(field)
            .copied()
            .ok_or_else(|| PrimitiveError::unknown_schema_field("unknown schema field", field))
    }
}

/// Per-backend schema binding for an entity kind.
///
/// An entity opts a backend in by implementing this trait with a
/// concrete `SCHEMA` constant. The `SchemaBackedSubstrate` blanket impl
/// then dispatches `EntityKind` → `&'static EntitySchema` for every
/// schema-backed operation.
pub trait SubstrateSchema<Sub: crate::substrate::Substrate>: crate::entity::Entity {
    const SCHEMA: EntitySchema<<Sub as crate::substrate::Substrate>::Slot>;
}
