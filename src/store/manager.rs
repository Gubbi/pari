//! [`StoreManager`] — state-custodian half of the store layer.
//!
//! Owns the five collections that make up the store's in-memory state
//! and serves [`StoreManagerRequest`]s one at a time. No substrate, no
//! validation, no `ActivityError` — every failure is a
//! [`PrimitiveError`] for the orchestrator above to classify.

use std::collections::{HashMap, HashSet};

use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};

use crate::{
    entity::{AnyEntityRef, TrackedEntity},
    error::primitive::PrimitiveError,
    store::lib::change::EntityChange,
};

// ---------------------------------------------------------------------------
// Message types
// ---------------------------------------------------------------------------

/// Internal request surface between [`EntityServer`](super::entity_server::EntityServer)
/// and the manager. Each variant corresponds to one state mutation or
/// query; the orchestrator composes these into the caller-facing
/// [`StoreRequest`](super::StoreRequest) operations.
pub(crate) enum StoreManagerRequest {
    // Reads
    GetEntity {
        any_ref: AnyEntityRef,
    },
    ContainsRef {
        any_ref: AnyEntityRef,
    },
    IsFieldLoaded {
        any_ref: AnyEntityRef,
        field: String,
    },
    PendingCheckoutCount,
    // Writes
    InsertStubs {
        refs: Vec<AnyEntityRef>,
    },
    InsertEntity {
        entity: TrackedEntity,
    },
    Checkout {
        any_ref: AnyEntityRef,
    },
    CommitCheckout {
        entity: TrackedEntity,
    },
    UndoCheckout {
        any_ref: AnyEntityRef,
    },
    UndoCommit {
        any_ref: AnyEntityRef,
    },
    RemoveEntity {
        any_ref: AnyEntityRef,
    },
    UnloadEntity {
        any_ref: AnyEntityRef,
    },
    InitializeField {
        any_ref: AnyEntityRef,
        loaded: TrackedEntity,
    },
    // Persist lifecycle
    TakePersistSnapshot,
    CommitPersist,
    // State queries
    IsAdded {
        any_ref: AnyEntityRef,
    },
}

pub(crate) enum StoreManagerResponse {
    Entity(TrackedEntity),
    MaybeEntity(Option<TrackedEntity>),
    Changes(Vec<EntityChange>),
    Bool(bool),
    Count(usize),
    Unit,
    Err(PrimitiveError),
}

pub(crate) struct StoreManagerMessage {
    pub(crate) request: StoreManagerRequest,
    pub(crate) reply: oneshot::Sender<StoreManagerResponse>,
}

// ---------------------------------------------------------------------------
// Actor
// ---------------------------------------------------------------------------

/// Sole custodian of the store's in-memory state.
///
/// `entities` holds every ref the store knows about — loaded, stubbed,
/// or locally added. The three change-tracking sets (`added`,
/// `modified`, `removed`) drive the persist snapshot. `checked_out`
/// enforces the single-checkout rule and gates `persist`,
/// `undo_commit`, `remove`, and `unload`.
pub(crate) struct StoreManager {
    entities: HashMap<AnyEntityRef, TrackedEntity>,
    added: HashSet<AnyEntityRef>,
    modified: HashSet<AnyEntityRef>,
    removed: HashSet<AnyEntityRef>,
    checked_out: HashSet<AnyEntityRef>,
}

impl StoreManager {
    pub(crate) fn new() -> Self {
        Self {
            entities: HashMap::new(),
            added: HashSet::new(),
            modified: HashSet::new(),
            removed: HashSet::new(),
            checked_out: HashSet::new(),
        }
    }

    /// Actor loop — processes messages strictly sequentially. No
    /// interleaving, no locking.
    pub(crate) async fn run(mut self, mut rx: mpsc::Receiver<StoreManagerMessage>) {
        while let Some(msg) = rx.next().await {
            let response = self.handle(msg.request);
            let _ = msg.reply.send(response);
        }
    }

