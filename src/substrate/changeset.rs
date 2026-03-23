//! [`ChangeSet`] — substrate-agnostic description of entity changes.
//!
//! Produced by [`EntityStore::collect_changes`](crate::schema::store::EntityStore::collect_changes)
//! and consumed by [`Substrate::atomic_persist`](crate::substrate::Substrate::atomic_persist).
//!
//! `ChangeSet` borrows from the [`EntityStore`](crate::schema::store::EntityStore).  The typical
//! call sequence is:
//!
//! ```rust,ignore
//! let cs = store.collect_changes();
//! substrate.atomic_persist(&cs)?;
//! // Drop `cs` before calling reset_tracked so the borrow is released.
//! store.reset_tracked();
//! ```

use crate::schema::entities::{
    hook::TrackedHook,
    relay::TrackedRelay,
    role::TrackedRole,
    task::TrackedTask,
    team::TrackedTeam,
    workflow::{TrackedSharedWorkflow, TrackedWorkflow},
};

// ---------------------------------------------------------------------------
// EntityKind
// ---------------------------------------------------------------------------

/// Discriminates which entity type an [`EntityChange`] refers to.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum EntityKind {
    Role,
    Hook,
    Team,
    Workflow,
    SharedWorkflow,
    Task,
    Relay,
}

// ---------------------------------------------------------------------------
// EntityData
// ---------------------------------------------------------------------------

/// Reference to a full tracked entity value, one variant per entity type.
///
/// Substrates access fields via `Deref` on the inner `Tracked<T>` fields —
/// no conversion back to plain types is required.
pub enum EntityData<'a> {
    Role(&'a TrackedRole),
    Hook(&'a TrackedHook),
    Team(&'a TrackedTeam),
    Workflow(&'a TrackedWorkflow),
    SharedWorkflow(&'a TrackedSharedWorkflow),
    Task(&'a TrackedTask),
    Relay(&'a TrackedRelay),
}

// ---------------------------------------------------------------------------
// ChangeOp
// ---------------------------------------------------------------------------

/// The operation that produced this change.
pub enum ChangeOp<'a> {
    /// Entity was newly inserted.
    Added(EntityData<'a>),
    /// Entity was modified; `dirty_fields` lists which fields changed.
    Modified {
        entity: EntityData<'a>,
        dirty_fields: Vec<String>,
    },
    /// Entity was removed.  No entity data is retained; use `path` and `id`
    /// for filesystem removal.
    Removed,
}

// ---------------------------------------------------------------------------
// EntityChange
// ---------------------------------------------------------------------------

/// A single entity change entry within a [`ChangeSet`].
pub struct EntityChange<'a> {
    /// Filesystem path to the entity's directory (relative to repo root).
    /// e.g. `"roles"`, `"workflows/Initiative/WriteProposal"`.
    pub path: String,
    /// The kind of entity.
    pub kind: EntityKind,
    /// The entity's id string.
    pub id: String,
    /// The operation.
    pub op: ChangeOp<'a>,
}

// ---------------------------------------------------------------------------
// ChangeSet
// ---------------------------------------------------------------------------

/// A flat collection of entity changes produced by
/// [`EntityStore::collect_changes`](crate::schema::store::EntityStore::collect_changes).
///
/// Borrows entity data from the source [`EntityStore`](crate::schema::store::EntityStore).
pub struct ChangeSet<'a> {
    pub changes: Vec<EntityChange<'a>>,
}

impl<'a> ChangeSet<'a> {
    pub fn new() -> Self {
        Self { changes: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.changes.len()
    }
}

impl<'a> Default for ChangeSet<'a> {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{
        entities::role::Role,
        types::Extensions,
    };

    fn tracked_role(id: &str) -> TrackedRole {
        TrackedRole::from(Role {
            id: id.into(),
            name: id.to_string(),
            purpose: "test".to_string(),
            traits: None,
            extensions: Extensions::default(),
        })
    }

    // --- 6.1: EntityKind variants ---

    #[test]
    fn entity_kind_variants_are_distinct() {
        assert_ne!(EntityKind::Role, EntityKind::Hook);
        assert_ne!(EntityKind::Team, EntityKind::Workflow);
        assert_ne!(EntityKind::SharedWorkflow, EntityKind::Task);
        assert_ne!(EntityKind::Task, EntityKind::Relay);
    }

    #[test]
    fn entity_kind_clone_and_eq() {
        let k = EntityKind::Role;
        assert_eq!(k.clone(), EntityKind::Role);
    }

    // --- 6.1: EntityData carries tracked entity types (via reference) ---

    #[test]
    fn entity_data_role_wraps_tracked_role() {
        let tr = tracked_role("eng-lead");
        let data = EntityData::Role(&tr);
        if let EntityData::Role(r) = data {
            assert_eq!(&**r.id, "eng-lead");
        } else {
            panic!("expected EntityData::Role");
        }
    }

    // --- 6.1: ChangeOp variants ---

    #[test]
    fn change_op_added_carries_entity_data() {
        let tr = tracked_role("eng-lead");
        let op = ChangeOp::Added(EntityData::Role(&tr));
        assert!(matches!(op, ChangeOp::Added(_)));
    }

    #[test]
    fn change_op_modified_carries_entity_data_and_dirty_fields() {
        let tr = tracked_role("eng-lead");
        let op = ChangeOp::Modified {
            entity: EntityData::Role(&tr),
            dirty_fields: vec!["name".to_string()],
        };
        if let ChangeOp::Modified { dirty_fields, .. } = op {
            assert_eq!(dirty_fields, vec!["name"]);
        } else {
            panic!("expected ChangeOp::Modified");
        }
    }

    #[test]
    fn change_op_removed_has_no_data() {
        let op: ChangeOp = ChangeOp::Removed;
        assert!(matches!(op, ChangeOp::Removed));
    }

    // --- 6.1: EntityChange construction ---

    #[test]
    fn entity_change_fields_accessible() {
        let tr = tracked_role("eng-lead");
        let change = EntityChange {
            path: "roles".to_string(),
            kind: EntityKind::Role,
            id: "eng-lead".to_string(),
            op: ChangeOp::Added(EntityData::Role(&tr)),
        };
        assert_eq!(change.path, "roles");
        assert_eq!(change.kind, EntityKind::Role);
        assert_eq!(change.id, "eng-lead");
    }

    // --- 6.1: ChangeSet ---

    #[test]
    fn changeset_starts_empty() {
        let cs: ChangeSet = ChangeSet::new();
        assert!(cs.is_empty());
        assert_eq!(cs.len(), 0);
    }

    #[test]
    fn changeset_with_changes_not_empty() {
        let tr = tracked_role("eng-lead");
        let mut cs = ChangeSet::new();
        cs.changes.push(EntityChange {
            path: "roles".to_string(),
            kind: EntityKind::Role,
            id: "eng-lead".to_string(),
            op: ChangeOp::Added(EntityData::Role(&tr)),
        });
        assert!(!cs.is_empty());
        assert_eq!(cs.len(), 1);
    }

    #[test]
    fn changeset_default_is_empty() {
        let cs: ChangeSet = ChangeSet::default();
        assert!(cs.is_empty());
    }
}
