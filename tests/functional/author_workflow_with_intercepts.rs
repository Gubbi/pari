//! User job: author a workflow with lifecycle intercepts.
//!
//! Intercepts bind a [`WorkflowTrigger`] to a [`HookCall`]. Cross-entity
//! validation runs at insert and confirms each hook ref exists; if the
//! hook declares required inputs, the call's `with` map must bind them.

use std::collections::HashMap;

use pari::{
    entities::hook::Hook,
    entity::{AnyEntityRef, EntityRef, TrackedEntity},
    substrate::RepoSubstrate,
    types::{HookCall, WorkflowTrigger},
    workspace::EntityClient,
};
use rstest::rstest;
use tempfile::TempDir;

use crate::{
    common::substrate::{run_with, SubstrateKind},
    fixtures::{
        hook::{a_hook_with_required_input, a_minimal_hook},
        role::a_minimal_role,
        workflow::a_workflow_with_intercepts,
    },
};

fn workflow_ref(id: &str) -> AnyEntityRef {
    AnyEntityRef::Workflow(EntityRef::new(id))
}

fn intercept(hook_id: &str, with: Option<HashMap<String, String>>) -> HookCall {
    HookCall {
        hook: EntityRef::<Hook>::new(hook_id),
        with,
    }
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn minimal_workflow_with_intercept(#[case] kind: SubstrateKind) {
    run_with(kind, || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        EntityClient::insert(a_minimal_hook("on-done-hook"))
            .await
            .unwrap();

        let mut intercepts = HashMap::new();
        intercepts.insert(WorkflowTrigger::OnDone, intercept("on-done-hook", None));

        EntityClient::insert(a_workflow_with_intercepts(
            "DesignFlow",
            "eng-lead",
            intercepts,
        ))
        .await
        .unwrap();
        EntityClient::persist().await.unwrap();

        let entity = EntityClient::resolve(workflow_ref("DesignFlow"))
            .await
            .unwrap();
        let TrackedEntity::Workflow(wf) = entity else {
            panic!("expected Workflow")
        };
        let intercepts = wf
            .intercepts()
            .await
            .unwrap()
            .expect("intercepts populated")
            .clone();
        assert_eq!(intercepts.len(), 1);
        let call = intercepts
            .get(&WorkflowTrigger::OnDone)
            .expect("OnDone intercept present");
        assert_eq!(call.hook.id(), "on-done-hook");
        assert!(call.with.is_none());
    })
    .await;
}

#[rstest]
#[case::in_memory(SubstrateKind::InMemory)]
#[case::repo(SubstrateKind::Repo)]
#[tokio::test]
async fn workflow_intercept_binds_hook_inputs(#[case] kind: SubstrateKind) {
    run_with(kind, || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        EntityClient::insert(a_hook_with_required_input("summary-hook", "summary"))
            .await
            .unwrap();

        let mut bindings = HashMap::new();
        bindings.insert("summary".to_string(), "Workflow complete.".to_string());
        let mut intercepts = HashMap::new();
        intercepts.insert(
            WorkflowTrigger::OnDone,
            intercept("summary-hook", Some(bindings)),
        );

        EntityClient::insert(a_workflow_with_intercepts(
            "DesignFlow",
            "eng-lead",
            intercepts,
        ))
        .await
        .unwrap();
        EntityClient::persist().await.unwrap();

        let entity = EntityClient::resolve(workflow_ref("DesignFlow"))
            .await
            .unwrap();
        let TrackedEntity::Workflow(wf) = entity else {
            panic!("expected Workflow")
        };
        let intercepts = wf
            .intercepts()
            .await
            .unwrap()
            .expect("intercepts populated")
            .clone();
        let call = intercepts
            .get(&WorkflowTrigger::OnDone)
            .expect("OnDone intercept present");
        let bound = call.with.as_ref().expect("bindings populated");
        assert_eq!(
            bound.get("summary").map(String::as_str),
            Some("Workflow complete.")
        );
    })
    .await;
}

/// Cold-start a fresh repo session and confirm the intercept survives,
/// plus that the on-disk file carries the `intercepts:` frontmatter
/// key.
#[tokio::test]
async fn workflow_with_intercepts_round_trips_repo_substrate() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    let workflow_file = path.join("workflows/DesignFlow/README.md");

    pari::with(RepoSubstrate::new(path.clone()).unwrap(), || async {
        EntityClient::insert(a_minimal_role("eng-lead"))
            .await
            .unwrap();
        EntityClient::insert(a_minimal_hook("on-done-hook"))
            .await
            .unwrap();

        let mut intercepts = HashMap::new();
        intercepts.insert(WorkflowTrigger::OnDone, intercept("on-done-hook", None));

        EntityClient::insert(a_workflow_with_intercepts(
            "DesignFlow",
            "eng-lead",
            intercepts,
        ))
        .await
        .unwrap();
        EntityClient::persist().await.unwrap();
    })
    .await;

    pari::with(RepoSubstrate::new(path.clone()).unwrap(), || async {
        let entity = EntityClient::resolve(workflow_ref("DesignFlow"))
            .await
            .unwrap();
        let TrackedEntity::Workflow(wf) = entity else {
            panic!("expected Workflow")
        };
        let intercepts = wf
            .intercepts()
            .await
            .unwrap()
            .expect("intercepts populated")
            .clone();
        assert!(intercepts.contains_key(&WorkflowTrigger::OnDone));
    })
    .await;

    let contents = std::fs::read_to_string(&workflow_file).unwrap();
    assert!(
        contents.contains("intercepts:"),
        "expected intercepts frontmatter key, got:\n{contents}"
    );
    assert!(
        contents.contains("on-done-hook"),
        "expected hook id in intercepts, got:\n{contents}"
    );
}