    fn handle(&mut self, request: StoreManagerRequest) -> StoreManagerResponse {
        match request {
            StoreManagerRequest::GetEntity { any_ref } => {
                StoreManagerResponse::MaybeEntity(self.entities.get(&any_ref).cloned())
            }
            StoreManagerRequest::ContainsRef { any_ref } => {
                StoreManagerResponse::Bool(self.entities.contains_key(&any_ref))
            }
            StoreManagerRequest::IsFieldLoaded { any_ref, field } => {
                let loaded = self
                    .entities
                    .get(&any_ref)
                    .map(|e| e.is_field_loaded(&field))
                    .unwrap_or(false);
                StoreManagerResponse::Bool(loaded)
            }
            StoreManagerRequest::PendingCheckoutCount => {
                StoreManagerResponse::Count(self.checked_out.len())
            }
            StoreManagerRequest::InsertStubs { refs } => {
                for any_ref in refs {
                    if !self.entities.contains_key(&any_ref) {
                        self.entities
                            .insert(any_ref.clone(), TrackedEntity::make_stub(&any_ref));
                    }
                }
                StoreManagerResponse::Unit
            }
            StoreManagerRequest::InsertEntity { entity } => {
                let any_ref = entity.any_ref();
                self.entities.insert(any_ref.clone(), entity);
                if self.removed.remove(&any_ref) {
                    self.modified.insert(any_ref);
                } else {
                    self.added.insert(any_ref);
                }
                StoreManagerResponse::Unit
            }
            StoreManagerRequest::Checkout { any_ref } => match self.checkout(&any_ref) {
                Ok(entity) => StoreManagerResponse::Entity(entity),
                Err(e) => StoreManagerResponse::Err(e),
            },
            StoreManagerRequest::CommitCheckout { entity } => {
                self.commit_checkout(entity);
                StoreManagerResponse::Unit
            }
            StoreManagerRequest::UndoCheckout { any_ref } => match self.undo_checkout(&any_ref) {
                Ok(()) => StoreManagerResponse::Unit,
                Err(e) => StoreManagerResponse::Err(e),
            },
            StoreManagerRequest::UndoCommit { any_ref } => match self.undo_commit(&any_ref) {
                Ok(()) => StoreManagerResponse::Unit,
                Err(e) => StoreManagerResponse::Err(e),
            },
            StoreManagerRequest::RemoveEntity { any_ref } => match self.remove_entity(&any_ref) {
                Ok(entity) => StoreManagerResponse::Entity(entity),
                Err(e) => StoreManagerResponse::Err(e),
            },
            StoreManagerRequest::UnloadEntity { any_ref } => match self.unload_entity(&any_ref) {
                Ok(()) => StoreManagerResponse::Unit,
                Err(e) => StoreManagerResponse::Err(e),
            },
            StoreManagerRequest::InitializeField { any_ref, loaded } => {
                match self.initialize_field(&any_ref, loaded) {
                    Ok(()) => StoreManagerResponse::Unit,
                    Err(e) => StoreManagerResponse::Err(e),
                }
            }
            StoreManagerRequest::TakePersistSnapshot => {
                StoreManagerResponse::Changes(self.take_persist_snapshot())
            }
            StoreManagerRequest::CommitPersist => {
                self.commit_persist();
                StoreManagerResponse::Unit
            }
            StoreManagerRequest::IsAdded { any_ref } => {
                StoreManagerResponse::Bool(self.added.contains(&any_ref))
            }
        }
    }

    // -----------------------------------------------------------------------
    // Operation implementations
    // -----------------------------------------------------------------------

    fn checkout(&mut self, any_ref: &AnyEntityRef) -> Result<TrackedEntity, PrimitiveError> {
        if self.checked_out.contains(any_ref) {
            return Err(PrimitiveError::already_checked_out(
                "entity already checked out",
                any_ref.id(),
            ));
        }
        match self.entities.get(any_ref) {
            None => Err(PrimitiveError::entity_not_found(
                "entity not found",
                any_ref.id(),
            )),
            Some(entity) => {
                self.checked_out.insert(any_ref.clone());
                Ok(entity.clone())
            }
        }
    }

    /// Merge a committed entity's dirty fields into the canonical store
    /// copy and update the change-tracking sets. For `added` entities
    /// with dirty fields, resets dirty after merge — added entities are
    /// always written in full on persist, so per-field dirty bits carry
    /// no additional information.
    fn commit_checkout(&mut self, entity: TrackedEntity) {
        let any_ref = entity.any_ref();
        self.checked_out.remove(&any_ref);
        if let Some(existing) = self.entities.get_mut(&any_ref) {
            entity.merge_dirty_into(existing);
            if entity.has_dirty_fields() {
                if self.added.contains(&any_ref) {
                    existing.reset_dirty();
                } else {
                    self.modified.insert(any_ref);
                }
            }
        }
    }

