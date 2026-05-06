//! Functional integration tests, one file per user job.
//!
//! All test files are currently disabled — they reference the
//! pre-refactor `EntityClient` / `pari::with` shape and need a
//! coordinated rewrite against the new `Workspace` / `XViewer` /
//! `XEditor` API. That rewrite is its own thread (Test Refactor) and
//! will re-enable the modules below.

#![allow(dead_code)]

#[cfg(any())]
#[path = "author_role.rs"]
pub mod author_role;

#[cfg(any())]
#[path = "author_team.rs"]
pub mod author_team;

#[cfg(any())]
#[path = "author_workflow.rs"]
pub mod author_workflow;

#[cfg(any())]
#[path = "modify_persisted_entity.rs"]
pub mod modify_persisted_entity;

#[cfg(any())]
#[path = "author_workflow_with_intercepts.rs"]
pub mod author_workflow_with_intercepts;

#[cfg(any())]
#[path = "author_embedded_workflow.rs"]
pub mod author_embedded_workflow;

#[cfg(any())]
#[path = "author_reusable_workflow.rs"]
pub mod author_reusable_workflow;

#[cfg(any())]
#[path = "author_relay.rs"]
pub mod author_relay;

#[cfg(any())]
#[path = "validation_failures.rs"]
pub mod validation_failures;

#[cfg(any())]
#[path = "lifecycle_failures.rs"]
pub mod lifecycle_failures;

#[cfg(any())]
#[path = "validation_timing.rs"]
pub mod validation_timing;

#[cfg(any())]
#[path = "abandon_in_progress_edit.rs"]
pub mod abandon_in_progress_edit;

#[cfg(any())]
#[path = "rollback_staged_change.rs"]
pub mod rollback_staged_change;

#[cfg(any())]
#[path = "refresh_entity_from_substrate.rs"]
pub mod refresh_entity_from_substrate;

#[cfg(any())]
#[path = "external_corruption.rs"]
pub mod external_corruption;
