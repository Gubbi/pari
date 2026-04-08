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

use crate::entity::{AnyEntityRef, StoreEntity};
use crate::validation::SubstrateError;

// ---------------------------------------------------------------------------
// Substrate trait (store-local; separate from substrate::Substrate)
// ---------------------------------------------------------------------------

pub trait Substrate: Send + Sync + 'static {
    fn exists(
        &self,
        any_ref: AnyEntityRef,
    ) -> impl Future<Output = Result<bool, SubstrateError>> + Send;

    fn load(
        &self,
        any_ref: AnyEntityRef,
        fields: Vec<String>,
    ) -> impl Future<Output = Result<StoreEntity, SubstrateError>> + Send;

    fn atomic_persist(
        &self,
        changes: Vec<StoreEntityChange>,
    ) -> impl Future<Output = Result<(), Vec<SubstrateError>>> + Send;
}

// ---------------------------------------------------------------------------
// StoreEntityChange
// ---------------------------------------------------------------------------

pub enum StoreEntityChange {
    Added(StoreEntity),
    Modified(StoreEntity, Vec<&'static str>),
    Removed(AnyEntityRef),
}

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

impl Substrate for InMemorySubstrate {
    async fn exists(&self, any_ref: AnyEntityRef) -> Result<bool, SubstrateError> {
        Ok(self.entities.lock().unwrap().contains_key(&any_ref))
    }

    async fn load(
        &self,
        any_ref: AnyEntityRef,
        _fields: Vec<String>,
    ) -> Result<StoreEntity, SubstrateError> {
        self.entities
            .lock()
            .unwrap()
            .get(&any_ref)
            .cloned()
            .ok_or_else(|| SubstrateError {
                path: any_ref.id().to_string(),
                message: "not found".to_string(),
            })
    }

