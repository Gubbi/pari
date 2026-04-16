use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

use tokio::sync::mpsc;

use crate::entity::{AnyEntityRef, TrackedEntity};
use crate::error::BatchError;
use crate::store::message::{StoreCommand, StoreMessage, StoreRequest, StoreResponse};
use crate::store_error::StoreError;
use crate::substrate::schema_registry::SchemaBackedSubstrate;
use crate::validation::{run_validations_for_entity, ValidationKind};
use crate::workspace::error::{CheckoutError, CommitError, LoadError, PersistError, ResolveError, UndoError};

use super::EntityChange;

#[derive(Debug)]
enum StoreOpError {
    NotFound,
    CheckedOut,
    WrongState,
}

pub(super) struct Store<S: crate::substrate::Substrate> {
    entities: HashMap<AnyEntityRef, TrackedEntity>,
    added: HashSet<AnyEntityRef>,
    modified: HashSet<AnyEntityRef>,
    removed: HashSet<AnyEntityRef>,
    checked_out: HashSet<AnyEntityRef>,
    substrate: S,
}

impl<S> Store<S>
where
    S: SchemaBackedSubstrate,
{
    pub(super) fn new(substrate: S) -> Self {
        Self {
            entities: HashMap::new(),
            added: HashSet::new(),
            modified: HashSet::new(),
            removed: HashSet::new(),
            checked_out: HashSet::new(),
            substrate,
        }
    }

    pub(super) async fn run(mut self, mut rx: mpsc::Receiver<StoreMessage>) {
        while let Some(msg) = rx.recv().await {
            match msg {
                StoreMessage::Request { request, reply } => {
                    let result = self.handle(request).await;
                    let _ = reply.send(result);
                }
                StoreMessage::Command(cmd) => self.execute(cmd),
            }
        }
    }

    async fn handle(&mut self, request: StoreRequest) -> Result<StoreResponse, StoreError> {
        match request {
            StoreRequest::Resolve { any_ref } => match self.resolve(&any_ref).await {
                Ok(entity) => Ok(StoreResponse::Entity(entity)),
                Err(e) => Ok(StoreResponse::ResolveErr(e)),
            },
            StoreRequest::Insert { entity } => match self.insert(entity).await {
                Ok(()) => Ok(StoreResponse::Unit),
                Err(e) => Ok(StoreResponse::CommitErr(e)),
            },
            StoreRequest::Checkout { any_ref } => match self.checkout(&any_ref) {
                Ok(entity) => Ok(StoreResponse::Entity(entity)),
                Err(e) => Ok(StoreResponse::CheckoutErr(e)),
            },
            StoreRequest::Commit { entity } => match self.commit(entity).await {
                Ok(()) => Ok(StoreResponse::Unit),
                Err(e) => Ok(StoreResponse::CommitErr(e)),
            },
            StoreRequest::Remove { any_ref } => match self.remove_entity(&any_ref) {
                Ok(entity) => Ok(StoreResponse::Entity(entity)),
                Err(_) => Err(StoreError::Unavailable),
            },
            StoreRequest::Persist => match self.persist().await {
                Ok(()) => Ok(StoreResponse::Unit),
                Err(e) => Ok(StoreResponse::PersistErr(e)),
            },
            StoreRequest::Load { any_ref, field } => match self.load_field(&any_ref, &field).await {
                Ok(()) => Ok(StoreResponse::Unit),
                Err(e) => Ok(StoreResponse::LoadErr(e)),
            },
            StoreRequest::EnsureMutable { any_ref, field } => {
                match self.ensure_mutable(&any_ref, &field).await {
                    Ok(()) => Ok(StoreResponse::Unit),
                    Err(e) => Ok(StoreResponse::LoadErr(e)),
                }
            }
            StoreRequest::UndoCommit { any_ref } => match self.undo_commit(&any_ref) {
                Ok(()) => Ok(StoreResponse::Unit),
                Err(StoreOpError::WrongState | StoreOpError::CheckedOut | StoreOpError::NotFound) => {
                    Ok(StoreResponse::UndoErr(UndoError::WrongState))
                }
            },
            StoreRequest::Unload { any_ref } => match self.unload(&any_ref) {
                Ok(()) => Ok(StoreResponse::Unit),
                Err(StoreOpError::WrongState | StoreOpError::CheckedOut | StoreOpError::NotFound) => {
                    Ok(StoreResponse::UndoErr(UndoError::WrongState))
                }
            },
        }
    }

    fn execute(&mut self, cmd: StoreCommand) {
        match cmd {
            StoreCommand::UndoCheckout { any_ref } => {
                self.checked_out.remove(&any_ref);
            }
        }
    }

    async fn insert(&mut self, entity: TrackedEntity) -> Result<(), CommitError> {
        self.validate_committed_entity(
            &entity,
            &[],
            &[
                ValidationKind::Structural,
                ValidationKind::Semantic,
                ValidationKind::CrossEntity,
            ],
        )
        .await?;

        let any_ref = entity.any_ref();
        self.entities.insert(any_ref.clone(), entity);
        self.added.insert(any_ref);
        Ok(())
    }

    async fn resolve(&mut self, any_ref: &AnyEntityRef) -> Result<TrackedEntity, ResolveError> {
        if let Some(entity) = self.entities.get(any_ref) {
            return Ok(entity.clone());
        }

        match self.substrate.exists(&[any_ref.clone()]).await {
            Err(e) => return Err(ResolveError::Substrate(e)),
            Ok(results) if !results[0] => {
                return Err(ResolveError::NotFound {
                    entity_ref: any_ref.id().to_string(),
                });
            }
            Ok(_) => {}
        }

        let stub = TrackedEntity::make_stub(any_ref);
        self.entities.insert(any_ref.clone(), stub);
        Ok(self.entities[any_ref].clone())
    }

    fn checkout(&mut self, any_ref: &AnyEntityRef) -> Result<TrackedEntity, CheckoutError> {
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

    async fn commit(&mut self, entity: TrackedEntity) -> Result<(), CommitError> {
        let any_ref = entity.any_ref();
        if self.added.contains(&any_ref) {
            self.validate_committed_entity(
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
            let dirty_fields = entity.dirty_fields();
            self.validate_committed_entity(
                &entity,
                dirty_fields.as_slice(),
                &[ValidationKind::CrossEntity],
            )
            .await?;
        }

        self.checked_out.remove(&any_ref);
        if let Some(existing) = self.entities.get_mut(&any_ref) {
            entity.merge_dirty_into(existing);
            if entity.has_dirty_fields() && !self.added.contains(&any_ref) {
                self.modified.insert(any_ref.clone());
            }
        }
        Ok(())
    }

    fn remove_entity(&mut self, any_ref: &AnyEntityRef) -> Result<TrackedEntity, StoreOpError> {
        if self.checked_out.contains(any_ref) {
            return Err(StoreOpError::CheckedOut);
        }
        match self.entities.remove(any_ref) {
            None => Err(StoreOpError::NotFound),
            Some(entity) => {
                if self.added.remove(any_ref) {
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

        let changes = self
            .added
            .iter()
            .filter_map(|r| self.entities.get(r))
            .map(EntityChange::Added)
            .chain(self.modified.iter().filter_map(|r| {
                self.entities
                    .get(r)
                    .map(|entity| EntityChange::Modified(entity, entity.dirty_fields()))
            }))
            .chain(self.removed.iter().map(EntityChange::Removed));

        self.substrate
            .persist(changes)
            .await
            .map_err(|errs| PersistError::SubstrateErrors(BatchError::new(errs)))?;

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

    async fn load_field(&mut self, any_ref: &AnyEntityRef, field: &str) -> Result<(), LoadError> {
        self.load_fields(any_ref, &[field], true).await
    }

    async fn ensure_mutable(
        &mut self,
        any_ref: &AnyEntityRef,
        field: &str,
    ) -> Result<(), LoadError> {
        let strategy = S::load_strategy(any_ref.kind(), field)
            .map_err(LoadError::Substrate)?;

        for prereq in strategy.prerequisites {
            self.load_fields(any_ref, &[prereq], true).await?;
        }

        if !strategy.mutable_without_load {
            self.load_fields(any_ref, &[field], false).await?;
        }

        Ok(())
    }

    fn undo_commit(&mut self, any_ref: &AnyEntityRef) -> Result<(), StoreOpError> {
        if self.added.contains(any_ref) {
            self.entities.remove(any_ref);
            self.added.remove(any_ref);
            Ok(())
        } else if self.modified.contains(any_ref) {
            let stub = TrackedEntity::make_stub(any_ref);
            self.entities.insert(any_ref.clone(), stub);
            self.modified.remove(any_ref);
            Ok(())
        } else {
            Err(StoreOpError::WrongState)
        }
    }

    fn unload(&mut self, any_ref: &AnyEntityRef) -> Result<(), StoreOpError> {
        if !self.entities.contains_key(any_ref) {
            return Err(StoreOpError::NotFound);
        }
        if self.added.contains(any_ref) || self.modified.contains(any_ref) {
            return Err(StoreOpError::WrongState);
        }
        let stub = TrackedEntity::make_stub(any_ref);
        self.entities.insert(any_ref.clone(), stub);
        Ok(())
    }

    fn load_fields<'a>(
        &'a mut self,
        any_ref: &'a AnyEntityRef,
        fields: &'a [&'a str],
        include_prerequisites: bool,
    ) -> Pin<Box<dyn Future<Output = Result<(), LoadError>> + Send + 'a>> {
        Box::pin(async move {
            let mut pending_fields = {
                let current = self.entities.get(any_ref).ok_or_else(|| LoadError::NotFound {
                    entity_ref: any_ref.id().to_string(),
                })?;
                fields
                    .iter()
                    .copied()
                    .filter(|field| !current.is_field_loaded(field))
                    .collect::<Vec<_>>()
            };

            if pending_fields.is_empty() {
                return Ok(());
            }

            if include_prerequisites {
                for field in pending_fields.clone() {
                    let strategy = S::load_strategy(any_ref.kind(), field)
                        .map_err(LoadError::Substrate)?;
                    for prereq in strategy.prerequisites {
                        self.load_fields(any_ref, &[prereq], true).await?;
                    }
                }
            }

            let current = self.entities.get(any_ref).ok_or_else(|| LoadError::NotFound {
                entity_ref: any_ref.id().to_string(),
            })?;
            pending_fields.retain(|field| !current.is_field_loaded(field));
            if pending_fields.is_empty() {
                return Ok(());
            }

            let loaded = self
                .substrate
                .load(current, pending_fields.as_slice())
                .await
                .map_err(LoadError::Substrate)?;

            self.validate_loaded_entity(
                &loaded,
                pending_fields.as_slice(),
                &[
                    ValidationKind::Structural,
                    ValidationKind::Semantic,
                    ValidationKind::CrossEntity,
                ],
            )
            .await?;

            loaded.initialize_into(self.entities.get_mut(any_ref).unwrap());

            let unresolved_refs = loaded
                .all_refs()
                .into_iter()
                .filter(|r| !self.entities.contains_key(r))
                .collect::<Vec<_>>();
            if unresolved_refs.is_empty() {
                return Ok(());
            }

            if let Ok(results) = self.substrate.exists(unresolved_refs.as_slice()).await {
                for (r, exists) in unresolved_refs.into_iter().zip(results) {
                    if exists {
                        let stub = TrackedEntity::make_stub(&r);
                        self.entities.insert(r, stub);
                    }
                }
            }

            Ok(())
        })
    }

    async fn validate_committed_entity(
        &self,
        entity: &TrackedEntity,
        fields: &[&str],
        kinds: &[ValidationKind],
    ) -> Result<(), CommitError> {
        let errors = run_validations_for_entity(entity, fields, kinds).await;
        if errors.is_empty() {
            Ok(())
        } else {
            Err(CommitError::ValidationFailed {
                error_count: errors.errors.len(),
                errors,
            })
        }
    }

    async fn validate_loaded_entity(
        &self,
        entity: &TrackedEntity,
        fields: &[&str],
        kinds: &[ValidationKind],
    ) -> Result<(), LoadError> {
        let errors = run_validations_for_entity(entity, fields, kinds).await;
        if errors.is_empty() {
            Ok(())
        } else {
            Err(LoadError::ValidationFailed {
                error_count: errors.errors.len(),
                errors,
            })
        }
    }

}
