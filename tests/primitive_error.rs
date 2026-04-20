use pari::error::{primitive::PrimitiveError, ErrorLayer, ErrorLocation, OTelEmit};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use tracing::{
    field::{Field, Visit},
    span::{Attributes, Id, Record},
    Event, Metadata, Subscriber,
};

#[test]
fn primitive_error_context_captures_common_fields() {
    let error = PrimitiveError::MalformedFrontmatter {
        context: PrimitiveError::context("invalid frontmatter"),
        raw_snippet: "---".to_string(),
    };

    assert_eq!(error.error_layer(), ErrorLayer::Primitive);
    assert_eq!(error.error_type(), "malformed_frontmatter");
    assert_eq!(error.message(), "invalid frontmatter");
    assert!(error.location().file.ends_with("tests/primitive_error.rs"));
    assert!(error.location().line > 0);
}

#[test]
fn primitive_error_context_with_location_uses_explicit_location() {
    let location = ErrorLocation {
        file: "domain.md".to_string(),
        line: 12,
        column: 9,
    };

    let error = PrimitiveError::MalformedFrontmatter {
        context: PrimitiveError::context_with_location(location.clone(), "invalid frontmatter"),
        raw_snippet: "---".to_string(),
    };

    assert_eq!(error.location(), &location);
}

#[test]
fn primitive_error_details_include_variant_fields() {
    let error = PrimitiveError::UnknownSchemaField {
        context: PrimitiveError::context("unknown schema field"),
        field: "owner".to_string(),
    };

    let details = error.details();
    assert_eq!(details.len(), 1);
    assert_eq!(details[0].field_name, "field");
    assert!(details[0].value.contains("owner"));
}

#[test]
fn primitive_error_emit_contains_expected_payload() {
    let error = PrimitiveError::UnknownSchemaField {
        context: PrimitiveError::context("unknown schema field"),
        field: "owner".to_string(),
    };
    let captured = Arc::new(Mutex::new(Vec::<CapturedEvent>::new()));
    let subscriber = TestSubscriber::new(Arc::clone(&captured));

    tracing::subscriber::with_default(subscriber, || {
        error.emit();
    });

    let events = captured.lock().unwrap();
    assert_eq!(events.len(), 1);

    let event = &events[0];
    assert_eq!(event.level, "ERROR");
    assert_eq!(event.fields.get("exception.type"), Some(&"unknown_schema_field".to_string()));
    assert_eq!(
        event.fields.get("exception.message"),
        Some(&"unknown schema field".to_string())
    );
    assert_eq!(
        event.fields.get("error.unknown_schema_field.field"),
        Some(&"\"owner\"".to_string())
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
