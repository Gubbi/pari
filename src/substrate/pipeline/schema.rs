use std::{
    collections::{HashMap, HashSet},
    sync::{LazyLock, Mutex},
};

use crate::{
    error::primitive::PrimitiveError,
    substrate::pipeline::{AssetKind, Slot},
};

#[derive(Clone, Copy)]
pub struct FieldMapping<S: Slot> {
    pub key: &'static str,
    pub slot: S,
}

pub struct RefAssetDef<S: Slot> {
    pub path_template: &'static str,
    pub kind: &'static AssetKind,
    pub fields: &'static [FieldMapping<S>],
}

pub struct AssetDef<S: Slot> {
    pub path_template: &'static str,
    pub kind: &'static AssetKind,
    pub fields: &'static [FieldMapping<S>],
    pub path_deps: &'static [&'static str],
}

pub struct EntitySchema<S: Slot> {
    pub ref_asset: RefAssetDef<S>,
    pub assets: &'static [AssetDef<S>],
}

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
    pub fn path_template(&self) -> &str {
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
            .ok_or_else(|| PrimitiveError::UnknownSchemaField {
                context: PrimitiveError::context("unknown schema field"),
                field: field.to_string(),
            })
    }
}
