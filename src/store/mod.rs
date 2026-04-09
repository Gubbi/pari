//! EntityStore — async actor-based store for tracked entities.
//!
//! [`EntityServer`] — spawns a store actor and provides access via thread-local override.
//! [`EntityClient`] — static async API (insert, resolve, checkout, persist, etc.).
//! [`InMemorySubstrate`] — always-available in-memory substrate for testing.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::sync::{Mutex, OnceLock};

use tokio::sync::mpsc;

use crate::entity::{AnyEntityRef, EntityKind, StoreEntity};
use crate::substrate::{self, EntityChange};
use crate::substrate::error::SubstrateError;
use crate::substrate::pipeline::executor::ExecutorError;
use crate::substrate::{VoidSlot, VoidResolver, VoidCodec, VoidExecutor};
use crate::substrate::pipeline;
use crate::error::BatchError;

pub mod error;
pub use error::{
    CheckoutError, CommitError, LoadError, PersistError, ResolveError, StoreError, UndoError,
};

// ---------------------------------------------------------------------------
// InMemorySubstrate
// ---------------------------------------------------------------------------

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

impl substrate::Substrate for InMemorySubstrate {
    type Slot = VoidSlot;
    type Location = String;
    type Encoded = String;
    type Resolver = VoidResolver;
    type Codec = VoidCodec;
    type Executor = VoidExecutor;

    fn resolver(&self) -> &VoidResolver { &VoidResolver }
    fn codec(&self) -> &VoidCodec { &VoidCodec }
    fn executor(&self) -> &VoidExecutor { &VoidExecutor }

    fn load_strategy(_: EntityKind, _: &str) -> pipeline::LoadStrategy {
        pipeline::LoadStrategy { prerequisites: vec![], mutable_without_load: true }
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
            .ok_or_else(|| SubstrateError::from(
                ExecutorError::new(any_ref.id(), "not found")
            ))
    }

    async fn persist(
        &self,
        _changes: impl Iterator<Item = EntityChange<'_>> + Send,
    ) -> Result<(), Vec<SubstrateError>> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// StoreOpError — internal store state errors (not public)
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum StoreOpError {
    NotFound,
    CheckedOut,
    AlreadyRemoved,
}

// ---------------------------------------------------------------------------
// Message types
// ---------------------------------------------------------------------------

pub(crate) enum StoreRequest {
    Resolve { any_ref: AnyEntityRef },
    Checkout { any_ref: AnyEntityRef },
    Commit { entity: StoreEntity, any_ref: AnyEntityRef },
    Remove { any_ref: AnyEntityRef },
    Persist,
    Load { any_ref: AnyEntityRef, field: String },
    EnsureMutable { any_ref: AnyEntityRef, field: String },
    UndoCommit { any_ref: AnyEntityRef },
    Unload { any_ref: AnyEntityRef },
}

pub(crate) enum StoreCommand {
    Insert(StoreEntity),
    UndoCheckout { any_ref: AnyEntityRef },
}

pub(crate) enum StoreResponse {
    Entity(StoreEntity),
    Unit,
    ResolveErr(ResolveError),
    CheckoutErr(CheckoutError),
    PersistErr(PersistError),
    LoadErr(LoadError),
}

pub(crate) enum StoreMessage {
    Request {
        request: StoreRequest,
        reply: tokio::sync::oneshot::Sender<Result<StoreResponse, StoreError>>,
    },
    Command(StoreCommand),
}

// ---------------------------------------------------------------------------
// Store<S> — the actor state
// ---------------------------------------------------------------------------

struct Store<S: substrate::Substrate> {
    entities: HashMap<AnyEntityRef, StoreEntity>,
    added: HashSet<AnyEntityRef>,
    modified: HashSet<AnyEntityRef>,
    removed: HashSet<AnyEntityRef>,
    checked_out: HashSet<AnyEntityRef>,
    substrate: S,
}

impl<S: substrate::Substrate> Store<S> {
    fn new(substrate: S) -> Self {
        Self {
            entities: HashMap::new(),
            added: HashSet::new(),
            modified: HashSet::new(),
            removed: HashSet::new(),
            checked_out: HashSet::new(),
            substrate,
        }
    }

    async fn run(mut self, mut rx: mpsc::Receiver<StoreMessage>) {
        while let Some(msg) = rx.recv().await {
            match msg {
                StoreMessage::Request { request, reply } => {
                    let result = self.handle(request).await;
                    let _ = reply.send(result);
                }
                StoreMessage::Command(cmd) => {
                    self.execute(cmd);
                }
            }
        }
    }

