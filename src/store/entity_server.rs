//! [`EntityServer`] — orchestration half of the store layer.
//!
//! Receives [`StoreRequest`]s from the workspace layer, decides which
//! manager operations, substrate calls, and validation runs each
//! request needs, and forwards typed replies back. State mutations
//! themselves live in the sibling `StoreManager` actor.
//!
//! Two senders provide the workspace entry point: a process-wide
//! `GLOBAL_SENDER` set by [`EntityServer::init`], and a thread-local
//! `OVERRIDE_SENDER` used by [`EntityServer::with`] for isolated test
//! scopes. [`store_sender`] prefers the override when present.

use std::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    sync::{Arc, OnceLock},
};

use futures::{
    channel::{mpsc as fmpsc, oneshot as foneshot},
    future::BoxFuture,
    stream::{FuturesUnordered, StreamExt},
    SinkExt,
};
use tokio::sync::mpsc;

use crate::{
    entity::{AnyEntityRef, TrackedEntity},
    error::{primitive::PrimitiveError, ActivityError},
    store::{
        lib::message::{StoreMessage, StoreRequest, StoreResponse},
        manager::{StoreManager, StoreManagerMessage, StoreManagerRequest, StoreManagerResponse},
    },
    substrate::SchemaBackedSubstrate,
    validation::{run_validations_for_entity, ValidationKind},
};

static GLOBAL_SENDER: OnceLock<mpsc::Sender<StoreMessage>> = OnceLock::new();

thread_local! {
    static OVERRIDE_SENDER: RefCell<Option<mpsc::Sender<StoreMessage>>> = RefCell::new(None);
}

// ---------------------------------------------------------------------------
// EntityServer
// ---------------------------------------------------------------------------

/// Orchestration actor for the store layer.
///
/// Holds a sender to the `StoreManager` actor and a
/// shared reference to the substrate. Cheap to clone — every clone
/// shares the same manager channel and substrate.
pub struct EntityServer<S> {
    store_tx: fmpsc::Sender<StoreManagerMessage>,
    substrate: Arc<S>,
}

impl<S> Clone for EntityServer<S> {
    fn clone(&self) -> Self {
        Self {
            store_tx: self.store_tx.clone(),
            substrate: Arc::clone(&self.substrate),
        }
    }
}

struct OverrideGuard {
    previous: Option<mpsc::Sender<StoreMessage>>,
}

impl Drop for OverrideGuard {
    fn drop(&mut self) {
        OVERRIDE_SENDER.with(|s| *s.borrow_mut() = self.previous.take());
    }
}

