//! [`Workspace`] — bounded session of entity work over a store dispatcher.
//!
//! Anyone can construct a workspace over an `Arc<dyn Dispatcher>` to a
//! [`StoreServer`](crate::store::StoreServer). Multiple workspaces over
//! the same server coexist; the dispatcher is shared by `Arc` clone.
//!
//! Public methods take typed [`EntityRef<T, T::Parent>`](crate::entity::EntityRef)
//! and return typed [`XViewer`] / [`<T as Entity>::Delegate`] handles.
//! The type↔erased conversion at the workspace↔store boundary is
//! handled inside this module; downstream layers see only
//! `AnyEntityRef` and `TrackedEntity`.

use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};

use jsonschema::Validator as JsonValidator;
use schemars::JsonSchema;

use crate::{
    entities::{
        artifact_kind::ArtifactKind,
        hook::Hook,
        relay::Relay,
        role::Role,
        task::Task,
        team::Team,
        workflow::{EmbeddedWorkflow, ReusableWorkflow, Workflow},
    },
    entity::{AnyEntityRef, Entity, EntityKind, EntityRef, TrackedEntity},
    error::{primitive::PrimitiveError, ActivityError},
    store::{Dispatcher, WorkspaceRequest, WorkspaceResponse},
    workspace::{editor::XEditor, validator::Validator, viewer::XViewer},
};

/// Per-kind cache of compiled JSON Schema validators for the full
/// entity shape. Used by `import_json` at the JSON ingest boundary so
/// callers can validate raw JSON before deserializing into a tracked
/// entity. Populated once at first access.
static FULL_VALIDATORS: LazyLock<HashMap<EntityKind, Arc<JsonValidator>>> = LazyLock::new(|| {
    fn entry<T: JsonSchema>() -> Arc<JsonValidator> {
        let schema = serde_json::to_value(schemars::schema_for!(T))
            .expect("schemars schema is always serializable");
        Arc::new(
            jsonschema::validator_for(&schema)
                .expect("schemars-derived schema compiles into a validator"),
        )
    }
    let mut m = HashMap::new();
    m.insert(EntityKind::Role, entry::<Role>());
    m.insert(EntityKind::Hook, entry::<Hook>());
    m.insert(EntityKind::Team, entry::<Team>());
    m.insert(EntityKind::ArtifactKind, entry::<ArtifactKind>());
    m.insert(EntityKind::Workflow, entry::<Workflow>());
    m.insert(EntityKind::ReusableWorkflow, entry::<ReusableWorkflow>());
    m.insert(EntityKind::EmbeddedWorkflow, entry::<EmbeddedWorkflow>());
    m.insert(EntityKind::Task, entry::<Task>());
    m.insert(EntityKind::Relay, entry::<Relay>());
    m
});

/// Caller-facing async API over a [`Dispatcher`].
pub struct Workspace {
    dispatcher: Arc<dyn Dispatcher>,
    validator: Validator,
}

impl Workspace {
    /// Construct a workspace over `dispatcher`.
    ///
    /// Cheap — one `Arc` clone plus a free `Validator` stamp.
    /// Per-request construction inside server-side validation paths is
    /// fine.
    pub fn new(dispatcher: Arc<dyn Dispatcher>) -> Self {
        Self {
            dispatcher,
            validator: Validator::new(),
        }
    }

    /// The workspace's validator.
    pub fn validator(&self) -> &Validator {
        &self.validator
    }

    /// The dispatcher this workspace routes through. Generated viewer
    /// accessors and editor setters reach the store through this.
    ///
    /// `#[doc(hidden)]`: this is a macro-internal touchpoint, not part
    /// of the curated public surface.
    #[doc(hidden)]
    pub fn __dispatcher(&self) -> &Arc<dyn Dispatcher> {
        &self.dispatcher
    }