    fn undo_checkout(&mut self, any_ref: &AnyEntityRef) -> Result<(), PrimitiveError> {
        if self.checked_out.remove(any_ref) {
            Ok(())
        } else {
            Err(PrimitiveError::entity_not_checked_out(
                "entity was not checked out",
                any_ref.id(),
            ))
        }
    }

    fn undo_commit(&mut self, any_ref: &AnyEntityRef) -> Result<(), PrimitiveError> {
        if self.checked_out.contains(any_ref) {
            return Err(PrimitiveError::entity_still_checked_out(
                "cannot undo commit while entity is checked out",
                any_ref.id(),
            ));
        }
        if self.added.contains(any_ref) {
            self.entities.remove(any_ref);
            self.added.remove(any_ref);
            Ok(())
        } else if self.modified.contains(any_ref) {
            self.entities
                .insert(any_ref.clone(), TrackedEntity::make_stub(any_ref));
            self.modified.remove(any_ref);
            Ok(())
        } else {
            Err(PrimitiveError::no_uncommitted_changes(
                "no uncommitted changes to undo",
                any_ref.id(),
            ))
        }
    }

    fn remove_entity(&mut self, any_ref: &AnyEntityRef) -> Result<TrackedEntity, PrimitiveError> {
        if self.checked_out.contains(any_ref) {
            return Err(PrimitiveError::entity_still_checked_out(
                "cannot remove a checked-out entity",
                any_ref.id(),
            ));
        }
        match self.entities.remove(any_ref) {
            None => Err(PrimitiveError::entity_not_found(
                "entity not found",
                any_ref.id(),
            )),
            Some(entity) => {
                if !self.added.remove(any_ref) {
                    self.removed.insert(any_ref.clone());
                }
                self.modified.remove(any_ref);
                Ok(entity)
            }
        }
    }

    fn unload_entity(&mut self, any_ref: &AnyEntityRef) -> Result<(), PrimitiveError> {
        if !self.entities.contains_key(any_ref) {
            return Err(PrimitiveError::entity_not_found(
                "entity not found",
                any_ref.id(),
            ));
        }
        if self.checked_out.contains(any_ref) {
            return Err(PrimitiveError::entity_still_checked_out(
                "cannot unload a checked-out entity",
                any_ref.id(),
            ));
        }
        if self.added.contains(any_ref) || self.modified.contains(any_ref) {
            return Err(PrimitiveError::entity_has_unsaved_changes(
                "entity has unsaved changes",
                any_ref.id(),
            ));
        }
        self.entities
            .insert(any_ref.clone(), TrackedEntity::make_stub(any_ref));
        Ok(())
    }

    fn initialize_field(
        &mut self,
        any_ref: &AnyEntityRef,
        loaded: TrackedEntity,
    ) -> Result<(), PrimitiveError> {
        match self.entities.get_mut(any_ref) {
            None => Err(PrimitiveError::entity_not_found(
                "entity not found",
                any_ref.id(),
            )),
            Some(existing) => {
                loaded.initialize_into(existing);
                Ok(())
            }
        }
    }

    /// Produce the list of changes to hand to the substrate. Does not
    /// mutate state — dirty-flag resets happen in
    /// [`Self::commit_persist`] only after the substrate succeeds.
    fn take_persist_snapshot(&self) -> Vec<EntityChange> {
        self.added
            .iter()
            .filter_map(|r| self.entities.get(r))
            .map(|e| EntityChange::Added(e.clone()))
            .chain(self.modified.iter().filter_map(|r| {
                self.entities
                    .get(r)
                    .map(|e| EntityChange::Modified(e.clone(), e.dirty_fields()))
            }))
            .chain(
                self.removed
                    .iter()
                    .map(|r| EntityChange::Removed(r.clone())),
            )
            .collect()
    }

    /// Clear change-tracking state after a successful substrate
    /// persist: reset dirty flags on modified entities and empty all
    /// three change sets.
    fn commit_persist(&mut self) {
        let modified_refs: Vec<AnyEntityRef> = self.modified.iter().cloned().collect();
        for any_ref in modified_refs {
            if let Some(entity) = self.entities.get_mut(&any_ref) {
                entity.reset_dirty();
            }
        }
        self.added.clear();
        self.modified.clear();
        self.removed.clear();
    }
}
