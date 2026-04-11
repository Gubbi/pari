//! Filesystem-backed substrate implementations.
//!
//! [`RepoSubstrate`] — new pipeline-based implementation.
//! [`storage::RepoSubstrate`] — legacy implementation (used by storage_integration tests).

pub(crate) mod lca;
pub(crate) mod render;
pub mod codec;
pub mod executor;
pub mod resolver;
pub mod schemas;
pub mod slot;
pub mod storage;

use std::{fs, path::{Path, PathBuf}};

use crate::{
    entity::{AnyEntityRef, EntityKind, StoreEntity},
    entities::{
        artifact_kind::{ArtifactKind, TrackedArtifactKind},
        hook::{Hook, TrackedHook},
        relay::{Relay, TrackedRelay},
        role::{Role, TrackedRole},
        task::{Task, TrackedTask},
        team::{Team, TrackedTeam},
        workflow::{
            EmbeddedWorkflow, ReusableWorkflow, TrackedEmbeddedWorkflow,
            TrackedReusableWorkflow, TrackedWorkflow, Workflow,
        },
    },
    substrate::{
        pipeline::{
            Codec, CodecError, EntitySchema, Executor, ExecutorError, LocationResolver,
            SubstrateSchema,
        },
        EntityChange, Substrate, SubstrateError,
    },
};

use codec::RepoCodec;
use executor::RepoExecutor;
use resolver::RepoLocationResolver;
use slot::RepoSlot;

// ---------------------------------------------------------------------------
// RepoSubstrate (new pipeline-based)
// ---------------------------------------------------------------------------

pub struct RepoSubstrate {
    pub resolver: RepoLocationResolver,
    pub codec: RepoCodec,
    pub executor: RepoExecutor,
}

impl RepoSubstrate {
    pub fn new(root: PathBuf) -> Result<Self, SubstrateError> {
        Self::cleanup_stale(&root)?;
        Ok(Self {
            resolver: RepoLocationResolver::new(root.clone()),
            codec: RepoCodec,
            executor: RepoExecutor::new(root),
        })
    }

