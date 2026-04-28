//! [`EntityServer`] — stateless orchestrator for the store layer.
//!
//! Holds an `Arc<S>` substrate handle and a sender into the singleton
//! `StoreManager`. Caller-facing operations from
//! [`workspace`](crate::workspace) arrive as [`StoreRequest`] values and
//! are handled directly via `&self` methods — no actor loop, no channel
//! between workspace and the server. Each caller's task drives its own
//! orchestration sequence (validation, substrate calls, manager
//! round-trips) for that request.
//!
//! Two registrations provide workspace's entry point: a process-wide
//! `GLOBAL_ENTITY_SERVER` published by [`install_global_entity_server`]
//! (used by [`crate::init`]), and a thread-local `OVERRIDE_ENTITY_SERVER`
//! installed by [`install_override_entity_server`] (used by
//! [`crate::with`]). [`active_entity_server`] prefers the override.

use std::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    sync::{Arc, OnceLock},
};

use futures::{
    channel::{mpsc, oneshot},
    future::BoxFuture,
    SinkExt,
};

use crate::{
    entity::{AnyEntityRef, TrackedEntity},
    error::{primitive::PrimitiveError, ActivityError},
    store::{
        lib::message::{StoreRequest, StoreResponse},
        manager::{StoreManagerMessage, StoreManagerRequest, StoreManagerResponse},
    },
    substrate::SchemaBackedSubstrate,
    validation::{run_validations_for_entity, ValidationKind},
};

// ---------------------------------------------------------------------------
// Dispatcher trait — type-erased entry surface used by the workspace layer
// ---------------------------------------------------------------------------

/// Type-erased dispatch surface. The workspace layer calls into this
/// trait so it does not need to be generic over the substrate.
pub(crate) trait Dispatcher: Send + Sync {
    fn dispatch<'a>(&'a self, request: StoreRequest) -> BoxFuture<'a, StoreResponse>;
}

// ---------------------------------------------------------------------------
// EntityServer
// ---------------------------------------------------------------------------

/// Stateless orchestrator for the store layer.
///
/// Holds a sender to the singleton `StoreManager` actor and a shared
/// substrate handle. Multiple instances may coexist; they all dispatch
/// into the same `StoreManager`.
pub(crate) struct EntityServer<S> {
    store_tx: mpsc::Sender<StoreManagerMessage>,
    substrate: Arc<S>,
}

impl<S> EntityServer<S>
where
    S: SchemaBackedSubstrate,
{
    pub(crate) fn new(substrate: S, store_tx: mpsc::Sender<StoreManagerMessage>) -> Self {
        Self {
            store_tx,
            substrate: Arc::new(substrate),
        }
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

        let mut stubs = match self
            .store_send(StoreManagerRequest::InsertStubs {
                refs: vec![any_ref.clone()],
            })
            .await?
        {
            StoreManagerResponse::Entities(v) => v,
            _ => unreachable!(),
        };

        Ok(stubs.pop().expect("InsertStubs returns one stub per ref"))
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

        match self
            .store_send(StoreManagerRequest::InsertEntity { entity })
            .await?
        {
            StoreManagerResponse::Unit => Ok(()),
            StoreManagerResponse::Err(e) => Err(map_store_primitive(e, "store.insert")),
            _ => unreachable!(),
        }
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

        match self
            .store_send(StoreManagerRequest::CommitCheckout { entity })
            .await?
        {
            StoreManagerResponse::Unit => Ok(()),
            StoreManagerResponse::Err(e) => Err(map_store_primitive(e, "store.commit")),
            _ => unreachable!(),
        }
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
        let (reply_tx, reply_rx) = oneshot::channel();
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

impl<S> Dispatcher for EntityServer<S>
where
    S: SchemaBackedSubstrate,
{
    fn dispatch<'a>(&'a self, request: StoreRequest) -> BoxFuture<'a, StoreResponse> {
        Box::pin(self.handle(request))
    }
}

// ---------------------------------------------------------------------------
// Active entity server — workspace's single entry point
// ---------------------------------------------------------------------------

static GLOBAL_ENTITY_SERVER: OnceLock<Arc<dyn Dispatcher>> = OnceLock::new();

thread_local! {
    static OVERRIDE_ENTITY_SERVER: RefCell<Option<Arc<dyn Dispatcher>>> = const { RefCell::new(None) };
}

/// The workspace layer's single entry point to whichever `EntityServer`
/// is currently installed. Prefers the thread-local override (set by
/// [`crate::with`]) over the process-wide registration.
pub(crate) fn active_entity_server() -> Arc<dyn Dispatcher> {
    OVERRIDE_ENTITY_SERVER
        .with(|o| o.borrow().clone())
        .unwrap_or_else(|| {
            GLOBAL_ENTITY_SERVER
                .get()
                .expect("EntityServer not initialized")
                .clone()
        })
}

/// Install the process-wide `EntityServer`. Panics if called twice.
pub(crate) fn install_global_entity_server(entity_server: Arc<dyn Dispatcher>) {
    GLOBAL_ENTITY_SERVER
        .set(entity_server)
        .ok()
        .expect("EntityServer already initialized");
}

/// Guard returned by [`install_override_entity_server`]; restores the
/// previous thread-local override on drop.
pub(crate) struct OverrideGuard {
    previous: Option<Arc<dyn Dispatcher>>,
}

impl Drop for OverrideGuard {
    fn drop(&mut self) {
        OVERRIDE_ENTITY_SERVER.with(|s| *s.borrow_mut() = self.previous.take());
    }
}

/// Install a thread-local `EntityServer` override; the returned guard
/// restores the previous value on drop. Used by [`crate::with`] to
/// isolate test scopes from the process-wide registration.
pub(crate) fn install_override_entity_server(entity_server: Arc<dyn Dispatcher>) -> OverrideGuard {
    let previous = OVERRIDE_ENTITY_SERVER.with(|s| s.borrow_mut().replace(entity_server));
    OverrideGuard { previous }
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
