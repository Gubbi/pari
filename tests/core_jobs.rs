#![cfg(any())]
// TODO: Re-enable once a design-aligned filesystem substrate replaces the removed RepoSubstrate.

//! End-to-end tests for the eight core jobs.
//!
//! All tests exercise the full stack:
//!   EntityClient → EntityServer → Store<RepoSubstrate> → filesystem
//!
//! Jobs:
//!   1. Read an entity
//!   2. Define a new entity
//!   3. Update an existing entity
//!   4. Remove an entity
//!   5. Save all pending changes
//!   6. Abandon an in-progress edit
//!   7. Roll back a staged change
//!   8. Refresh an entity from the substrate

use std::collections::HashMap;

use tempfile::TempDir;

use pari::entities::role::{Role, TrackedRole};
use pari::entity::{AnyEntityRef, EntityRef, StoreEntity};
use pari::store::{EntityClient, EntityServer};
use pari::substrate::repo::RepoSubstrate;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn a_role(id: &str, name: &str) -> StoreEntity {
    StoreEntity::from_role(TrackedRole::from(Role {
        entity_ref:  EntityRef::new(id),
        name:        name.to_string(),
        description: None,
        purpose:     "test purpose".to_string(),
        traits:      None,
        extensions:  HashMap::new(),
    }))
}

fn role_ref(id: &str) -> AnyEntityRef {
    AnyEntityRef::Role(EntityRef::new(id))
}

fn repo(dir: &TempDir) -> RepoSubstrate {
    RepoSubstrate::new(dir.path().to_path_buf()).expect("RepoSubstrate::new")
}

// ---------------------------------------------------------------------------
// Job 1 — Read an entity
//
// When the substrate holds an entity I did not insert in this session,
// resolving it loads it from disk and makes its fields readable.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn job_1_read_entity() {
    let dir = TempDir::new().unwrap();

    // Setup: persist a role to disk via one server session.
    EntityServer::with_test(repo(&dir), || async {
        EntityClient::insert(a_role("eng-lead", "Engineering Lead")).await.unwrap();
        EntityClient::persist().await.unwrap();
    }).await;

    // Job: fresh server — resolve reads the entity from the substrate.
    EntityServer::with_test(repo(&dir), || async {
        let entity = EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        let StoreEntity::Role(role) = entity else { panic!("expected Role") };
        assert_eq!(role.name().await.unwrap(), "Engineering Lead");
    }).await;
}

// ---------------------------------------------------------------------------
// Job 2 — Define a new entity
//
// Inserting a new entity and persisting creates its file on disk.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn job_2_define_new_entity() {
    let dir = TempDir::new().unwrap();
    let role_file = dir.path().join("roles/designer.md");

    EntityServer::with_test(repo(&dir), || async {
        EntityClient::insert(a_role("designer", "Designer")).await.unwrap();
        EntityClient::persist().await.unwrap();
    }).await;

    assert!(role_file.exists(), "role file must be created after persist");
}

// ---------------------------------------------------------------------------
// Job 3 — Update an existing entity
//
// Checking out an entity, changing a field, committing, and persisting
// overwrites the file on disk with the new value.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn job_3_update_entity() {
    let dir = TempDir::new().unwrap();

    // Setup: persist initial state.
    EntityServer::with_test(repo(&dir), || async {
        EntityClient::insert(a_role("eng-lead", "Original Name")).await.unwrap();
        EntityClient::persist().await.unwrap();
    }).await;

    // Job: load → checkout → mutate → commit → persist.
    EntityServer::with_test(repo(&dir), || async {
        EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        let mut entity = EntityClient::checkout(role_ref("eng-lead")).await.unwrap();
        if let StoreEntity::Role(ref mut r) = entity {
            r.set_name("Updated Name".to_string()).await.unwrap();
        }
        entity.commit().await.unwrap();
        EntityClient::persist().await.unwrap();
    }).await;

    // Verify: fresh server sees the updated value.
    EntityServer::with_test(repo(&dir), || async {
        let entity = EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        let StoreEntity::Role(role) = entity else { panic!("expected Role") };
        assert_eq!(role.name().await.unwrap(), "Updated Name");
    }).await;
}

// ---------------------------------------------------------------------------
// Job 4 — Remove an entity
//
// Removing an entity and persisting deletes its file from disk.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn job_4_remove_entity() {
    let dir = TempDir::new().unwrap();
    let role_file = dir.path().join("roles/eng-lead.md");

    // Setup: create the file.
    EntityServer::with_test(repo(&dir), || async {
        EntityClient::insert(a_role("eng-lead", "Engineering Lead")).await.unwrap();
        EntityClient::persist().await.unwrap();
    }).await;

    assert!(role_file.exists(), "file must exist before removal");

    // Job: load → remove → persist.
    EntityServer::with_test(repo(&dir), || async {
        EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        EntityClient::remove(role_ref("eng-lead")).await.unwrap();
        EntityClient::persist().await.unwrap();
    }).await;

    assert!(!role_file.exists(), "file must be deleted after persist");
}

