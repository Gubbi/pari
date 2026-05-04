//! [`StoreServer`] — stateless orchestrator for the store layer.
//!
//! Holds an `Arc<S>` substrate handle and a sender into the [`Store`]
//! actor. Caller-facing operations from [`workspace`](crate::workspace)
//! arrive as [`WorkspaceRequest`] values and are handled directly via
//! `&self` methods — no actor loop, no channel between workspace and
//! the server. Each caller's task drives its own orchestration sequence
//! (validation, substrate calls, store round-trips) for that request.
//!
//! Two registrations provide workspace's entry point: a process-wide
//! `GLOBAL_STORE_SERVER` published by [`install_global_store_server`]
//! (used by [`crate::init`]), and a thread-local `OVERRIDE_STORE_SERVER`
//! installed by [`install_override_store_server`] (used by
//! [`crate::with`]). [`active_store_server`] prefers the override.

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
    store::lib::{
        store_request::{StoreMessage, StoreRequest, StoreResponse},
        workspace_request::{WorkspaceRequest, WorkspaceResponse},
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
    fn dispatch<'a>(&'a self, request: WorkspaceRequest) -> BoxFuture<'a, WorkspaceResponse>;
}

// ---------------------------------------------------------------------------
// StoreServer
// ---------------------------------------------------------------------------

/// Stateless orchestrator for the store layer.
///
/// Holds a sender to the [`Store`] actor and a shared substrate handle.
/// Multiple instances may coexist; they all dispatch into the same
/// [`Store`].
pub(crate) struct StoreServer<S> {
    store_tx: mpsc::Sender<StoreMessage>,
    substrate: Arc<S>,
}

