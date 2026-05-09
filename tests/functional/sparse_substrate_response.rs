//! Substrate contract: a load response may carry a subset of the
//! requested fields. The store treats the asset's projected schema as
//! the arbiter — required-missing surfaces a structured error,
//! optional-missing is loaded as `null`. Either way every field the
//! asset is responsible for must be marked loaded so subsequent
//! accessors don't re-trigger Load.
//!
//! Standard backends (`InMemoryCodec`, `RepoCodec`) always emit every
//! schema field with explicit `null` for absent values, so this case
//! is latent unless a third-party `Substrate` impl chooses to emit
//! sparse responses. The tests pin the contract via
//! `InMemoryStorage::put` — pre-seeded sparse blobs flow through the
//! standard codec / load path.

use pari::{
    entities::role::Role,
    entity::EntityRef,
    error::{primitive::PrimitiveError, ActivityError},
    substrate::{InMemoryStorage, InMemorySubstrate},
};

use crate::common::substrate::with_workspace;

fn role_ref(id: &str) -> EntityRef<Role> {
    EntityRef::new(id)
}

fn substrate_with_sparse_role(content: &str) -> InMemorySubstrate {
    let storage = InMemoryStorage::new();
    storage.put("common/roles/eng-lead", content);
    InMemorySubstrate::with_storage(storage)
}

/// Stored slice omits two optional fields (`description`, `traits`)
/// while keeping every required one. Schema gate accepts; the load
/// path fills the absent optionals with `null` so the workspace's
/// accessors return `None` and don't re-issue Load.
#[tokio::test]
async fn optional_field_absent_in_response_loads_as_null() {
    let substrate = substrate_with_sparse_role(r#"{"name":"Engineering Lead","purpose":"test"}"#);

    with_workspace(substrate, |workspace| async move {
        let role = workspace.resolve(role_ref("eng-lead")).await.unwrap();
        // Required fields load normally.
        assert_eq!(role.name().await.unwrap(), "Engineering Lead");
        assert_eq!(role.purpose().await.unwrap(), "test");
        // Optional fields absent in the response — must surface as
        // `None`, not panic with "field not loaded".
        assert_eq!(role.description().await.unwrap(), None);
        assert_eq!(role.traits().await.unwrap(), None);
    })
    .await;
}

/// Stored slice omits a required field. Schema gate rejects the slice
/// before merge — load surfaces an `UnpersistableDefinition` whose
/// cause cites the missing field.
#[tokio::test]
async fn required_field_absent_in_response_surfaces_schema_error() {
    let substrate = substrate_with_sparse_role(r#"{"name":"Engineering Lead"}"#);

    with_workspace(substrate, |workspace| async move {
        let role = workspace.resolve(role_ref("eng-lead")).await.unwrap();
        let err = role
            .purpose()
            .await
            .err()
            .expect("required-missing should error");
        let cause = match &err {
            ActivityError::UnpersistableDefinition { cause, .. } => cause,
            _ => panic!("expected UnpersistableDefinition, got: {err:?}"),
        };
        let reason = match cause {
            PrimitiveError::PartialPayloadDeserialization { reason, .. } => reason,
            _ => panic!("expected PartialPayloadDeserialization, got: {cause:?}"),
        };
        assert!(
            reason.to_lowercase().contains("purpose") || reason.to_lowercase().contains("required"),
            "expected reason to cite the missing required field, got: {reason}"
        );
    })
    .await;
}