    async fn atomic_persist(
        &self,
        _changes: Vec<StoreEntityChange>,
    ) -> Result<(), Vec<SubstrateError>> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum StoreError {
    Unavailable,
    /// Entity is already checked out (encoded internally from CheckoutError)
    CheckedOut,
    /// Persist blocked by pending checkouts (encoded internally from PersistError)
    PendingCheckouts,
    /// Undo operation is in wrong state (encoded internally from StoreOpError)
    UndoWrongState,
}

#[derive(Debug)]
pub enum StoreOpError {
    NotFound,
    CheckedOut,
    AlreadyRemoved,
}

#[derive(Debug)]
pub enum ResolveError {
    NotFound,
    SubstrateError(SubstrateError),
}

#[derive(Debug)]
pub enum CheckoutError {
    AlreadyCheckedOut,
    EntityNotFound,
    StoreUnavailable,
}

#[derive(Debug)]
pub enum CommitError {
    NotCheckedOut,
    StoreUnavailable,
}

#[derive(Debug)]
pub enum PersistError {
    PendingCheckouts,
    StoreUnavailable,
    SubstrateError(Vec<SubstrateError>),
}

#[derive(Debug)]
pub enum UndoError {
    WrongState,
    StoreUnavailable,
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
    Load { any_ref: AnyEntityRef, fields: Vec<String> },
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

struct Store<S: Substrate> {
    entities: HashMap<AnyEntityRef, StoreEntity>,
    added: HashSet<AnyEntityRef>,
    modified: HashSet<AnyEntityRef>,
    removed: HashSet<AnyEntityRef>,
    checked_out: HashSet<AnyEntityRef>,
    substrate: S,
}

impl<S: Substrate> Store<S> {
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
                    Err(_) => Err(StoreError::Unavailable),
                }
            }
            StoreRequest::Checkout { any_ref } => {
                match self.checkout(&any_ref) {
                    Ok(entity) => Ok(StoreResponse::Entity(entity)),
                    Err(CheckoutError::AlreadyCheckedOut) => Err(StoreError::CheckedOut),
                    Err(_) => Err(StoreError::Unavailable),
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
                    Err(PersistError::PendingCheckouts) => Err(StoreError::PendingCheckouts),
                    Err(_) => Err(StoreError::Unavailable),
                }
            }
            StoreRequest::Load { any_ref, fields } => {
                match self.load_from_substrate(&any_ref, fields).await {
                    Ok(entity) => Ok(StoreResponse::Entity(entity)),
                    Err(_) => Err(StoreError::Unavailable),
                }
            }
            StoreRequest::UndoCommit { any_ref } => {
                match self.undo_commit(&any_ref) {
                    Ok(()) => Ok(StoreResponse::Unit),
                    Err(_) => Err(StoreError::UndoWrongState),
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
        // If already in entities (and not in removed), return it
        if self.entities.contains_key(any_ref) {
            return Ok(self.entities[any_ref].clone());
        }

        // Otherwise check substrate
        match self.substrate.exists(any_ref.clone()).await {
            Ok(false) => Err(ResolveError::NotFound),
            Ok(true) => {
                // Load from substrate
                match self.substrate.load(any_ref.clone(), vec![]).await {
                    Ok(entity) => {
                        self.entities.insert(any_ref.clone(), entity.clone());
                        Ok(entity)
                    }
                    Err(e) => Err(ResolveError::SubstrateError(e)),
                }
            }
            Err(e) => Err(ResolveError::SubstrateError(e)),
        }
    }

    fn checkout(&mut self, any_ref: &AnyEntityRef) -> Result<StoreEntity, CheckoutError> {
        if self.checked_out.contains(any_ref) {
            return Err(CheckoutError::AlreadyCheckedOut);
        }
        match self.entities.get(any_ref) {
            None => Err(CheckoutError::EntityNotFound),
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
            return Err(PersistError::PendingCheckouts);
        }

        let mut changes: Vec<StoreEntityChange> = Vec::new();

        for any_ref in &self.added {
            if let Some(entity) = self.entities.get(any_ref) {
                changes.push(StoreEntityChange::Added(entity.clone()));
            }
        }

        for any_ref in &self.modified {
            if let Some(entity) = self.entities.get(any_ref) {
                let dirty = entity.dirty_fields();
                changes.push(StoreEntityChange::Modified(entity.clone(), dirty));
            }
        }

        for any_ref in &self.removed {
            changes.push(StoreEntityChange::Removed(any_ref.clone()));
        }

        self.substrate.atomic_persist(changes).await.map_err(PersistError::SubstrateError)?;

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

    async fn load_from_substrate(
        &mut self,
        any_ref: &AnyEntityRef,
        fields: Vec<String>,
    ) -> Result<StoreEntity, SubstrateError> {
        let loaded = self.substrate.load(any_ref.clone(), fields).await?;

        // Merge into existing entity or insert
        if let Some(existing) = self.entities.get_mut(any_ref) {
            loaded.initialize_into(existing);
        } else {
            self.entities.insert(any_ref.clone(), loaded.clone());
        }

        // Register all_refs as stubs if not already in store
        for r in loaded.all_refs() {
            if !self.entities.contains_key(&r) {
                let stub = StoreEntity::make_stub(&r);
                self.entities.insert(r, stub);
            }
        }

        Ok(self.entities[any_ref].clone())
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
    pub fn init(substrate: impl Substrate) {
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

    pub async fn with_test<F, Fut>(substrate: impl Substrate, f: F)
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
    pub async fn resolve(any_ref: AnyEntityRef) -> Result<StoreEntity, StoreError> {
        match request(StoreRequest::Resolve { any_ref }).await? {
            StoreResponse::Entity(e) => Ok(e),
            StoreResponse::Unit => unreachable!(),
        }
    }

    pub async fn insert(entity: StoreEntity) -> Result<(), StoreError> {
        send(StoreCommand::Insert(entity)).await
    }

    pub async fn remove(any_ref: AnyEntityRef) -> Result<StoreEntity, StoreError> {
        match request(StoreRequest::Remove { any_ref }).await? {
            StoreResponse::Entity(e) => Ok(e),
            StoreResponse::Unit => unreachable!(),
        }
    }

    pub async fn checkout(any_ref: AnyEntityRef) -> Result<StoreEntity, CheckoutError> {
        match request(StoreRequest::Checkout { any_ref }).await {
            Ok(StoreResponse::Entity(e)) => Ok(e),
            Ok(StoreResponse::Unit) => unreachable!(),
            Err(StoreError::CheckedOut) => Err(CheckoutError::AlreadyCheckedOut),
            Err(_) => Err(CheckoutError::StoreUnavailable),
        }
    }

    pub async fn persist() -> Result<(), PersistError> {
        match request(StoreRequest::Persist).await {
            Ok(StoreResponse::Unit) => Ok(()),
            Ok(StoreResponse::Entity(_)) => unreachable!(),
            Err(StoreError::PendingCheckouts) => Err(PersistError::PendingCheckouts),
            Err(_) => Err(PersistError::StoreUnavailable),
        }
    }

    pub async fn undo_commit(any_ref: AnyEntityRef) -> Result<(), UndoError> {
        match request(StoreRequest::UndoCommit { any_ref }).await {
            Ok(StoreResponse::Unit) => Ok(()),
            Ok(StoreResponse::Entity(_)) => unreachable!(),
            Err(StoreError::UndoWrongState) => Err(UndoError::WrongState),
            Err(_) => Err(UndoError::StoreUnavailable),
        }
    }

    pub async fn unload(any_ref: AnyEntityRef) -> Result<(), UndoError> {
        match request(StoreRequest::Unload { any_ref }).await {
            Ok(StoreResponse::Unit) => Ok(()),
            Ok(StoreResponse::Entity(_)) => unreachable!(),
            Err(_) => Err(UndoError::StoreUnavailable),
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
            Err(_) => Err(CommitError::StoreUnavailable),
        }
    }

    pub async fn undo_checkout(&self) -> Result<(), StoreError> {
        let any_ref = self.any_ref();
        send(StoreCommand::UndoCheckout { any_ref }).await
    }
}