// ---------------------------------------------------------------------------
// Job 5 — Save all pending changes
//
// Multiple inserts, updates, and removes staged within one session are all
// flushed atomically by a single persist call.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn job_5_save_all_pending_changes() {
    let dir = TempDir::new().unwrap();

    // Setup: write role-b to disk so it can be updated.
    EntityServer::with_test(repo(&dir), || async {
        EntityClient::insert(a_role("role-b", "Role B Original")).await.unwrap();
        EntityClient::persist().await.unwrap();
    }).await;

    // Job: stage an add, an update, and a remove — then persist once.
    EntityServer::with_test(repo(&dir), || async {
        // Add
        EntityClient::insert(a_role("role-a", "Role A")).await.unwrap();

        // Update role-b
        EntityClient::resolve(role_ref("role-b")).await.unwrap();
        let mut entity = EntityClient::checkout(role_ref("role-b")).await.unwrap();
        if let StoreEntity::Role(ref mut r) = entity { r.set_name("Role B Updated".to_string()).await.unwrap(); }
        entity.commit().await.unwrap();

        // Remove role-b (after committing the checkout we can remove it)
        EntityClient::remove(role_ref("role-b")).await.unwrap();

        // Single persist flushes all three changes.
        EntityClient::persist().await.unwrap();
    }).await;

    assert!(dir.path().join("roles/role-a.md").exists(), "added file must exist");
    assert!(!dir.path().join("roles/role-b.md").exists(), "removed file must be gone");
}

// ---------------------------------------------------------------------------
// Job 6 — Abandon an in-progress edit
//
// After checking out an entity and mutating it, calling undo_checkout discards
// the changes and releases the lock. The entity retains its previous state.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn job_6_abandon_in_progress_edit() {
    let dir = TempDir::new().unwrap();

    EntityServer::with_test(repo(&dir), || async {
        EntityClient::insert(a_role("eng-lead", "Original")).await.unwrap();
        EntityClient::persist().await.unwrap();
    }).await;

    EntityServer::with_test(repo(&dir), || async {
        EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        let mut entity = EntityClient::checkout(role_ref("eng-lead")).await.unwrap();
        if let StoreEntity::Role(ref mut r) = entity {
            r.set_name("Abandoned".to_string()).await.unwrap();
        }
        // Abandon: discard changes and release the lock.
        entity.undo_checkout().await.unwrap();

        // The entity in the store is back to its pre-checkout state.
        let entity = EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        let StoreEntity::Role(role) = entity else { panic!("expected Role") };
        assert_eq!(role.name().await.unwrap(), "Original");

        // A second checkout is now possible (lock was released).
        let checkout2 = EntityClient::checkout(role_ref("eng-lead")).await;
        assert!(checkout2.is_ok(), "lock should be released after undo_checkout");
    }).await;
}

// ---------------------------------------------------------------------------
// Job 7 — Roll back a staged change
//
// After committing a change (staged but not yet persisted), undo_commit reverts
// the entity to its last persisted state. A subsequent persist is a no-op for
// that entity.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn job_7_rollback_staged_change() {
    let dir = TempDir::new().unwrap();

    EntityServer::with_test(repo(&dir), || async {
        EntityClient::insert(a_role("eng-lead", "Original")).await.unwrap();
        EntityClient::persist().await.unwrap();
    }).await;

    EntityServer::with_test(repo(&dir), || async {
        EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        let mut entity = EntityClient::checkout(role_ref("eng-lead")).await.unwrap();
        if let StoreEntity::Role(ref mut r) = entity {
            r.set_name("Staged".to_string()).await.unwrap();
        }
        entity.commit().await.unwrap(); // staged, not yet persisted

        // Roll back the staged change.
        EntityClient::undo_commit(role_ref("eng-lead")).await.unwrap();

        // Re-resolving reloads from the substrate — still "Original".
        let entity = EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        let StoreEntity::Role(role) = entity else { panic!("expected Role") };
        assert_eq!(role.name().await.unwrap(), "Original");
    }).await;

    // Disk is also unchanged.
    EntityServer::with_test(repo(&dir), || async {
        let entity = EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        let StoreEntity::Role(role) = entity else { panic!("expected Role") };
        assert_eq!(role.name().await.unwrap(), "Original");
    }).await;
}

// ---------------------------------------------------------------------------
// Job 8 — Refresh an entity from the substrate
//
// When the substrate changes externally (another writer updated the file),
// an entity loaded in memory holds stale data. Unloading it and re-resolving
// pulls the latest version from the substrate.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn job_8_refresh_from_substrate() {
    let dir = TempDir::new().unwrap();

    // Block 1: persist "v1" to disk.
    EntityServer::with_test(repo(&dir), || async {
        EntityClient::insert(a_role("eng-lead", "v1")).await.unwrap();
        EntityClient::persist().await.unwrap();
    }).await;

    // Block 2: resolve loads "v1" into memory (clean, unloadable).
    // While the session is live, overwrite the file on disk with "v2" to
    // simulate an external writer changing the substrate.
    EntityServer::with_test(repo(&dir), || async {
        // Load "v1" from disk — entity is now clean in the store (not in added/modified).
        let resolved = EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        let StoreEntity::Role(ref role) = resolved else { panic!("expected Role") };
        assert_eq!(role.name().await.unwrap(), "v1", "should see v1 before external write");

        // Simulate external write: another writer updates the substrate file.
        // The markdown format is: YAML frontmatter (purpose), then `# <name>` heading.
        std::fs::write(
            dir.path().join("roles/eng-lead.md"),
            "---\npurpose: test purpose\n---\n\n# v2\n",
        ).unwrap();

        // The in-memory entity is still stale — resolve returns the cached copy.
        let stale = EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        let StoreEntity::Role(ref role) = stale else { panic!("expected Role") };
        assert_eq!(role.name().await.unwrap(), "v1", "should see stale v1 before refresh");

        // Unload evicts the stale in-memory copy.
        EntityClient::unload(role_ref("eng-lead")).await.unwrap();

        // Re-resolve now loads fresh from the substrate — gets "v2".
        let refreshed = EntityClient::resolve(role_ref("eng-lead")).await.unwrap();
        let StoreEntity::Role(role) = refreshed else { panic!("expected Role") };
        assert_eq!(role.name().await.unwrap(), "v2", "should see fresh v2 after refresh");
    }).await;
}
