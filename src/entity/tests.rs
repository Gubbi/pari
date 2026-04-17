use std::collections::HashMap;

use super::{
    entities::workflow::{EmbeddedWorkflow, ReusableWorkflow, Workflow},
    load_strategy, AnyEntityRef, Entity, EntityKind, EntityRef, NoParent, Tracked, TrackedEntity,
    TrackedFor, ValidationSchema, WorkflowParent,
};

#[test]
fn no_parent_instances_are_equal() {
    assert_eq!(NoParent, NoParent);
}

#[test]
fn workflow_parent_equality_based_on_parent_ref() {
    let a = WorkflowParent::Workflow(EntityRef::<Workflow>::new("InitiativeWorkflow"));
    let b = WorkflowParent::Workflow(EntityRef::<Workflow>::new("InitiativeWorkflow"));
    let c = WorkflowParent::Workflow(EntityRef::<Workflow>::new("OtherWorkflow"));
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn entity_kind_variants_are_copy() {
    let k = EntityKind::Role;
    let k2 = k;
    assert_eq!(k, k2);
}

#[test]
fn entity_kind_variants_are_distinct() {
    assert_ne!(EntityKind::Role, EntityKind::Hook);
    assert_ne!(EntityKind::Task, EntityKind::Relay);
}

struct TestEntity;

impl Entity for TestEntity {
    const KIND: EntityKind = EntityKind::Role;

    fn validation_schema() -> &'static ValidationSchema<Self> {
        static S: std::sync::OnceLock<ValidationSchema<TestEntity>> = std::sync::OnceLock::new();
        S.get_or_init(ValidationSchema::empty)
    }

    type Parent = NoParent;
    type Tracked = TestTrackedEntity;

    fn to_any_ref(_: &EntityRef<Self, Self::Parent>) -> AnyEntityRef {
        unimplemented!()
    }

    fn extract(_: &TrackedEntity) -> Option<&Self::Tracked> {
        unimplemented!()
    }
}

struct TestTrackedEntity;

impl TrackedFor for TestTrackedEntity {
    type Entity = TestEntity;
}

#[test]
fn entity_ref_id_roundtrip() {
    let r: EntityRef<TestEntity> = EntityRef::new("eng-lead");
    assert_eq!(r.id(), "eng-lead");
}

#[test]
fn entity_ref_equality_same_id() {
    let a: EntityRef<TestEntity> = EntityRef::new("eng-lead");
    let b: EntityRef<TestEntity> = EntityRef::new("eng-lead");
    assert_eq!(a, b);
}

#[test]
fn entity_ref_inequality_different_id() {
    let a: EntityRef<TestEntity> = EntityRef::new("eng-lead");
    let b: EntityRef<TestEntity> = EntityRef::new("pm");
    assert_ne!(a, b);
}

#[test]
fn entity_ref_usable_as_hashmap_key() {
    let mut map: HashMap<EntityRef<TestEntity>, u32> = HashMap::new();
    let r = EntityRef::new("eng-lead");
    map.insert(r.clone(), 42);
    assert_eq!(map[&r], 42);
}

#[test]
fn top_level_entity_ref_parent_is_none() {
    let r: EntityRef<TestEntity> = EntityRef::new("eng-lead");
    assert!(r.parent().is_none());
}

struct EmbeddedTest;

impl Entity for EmbeddedTest {
    const KIND: EntityKind = EntityKind::Task;

    fn validation_schema() -> &'static ValidationSchema<Self> {
        static S: std::sync::OnceLock<ValidationSchema<EmbeddedTest>> = std::sync::OnceLock::new();
        S.get_or_init(ValidationSchema::empty)
    }

    type Parent = WorkflowParent;
    type Tracked = EmbeddedTracked;

    fn to_any_ref(_: &EntityRef<Self, Self::Parent>) -> AnyEntityRef {
        unimplemented!()
    }

    fn extract(_: &TrackedEntity) -> Option<&Self::Tracked> {
        unimplemented!()
    }
}

struct EmbeddedTracked;

impl TrackedFor for EmbeddedTracked {
    type Entity = EmbeddedTest;
}

