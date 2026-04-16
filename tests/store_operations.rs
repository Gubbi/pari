use pari::store::EntityServer;
use pari::substrate::{InMemoryStorage, InMemorySubstrate};
use pari::workspace::{CheckoutError, EntityClient, PersistError, ResolveError};
use pari::entity::{AnyEntityRef, EntityRef, TrackedEntity};
use pari::entities::role::{Role, TrackedRole};
use std::collections::HashMap;

fn make_role(id: &str) -> Role {
    Role {
        entity_ref:  EntityRef::new(id),
        name:        format!("{} Name", id),
        description: None,
        purpose:     "Purpose".to_string(),
        traits:      None,
        extensions:  HashMap::new(),
    }
}

fn role_any_ref(id: &str) -> AnyEntityRef {
    AnyEntityRef::Role(EntityRef::new(id))
}

#[tokio::test]
async fn insert_then_resolve_returns_entity() {
    EntityServer::with(InMemorySubstrate::new(), || async {
        let role = make_role("eng-lead");
        EntityClient::insert(TrackedEntity::from_role(TrackedRole::from(role))).await.unwrap();
        let resolved = EntityClient::resolve(role_any_ref("eng-lead")).await.unwrap();
        assert_eq!(resolved.any_ref().id(), "eng-lead");
    }).await;
}

#[tokio::test]
async fn resolve_absent_entity_creates_stub_via_substrate() {
    let storage = InMemoryStorage::new();

    EntityServer::with(InMemorySubstrate::with_storage(storage.clone()), || async {
        let role = make_role("pm");
        EntityClient::insert(TrackedEntity::from_role(TrackedRole::from(role))).await.unwrap();
        EntityClient::persist().await.unwrap();
    }).await;

    EntityServer::with(InMemorySubstrate::with_storage(storage), || async {
        let resolved = EntityClient::resolve(role_any_ref("pm")).await.unwrap();
        assert_eq!(resolved.any_ref().id(), "pm");
    }).await;
}

#[tokio::test]
async fn resolve_nonexistent_entity_returns_error() {
    EntityServer::with(InMemorySubstrate::new(), || async {
        let result = EntityClient::resolve(role_any_ref("ghost")).await;
        assert!(matches!(result, Err(ResolveError::NotFound { entity_ref }) if entity_ref == "ghost"));
    }).await;
}

#[tokio::test]
async fn insert_adds_to_added_set() {
    EntityServer::with(InMemorySubstrate::new(), || async {
        let role = make_role("eng-lead");
        EntityClient::insert(TrackedEntity::from_role(TrackedRole::from(role))).await.unwrap();
        EntityClient::persist().await.unwrap();
    }).await;
}

#[tokio::test]
async fn checkout_then_commit_updates_entity() {
    EntityServer::with(InMemorySubstrate::new(), || async {
        let role = make_role("eng-lead");
        EntityClient::insert(TrackedEntity::from_role(TrackedRole::from(role))).await.unwrap();

        let mut entity = EntityClient::checkout(role_any_ref("eng-lead")).await.unwrap();
        if let TrackedEntity::Role(ref mut r) = entity {
            r.name = std::sync::Arc::new(pari::tracked::TrackedField::mutated("New Name".to_string()));
        }
        entity.commit().await.unwrap();

        let resolved = EntityClient::resolve(role_any_ref("eng-lead")).await.unwrap();
        if let TrackedEntity::Role(r) = resolved {
            assert_eq!(r.name.get(), Some(&"New Name".to_string()));
        }
    }).await;
}

#[tokio::test]
async fn double_checkout_returns_error() {
    EntityServer::with(InMemorySubstrate::new(), || async {
        let role = make_role("eng-lead");
        EntityClient::insert(TrackedEntity::from_role(TrackedRole::from(role))).await.unwrap();

        let _checkout1 = EntityClient::checkout(role_any_ref("eng-lead")).await.unwrap();
        let checkout2 = EntityClient::checkout(role_any_ref("eng-lead")).await;
        assert!(matches!(checkout2, Err(CheckoutError::AlreadyCheckedOut { entity_ref: _ })));
    }).await;
}

#[tokio::test]
async fn undo_checkout_releases_lock() {
    EntityServer::with(InMemorySubstrate::new(), || async {
        let role = make_role("eng-lead");
        EntityClient::insert(TrackedEntity::from_role(TrackedRole::from(role))).await.unwrap();

        let checkout = EntityClient::checkout(role_any_ref("eng-lead")).await.unwrap();
        checkout.undo_checkout().await.unwrap();
        let _ = EntityClient::checkout(role_any_ref("eng-lead")).await.unwrap();
    }).await;
}

#[tokio::test]
async fn remove_then_resolve_returns_error() {
    EntityServer::with(InMemorySubstrate::new(), || async {
        let role = make_role("eng-lead");
        EntityClient::insert(TrackedEntity::from_role(TrackedRole::from(role))).await.unwrap();

        EntityClient::remove(role_any_ref("eng-lead")).await.unwrap();
        let result = EntityClient::resolve(role_any_ref("eng-lead")).await;
        assert!(matches!(result, Err(ResolveError::NotFound { entity_ref }) if entity_ref == "eng-lead"));
    }).await;
}

#[tokio::test]
async fn persist_fails_with_pending_checkouts() {
    EntityServer::with(InMemorySubstrate::new(), || async {
        let role = make_role("eng-lead");
        EntityClient::insert(TrackedEntity::from_role(TrackedRole::from(role))).await.unwrap();

        let _checkout = EntityClient::checkout(role_any_ref("eng-lead")).await.unwrap();
        let result = EntityClient::persist().await;
        assert!(matches!(result, Err(PersistError::PendingCheckouts { checked_out_count: _ })));
    }).await;
}

#[tokio::test]
async fn undo_commit_on_added_entity_removes_it() {
    EntityServer::with(InMemorySubstrate::new(), || async {
        let role = make_role("eng-lead");
        EntityClient::insert(TrackedEntity::from_role(TrackedRole::from(role))).await.unwrap();

        EntityClient::undo_commit(role_any_ref("eng-lead")).await.unwrap();
        let result = EntityClient::resolve(role_any_ref("eng-lead")).await;
        assert!(matches!(result, Err(ResolveError::NotFound { entity_ref }) if entity_ref == "eng-lead"));
    }).await;
}

#[tokio::test]
async fn unload_on_clean_entity_creates_stub() {
    let storage = InMemoryStorage::new();

    EntityServer::with(InMemorySubstrate::with_storage(storage.clone()), || async {
        let role = make_role("eng-lead");
        EntityClient::insert(TrackedEntity::from_role(TrackedRole::from(role))).await.unwrap();
        EntityClient::persist().await.unwrap();
    }).await;

    EntityServer::with(InMemorySubstrate::with_storage(storage), || async {
        EntityClient::resolve(role_any_ref("eng-lead")).await.unwrap();
        EntityClient::unload(role_any_ref("eng-lead")).await.unwrap();
        let resolved = EntityClient::resolve(role_any_ref("eng-lead")).await.unwrap();
        assert_eq!(resolved.any_ref().id(), "eng-lead");
    }).await;
}