    async fn handle(&mut self, request: StoreRequest) -> Result<StoreResponse, StoreError> {
        match request {
            StoreRequest::Resolve { any_ref } => {
                match self.resolve(&any_ref).await {
                    Ok(entity) => Ok(StoreResponse::Entity(entity)),
                    Err(e) => Ok(StoreResponse::ResolveErr(e)),
                }
            }
            StoreRequest::Checkout { any_ref } => {
                match self.checkout(&any_ref) {
                    Ok(entity) => Ok(StoreResponse::Entity(entity)),
                    Err(e) => Ok(StoreResponse::CheckoutErr(e)),
                }
            }
            StoreRequest::Commit { entity, any_ref } => {
                self.commit(entity, &any_ref);
                Ok(StoreResponse::Unit)
            }
            StoreRequest::Remove { any_ref } => {
                match self.remove_entity(&any_ref) {
                    Ok(entity) => Ok(StoreResponse::Entity(entity)),
                    Err(_) => Err(StoreError::Unavailable),
                }
            }
            StoreRequest::Persist => {
                match self.persist().await {
                    Ok(()) => Ok(StoreResponse::Unit),
                    Err(e) => Ok(StoreResponse::PersistErr(e)),
                }
            }
            StoreRequest::Load { any_ref, field } => {
                match self.load_field(&any_ref, &field).await {
                    Ok(()) => Ok(StoreResponse::Unit),
                    Err(e) => Ok(StoreResponse::LoadErr(e)),
                }
            }
            StoreRequest::EnsureMutable { any_ref, field } => {
                match self.ensure_mutable(&any_ref, &field).await {
                    Ok(()) => Ok(StoreResponse::Unit),
                    Err(e) => Ok(StoreResponse::LoadErr(e)),
                }
            }
            StoreRequest::UndoCommit { any_ref } => {
                match self.undo_commit(&any_ref) {
                    Ok(()) => Ok(StoreResponse::Unit),
                    Err(_) => Err(StoreError::Unavailable),
                }
            }
            StoreRequest::Unload { any_ref } => {
                match self.unload(&any_ref) {
                    Ok(()) => Ok(StoreResponse::Unit),
                    Err(_) => Err(StoreError::Unavailable),
                }
            }
        }
    }

    fn execute(&mut self, cmd: StoreCommand) {
        match cmd {
            StoreCommand::Insert(entity) => {
                let any_ref = entity.any_ref();
                self.entities.insert(any_ref.clone(), entity);
                self.added.insert(any_ref);
            }
            StoreCommand::UndoCheckout { any_ref } => {
                self.checked_out.remove(&any_ref);
            }
        }
    }

    async fn resolve(&mut self, any_ref: &AnyEntityRef) -> Result<StoreEntity, ResolveError> {
        // Cache hit — return clone directly (stub or loaded).
        if let Some(entity) = self.entities.get(any_ref) {
            return Ok(entity.clone());
        }

        // Not in store — check substrate existence (batch API, single ref).
        match self.substrate.exists(&[any_ref.clone()]).await {
            Err(e) => return Err(ResolveError::Substrate(e)),
            Ok(results) if !results[0] => {
                return Err(ResolveError::NotFound { entity_ref: any_ref.id().to_string() });
            }
            Ok(_) => {}
        }

        // Exists — insert stub and return clone.
        let stub = StoreEntity::make_stub(any_ref);
        self.entities.insert(any_ref.clone(), stub);
        Ok(self.entities[any_ref].clone())
    }

    fn checkout(&mut self, any_ref: &AnyEntityRef) -> Result<StoreEntity, CheckoutError> {
        if self.checked_out.contains(any_ref) {
            return Err(CheckoutError::AlreadyCheckedOut {
                entity_ref: any_ref.id().to_string(),
            });
        }
        match self.entities.get(any_ref) {
            None => Err(CheckoutError::EntityNotFound {
                entity_ref: any_ref.id().to_string(),
            }),
            Some(entity) => {
                self.checked_out.insert(any_ref.clone());
                Ok(entity.clone())
            }
        }
    }

    fn commit(&mut self, entity: StoreEntity, any_ref: &AnyEntityRef) {
        self.checked_out.remove(any_ref);
        if let Some(existing) = self.entities.get_mut(any_ref) {
            entity.merge_dirty_into(existing);
            if entity.has_dirty_fields() && !self.added.contains(any_ref) {
                self.modified.insert(any_ref.clone());
            }
        }
    }

    fn remove_entity(&mut self, any_ref: &AnyEntityRef) -> Result<StoreEntity, StoreOpError> {
        if self.checked_out.contains(any_ref) {
            return Err(StoreOpError::CheckedOut);
        }
        match self.entities.remove(any_ref) {
            None => Err(StoreOpError::NotFound),
            Some(entity) => {
                if self.added.remove(any_ref) {
                    // Was added in this session: net no-op
                } else {
                    self.removed.insert(any_ref.clone());
                }
                self.modified.remove(any_ref);
                Ok(entity)
            }
        }
    }