#[test]
fn embedded_entity_ref_equality_requires_matching_parent() {
    let a = EntityRef::<EmbeddedTest, WorkflowParent>::with_parent(
        "WriteProposal",
        WorkflowParent::Workflow(EntityRef::<Workflow>::new("InitiativeWorkflow")),
    );
    let b = EntityRef::<EmbeddedTest, WorkflowParent>::with_parent(
        "WriteProposal",
        WorkflowParent::Workflow(EntityRef::<Workflow>::new("InitiativeWorkflow")),
    );
    let c = EntityRef::<EmbeddedTest, WorkflowParent>::with_parent(
        "WriteProposal",
        WorkflowParent::Workflow(EntityRef::<Workflow>::new("OtherWorkflow")),
    );
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn embedded_entity_ref_parent_is_some() {
    let r = EntityRef::<EmbeddedTest, WorkflowParent>::with_parent(
        "WriteProposal",
        WorkflowParent::Workflow(EntityRef::<Workflow>::new("InitiativeWorkflow")),
    );
    assert!(matches!(
        r.parent(),
        Some(WorkflowParent::Workflow(parent)) if parent.id() == "InitiativeWorkflow"
    ));
}

#[test]
fn entity_ref_serde_roundtrip_top_level() {
    let original: EntityRef<Workflow> = EntityRef::new("InitiativeWorkflow");
    let json = serde_json::to_value(&original).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "id": "InitiativeWorkflow",
            "kind": "Workflow",
        })
    );

    let decoded: EntityRef<Workflow> = serde_json::from_value(json).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn entity_ref_serde_roundtrip_embedded_with_recursive_parent() {
    let parent = WorkflowParent::EmbeddedWorkflow(Box::new(EntityRef::<
        EmbeddedWorkflow,
        WorkflowParent,
    >::with_parent(
        "SubWorkflow",
        WorkflowParent::Workflow(EntityRef::<Workflow>::new("InitiativeWorkflow")),
    )));
    let original: EntityRef<EmbeddedTest, WorkflowParent> =
        EntityRef::with_parent("WriteProposal", parent);

    let json = serde_json::to_value(&original).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "id": "WriteProposal",
            "kind": "Task",
            "parent": {
                "id": "SubWorkflow",
                "kind": "EmbeddedWorkflow",
                "parent": {
                    "id": "InitiativeWorkflow",
                    "kind": "Workflow",
                }
            }
        })
    );

    let decoded: EntityRef<EmbeddedTest, WorkflowParent> = serde_json::from_value(json).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn entity_ref_deserialize_rejects_kind_mismatch() {
    let err = serde_json::from_value::<EntityRef<Workflow>>(serde_json::json!({
        "id": "InitiativeWorkflow",
        "kind": "Role",
    }))
    .unwrap_err();
    assert!(err.to_string().contains("entity kind mismatch"));
}

#[test]
fn entity_ref_deserialize_rejects_parent_for_top_level_refs() {
    let err = serde_json::from_value::<EntityRef<Workflow>>(serde_json::json!({
        "id": "InitiativeWorkflow",
        "kind": "Workflow",
        "parent": {
            "id": "ParentWorkflow",
            "kind": "Workflow",
        }
    }))
    .unwrap_err();
    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn workflow_parent_to_any_ref_preserves_parent_variant() {
    let workflow_parent =
        WorkflowParent::Workflow(EntityRef::<Workflow>::new("InitiativeWorkflow"));
    let reusable_parent =
        WorkflowParent::ReusableWorkflow(EntityRef::<ReusableWorkflow>::new("CommonFlow"));
    let embedded_parent = WorkflowParent::EmbeddedWorkflow(Box::new(EntityRef::<
        EmbeddedWorkflow,
        WorkflowParent,
    >::with_parent(
        "SubWorkflow",
        WorkflowParent::Workflow(EntityRef::<Workflow>::new("InitiativeWorkflow")),
    )));

    assert!(matches!(
        workflow_parent.to_any_ref(),
        AnyEntityRef::Workflow(r) if r.id() == "InitiativeWorkflow"
    ));
    assert!(matches!(
        reusable_parent.to_any_ref(),
        AnyEntityRef::ReusableWorkflow(r) if r.id() == "CommonFlow"
    ));
    assert!(matches!(
        embedded_parent.to_any_ref(),
        AnyEntityRef::EmbeddedWorkflow(r) if r.id() == "SubWorkflow"
    ));
}

#[test]
fn tracked_alias_resolves_to_tracked_entity() {
    fn _check(_: Tracked<TestEntity>) {}
    let _ = |t: TestTrackedEntity| _check(t);
}

#[test]
fn entity_kind_all_variants_distinct() {
    let kinds = [
        EntityKind::Role,
        EntityKind::Hook,
        EntityKind::Team,
        EntityKind::Workflow,
        EntityKind::ReusableWorkflow,
        EntityKind::ArtifactKind,
        EntityKind::Task,
        EntityKind::Relay,
        EntityKind::EmbeddedWorkflow,
    ];
    for (i, a) in kinds.iter().enumerate() {
        for (j, b) in kinds.iter().enumerate() {
            if i != j {
                assert_ne!(a, b);
            }
        }
    }
}

#[test]
fn entity_kind_all_variants_is_copy() {
    let k = EntityKind::Workflow;
    let k2 = k;
    assert_eq!(k, k2);
}

#[test]
fn load_strategy_returns_correct_kind_for_each_entity() {
    assert_eq!(load_strategy(EntityKind::Role).kind(), EntityKind::Role);
    assert_eq!(load_strategy(EntityKind::Hook).kind(), EntityKind::Hook);
    assert_eq!(load_strategy(EntityKind::Team).kind(), EntityKind::Team);
    assert_eq!(
        load_strategy(EntityKind::Workflow).kind(),
        EntityKind::Workflow
    );
    assert_eq!(
        load_strategy(EntityKind::ReusableWorkflow).kind(),
        EntityKind::ReusableWorkflow
    );
    assert_eq!(
        load_strategy(EntityKind::ArtifactKind).kind(),
        EntityKind::ArtifactKind
    );
    assert_eq!(load_strategy(EntityKind::Task).kind(), EntityKind::Task);
    assert_eq!(load_strategy(EntityKind::Relay).kind(), EntityKind::Relay);
    assert_eq!(
        load_strategy(EntityKind::EmbeddedWorkflow).kind(),
        EntityKind::EmbeddedWorkflow
    );
}

#[test]
fn any_entity_ref_kind_id_parent_methods_exist() {
    let _: fn(&AnyEntityRef) -> EntityKind = AnyEntityRef::kind;
    let _: fn(&AnyEntityRef) -> &str = AnyEntityRef::id;
}
