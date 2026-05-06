//! [`StoreServer`] — stateless orchestrator for the store layer.
//!
//! Holds an `Arc<S>` substrate handle and a [`StoreDispatcher`] into the
//! [`Store`] actor. Caller-facing operations from
//! [`workspace`](crate::workspace) arrive as [`WorkspaceRequest`] values
//! and are handled directly via `&self` methods — no actor loop, no
//! channel between workspace and the server. Each caller's task drives
//! its own orchestration sequence (validation, substrate calls, store
//! round-trips) for that request.
//!
//! Constructed through [`StoreServer::start`], which returns the
//! workspace-facing [`Dispatcher`] handle. Integrators wire it directly
//! into a [`Workspace`](crate::workspace::Workspace).

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Weak},
};

use futures::future::BoxFuture;

use crate::{
    entity::{AnyEntityRef, TrackedEntity},
    error::{primitive::PrimitiveError, ActivityError},
    store::{
        lib::{
            store_request::{StoreRequest, StoreResponse},
            workspace_request::{WorkspaceRequest, WorkspaceResponse},
        },
        store::StoreDispatcher,
    },
    substrate::SchemaBackedSubstrate,
    validation::ValidationKind,
};

// ---------------------------------------------------------------------------
// Dispatcher trait — type-erased entry surface used by the workspace layer
// ---------------------------------------------------------------------------

/// Type-erased dispatch surface. The workspace layer calls into this
/// trait so it does not need to be generic over the substrate.
pub trait Dispatcher: Send + Sync {
    fn dispatch<'a>(&'a self, request: WorkspaceRequest) -> BoxFuture<'a, WorkspaceResponse>;
}

// ---------------------------------------------------------------------------
// StoreServer
// ---------------------------------------------------------------------------

/// Stateless orchestrator for the store layer.
///
/// Holds a [`StoreDispatcher`] into the [`Store`] actor and a shared
/// substrate handle, plus a weak self-reference so per-request
/// workspaces can dispatch through the same outward surface callers
/// hold. Multiple instances may dispatch into the same store.
pub struct StoreServer<S> {
    store_dispatcher: Arc<dyn StoreDispatcher>,
    substrate: Arc<S>,
    self_dispatcher: Weak<dyn Dispatcher>,
}

impl<S> StoreServer<S>
where
    S: SchemaBackedSubstrate,
{
    /// Wire a [`StoreServer`] over `substrate` and a `StoreDispatcher`
    /// into the [`Store`] actor, and return its workspace-facing
    /// [`Dispatcher`] handle.
    ///
    /// Uses [`Arc::new_cyclic`] so the server can hold a `Weak` back-
    /// reference to its own outward dispatcher; the strong reference
    /// the caller holds is what keeps the cycle balanced. The strong
    /// edge is dispatcher → server; the weak edge is server →
    /// dispatcher.
    pub fn start(substrate: S, store_dispatcher: Arc<dyn StoreDispatcher>) -> Arc<dyn Dispatcher> {
        let substrate = Arc::new(substrate);
        Arc::new_cyclic(|weak: &Weak<StoreServer<S>>| StoreServer {
            store_dispatcher,
            substrate,
            self_dispatcher: weak.clone() as Weak<dyn Dispatcher>,
        })
    }

    /// Construct a per-request [`Workspace`](crate::workspace::Workspace)
    /// over the server's own dispatcher. Used by validation invocation
    /// sites that need workspace-bound viewer access to in-store
    /// entities.
    fn per_request_workspace(&self) -> crate::workspace::Workspace {
        let dispatcher = self
            .self_dispatcher
            .upgrade()
            .expect("StoreServer self-dispatcher dropped while server is in use");
        crate::workspace::Workspace::new(dispatcher)
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
            WorkspaceRequest::Insert { json } => match self.insert(json).await {
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

    async fn insert(&self, json: serde_json::Value) -> Result<(), ActivityError> {
        let entity = self
            .json_to_verified_tracked(
                json,
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

    /// Pure conversion: JSON → [`TrackedEntity`]. Reads the entity-ref
    /// payload to resolve the kind, then deserializes per-kind into
    /// the matching tracked variant. Wraps deserialization errors as
    /// `unpersistable_definition`.
    fn json_to_tracked_state(
        &self,
        json: serde_json::Value,
    ) -> Result<TrackedEntity, ActivityError> {
        let any_ref_value = json.get("entity_ref").cloned().ok_or_else(|| {
            ActivityError::unpersistable_definition(
                "store.json_pipeline",
                PrimitiveError::partial_payload_deserialization(
                    "missing entity_ref in payload",
                    "<unknown>".to_string(),
                    "no `entity_ref` key".to_string(),
                ),
            )
        })?;
        let any_ref = AnyEntityRef::from_json_value(any_ref_value)
            .map_err(|e| ActivityError::unpersistable_definition("store.json_pipeline", e))?;
        TrackedEntity::from_json_value(&any_ref, json).map_err(|e| {
            ActivityError::unpersistable_definition(
                "store.json_pipeline",
                PrimitiveError::partial_payload_deserialization(
                    "tracked entity deserialization failed",
                    any_ref.id().to_string(),
                    e.to_string(),
                ),
            )
        })
    }

    /// Full JSON pipeline: deserialize, import into a per-request
    /// workspace, validate. Returns the verified `TrackedEntity` for
    /// the caller to hand to the store actor.
    async fn json_to_verified_tracked(
        &self,
        json: serde_json::Value,
        fields: &[&str],
        kinds: &[ValidationKind],
    ) -> Result<TrackedEntity, ActivityError> {
        let tracked = self.json_to_tracked_state(json)?;
        let workspace = self.per_request_workspace();
        workspace
            .validate_tracked(tracked.clone(), fields, kinds)
            .await?;
        Ok(tracked)
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

        // Commit only re-runs CrossEntity. Structural and Semantic are
        // already covered: insert ran them whole-entity at creation,
        // and per-field setters ran them on each mutation. The scope
        // is the only thing that differs — whole entity for a newly
        // added one, dirty fields otherwise.
        if is_added || entity.has_dirty_fields() {
            let dirty = if is_added {
                Vec::new()
            } else {
                entity.dirty_fields()
            };
            self.per_request_workspace()
                .validate_tracked(entity.clone(), &dirty, &[ValidationKind::CrossEntity])
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

            // Fetch the loaded fields as JSON, then run the same JSON
            // pipeline insert uses: deserialize, import into a
            // per-request workspace, validate.
            let loaded_json = self.substrate.load(&current, &still_pending).await?;
            let loaded = self
                .json_to_verified_tracked(
                    loaded_json,
                    &still_pending,
                    &[
                        ValidationKind::Structural,
                        ValidationKind::Semantic,
                        ValidationKind::CrossEntity,
                    ],
                )
                .await
                .map_err(|e| {
                    ActivityError::unpersistable_definition("store.load", e.into_cause())
                })?;

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
        self.store_dispatcher
            .dispatch(request)
            .await
            .map_err(|e| ActivityError::store_unavailable("store_server", e))
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