    fn cleanup_stale(root: &Path) -> Result<(), SubstrateError> {
        if !root.exists() {
            return Ok(());
        }
        let entries = fs::read_dir(root)
            .map_err(|e| SubstrateError::Executor(ExecutorError::new(root.to_string_lossy(), e.to_string())))?;
        for entry in entries {
            let entry = entry
                .map_err(|e| SubstrateError::Executor(ExecutorError::new(root.to_string_lossy(), e.to_string())))?;
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                if name.ends_with(".part") || name.ends_with(".old") {
                    fs::remove_dir_all(&path)
                        .map_err(|e| SubstrateError::Executor(ExecutorError::new(path.to_string_lossy(), e.to_string())))?;
                }
            }
        }
        Ok(())
    }

    /// Return the `EntitySchema<RepoSlot>` for a given entity kind.
    fn schema_for(kind: EntityKind) -> &'static EntitySchema<RepoSlot> {
        match kind {
            EntityKind::Role             => &<Role             as SubstrateSchema<RepoSubstrate>>::SCHEMA,
            EntityKind::Hook             => &<Hook             as SubstrateSchema<RepoSubstrate>>::SCHEMA,
            EntityKind::Team             => &<Team             as SubstrateSchema<RepoSubstrate>>::SCHEMA,
            EntityKind::ArtifactKind     => &<ArtifactKind     as SubstrateSchema<RepoSubstrate>>::SCHEMA,
            EntityKind::Workflow         => &<Workflow         as SubstrateSchema<RepoSubstrate>>::SCHEMA,
            EntityKind::ReusableWorkflow => &<ReusableWorkflow as SubstrateSchema<RepoSubstrate>>::SCHEMA,
            EntityKind::EmbeddedWorkflow => &<EmbeddedWorkflow as SubstrateSchema<RepoSubstrate>>::SCHEMA,
            EntityKind::Task             => &<Task             as SubstrateSchema<RepoSubstrate>>::SCHEMA,
            EntityKind::Relay            => &<Relay            as SubstrateSchema<RepoSubstrate>>::SCHEMA,
        }
    }

    /// Serialize a `StoreEntity` variant to `serde_json::Value`.
    fn entity_to_json(entity: &StoreEntity) -> serde_json::Value {
        match entity {
            StoreEntity::Role(r)             => serde_json::to_value(r).unwrap_or_default(),
            StoreEntity::Hook(h)             => serde_json::to_value(h).unwrap_or_default(),
            StoreEntity::Team(t)             => serde_json::to_value(t).unwrap_or_default(),
            StoreEntity::ArtifactKind(a)     => serde_json::to_value(a).unwrap_or_default(),
            StoreEntity::Workflow(w)         => serde_json::to_value(w).unwrap_or_default(),
            StoreEntity::ReusableWorkflow(w) => serde_json::to_value(w).unwrap_or_default(),
            StoreEntity::EmbeddedWorkflow(e) => serde_json::to_value(e).unwrap_or_default(),
            StoreEntity::Task(t)             => serde_json::to_value(t).unwrap_or_default(),
            StoreEntity::Relay(r)            => serde_json::to_value(r).unwrap_or_default(),
        }
    }

    /// Build a minimal JSON object for path-template resolution from an `AnyEntityRef`.
    fn any_ref_to_json(any_ref: &AnyEntityRef) -> serde_json::Value {
        match any_ref {
            AnyEntityRef::Role(r) => serde_json::json!({ "entity_ref": serde_json::to_value(r).unwrap_or_default() }),
            AnyEntityRef::Hook(r) => serde_json::json!({ "entity_ref": serde_json::to_value(r).unwrap_or_default() }),
            AnyEntityRef::Team(r) => serde_json::json!({ "entity_ref": serde_json::to_value(r).unwrap_or_default() }),
            AnyEntityRef::ArtifactKind(r) => serde_json::json!({ "entity_ref": serde_json::to_value(r).unwrap_or_default() }),
            AnyEntityRef::Workflow(r) => serde_json::json!({ "entity_ref": serde_json::to_value(r).unwrap_or_default() }),
            AnyEntityRef::ReusableWorkflow(r) => serde_json::json!({ "entity_ref": serde_json::to_value(r).unwrap_or_default() }),
            AnyEntityRef::EmbeddedWorkflow(r) => serde_json::json!({ "entity_ref": serde_json::to_value(r).unwrap_or_default() }),
            AnyEntityRef::Task(r) => serde_json::json!({ "entity_ref": serde_json::to_value(r).unwrap_or_default() }),
            AnyEntityRef::Relay(r) => serde_json::json!({ "entity_ref": serde_json::to_value(r).unwrap_or_default() }),
        }
    }

    /// Decode a file into a `StoreEntity` for the given `AnyEntityRef`.
    fn decode_to_store_entity(
        any_ref: &AnyEntityRef,
        content: &str,
        schema: &EntitySchema<RepoSlot>,
    ) -> Result<StoreEntity, SubstrateError> {
        let decoded = RepoCodec.decode(&content.to_string(), schema.ref_asset.fields)
            .map_err(SubstrateError::Codec)?;

        let entity_ref_json = Self::any_ref_to_json(any_ref)["entity_ref"].clone();
        let mut map = serde_json::Map::new();
        map.insert("entity_ref".to_string(), entity_ref_json);
        for (k, v) in decoded {
            map.insert(k, v);
        }
        let json = serde_json::Value::Object(map);

        let entity = match any_ref {
            AnyEntityRef::Role(_) => {
                let t: TrackedRole = serde_json::from_value(json)
                    .map_err(|e| SubstrateError::Codec(CodecError::new(any_ref.id(), e.to_string())))?;
                StoreEntity::Role(t)
            }
            AnyEntityRef::Hook(_) => {
                let t: TrackedHook = serde_json::from_value(json)
                    .map_err(|e| SubstrateError::Codec(CodecError::new(any_ref.id(), e.to_string())))?;
                StoreEntity::Hook(t)
            }
            AnyEntityRef::Team(_) => {
                let t: TrackedTeam = serde_json::from_value(json)
                    .map_err(|e| SubstrateError::Codec(CodecError::new(any_ref.id(), e.to_string())))?;
                StoreEntity::Team(t)
            }
            AnyEntityRef::ArtifactKind(_) => {
                let t: TrackedArtifactKind = serde_json::from_value(json)
                    .map_err(|e| SubstrateError::Codec(CodecError::new(any_ref.id(), e.to_string())))?;
                StoreEntity::ArtifactKind(t)
            }
            AnyEntityRef::Workflow(_) => {
                let t: TrackedWorkflow = serde_json::from_value(json)
                    .map_err(|e| SubstrateError::Codec(CodecError::new(any_ref.id(), e.to_string())))?;
                StoreEntity::Workflow(t)
            }
            AnyEntityRef::ReusableWorkflow(_) => {
                let t: TrackedReusableWorkflow = serde_json::from_value(json)
                    .map_err(|e| SubstrateError::Codec(CodecError::new(any_ref.id(), e.to_string())))?;
                StoreEntity::ReusableWorkflow(t)
            }
            AnyEntityRef::EmbeddedWorkflow(_) => {
                let t: TrackedEmbeddedWorkflow = serde_json::from_value(json)
                    .map_err(|e| SubstrateError::Codec(CodecError::new(any_ref.id(), e.to_string())))?;
                StoreEntity::EmbeddedWorkflow(t)
            }
            AnyEntityRef::Task(_) => {
                let t: TrackedTask = serde_json::from_value(json)
                    .map_err(|e| SubstrateError::Codec(CodecError::new(any_ref.id(), e.to_string())))?;
                StoreEntity::Task(t)
            }
            AnyEntityRef::Relay(_) => {
                let t: TrackedRelay = serde_json::from_value(json)
                    .map_err(|e| SubstrateError::Codec(CodecError::new(any_ref.id(), e.to_string())))?;
                StoreEntity::Relay(t)
            }
        };
        Ok(entity)
    }
}

