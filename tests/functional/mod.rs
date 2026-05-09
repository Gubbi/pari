//! Functional integration tests, one file per user job.
//!
//! All test files are currently disabled — they reference the
//! pre-refactor `EntityClient` / `pari::with` shape and need a
//! coordinated rewrite against the new `Workspace` / `XViewer` /
//! `XEditor` API. That rewrite is its own thread (Test Refactor) and
//! will re-enable the modules below.

#![allow(dead_code)]

#[path = "author_role.rs"]
pub mod author_role;

#[path = "author_team.rs"]
pub mod author_team;

#[path = "author_workflow.rs"]
pub mod author_workflow;

#[path = "modify_persisted_entity.rs"]
pub mod modify_persisted_entity;

#[path = "author_workflow_with_intercepts.rs"]
pub mod author_workflow_with_intercepts;

#[path = "author_embedded_workflow.rs"]
pub mod author_embedded_workflow;

#[path = "author_reusable_workflow.rs"]
pub mod author_reusable_workflow;

#[path = "author_relay.rs"]
pub mod author_relay;

#[path = "validation_failures.rs"]
pub mod validation_failures;

#[path = "lifecycle_failures.rs"]
pub mod lifecycle_failures;

#[path = "validation_timing.rs"]
pub mod validation_timing;

#[path = "abandon_in_progress_edit.rs"]
pub mod abandon_in_progress_edit;

#[path = "rollback_staged_change.rs"]
pub mod rollback_staged_change;

#[path = "refresh_entity_from_substrate.rs"]
pub mod refresh_entity_from_substrate;

#[path = "external_corruption.rs"]
pub mod external_corruption;

#[path = "import_from_json.rs"]
pub mod import_from_json;

#[path = "substrate_load_boundary.rs"]
pub mod substrate_load_boundary;

#[path = "store_unavailable.rs"]
pub mod store_unavailable;

#[path = "sparse_substrate_response.rs"]
pub mod sparse_substrate_response;