impl<S> EntityServer<S>
where
    S: SchemaBackedSubstrate,
{
    fn new(substrate: S) -> (Self, mpsc::Sender<StoreMessage>) {
        let (store_tx, store_rx) = fmpsc::channel(32);
        tokio::spawn(StoreManager::new().run(store_rx));

        let (tx, rx) = mpsc::channel(32);
        let server = EntityServer {
            store_tx,
            substrate: Arc::new(substrate),
        };
        tokio::spawn(server.clone().run(rx));

        (server, tx)
    }

    /// Spawn the server + manager pair and publish their sender as the
    /// process-wide workspace entry point. Panics if called twice.
    pub fn init(substrate: S) {
        let (_, tx) = Self::new(substrate);
        GLOBAL_SENDER
            .set(tx)
            .expect("EntityServer already initialized");
    }

    /// Scoped test entry point: spawn a fresh server + manager, run `f`
    /// with the test-local sender installed, and restore the previous
    /// sender on drop. Tests using this path are isolated from the
    /// process-wide server and from each other.
    pub async fn with<F, Fut>(substrate: S, f: F)
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = ()>,
    {
        let (_, tx) = Self::new(substrate);
        let previous = OVERRIDE_SENDER.with(|s| s.borrow_mut().replace(tx));
        let _guard = OverrideGuard { previous };
        f().await;
    }

    // -----------------------------------------------------------------------
    // Actor loop
    // -----------------------------------------------------------------------

    /// Actor loop — pulls messages off the workspace-facing channel and
    /// spawns each dispatch onto a `FuturesUnordered` so long-running
    /// orchestration (substrate round-trips, validation) interleaves
    /// across requests.
    async fn run(self, mut rx: mpsc::Receiver<StoreMessage>) {
        let mut in_flight: FuturesUnordered<BoxFuture<'static, ()>> = FuturesUnordered::new();

        loop {
            if in_flight.is_empty() {
                match rx.recv().await {
                    Some(msg) => in_flight.push(Box::pin(self.clone().dispatch(msg))),
                    None => break,
                }
            } else {
                tokio::select! {
                    msg = rx.recv() => match msg {
                        Some(msg) => in_flight.push(Box::pin(self.clone().dispatch(msg))),
                        None => {
                            while in_flight.next().await.is_some() {}
                            return;
                        }
                    },
                    _ = in_flight.next() => {}
                }
            }
        }
    }

    async fn dispatch(self, msg: StoreMessage) {
        let StoreMessage::Request { request, reply } = msg;
        let result = self.handle(request).await;
        let _ = reply.send(result);
    }

    // -----------------------------------------------------------------------
    // Request dispatch
    // -----------------------------------------------------------------------

    async fn handle(&self, request: StoreRequest) -> StoreResponse {
        match request {
            StoreRequest::Resolve { any_ref } => match self.resolve(any_ref).await {
                Ok(entity) => StoreResponse::Entity(entity),
                Err(e) => StoreResponse::Err(e),
            },
            StoreRequest::HasRef { any_ref } => match self.resolve(any_ref).await {
                Ok(_) => StoreResponse::Bool(true),
                Err(ActivityError::NonExistentData { .. }) => StoreResponse::Bool(false),
                Err(e) => StoreResponse::Err(e),
            },
            StoreRequest::Insert { entity } => match self.insert(entity).await {
                Ok(()) => StoreResponse::Unit,
                Err(e) => StoreResponse::Err(e),
            },
            StoreRequest::Checkout { any_ref } => match self.checkout(any_ref).await {
                Ok(entity) => StoreResponse::Entity(entity),
                Err(e) => StoreResponse::Err(e),
            },
            StoreRequest::Commit { entity } => match self.commit(entity).await {
                Ok(()) => StoreResponse::Unit,
                Err(e) => StoreResponse::Err(e),
            },
            StoreRequest::Remove { any_ref } => match self.remove(any_ref).await {
                Ok(entity) => StoreResponse::Entity(entity),
                Err(e) => StoreResponse::Err(e),
            },
            StoreRequest::Persist => match self.persist().await {
                Ok(()) => StoreResponse::Unit,
                Err(e) => StoreResponse::Err(e),
            },
            StoreRequest::Load { any_ref, field } => {
                match self.load_fields(&any_ref, &[&field], true).await {
                    Ok(()) => StoreResponse::Unit,
                    Err(e) => StoreResponse::Err(e),
                }
            }
            StoreRequest::EnsureMutable { any_ref, field } => {
                match self.ensure_mutable(&any_ref, &field).await {
                    Ok(()) => StoreResponse::Unit,
                    Err(e) => StoreResponse::Err(e),
                }
            }
            StoreRequest::UndoCheckout { any_ref } => match self.undo_checkout(any_ref).await {
                Ok(()) => StoreResponse::Unit,
                Err(e) => StoreResponse::Err(e),
            },
            StoreRequest::UndoCommit { any_ref } => match self.undo_commit(any_ref).await {
                Ok(()) => StoreResponse::Unit,
                Err(e) => StoreResponse::Err(e),
            },
            StoreRequest::Unload { any_ref } => match self.unload(any_ref).await {
                Ok(()) => StoreResponse::Unit,
                Err(e) => StoreResponse::Err(e),
            },
        }
    }

    // -----------------------------------------------------------------------
    // Handlers
    // -----------------------------------------------------------------------

    async fn resolve(&self, any_ref: AnyEntityRef) -> Result<TrackedEntity, ActivityError> {
        if let Some(entity) = self.store_get_entity(&any_ref).await? {
            return Ok(entity);
        }

        let exists = self
            .substrate
            .exists(&[any_ref.clone()])
            .await
            .map(|v| v[0])?;

        if !exists {
            return Err(ActivityError::non_existent_data(
                "store.resolve",
                PrimitiveError::entity_not_found("entity not found", any_ref.id()),
            ));
        }

        self.store_send(StoreManagerRequest::InsertStubs {
            refs: vec![any_ref.clone()],
        })
        .await?;

        Ok(TrackedEntity::make_stub(&any_ref))
    }

    async fn insert(&self, entity: TrackedEntity) -> Result<(), ActivityError> {
        run_validations_for_entity(
            &entity,
            &[],
            &[
                ValidationKind::Structural,
                ValidationKind::Semantic,
                ValidationKind::CrossEntity,
            ],
        )
        .await?;

        self.store_send(StoreManagerRequest::InsertEntity { entity })
            .await?;
        Ok(())
    }

    async fn checkout(&self, any_ref: AnyEntityRef) -> Result<TrackedEntity, ActivityError> {
        match self
            .store_send(StoreManagerRequest::Checkout { any_ref })
            .await?
        {
            StoreManagerResponse::Entity(e) => Ok(e),
            StoreManagerResponse::Err(e) => Err(map_store_primitive(e, "store.checkout")),
            _ => unreachable!(),
        }
    }

    async fn commit(&self, entity: TrackedEntity) -> Result<(), ActivityError> {
        let any_ref = entity.any_ref();

        let is_added = match self
            .store_send(StoreManagerRequest::IsAdded {
                any_ref: any_ref.clone(),
            })
            .await?
        {
            StoreManagerResponse::Bool(b) => b,
            _ => unreachable!(),
        };

        if is_added {
            run_validations_for_entity(
                &entity,
                &[],
                &[
                    ValidationKind::Structural,
                    ValidationKind::Semantic,
                    ValidationKind::CrossEntity,
                ],
            )
            .await?;
        } else if entity.has_dirty_fields() {
            let dirty = entity.dirty_fields();
            run_validations_for_entity(&entity, dirty.as_slice(), &[ValidationKind::CrossEntity])
                .await?;
        }

        self.store_send(StoreManagerRequest::CommitCheckout { entity })
            .await?;
        Ok(())
    }

    async fn remove(&self, any_ref: AnyEntityRef) -> Result<TrackedEntity, ActivityError> {
        match self
            .store_send(StoreManagerRequest::RemoveEntity { any_ref })
            .await?
        {
            StoreManagerResponse::Entity(e) => Ok(e),
            StoreManagerResponse::Err(e) => Err(map_store_primitive(e, "store.remove")),
            _ => unreachable!(),
        }
    }

    async fn persist(&self) -> Result<(), ActivityError> {
        let count = match self
            .store_send(StoreManagerRequest::PendingCheckoutCount)
            .await?
        {
            StoreManagerResponse::Count(n) => n,
            _ => unreachable!(),
        };

        if count > 0 {
            return Err(ActivityError::workspace_not_clean(
                "store.persist",
                PrimitiveError::pending_checkouts("persist blocked by pending checkouts", count),
            ));
        }

        let changes = match self
            .store_send(StoreManagerRequest::TakePersistSnapshot)
            .await?
        {
            StoreManagerResponse::Changes(c) => c,
            _ => unreachable!(),
        };

        self.substrate.persist(changes.into_iter()).await?;

        self.store_send(StoreManagerRequest::CommitPersist).await?;
        Ok(())
    }

    async fn undo_checkout(&self, any_ref: AnyEntityRef) -> Result<(), ActivityError> {
        match self
            .store_send(StoreManagerRequest::UndoCheckout { any_ref })
            .await?
        {
            StoreManagerResponse::Unit => Ok(()),
            StoreManagerResponse::Err(e) => Err(map_store_primitive(e, "store.undo_checkout")),
            _ => unreachable!(),
        }
    }

    async fn undo_commit(&self, any_ref: AnyEntityRef) -> Result<(), ActivityError> {
        match self
            .store_send(StoreManagerRequest::UndoCommit { any_ref })
            .await?
        {
            StoreManagerResponse::Unit => Ok(()),
            StoreManagerResponse::Err(e) => Err(map_store_primitive(e, "store.undo_commit")),
            _ => unreachable!(),
        }
    }

    async fn unload(&self, any_ref: AnyEntityRef) -> Result<(), ActivityError> {
        match self
            .store_send(StoreManagerRequest::UnloadEntity { any_ref })
            .await?
        {
            StoreManagerResponse::Unit => Ok(()),
            StoreManagerResponse::Err(e) => Err(map_store_primitive(e, "store.unload")),
            _ => unreachable!(),
        }
    }

    /// Prepare `field` on `any_ref` for overwrite by generated setters.
    ///
    /// Loads any declared prerequisites unconditionally; loads the field
    /// itself only when the substrate's load strategy reports
    /// `!mutable_without_load`. Prerequisites run even when the field
    /// itself needs no pre-load, so path resolution remains sound.
    async fn ensure_mutable(
        &self,
        any_ref: &AnyEntityRef,
        field: &str,
    ) -> Result<(), ActivityError> {
        let strategy = S::load_strategy(any_ref.kind(), field)?;

        for prereq in strategy.prerequisites {
            self.load_fields(any_ref, &[prereq], true).await?;
        }

        if !strategy.mutable_without_load {
            self.load_fields(any_ref, &[field], false).await?;
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // load_fields — recursive, boxed for async recursion
    // -----------------------------------------------------------------------

    /// Progressive load orchestration.
    ///
    /// Pulls uninitialized `fields` from the substrate across one or
    /// more rounds. Each round: determine what is still pending,
    /// resolve prerequisites (recursively), fetch from the substrate,
    /// validate the loaded snapshot before merge, initialize the
    /// store's canonical entity in place, and stub any newly surfaced
    /// cross-entity refs confirmed by `substrate.exists`.
    ///
    /// Validation failures on loaded data wrap as
    /// [`ActivityError::unpersistable_definition`]; the failing data
    /// is not merged.
    ///
    /// Boxed to satisfy async recursion (prerequisite resolution).
    fn load_fields<'a>(
        &'a self,
        any_ref: &'a AnyEntityRef,
        fields: &'a [&'a str],
        include_prerequisites: bool,
    ) -> Pin<Box<dyn Future<Output = Result<(), ActivityError>> + Send + 'a>> {
        Box::pin(async move {
            // Determine which fields still need loading.
            let mut pending = Vec::new();
            for field in fields {
                match self
                    .store_send(StoreManagerRequest::IsFieldLoaded {
                        any_ref: any_ref.clone(),
                        field: field.to_string(),
                    })
                    .await?
                {
                    StoreManagerResponse::Bool(false) => pending.push(*field),
                    _ => {}
                }
            }
            if pending.is_empty() {
                return Ok(());
            }

            // Load prerequisites first.
            if include_prerequisites {
                for field in pending.clone() {
                    let strategy = S::load_strategy(any_ref.kind(), field)?;
                    for prereq in strategy.prerequisites {
                        self.load_fields(any_ref, &[prereq], true).await?;
                    }
                }
            }

            // Re-check — prereqs may have caused side-effect loads.
            let mut still_pending = Vec::new();
            for field in &pending {
                match self
                    .store_send(StoreManagerRequest::IsFieldLoaded {
                        any_ref: any_ref.clone(),
                        field: field.to_string(),
                    })
                    .await?
                {
                    StoreManagerResponse::Bool(false) => still_pending.push(*field),
                    _ => {}
                }
            }
            if still_pending.is_empty() {
                return Ok(());
            }

            // Fetch current entity snapshot for the substrate load call.
            let current = match self
                .store_send(StoreManagerRequest::GetEntity {
                    any_ref: any_ref.clone(),
                })
                .await?
            {
                StoreManagerResponse::MaybeEntity(Some(e)) => e,
                _ => {
                    return Err(ActivityError::non_existent_data(
                        "store.load",
                        PrimitiveError::entity_not_found("entity not found", any_ref.id()),
                    ))
                }
            };

            let loaded = self.substrate.load(&current, &still_pending).await?;

            // Validate loaded fields, wrapped as unpersistable if they fail.
            run_validations_for_entity(
                &loaded,
                &still_pending,
                &[
                    ValidationKind::Structural,
                    ValidationKind::Semantic,
                    ValidationKind::CrossEntity,
                ],
            )
            .await
            .map_err(|e| ActivityError::unpersistable_definition("store.load", e.into_cause()))?;

            // Merge loaded fields into the store entity.
            self.store_send(StoreManagerRequest::InitializeField {
                any_ref: any_ref.clone(),
                loaded: loaded.clone(),
            })
            .await?;

            // Auto-stub any refs found in the loaded entity that are not yet in the store.
            let all_refs = loaded.all_refs();
            if !all_refs.is_empty() {
                let mut not_in_store = Vec::new();
                for r in all_refs {
                    match self
                        .store_send(StoreManagerRequest::ContainsRef { any_ref: r.clone() })
                        .await?
                    {
                        StoreManagerResponse::Bool(false) => not_in_store.push(r),
                        _ => {}
                    }
                }
                if !not_in_store.is_empty() {
                    if let Ok(results) = self.substrate.exists(&not_in_store).await {
                        let stubs: Vec<AnyEntityRef> = not_in_store
                            .into_iter()
                            .zip(results)
                            .filter_map(|(r, exists)| if exists { Some(r) } else { None })
                            .collect();
                        if !stubs.is_empty() {
                            self.store_send(StoreManagerRequest::InsertStubs { refs: stubs })
                                .await?;
                        }
                    }
                }
            }

            Ok(())
        })
    }

    // -----------------------------------------------------------------------
    // StoreManager communication helper
    // -----------------------------------------------------------------------

    async fn store_send(
        &self,
        request: StoreManagerRequest,
    ) -> Result<StoreManagerResponse, ActivityError> {
        let (reply_tx, reply_rx) = foneshot::channel();
        let mut tx = self.store_tx.clone();
        tx.send(StoreManagerMessage {
            request,
            reply: reply_tx,
        })
        .await
        .map_err(|_| {
            ActivityError::store_unavailable(
                "entity_server",
                PrimitiveError::store_unavailable("store manager unavailable"),
            )
        })?;
        reply_rx.await.map_err(|_| {
            ActivityError::store_unavailable(
                "entity_server",
                PrimitiveError::store_unavailable("store manager reply dropped"),
            )
        })
    }

    async fn store_get_entity(
        &self,
        any_ref: &AnyEntityRef,
    ) -> Result<Option<TrackedEntity>, ActivityError> {
        match self
            .store_send(StoreManagerRequest::GetEntity {
                any_ref: any_ref.clone(),
            })
            .await?
        {
            StoreManagerResponse::MaybeEntity(e) => Ok(e),
            _ => unreachable!(),
        }
    }
}