    /// Read-only handle to an entity. Stub-fetches from substrate on
    /// miss so a fresh viewer is observable to subsequent calls.
    pub async fn resolve<T: Entity>(
        &self,
        entity_ref: EntityRef<T, T::Parent>,
    ) -> Result<XViewer<'_, T>, ActivityError> {
        let any_ref = entity_ref.to_any_ref();
        match self
            .dispatcher
            .dispatch(WorkspaceRequest::Resolve { any_ref })
            .await
        {
            WorkspaceResponse::Entity(entity) => {
                let inner = T::take(entity)
                    .unwrap_or_else(|_| unreachable!("store returned mismatched variant for T"));
                Ok(XViewer::new(inner, self))
            }
            WorkspaceResponse::Err(e) => Err(e),
            _ => unreachable!("Resolve must reply with Entity or Err"),
        }
    }

    /// Existence check; same machinery as [`Self::resolve`] but does
    /// not surface a not-found as an error.
    pub async fn has_ref<T: Entity>(
        &self,
        entity_ref: EntityRef<T, T::Parent>,
    ) -> Result<bool, ActivityError> {
        let any_ref = entity_ref.to_any_ref();
        match self
            .dispatcher
            .dispatch(WorkspaceRequest::HasRef { any_ref })
            .await
        {
            WorkspaceResponse::Bool(b) => Ok(b),
            WorkspaceResponse::Err(e) => Err(e),
            _ => unreachable!("HasRef must reply with Bool or Err"),
        }
    }

    /// Add a new entity to the store.
    ///
    /// Serializes the plain entity to JSON at the workspace boundary
    /// and ships JSON across the wire. The store completes the
    /// JSON → tracked → validate pipeline before the entity reaches
    /// the canonical in-memory copy.
    pub async fn insert<T>(&self, plain: T) -> Result<(), ActivityError>
    where
        T: Entity + serde::Serialize,
    {
        let json = serde_json::to_value(&plain).map_err(|e| {
            ActivityError::unpersistable_definition(
                "workspace.insert",
                crate::error::primitive::PrimitiveError::entity_projection(
                    "entity serialization failed",
                    "<insert>".to_string(),
                    e.to_string(),
                ),
            )
        })?;
        match self
            .dispatcher
            .dispatch(WorkspaceRequest::Insert { json })
            .await
        {
            WorkspaceResponse::Unit => Ok(()),
            WorkspaceResponse::Err(e) => Err(e),
            _ => unreachable!("Insert must reply with Unit or Err"),
        }
    }

    /// Acquire single-writer mutation rights to an entity. The returned
    /// [`XEditor`] borrows this workspace; setters and the
    /// `commit(self)` / `undo_checkout(self)` lifecycle dispatch
    /// through the same surface every other workspace operation uses.
    pub async fn checkout<T: Entity>(
        &self,
        entity_ref: EntityRef<T, T::Parent>,
    ) -> Result<XEditor<'_, T>, ActivityError> {
        let any_ref = entity_ref.to_any_ref();
        let entity = match self
            .dispatcher
            .dispatch(WorkspaceRequest::Checkout { any_ref })
            .await
        {
            WorkspaceResponse::Entity(e) => e,
            WorkspaceResponse::Err(e) => return Err(e),
            _ => unreachable!("Checkout must reply with Entity or Err"),
        };
        let tracked = T::take(entity)
            .unwrap_or_else(|_| unreachable!("store returned mismatched variant for T"));
        Ok(XEditor::new(XViewer::new(tracked, self)))
    }

    /// Evict an entity from the store. Returns a viewer over the
    /// just-removed state; the underlying entity is no longer in the
    /// store, so lazy-loading any unloaded fields will error.
    pub async fn remove<T: Entity>(
        &self,
        entity_ref: EntityRef<T, T::Parent>,
    ) -> Result<XViewer<'_, T>, ActivityError> {
        let any_ref = entity_ref.to_any_ref();
        match self
            .dispatcher
            .dispatch(WorkspaceRequest::Remove { any_ref })
            .await
        {
            WorkspaceResponse::Entity(entity) => {
                let inner = T::take(entity)
                    .unwrap_or_else(|_| unreachable!("store returned mismatched variant for T"));
                Ok(XViewer::new(inner, self))
            }
            WorkspaceResponse::Err(e) => Err(e),
            _ => unreachable!("Remove must reply with Entity or Err"),
        }
    }

    /// Flush pending changes to the substrate. Fails if any entity is
    /// currently checked out.
    pub async fn persist(&self) -> Result<(), ActivityError> {
        match self.dispatcher.dispatch(WorkspaceRequest::Persist).await {
            WorkspaceResponse::Unit => Ok(()),
            WorkspaceResponse::Err(e) => Err(e),
            _ => unreachable!("Persist must reply with Unit or Err"),
        }
    }

    /// Roll an entity back to its last persisted state and drop it
    /// from the in-memory view.
    ///
    /// For a freshly added entity the entry is removed entirely; for a
    /// modified-but-not-yet-persisted entity it resets to a stub and
    /// loaded fields are dropped. After this call the entity is back
    /// to substrate-truth in both cases — no local mutations remain
    /// and any subsequent access re-fetches lazily.
    pub async fn revert_and_forget<T: Entity>(
        &self,
        entity_ref: EntityRef<T, T::Parent>,
    ) -> Result<(), ActivityError> {
        let any_ref = entity_ref.to_any_ref();
        match self
            .dispatcher
            .dispatch(WorkspaceRequest::Revert { any_ref })
            .await
        {
            WorkspaceResponse::Unit => Ok(()),
            WorkspaceResponse::Err(e) => Err(e),
            _ => unreachable!("Revert must reply with Unit or Err"),
        }
    }

    /// Drop a clean entity's loaded fields, leaving a stub for
    /// re-fetch on next access.
    pub async fn forget<T: Entity>(
        &self,
        entity_ref: EntityRef<T, T::Parent>,
    ) -> Result<(), ActivityError> {
        let any_ref = entity_ref.to_any_ref();
        match self
            .dispatcher
            .dispatch(WorkspaceRequest::Forget { any_ref })
            .await
        {
            WorkspaceResponse::Unit => Ok(()),
            WorkspaceResponse::Err(e) => Err(e),
            _ => unreachable!("Forget must reply with Unit or Err"),
        }
    }

    /// Wrap a transient `Tracked<T>` as a viewer bound to this
    /// workspace. Useful for validating an entity outside the store.
    pub fn import<T: Entity>(&self, tracked: T::Tracked) -> XViewer<'_, T> {
        XViewer::new(tracked, self)
    }

    /// Schema-validate raw JSON against `T`'s entity schema, deserialize
    /// into the tracked form, and wrap as a viewer bound to this
    /// workspace. The intended ingest path for callers that start with
    /// JSON (CLI input, API request bodies, etc.) — schema failures
    /// surface as `ValidationFailed` before a malformed payload reaches
    /// the store.
    pub fn import_json<T>(&self, value: serde_json::Value) -> Result<XViewer<'_, T>, ActivityError>
    where
        T: Entity + JsonSchema,
        T::Tracked: serde::de::DeserializeOwned,
    {
        let validator = FULL_VALIDATORS
            .get(&T::KIND)
            .expect("validator registered for every entity kind");
        validator.validate(&value).map_err(|err| {
            ActivityError::validation_failed(
                "workspace.import_json",
                PrimitiveError::partial_payload_deserialization(
                    "JSON failed entity schema validation",
                    "<unknown>".to_string(),
                    err.to_string(),
                ),
            )
        })?;
        let tracked: T::Tracked = serde_json::from_value(value).map_err(|err| {
            ActivityError::validation_failed(
                "workspace.import_json",
                PrimitiveError::partial_payload_deserialization(
                    "JSON could not be deserialized into the tracked entity",
                    "<unknown>".to_string(),
                    err.to_string(),
                ),
            )
        })?;
        Ok(XViewer::new(tracked, self))
    }

    // -----------------------------------------------------------------------
    // Type-erased helpers used by validation rule bodies and other
    // cross-cutting paths that hold an `AnyEntityRef`. Not part of the
    // typed public surface.
    // -----------------------------------------------------------------------

    pub(crate) async fn resolve_any(
        &self,
        any_ref: AnyEntityRef,
    ) -> Result<TrackedEntity, ActivityError> {
        match self
            .dispatcher
            .dispatch(WorkspaceRequest::Resolve { any_ref })
            .await
        {
            WorkspaceResponse::Entity(entity) => Ok(entity),
            WorkspaceResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }

    pub(crate) async fn has_any(&self, any_ref: AnyEntityRef) -> Result<bool, ActivityError> {
        match self
            .dispatcher
            .dispatch(WorkspaceRequest::HasRef { any_ref })
            .await
        {
            WorkspaceResponse::Bool(b) => Ok(b),
            WorkspaceResponse::Err(e) => Err(e),
            _ => unreachable!(),
        }
    }
}
