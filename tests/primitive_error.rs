use pari::error::{ErrorLayer, OTelEmit};
use pari_macros::primitive_error;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use tracing::{
    field::{Field, Visit},
    span::{Attributes, Id, Record},
    Event, Metadata, Subscriber,
};

#[primitive_error]
pub struct MalformedFrontmatter {
    pub line: usize,
    pub raw_snippet: String,
}

#[primitive_error(error_type = "rename_failed")]
pub struct RenameFailure {
    pub from: String,
    pub to: String,
}

#[test]
fn primitive_error_new_captures_common_fields() {
    let error = MalformedFrontmatter::new("invalid frontmatter", 7, "---".to_string());

    assert_eq!(error.error_layer(), ErrorLayer::Primitive);
    assert_eq!(error.error_type(), "malformed_frontmatter");
    assert_eq!(error.message(), "invalid frontmatter");
    assert!(error.location().file.ends_with("tests/primitive_error.rs"));
    assert!(error.location().line > 0);
}

#[test]
fn primitive_error_override_error_type() {
    let error = RenameFailure::new("rename failed", "a".to_string(), "b".to_string());

    assert_eq!(error.error_type(), "rename_failed");
}

#[test]
fn primitive_error_new_with_location_uses_explicit_location() {
    let location = pari::error::ErrorLocation {
        file: "domain.md".to_string(),
        line: 12,
        column: 9,
    };

    let error = MalformedFrontmatter::new_with_location(
        location.clone(),
        "invalid frontmatter",
        3,
        "---".to_string(),
    );

    assert_eq!(error.location(), &location);
}

#[test]
fn primitive_error_details_include_struct_fields() {
    let error = RenameFailure::new("rename failed", "old.md".to_string(), "new.md".to_string());

    let details = error.details();
    assert_eq!(details.len(), 2);
    assert_eq!(details[0].field_name, "from");
    assert!(details[0].value.contains("old.md"));
    assert_eq!(details[1].field_name, "to");
    assert!(details[1].value.contains("new.md"));
}

#[test]
fn primitive_error_emit_contains_expected_payload() {
    let error = RenameFailure::new("rename failed", "old.md".to_string(), "new.md".to_string());
    let captured = Arc::new(Mutex::new(Vec::<CapturedEvent>::new()));
    let subscriber = TestSubscriber::new(Arc::clone(&captured));

    tracing::subscriber::with_default(subscriber, || {
        error.emit();
    });

    let events = captured.lock().unwrap();
    assert_eq!(events.len(), 1);

    let event = &events[0];
    assert_eq!(event.level, "ERROR");
    assert_eq!(event.fields.get("exception.type"), Some(&"rename_failed".to_string()));
    assert_eq!(
        event.fields.get("exception.message"),
        Some(&"rename failed".to_string())
    );
    assert_eq!(event.fields.get("code.file.path").is_some(), true);
    assert_eq!(event.fields.get("code.line.number").is_some(), true);
    assert_eq!(event.fields.get("code.column.number").is_some(), true);
    assert_eq!(
        event.fields.get("error.rename_failed.from"),
        Some(&"\"old.md\"".to_string())
    );
    assert_eq!(
        event.fields.get("error.rename_failed.to"),
        Some(&"\"new.md\"".to_string())
    );
}

#[derive(Debug, Clone)]
struct CapturedEvent {
    level: String,
    fields: BTreeMap<String, String>,
}

struct FieldRecorder {
    fields: BTreeMap<String, String>,
}

impl FieldRecorder {
    fn new() -> Self {
        Self {
            fields: BTreeMap::new(),
        }
    }

    fn record_value(&mut self, field: &Field, value: String) {
        self.fields.insert(field.name().to_string(), value);
    }
}

impl Visit for FieldRecorder {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.record_value(field, format!("{value:?}"));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.record_value(field, value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.record_value(field, value.to_string());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.record_value(field, value.to_string());
    }
}

struct TestSubscriber {
    captured: Arc<Mutex<Vec<CapturedEvent>>>,
}

impl TestSubscriber {
    fn new(captured: Arc<Mutex<Vec<CapturedEvent>>>) -> Self {
        Self { captured }
    }
}

impl Subscriber for TestSubscriber {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut visitor = FieldRecorder::new();
        event.record(&mut visitor);
        self.captured.lock().unwrap().push(CapturedEvent {
            level: event.metadata().level().to_string(),
            fields: visitor.fields,
        });
    }

    fn enter(&self, _: &Id) {}

    fn exit(&self, _: &Id) {}
}