// ---------------------------------------------------------------------------
// Sender access — single entry point for workspace request dispatch
// ---------------------------------------------------------------------------

/// The workspace layer's single entry point to whichever
/// [`EntityServer`] is currently installed. Prefers the thread-local
/// override (set by [`EntityServer::with`]) over the process-wide
/// sender.
pub(crate) fn store_sender() -> mpsc::Sender<StoreMessage> {
    OVERRIDE_SENDER
        .with(|o| o.borrow().clone())
        .unwrap_or_else(|| {
            GLOBAL_SENDER
                .get()
                .expect("EntityServer not initialized")
                .clone()
        })
}

// ---------------------------------------------------------------------------
// PrimitiveError → ActivityError mapping for store data operations
// ---------------------------------------------------------------------------

/// Classify a `StoreManager`-emitted [`PrimitiveError`] into the
/// appropriate [`ActivityError`] kind for the orchestrating data
/// operation.
fn map_store_primitive(e: PrimitiveError, component: &'static str) -> ActivityError {
    match &e {
        PrimitiveError::EntityNotFound { .. } => ActivityError::non_existent_data(component, e),
        PrimitiveError::PendingCheckouts { .. } => ActivityError::workspace_not_clean(component, e),
        _ => ActivityError::checkout_lifecycle_violation(component, e),
    }
}
