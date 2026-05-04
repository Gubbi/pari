//! [`Store`] — state-custodian half of the store layer.
//!
//! Owns the five collections that make up the in-memory state and serves
//! [`StoreRequest`]s one at a time. No substrate, no validation, no
//! `ActivityError` — every failure is a [`PrimitiveError`] for the
//! orchestrator above to classify.

use std::collections::{HashMap, HashSet};

use futures::{channel::mpsc, StreamExt};

use crate::{
    entity::{AnyEntityRef, TrackedEntity},
    error::primitive::PrimitiveError,
    store::lib::{
        change::EntityChange,
        store_request::{StoreMessage, StoreRequest, StoreResponse},
    },
};

/// Sole custodian of the store's in-memory state.
///
/// `entities` holds every ref the store knows about — loaded, stubbed,
/// or locally added. The three change-tracking sets (`added`,
/// `modified`, `removed`) drive the persist snapshot. `checked_out`
/// enforces the single-checkout rule and gates `persist`, `revert`,
/// `remove`, and `forget`.
pub(crate) struct Store {
    entities: HashMap<AnyEntityRef, TrackedEntity>,
    added: HashSet<AnyEntityRef>,
    modified: HashSet<AnyEntityRef>,
    removed: HashSet<AnyEntityRef>,
    checked_out: HashSet<AnyEntityRef>,
}

impl Store {
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
    pub(crate) async fn run(mut self, mut rx: mpsc::Receiver<StoreMessage>) {
        while let Some(msg) = rx.next().await {
            let response = self.handle(msg.request);
            let _ = msg.reply.send(response);
        }
    }

    fn handle(&mut self, request: StoreRequest) -> StoreResponse {
        match request {
            StoreRequest::GetEntity { any_ref } => {
                StoreResponse::MaybeEntity(self.entities.get(&any_ref).cloned())
            }
            StoreRequest::ContainsRef { any_ref } => {
                StoreResponse::Bool(self.entities.contains_key(&any_ref))
            }
            StoreRequest::IsFieldLoaded { any_ref, field } => {
                let loaded = self
                    .entities
                    .get(&any_ref)
                    .map(|e| e.is_field_loaded(&field))
                    .unwrap_or(false);
                StoreResponse::Bool(loaded)
            }
            StoreRequest::PendingCheckoutCount => StoreResponse::Count(self.checked_out.len()),
            StoreRequest::InsertStubs { refs } => {
                let mut out = Vec::with_capacity(refs.len());
                for any_ref in refs {
                    let stub = self
                        .entities
                        .entry(any_ref.clone())
                        .or_insert_with(|| TrackedEntity::make_stub(&any_ref));
                    out.push(stub.clone());
                }
                StoreResponse::Entities(out)
            }
            StoreRequest::InsertEntity { entity } => {
                let any_ref = entity.any_ref();
                // A previously-loaded stub or a removed-then-readded entry
                // may already occupy the slot; both are legitimate
                // insert paths. Only an active occupant (added or
                // modified) is a duplicate.
                if self.added.contains(&any_ref) || self.modified.contains(&any_ref) {
                    StoreResponse::Err(PrimitiveError::entity_already_exists(
                        "entity already exists",
                        any_ref.id(),
                    ))
                } else {
                    self.entities.insert(any_ref.clone(), entity);
                    if self.removed.remove(&any_ref) {
                        self.modified.insert(any_ref);
                    } else {
                        self.added.insert(any_ref);
                    }
                    StoreResponse::Unit
                }
            }
            StoreRequest::Checkout { any_ref } => match self.checkout(&any_ref) {
                Ok(entity) => StoreResponse::Entity(entity),
                Err(e) => StoreResponse::Err(e),
            },
            StoreRequest::CommitCheckout { entity } => match self.commit_checkout(entity) {
                Ok(()) => StoreResponse::Unit,
                Err(e) => StoreResponse::Err(e),
            },
            StoreRequest::UndoCheckout { any_ref } => match self.undo_checkout(&any_ref) {
                Ok(()) => StoreResponse::Unit,
                Err(e) => StoreResponse::Err(e),
            },
            StoreRequest::Revert { any_ref } => match self.revert(&any_ref) {
                Ok(()) => StoreResponse::Unit,
                Err(e) => StoreResponse::Err(e),
            },
            StoreRequest::RemoveEntity { any_ref } => match self.remove_entity(&any_ref) {
                Ok(entity) => StoreResponse::Entity(entity),
                Err(e) => StoreResponse::Err(e),
            },
            StoreRequest::Forget { any_ref } => match self.forget(&any_ref) {
                Ok(()) => StoreResponse::Unit,
                Err(e) => StoreResponse::Err(e),
            },
            StoreRequest::InitializeField { any_ref, loaded } => {
                match self.initialize_field(&any_ref, loaded) {
                    Ok(()) => StoreResponse::Unit,
                    Err(e) => StoreResponse::Err(e),
                }
            }
            StoreRequest::TakePersistSnapshot => {
                StoreResponse::Changes(self.take_persist_snapshot())
            }
            StoreRequest::CommitPersist => {
                self.commit_persist();
                StoreResponse::Unit
            }
            StoreRequest::IsAdded { any_ref } => StoreResponse::Bool(self.added.contains(&any_ref)),
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
    fn commit_checkout(&mut self, entity: TrackedEntity) -> Result<(), PrimitiveError> {
        let any_ref = entity.any_ref();
        if !self.checked_out.remove(&any_ref) {
            return Err(PrimitiveError::entity_not_checked_out(
                "entity was not checked out",
                any_ref.id(),
            ));
        }
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
        Ok(())
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

    fn revert(&mut self, any_ref: &AnyEntityRef) -> Result<(), PrimitiveError> {
        if self.checked_out.contains(any_ref) {
            return Err(PrimitiveError::entity_still_checked_out(
                "cannot revert while entity is checked out",
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
                "no uncommitted changes to revert",
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

    fn forget(&mut self, any_ref: &AnyEntityRef) -> Result<(), PrimitiveError> {
        if !self.entities.contains_key(any_ref) {
            return Err(PrimitiveError::entity_not_found(
                "entity not found",
                any_ref.id(),
            ));
        }
        if self.checked_out.contains(any_ref) {
            return Err(PrimitiveError::entity_still_checked_out(
                "cannot forget a checked-out entity",
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