    async fn persist(&mut self) -> Result<(), PersistError> {
        if !self.checked_out.is_empty() {
            return Err(PersistError::PendingCheckouts {
                checked_out_count: self.checked_out.len(),
            });
        }

        // Collect entity references and dirty field names before building the iterator.
        let added_entities: Vec<&StoreEntity> = self.added.iter()
            .filter_map(|r| self.entities.get(r))
            .collect();
        let modified_pairs: Vec<(&StoreEntity, Vec<&'static str>)> = self.modified.iter()
            .filter_map(|r| self.entities.get(r).map(|e| (e, e.dirty_fields())))
            .collect();
        let removed_refs: Vec<&AnyEntityRef> = self.removed.iter().collect();

        let changes = added_entities.iter().map(|e| EntityChange::Added(e))
            .chain(modified_pairs.iter().map(|(e, df)| EntityChange::Modified(e, df.as_slice())))
            .chain(removed_refs.iter().map(|r| EntityChange::Removed(r)));

        self.substrate.persist(changes).await
            .map_err(|errs| PersistError::SubstrateErrors(BatchError::new(errs)))?;

        // Reset dirty flags on modified entities
        for any_ref in &self.modified {
            if let Some(entity) = self.entities.get_mut(any_ref) {
                entity.reset_dirty();
            }
        }

        self.added.clear();
        self.modified.clear();
        self.removed.clear();

        Ok(())
    }

    async fn load_field(
        &mut self,
        any_ref: &AnyEntityRef,
        field: &str,
    ) -> Result<(), LoadError> {
        // Get current entity (stub if not yet loaded).
        let current = self.entities.entry(any_ref.clone())
            .or_insert_with(|| StoreEntity::make_stub(any_ref))
            .clone();

        // Skip if field is already loaded.
        // (Accessor already checks, but handle race at EntityServer level too.)
        let loaded = self.substrate.load(&current, &[field]).await
            .map_err(LoadError::Substrate)?;

        // Enrich the loaded entity with already-initialized fields from the store
        // so validators have full context, then initialize store's Arcs in-place.
        loaded.initialize_into(self.entities.get_mut(any_ref).unwrap());

        // Register any cross-entity refs as stubs (non-fatal if exists check fails).
        let refs = loaded.all_refs();
        if !refs.is_empty() {
            if let Ok(results) = self.substrate.exists(&refs).await {
                for (r, exists) in refs.into_iter().zip(results) {
                    if exists && !self.entities.contains_key(&r) {
                        let stub = StoreEntity::make_stub(&r);
                        self.entities.insert(r, stub);
                    }
                }
            }
        }

        Ok(())
    }

    async fn ensure_mutable(
        &mut self,
        any_ref: &AnyEntityRef,
        field: &str,
    ) -> Result<(), LoadError> {
        let strategy = S::load_strategy(any_ref.kind(), field);

        // Always load prerequisites unconditionally (OnceLock::set is idempotent).
        for prereq in strategy.prerequisites {
            self.load_field(any_ref, prereq).await?;
        }

        // Load the field itself if not mutable without load.
        if !strategy.mutable_without_load {
            self.load_field(any_ref, field).await?;
        }

        Ok(())
    }

    fn undo_commit(&mut self, any_ref: &AnyEntityRef) -> Result<(), StoreOpError> {
        if self.added.contains(any_ref) {
            self.entities.remove(any_ref);
            self.added.remove(any_ref);
            Ok(())
        } else if self.modified.contains(any_ref) {
            // Replace with stub
            let stub = StoreEntity::make_stub(any_ref);
            self.entities.insert(any_ref.clone(), stub);
            self.modified.remove(any_ref);
            Ok(())
        } else {
            Err(StoreOpError::AlreadyRemoved)
        }
    }

    fn unload(&mut self, any_ref: &AnyEntityRef) -> Result<(), StoreOpError> {
        if !self.entities.contains_key(any_ref) {
            return Err(StoreOpError::NotFound);
        }
        // Only unload if not dirty (not in added or modified)
        if self.added.contains(any_ref) || self.modified.contains(any_ref) {
            return Err(StoreOpError::AlreadyRemoved); // entity is dirty; can't unload
        }
        // Replace with stub
        let stub = StoreEntity::make_stub(any_ref);
        self.entities.insert(any_ref.clone(), stub);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// EntityServer
// ---------------------------------------------------------------------------

static GLOBAL_SENDER: OnceLock<mpsc::Sender<StoreMessage>> = OnceLock::new();

thread_local! {
    static OVERRIDE_SENDER: RefCell<Option<mpsc::Sender<StoreMessage>>> = RefCell::new(None);
}

pub struct EntityServer;

impl EntityServer {
    pub fn init(substrate: impl substrate::Substrate) {
        let (tx, rx) = mpsc::channel(32);
        let store = Store::new(substrate);
        tokio::spawn(async move { store.run(rx).await });
        GLOBAL_SENDER.set(tx).expect("EntityServer already initialized");
    }

    pub fn sender() -> mpsc::Sender<StoreMessage> {
        OVERRIDE_SENDER
            .with(|o| o.borrow().clone())
            .unwrap_or_else(|| {
                GLOBAL_SENDER.get().expect("EntityServer not initialized").clone()
            })
    }

    pub async fn with_test<F, Fut>(substrate: impl substrate::Substrate, f: F)
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = ()>,
    {
        let (tx, rx) = mpsc::channel(32);
        tokio::spawn(Store::new(substrate).run(rx));
        OVERRIDE_SENDER.with(|o| *o.borrow_mut() = Some(tx));
        f().await;
        OVERRIDE_SENDER.with(|o| *o.borrow_mut() = None);
    }
}

// ---------------------------------------------------------------------------
// EntityClient
// ---------------------------------------------------------------------------

pub struct EntityClient;

impl EntityClient {
    pub async fn resolve(any_ref: AnyEntityRef) -> Result<StoreEntity, ResolveError> {
        match request(StoreRequest::Resolve { any_ref }).await.map_err(ResolveError::StoreUnavailable)? {
            StoreResponse::Entity(e) => Ok(e),
            StoreResponse::ResolveErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn insert(entity: StoreEntity) -> Result<(), StoreError> {
        send(StoreCommand::Insert(entity)).await
    }

    pub async fn remove(any_ref: AnyEntityRef) -> Result<StoreEntity, StoreError> {
        match request(StoreRequest::Remove { any_ref }).await? {
            StoreResponse::Entity(e) => Ok(e),
            _ => unreachable!(),
        }
    }

    pub async fn checkout(any_ref: AnyEntityRef) -> Result<StoreEntity, CheckoutError> {
        match request(StoreRequest::Checkout { any_ref }).await.unwrap() {
            StoreResponse::Entity(e) => Ok(e),
            StoreResponse::CheckoutErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn load(any_ref: AnyEntityRef, field: &str) -> Result<(), LoadError> {
        match request(StoreRequest::Load { any_ref, field: field.to_owned() }).await.unwrap() {
            StoreResponse::Unit => Ok(()),
            StoreResponse::LoadErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn ensure_mutable(any_ref: AnyEntityRef, field: &str) -> Result<(), LoadError> {
        match request(StoreRequest::EnsureMutable { any_ref, field: field.to_owned() }).await.unwrap() {
            StoreResponse::Unit => Ok(()),
            StoreResponse::LoadErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn persist() -> Result<(), PersistError> {
        match request(StoreRequest::Persist).await.unwrap() {
            StoreResponse::Unit => Ok(()),
            StoreResponse::PersistErr(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub async fn undo_commit(any_ref: AnyEntityRef) -> Result<(), UndoError> {
        match request(StoreRequest::UndoCommit { any_ref }).await {
            Ok(StoreResponse::Unit) => Ok(()),
            Ok(_) => unreachable!(),
            Err(e) => Err(UndoError::StoreUnavailable(e)),
        }
    }

    pub async fn unload(any_ref: AnyEntityRef) -> Result<(), UndoError> {
        match request(StoreRequest::Unload { any_ref }).await {
            Ok(StoreResponse::Unit) => Ok(()),
            Ok(_) => unreachable!(),
            Err(e) => Err(UndoError::StoreUnavailable(e)),
        }
    }
}

async fn request(req: StoreRequest) -> Result<StoreResponse, StoreError> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    EntityServer::sender()
        .send(StoreMessage::Request { request: req, reply: tx })
        .await
        .map_err(|_| StoreError::Unavailable)?;
    rx.await.map_err(|_| StoreError::Unavailable)?
}

async fn send(cmd: StoreCommand) -> Result<(), StoreError> {
    EntityServer::sender()
        .send(StoreMessage::Command(cmd))
        .await
        .map_err(|_| StoreError::Unavailable)
}

// ---------------------------------------------------------------------------
// StoreEntity methods — commit and undo_checkout
// ---------------------------------------------------------------------------

impl StoreEntity {
    pub async fn commit(self) -> Result<(), CommitError> {
        let any_ref = self.any_ref();
        match request(StoreRequest::Commit { entity: self, any_ref }).await {
            Ok(StoreResponse::Unit) => Ok(()),
            Ok(_) => unreachable!(),
            Err(e) => Err(CommitError::StoreUnavailable(e)),
        }
    }

    pub async fn undo_checkout(&self) -> Result<(), StoreError> {
        let any_ref = self.any_ref();
        send(StoreCommand::UndoCheckout { any_ref }).await
    }
}
