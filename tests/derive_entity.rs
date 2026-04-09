use pari::entity::{Entity, EntityKind, EntityRef, TrackedFor};
use pari::tracked::TrackedField;
use std::sync::Arc;

// Minimal test entity with two domain fields.
// Note: #[entity(...)] is a derive helper — must come AFTER #[derive(Entity)].
#[derive(pari_macros::Entity)]
#[entity(kind = EntityKind::Role, no_dispatch)]
pub struct TestRole {
    pub entity_ref: EntityRef<TestRole>,
    pub name: String,
    pub count: Option<u32>,
}

// --- From conversion ---

#[test]
fn from_plain_initializes_all_fields() {
    let plain = TestRole {
        entity_ref: EntityRef::new("test-role"),
        name: "Eng Lead".to_string(),
        count: Some(3),
    };
    let tracked = TrackedTestRole::from(plain);
    assert_eq!(tracked.name.get(), Some(&"Eng Lead".to_string()));
    assert_eq!(tracked.count.get(), Some(&Some(3)));
}

#[test]
fn from_plain_fields_are_clean() {
    let plain = TestRole { entity_ref: EntityRef::new("r"), name: "X".into(), count: None };
    let tracked = TrackedTestRole::from(plain);
    assert!(!tracked.name.is_dirty());
    assert!(!tracked.count.is_dirty());
}

// --- Dirty operations ---

#[test]
fn has_dirty_fields_false_after_from_conversion() {
    let plain = TestRole { entity_ref: EntityRef::new("r"), name: "N".into(), count: None };
    let tracked = TrackedTestRole::from(plain);
    assert!(!tracked.has_dirty_fields());
}

#[test]
fn dirty_fields_empty_after_from_conversion() {
    let plain = TestRole { entity_ref: EntityRef::new("r"), name: "N".into(), count: None };
    let tracked = TrackedTestRole::from(plain);
    assert!(tracked.dirty_fields().is_empty());
}

#[test]
fn has_dirty_fields_true_after_cow_replacement() {
    let plain = TestRole { entity_ref: EntityRef::new("r"), name: "Old".into(), count: None };
    let mut tracked = TrackedTestRole::from(plain);
    tracked.name = Arc::new(TrackedField::with_value("New".to_string()));
    assert!(tracked.has_dirty_fields());
    assert_eq!(tracked.dirty_fields(), vec!["name"]);
}

#[test]
fn merge_dirty_into_copies_only_dirty_fields() {
    let base = TestRole { entity_ref: EntityRef::new("r"), name: "Old".into(), count: Some(1) };
    let mut target = TrackedTestRole::from(base);

    let source_plain =
        TestRole { entity_ref: EntityRef::new("r"), name: "Old".into(), count: Some(1) };
    let mut source = TrackedTestRole::from(source_plain);
    source.name = Arc::new(TrackedField::with_value("New".to_string()));

    source.merge_dirty_into(&mut target);

    assert_eq!(target.name.get(), Some(&"New".to_string()));
    assert_eq!(target.count.get(), Some(&Some(1)));
}

#[test]
fn reset_dirty_clears_all_dirty_flags() {
    let plain = TestRole { entity_ref: EntityRef::new("r"), name: "Old".into(), count: None };
    let mut tracked = TrackedTestRole::from(plain);
    tracked.name = Arc::new(TrackedField::with_value("New".to_string()));
    assert!(tracked.has_dirty_fields());

    tracked.reset_dirty();

    assert!(!tracked.has_dirty_fields());
    assert_eq!(tracked.dirty_fields(), vec![] as Vec<&str>);
    assert_eq!(tracked.name.get(), Some(&"New".to_string()));
}

// --- Entity trait ---

#[test]
fn entity_kind_is_correct() {
    assert_eq!(<TestRole as Entity>::KIND, EntityKind::Role);
}

// --- TrackedFor companion trait ---

#[test]
fn tracked_for_roundtrip_compiles() {
    fn _check(_: <TrackedTestRole as TrackedFor>::Entity) {}
    let _ = |r: TestRole| _check(r);
}

// --- entity_ref accessor ---

#[test]
fn entity_ref_accessor_returns_ref() {
    let plain = TestRole { entity_ref: EntityRef::new("my-id"), name: "N".into(), count: None };
    let tracked = TrackedTestRole::from(plain);
    assert_eq!(tracked.entity_ref().id(), "my-id");
}

// --- Async accessor ---

#[tokio::test]
async fn accessor_returns_value_when_initialized() {
    let plain =
        TestRole { entity_ref: EntityRef::new("r"), name: "Eng Lead".into(), count: Some(5) };
    let tracked = TrackedTestRole::from(plain);

    let name = tracked.name().await.unwrap();
    assert_eq!(name, "Eng Lead");

    let count = tracked.count().await.unwrap();
    assert_eq!(count.copied(), Some(5));
}

#[tokio::test]
#[should_panic(expected = "field not loaded")]
async fn accessor_returns_error_when_uninitialized() {
    let tracked = TrackedTestRole {
        entity_ref: EntityRef::new("r"),
        name: Arc::new(TrackedField::new()),
        count: Arc::new(TrackedField::new()),
    };
    let _ = tracked.name().await;
}

// --- Async setter ---

#[tokio::test]
async fn setter_replaces_field_value() {
    let plain = TestRole { entity_ref: EntityRef::new("r"), name: "Old".into(), count: None };
    let mut tracked = TrackedTestRole::from(plain);

    tracked.set_name("New".to_string()).await.unwrap();

    assert_eq!(tracked.name.get(), Some(&"New".to_string()));
    assert!(tracked.name.is_dirty());
}

#[tokio::test]
async fn setter_marks_field_dirty() {
    let plain = TestRole { entity_ref: EntityRef::new("r"), name: "X".into(), count: None };
    let mut tracked = TrackedTestRole::from(plain);
    assert!(!tracked.has_dirty_fields());

    tracked.set_name("Y".to_string()).await.unwrap();
    assert!(tracked.has_dirty_fields());
}
