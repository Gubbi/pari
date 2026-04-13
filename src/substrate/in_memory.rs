use std::collections::HashMap;
use std::sync::Mutex;

use crate::entity::{AnyEntityRef, EntityKind, StoreEntity};
use crate::store::EntityChange;
use crate::substrate::error::SubstrateError;
use crate::substrate::pipeline::{self, Codec, CodecError, Executor, ExecutorError, FieldMapping, LoadStrategy, LocationResolver, Slot};
use crate::substrate::{Substrate};

#[derive(Clone, Copy)]
pub enum InMemorySlot {
    Value,
}

impl Slot for InMemorySlot {}

pub struct InMemoryResolver;

impl LocationResolver for InMemoryResolver {
    type Location = String;

    fn resolve(&self, path_template: &str, entity_json: &serde_json::Value) -> Self::Location {
        let id = entity_json
            .get("entity_ref")
            .and_then(|v| v.get("id"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        path_template.replace("{id}", id)
    }
}

pub struct InMemoryCodec;

impl Codec for InMemoryCodec {
    type Slot = InMemorySlot;
    type Encoded = String;

    fn encode(
        &self,
        fields: &HashMap<&str, serde_json::Value>,
        _schema: &[FieldMapping<Self::Slot>],
    ) -> Result<Self::Encoded, CodecError> {
        serde_json::to_string(fields).map_err(|e| CodecError::new("in_memory", e.to_string()))
    }

    fn decode(
        &self,
        raw: &Self::Encoded,
        _schema: &[FieldMapping<Self::Slot>],
    ) -> Result<HashMap<String, serde_json::Value>, CodecError> {
        serde_json::from_str(raw).map_err(|e| CodecError::new("in_memory", e.to_string()))
    }
}

pub struct InMemoryExecutor;

impl Executor for InMemoryExecutor {
    type Location = String;
    type Encoded = String;

    fn execute(
        &self,
        ops: Vec<pipeline::AssetRequest<Self::Location, Self::Encoded>>,
    ) -> Result<Vec<pipeline::AssetResponse<Self::Encoded>>, Vec<ExecutorError>> {
        Ok(ops
            .into_iter()
            .map(|req| match req.op {
                pipeline::AssetOp::Get => pipeline::AssetResponse::Data(String::new()),
                pipeline::AssetOp::Head => pipeline::AssetResponse::Exists(false),
                _ => pipeline::AssetResponse::Done,
            })
            .collect())
    }
}

/// In-memory substrate for tests that need a simple concrete backend.
pub struct InMemorySubstrate {
    entities: Mutex<HashMap<AnyEntityRef, StoreEntity>>,
}

impl InMemorySubstrate {
    pub fn new() -> Self {
        Self { entities: Mutex::new(HashMap::new()) }
    }

    pub fn seed(&self, any_ref: AnyEntityRef, entity: StoreEntity) {
        self.entities.lock().unwrap().insert(any_ref, entity);
    }
}

impl Default for InMemorySubstrate {
    fn default() -> Self {
        Self::new()
    }
}

impl Substrate for InMemorySubstrate {
    type Slot = InMemorySlot;
    type Location = String;
    type Encoded = String;
    type Resolver = InMemoryResolver;
    type Codec = InMemoryCodec;
    type Executor = InMemoryExecutor;

    fn resolver(&self) -> &InMemoryResolver { &InMemoryResolver }
    fn codec(&self) -> &InMemoryCodec { &InMemoryCodec }
    fn executor(&self) -> &InMemoryExecutor { &InMemoryExecutor }

    fn load_strategy(_: EntityKind, _: &str) -> LoadStrategy {
        LoadStrategy { prerequisites: vec![], mutable_without_load: true }
    }

    async fn exists(&self, refs: &[AnyEntityRef]) -> Result<Vec<bool>, SubstrateError> {
        let guard = self.entities.lock().unwrap();
        Ok(refs.iter().map(|r| guard.contains_key(r)).collect())
    }

    async fn load(
        &self,
        entity: &StoreEntity,
        _fields: &[&str],
    ) -> Result<StoreEntity, SubstrateError> {
        let any_ref = entity.any_ref();
        self.entities
            .lock()
            .unwrap()
            .get(&any_ref)
            .cloned()
            .ok_or_else(|| SubstrateError::from(ExecutorError::new(any_ref.id(), "not found")))
    }

    async fn persist(
        &self,
        changes: impl Iterator<Item = EntityChange<'_>> + Send,
    ) -> Result<(), Vec<SubstrateError>> {
        let mut entities = self.entities.lock().unwrap();
        for change in changes {
            match change {
                EntityChange::Added(entity) => {
                    entities.insert(entity.any_ref(), entity.clone());
                }
                EntityChange::Modified(entity, _) => {
                    entities.insert(entity.any_ref(), entity.clone());
                }
                EntityChange::Removed(any_ref) => {
                    entities.remove(any_ref);
                }
            }
        }
        Ok(())
    }
}