// ---------------------------------------------------------------------------
// substrate::Substrate impl (pipeline trait)
// ---------------------------------------------------------------------------

impl Substrate for RepoSubstrate {
    type Slot = RepoSlot;
    type Location = PathBuf;
    type Encoded = String;
    type Resolver = RepoLocationResolver;
    type Codec = RepoCodec;
    type Executor = RepoExecutor;

    fn resolver(&self) -> &RepoLocationResolver { &self.resolver }
    fn codec(&self) -> &RepoCodec { &self.codec }
    fn executor(&self) -> &RepoExecutor { &self.executor }

    fn load_strategy(kind: EntityKind, field: &str) -> crate::substrate::pipeline::LoadStrategy {
        Self::schema_for(kind).load_strategy_for(field)
    }

    async fn exists(&self, refs: &[AnyEntityRef]) -> Result<Vec<bool>, SubstrateError> {
        let results = refs.iter().map(|any_ref| {
            let entity_json = Self::any_ref_to_json(any_ref);
            let schema = Self::schema_for(any_ref.kind());
            let path = self.resolver.resolve(schema.ref_asset.path_template, &entity_json);
            path.exists()
        }).collect();
        Ok(results)
    }

    async fn load(
        &self,
        entity: &StoreEntity,
        _fields: &[&str],
    ) -> Result<StoreEntity, SubstrateError> {
        let any_ref = entity.any_ref();
        let entity_json = Self::any_ref_to_json(&any_ref);
        let schema = Self::schema_for(any_ref.kind());
        let path = self.resolver.resolve(schema.ref_asset.path_template, &entity_json);
        let content = fs::read_to_string(&path)
            .map_err(|e| SubstrateError::Executor(ExecutorError::new(path.to_string_lossy(), e.to_string())))?;
        Self::decode_to_store_entity(&any_ref, &content, schema)
    }

    async fn persist(
        &self,
        changes: impl Iterator<Item = EntityChange<'_>> + Send,
    ) -> Result<(), Vec<SubstrateError>> {
        use crate::substrate::pipeline::{AssetOp, AssetRequest};
        let mut ops = Vec::new();
        let mut errors = Vec::new();

        for change in changes {
            match change {
                EntityChange::Added(entity) | EntityChange::Modified(entity, _) => {
                    let json = Self::entity_to_json(entity);
                    let schema = Self::schema_for(entity.any_ref().kind());
                    let path = self.resolver.resolve(schema.ref_asset.path_template, &json);

                    let field_map: std::collections::HashMap<&str, serde_json::Value> =
                        schema.ref_asset.fields.iter()
                            .filter_map(|fm| json.get(fm.key).map(|v| (fm.key, v.clone())))
                            .collect();

                    match self.codec.encode(&field_map, schema.ref_asset.fields) {
                        Ok(encoded) => ops.push(AssetRequest { location: path, op: AssetOp::Put(encoded) }),
                        Err(e) => errors.push(SubstrateError::Codec(e)),
                    }
                }
                EntityChange::Removed(any_ref) => {
                    let entity_json = Self::any_ref_to_json(any_ref);
                    let schema = Self::schema_for(any_ref.kind());
                    let path = self.resolver.resolve(schema.ref_asset.path_template, &entity_json);
                    ops.push(AssetRequest { location: path, op: AssetOp::Delete });
                }
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        if ops.is_empty() {
            return Ok(());
        }

        self.executor.execute(ops).map(|_| ()).map_err(|errs| {
            errs.into_iter().map(SubstrateError::Executor).collect()
        })
    }
}