impl<S> StoreServer<S>
where
    S: SchemaBackedSubstrate,
{
    pub(crate) fn new(substrate: S, store_tx: mpsc::Sender<StoreMessage>) -> Self {
        Self {
            store_tx,
            substrate: Arc::new(substrate),
        }
    }

    // -----------------------------------------------------------------------
    // Request dispatch
    // -----------------------------------------------------------------------

    async fn handle(&self, request: WorkspaceRequest) -> WorkspaceResponse {
        match request {
            WorkspaceRequest::Resolve { any_ref } => match self.resolve(any_ref).await {
                Ok(entity) => WorkspaceResponse::Entity(entity),
                Err(e) => WorkspaceResponse::Err(e),
            },
            WorkspaceRequest::HasRef { any_ref } => match self.resolve(any_ref).await {
                Ok(_) => WorkspaceResponse::Bool(true),
                Err(ActivityError::NonExistentData { .. }) => WorkspaceResponse::Bool(false),
                Err(e) => WorkspaceResponse::Err(e),
            },
            WorkspaceRequest::Insert { entity } => match self.insert(entity).await {
                Ok(()) => WorkspaceResponse::Unit,
                Err(e) => WorkspaceResponse::Err(e),
            },
            WorkspaceRequest::Checkout { any_ref } => match self.checkout(any_ref).await {
                Ok(entity) => WorkspaceResponse::Entity(entity),
                Err(e) => WorkspaceResponse::Err(e),
            },
            WorkspaceRequest::Commit { entity } => match self.commit(entity).await {
                Ok(()) => WorkspaceResponse::Unit,
                Err(e) => WorkspaceResponse::Err(e),
            },
            WorkspaceRequest::Remove { any_ref } => match self.remove(any_ref).await {
                Ok(entity) => WorkspaceResponse::Entity(entity),
                Err(e) => WorkspaceResponse::Err(e),
            },
            WorkspaceRequest::Persist => match self.persist().await {
                Ok(()) => WorkspaceResponse::Unit,
                Err(e) => WorkspaceResponse::Err(e),
            },
            WorkspaceRequest::Load { any_ref, field } => {
                match self.load_fields(&any_ref, &[&field], true).await {
                    Ok(()) => WorkspaceResponse::Unit,
                    Err(e) => WorkspaceResponse::Err(e),
                }
            }
            WorkspaceRequest::EnsureMutable { any_ref, field } => {
                match self.ensure_mutable(&any_ref, &field).await {
                    Ok(()) => WorkspaceResponse::Unit,
                    Err(e) => WorkspaceResponse::Err(e),
                }
            }
            WorkspaceRequest::UndoCheckout { any_ref } => match self.undo_checkout(any_ref).await {
                Ok(()) => WorkspaceResponse::Unit,
                Err(e) => WorkspaceResponse::Err(e),
            },
            WorkspaceRequest::Revert { any_ref } => match self.revert(any_ref).await {
                Ok(()) => WorkspaceResponse::Unit,
                Err(e) => WorkspaceResponse::Err(e),
            },
            WorkspaceRequest::Forget { any_ref } => match self.forget(any_ref).await {
                Ok(()) => WorkspaceResponse::Unit,
                Err(e) => WorkspaceResponse::Err(e),
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
            .store_send(StoreRequest::InsertStubs {
                refs: vec![any_ref.clone()],
            })
            .await?
        {
            StoreResponse::Entities(v) => v,
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
            .store_send(StoreRequest::InsertEntity { entity })
            .await?
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::Err(e) => Err(map_store_primitive(e, "store.insert")),
            _ => unreachable!(),
        }
    }

    async fn checkout(&self, any_ref: AnyEntityRef) -> Result<TrackedEntity, ActivityError> {
        match self.store_send(StoreRequest::Checkout { any_ref }).await? {
            StoreResponse::Entity(e) => Ok(e),
            StoreResponse::Err(e) => Err(map_store_primitive(e, "store.checkout")),
            _ => unreachable!(),
        }
    }

    async fn commit(&self, entity: TrackedEntity) -> Result<(), ActivityError> {
        let any_ref = entity.any_ref();

        let is_added = match self
            .store_send(StoreRequest::IsAdded {
                any_ref: any_ref.clone(),
            })
            .await?
        {
            StoreResponse::Bool(b) => b,
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
            .store_send(StoreRequest::CommitCheckout { entity })
            .await?
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::Err(e) => Err(map_store_primitive(e, "store.commit")),
            _ => unreachable!(),
        }
    }

    async fn remove(&self, any_ref: AnyEntityRef) -> Result<TrackedEntity, ActivityError> {
        match self
            .store_send(StoreRequest::RemoveEntity { any_ref })
            .await?
        {
            StoreResponse::Entity(e) => Ok(e),
            StoreResponse::Err(e) => Err(map_store_primitive(e, "store.remove")),
            _ => unreachable!(),
        }
    }

    async fn persist(&self) -> Result<(), ActivityError> {
        let count = match self.store_send(StoreRequest::PendingCheckoutCount).await? {
            StoreResponse::Count(n) => n,
            _ => unreachable!(),
        };

        if count > 0 {
            return Err(ActivityError::workspace_not_clean(
                "store.persist",
                PrimitiveError::pending_checkouts("persist blocked by pending checkouts", count),
            ));
        }

        let changes = match self.store_send(StoreRequest::TakePersistSnapshot).await? {
            StoreResponse::Changes(c) => c,
            _ => unreachable!(),
        };

        self.substrate.persist(changes.into_iter()).await?;

        self.store_send(StoreRequest::CommitPersist).await?;
        Ok(())
    }

    async fn undo_checkout(&self, any_ref: AnyEntityRef) -> Result<(), ActivityError> {
        match self
            .store_send(StoreRequest::UndoCheckout { any_ref })
            .await?
        {
            StoreResponse::Unit => Ok(()),
            StoreResponse::Err(e) => Err(map_store_primitive(e, "store.undo_checkout")),
            _ => unreachable!(),
        }
    }

    async fn revert(&self, any_ref: AnyEntityRef) -> Result<(), ActivityError> {
        match self.store_send(StoreRequest::Revert { any_ref }).await? {
            StoreResponse::Unit => Ok(()),
            StoreResponse::Err(e) => Err(map_store_primitive(e, "store.revert")),
            _ => unreachable!(),
        }
    }

    async fn forget(&self, any_ref: AnyEntityRef) -> Result<(), ActivityError> {
        match self.store_send(StoreRequest::Forget { any_ref }).await? {
            StoreResponse::Unit => Ok(()),
            StoreResponse::Err(e) => Err(map_store_primitive(e, "store.forget")),
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
        let strategy = S::load_strategy(any_ref, field)?;

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
                    .store_send(StoreRequest::IsFieldLoaded {
                        any_ref: any_ref.clone(),
                        field: field.to_string(),
                    })
                    .await?
                {
                    StoreResponse::Bool(false) => pending.push(*field),
                    _ => {}
                }
            }
            if pending.is_empty() {
                return Ok(());
            }

            // Load prerequisites first.
            if include_prerequisites {
                for field in pending.clone() {
                    let strategy = S::load_strategy(any_ref, field)?;
                    for prereq in strategy.prerequisites {
                        self.load_fields(any_ref, &[prereq], true).await?;
                    }
                }
            }

            // Re-check — prereqs may have caused side-effect loads.
            let mut still_pending = Vec::new();
            for field in &pending {
                match self
                    .store_send(StoreRequest::IsFieldLoaded {
                        any_ref: any_ref.clone(),
                        field: field.to_string(),
                    })
                    .await?
                {
                    StoreResponse::Bool(false) => still_pending.push(*field),
                    _ => {}
                }
            }
            if still_pending.is_empty() {
                return Ok(());
            }

            // Fetch current entity snapshot for the substrate load call.
            let current = match self
                .store_send(StoreRequest::GetEntity {
                    any_ref: any_ref.clone(),
                })
                .await?
            {
                StoreResponse::MaybeEntity(Some(e)) => e,
                _ => {
                    return Err(ActivityError::non_existent_data(
                        "store.load",
                        PrimitiveError::entity_not_found("entity not found", any_ref.id()),
                    ))
                }
            };

            let loaded_json = self.substrate.load(&current, &still_pending).await?;
            let loaded = TrackedEntity::from_json_value(any_ref, loaded_json).map_err(|e| {
                ActivityError::unpersistable_definition(
                    "store.load",
                    PrimitiveError::partial_payload_deserialization(
                        "partial payload deserialization failed",
                        any_ref.id().to_string(),
                        e.to_string(),
                    ),
                )
            })?;

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
            self.store_send(StoreRequest::InitializeField {
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
                        .store_send(StoreRequest::ContainsRef { any_ref: r.clone() })
                        .await?
                    {
                        StoreResponse::Bool(false) => not_in_store.push(r),
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
                            self.store_send(StoreRequest::InsertStubs { refs: stubs })
                                .await?;
                        }
                    }
                }
            }

            Ok(())
        })
    }

    // -----------------------------------------------------------------------
    // Store communication helper
    // -----------------------------------------------------------------------

    async fn store_send(&self, request: StoreRequest) -> Result<StoreResponse, ActivityError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let mut tx = self.store_tx.clone();
        tx.send(StoreMessage {
            request,
            reply: reply_tx,
        })
        .await
        .map_err(|_| {
            ActivityError::store_unavailable(
                "store_server",
                PrimitiveError::store_unavailable("store unavailable"),
            )
        })?;
        reply_rx.await.map_err(|_| {
            ActivityError::store_unavailable(
                "store_server",
                PrimitiveError::store_unavailable("store reply dropped"),
            )
        })
    }

    async fn store_get_entity(
        &self,
        any_ref: &AnyEntityRef,
    ) -> Result<Option<TrackedEntity>, ActivityError> {
        match self
            .store_send(StoreRequest::GetEntity {
                any_ref: any_ref.clone(),
            })
            .await?
        {
            StoreResponse::MaybeEntity(e) => Ok(e),
            _ => unreachable!(),
        }
    }
}

