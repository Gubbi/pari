use std::collections::{HashMap, HashSet};

use crate::entity::{AnyEntityRef, TrackedEntity};

/// A single tracked-entity change emitted by the store for persistence.
pub enum EntityChange<'a> {
    Added(&'a TrackedEntity),
    Modified(&'a TrackedEntity, Vec<&'static str>),
    Removed(&'a AnyEntityRef),
}

/// Lazy store-owned view over the current persist set.
pub(crate) struct PersistChanges<'a> {
    entities: &'a HashMap<AnyEntityRef, TrackedEntity>,
    added: &'a HashSet<AnyEntityRef>,
    modified: &'a HashSet<AnyEntityRef>,
    removed: &'a HashSet<AnyEntityRef>,
}

impl<'a> PersistChanges<'a> {
    pub(crate) fn new(
        entities: &'a HashMap<AnyEntityRef, TrackedEntity>,
        added: &'a HashSet<AnyEntityRef>,
        modified: &'a HashSet<AnyEntityRef>,
        removed: &'a HashSet<AnyEntityRef>,
    ) -> Self {
        Self {
            entities,
            added,
            modified,
            removed,
        }
    }

    pub(crate) fn iter(&'a self) -> impl Iterator<Item = EntityChange<'a>> + Send + 'a {
        self.added
            .iter()
            .filter_map(|any_ref| self.entities.get(any_ref))
            .map(EntityChange::Added)
            .chain(self.modified.iter().filter_map(|any_ref| {
                self.entities
                    .get(any_ref)
                    .map(|entity| EntityChange::Modified(entity, entity.dirty_fields()))
            }))
            .chain(self.removed.iter().map(EntityChange::Removed))
    }
}
