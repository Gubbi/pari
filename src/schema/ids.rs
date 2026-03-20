//! Newtype wrappers for entity ID fields.
//!
//! Each newtype wraps `String`, enforces format via `JsonSchema`, and serialises
//! transparently (as a plain string in JSON/YAML). Validation of the format
//! constraint at runtime remains in `validation.rs` helpers (`is_kebab_case`,
//! `is_camel_case`).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// --- Macro ---

/// Generates a string newtype ID type with Serialize/Deserialize (transparent),
/// a hand-rolled `JsonSchema` impl that includes the regex pattern, and
/// convenience trait impls (Deref, Display, `AsRef`, From, `PartialEq` helpers).
macro_rules! define_id {
    ($name:ident, $pattern:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl JsonSchema for $name {
            fn schema_name() -> String {
                stringify!($name).to_string()
            }

            fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
                use schemars::schema::{InstanceType, Schema, SchemaObject, SingleOrVec};
                let mut obj = SchemaObject::default();
                obj.instance_type = Some(SingleOrVec::Single(Box::new(InstanceType::String)));
                obj.string().pattern = Some($pattern.to_string());
                Schema::Object(obj)
            }
        }

        impl std::ops::Deref for $name {
            type Target = str;
            fn deref(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                $name(s.to_string())
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                $name(s)
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.0 == *other
            }
        }

        impl PartialEq<str> for $name {
            fn eq(&self, other: &str) -> bool {
                self.0 == other
            }
        }

        impl PartialEq<String> for $name {
            fn eq(&self, other: &String) -> bool {
                &self.0 == other
            }
        }
    };
}

// --- Kebab-case IDs ---

define_id!(RoleId, r"^[a-z][a-z0-9-]*$");
define_id!(TeamId, r"^[a-z][a-z0-9-]*$");

// --- CamelCase IDs ---

define_id!(HookId, r"^[A-Z][A-Za-z0-9]*$");
define_id!(WorkflowId, r"^[A-Z][A-Za-z0-9]*$");
define_id!(TaskId, r"^[A-Z][A-Za-z0-9]*$");
define_id!(RelayId, r"^[A-Z][A-Za-z0-9]*$");

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    // --- 2.1 / 3.1: Serialisation as plain string ---

    #[test]
    fn role_id_serializes_as_plain_string() {
        let id = RoleId::from("eng-lead");
        assert_eq!(serde_json::to_string(&id).unwrap(), "\"eng-lead\"");
    }

    #[test]
    fn role_id_deserializes_from_plain_string() {
        let id: RoleId = serde_json::from_str("\"eng-lead\"").unwrap();
        assert_eq!(&*id, "eng-lead");
    }

    #[test]
    fn team_id_round_trips() {
        let id = TeamId::from("platform-team");
        let json = serde_json::to_string(&id).unwrap();
        let back: TeamId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, "platform-team");
    }

    #[test]
    fn hook_id_round_trips() {
        let id = HookId::from("NotifySlack");
        let json = serde_json::to_string(&id).unwrap();
        let back: HookId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, "NotifySlack");
    }

    #[test]
    fn workflow_id_round_trips() {
        let id = WorkflowId::from("InitiativeWorkflow");
        let json = serde_json::to_string(&id).unwrap();
        let back: WorkflowId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, "InitiativeWorkflow");
    }

    #[test]
    fn task_id_round_trips() {
        let id = TaskId::from("Proposal");
        let json = serde_json::to_string(&id).unwrap();
        let back: TaskId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, "Proposal");
    }

    #[test]
    fn relay_id_round_trips() {
        let id = RelayId::from("LegalSignoff");
        let json = serde_json::to_string(&id).unwrap();
        let back: RelayId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, "LegalSignoff");
    }

    // --- 2.1 / 3.1: JSON schema contains regex pattern ---

    #[test]
    fn role_id_schema_contains_kebab_pattern() {
        use schemars::schema_for;
        let schema = schema_for!(RoleId);
        let json = serde_json::to_value(&schema).unwrap();
        let s = json.to_string();
        assert!(
            s.contains(r"^[a-z][a-z0-9-]*$"),
            "expected kebab regex in schema, got: {}",
            json
        );
    }

    #[test]
    fn team_id_schema_contains_kebab_pattern() {
        use schemars::schema_for;
        let schema = schema_for!(TeamId);
        let json = serde_json::to_value(&schema).unwrap();
        let s = json.to_string();
        assert!(
            s.contains(r"^[a-z][a-z0-9-]*$"),
            "expected kebab regex in schema, got: {}",
            json
        );
    }

    #[test]
    fn hook_id_schema_contains_camel_pattern() {
        use schemars::schema_for;
        let schema = schema_for!(HookId);
        let json = serde_json::to_value(&schema).unwrap();
        let s = json.to_string();
        assert!(
            s.contains(r"^[A-Z][A-Za-z0-9]*$"),
            "expected CamelCase regex in schema, got: {}",
            json
        );
    }

    #[test]
    fn workflow_id_schema_contains_camel_pattern() {
        use schemars::schema_for;
        let schema = schema_for!(WorkflowId);
        let json = serde_json::to_value(&schema).unwrap();
        let s = json.to_string();
        assert!(
            s.contains(r"^[A-Z][A-Za-z0-9]*$"),
            "expected CamelCase regex in schema, got: {}",
            json
        );
    }

    #[test]
    fn task_id_schema_contains_camel_pattern() {
        use schemars::schema_for;
        let schema = schema_for!(TaskId);
        let json = serde_json::to_value(&schema).unwrap();
        let s = json.to_string();
        assert!(
            s.contains(r"^[A-Z][A-Za-z0-9]*$"),
            "expected CamelCase regex in schema, got: {}",
            json
        );
    }

    #[test]
    fn relay_id_schema_contains_camel_pattern() {
        use schemars::schema_for;
        let schema = schema_for!(RelayId);
        let json = serde_json::to_value(&schema).unwrap();
        let s = json.to_string();
        assert!(
            s.contains(r"^[A-Z][A-Za-z0-9]*$"),
            "expected CamelCase regex in schema, got: {}",
            json
        );
    }

    // --- Deref and Display ---

    #[test]
    fn role_id_derefs_to_str() {
        let id = RoleId::from("eng-lead");
        assert_eq!(&*id, "eng-lead");
    }

    #[test]
    fn role_id_displays_as_inner_string() {
        let id = RoleId::from("eng-lead");
        assert_eq!(format!("{}", id), "eng-lead");
    }

    #[test]
    fn role_id_from_string() {
        let id = RoleId::from("eng-lead".to_string());
        assert_eq!(id, "eng-lead");
    }
}