impl<S> Dispatcher for StoreServer<S>
where
    S: SchemaBackedSubstrate,
{
    fn dispatch<'a>(&'a self, request: WorkspaceRequest) -> BoxFuture<'a, WorkspaceResponse> {
        Box::pin(self.handle(request))
    }
}

// ---------------------------------------------------------------------------
// Active store server — workspace's single entry point
// ---------------------------------------------------------------------------

static GLOBAL_STORE_SERVER: OnceLock<Arc<dyn Dispatcher>> = OnceLock::new();

thread_local! {
    static OVERRIDE_STORE_SERVER: RefCell<Option<Arc<dyn Dispatcher>>> = const { RefCell::new(None) };
}

/// The workspace layer's single entry point to whichever `StoreServer`
/// is currently installed. Prefers the thread-local override (set by
/// [`crate::with`]) over the process-wide registration.
pub(crate) fn active_store_server() -> Arc<dyn Dispatcher> {
    OVERRIDE_STORE_SERVER
        .with(|o| o.borrow().clone())
        .unwrap_or_else(|| {
            GLOBAL_STORE_SERVER
                .get()
                .expect("StoreServer not initialized")
                .clone()
        })
}

/// Install the process-wide `StoreServer`. Panics if called twice.
pub(crate) fn install_global_store_server(store_server: Arc<dyn Dispatcher>) {
    GLOBAL_STORE_SERVER
        .set(store_server)
        .ok()
        .expect("StoreServer already initialized");
}

/// Guard returned by [`install_override_store_server`]; restores the
/// previous thread-local override on drop.
pub(crate) struct OverrideGuard {
    previous: Option<Arc<dyn Dispatcher>>,
}

impl Drop for OverrideGuard {
    fn drop(&mut self) {
        OVERRIDE_STORE_SERVER.with(|s| *s.borrow_mut() = self.previous.take());
    }
}

/// Install a thread-local `StoreServer` override; the returned guard
/// restores the previous value on drop. Used by [`crate::with`] to
/// isolate test scopes from the process-wide registration.
pub(crate) fn install_override_store_server(store_server: Arc<dyn Dispatcher>) -> OverrideGuard {
    let previous = OVERRIDE_STORE_SERVER.with(|s| s.borrow_mut().replace(store_server));
    OverrideGuard { previous }
}

// ---------------------------------------------------------------------------
// PrimitiveError → ActivityError mapping for store data operations
// ---------------------------------------------------------------------------

/// Classify a `Store`-emitted [`PrimitiveError`] into the appropriate
/// [`ActivityError`] kind for the orchestrating data operation.
fn map_store_primitive(e: PrimitiveError, component: &'static str) -> ActivityError {
    match &e {
        PrimitiveError::EntityNotFound { .. } => ActivityError::non_existent_data(component, e),
        PrimitiveError::PendingCheckouts { .. } => ActivityError::workspace_not_clean(component, e),
        _ => ActivityError::checkout_lifecycle_violation(component, e),
    }
}
