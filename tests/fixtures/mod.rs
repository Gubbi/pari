//! Per-entity fixtures.
//!
//! All fixture files are currently disabled — they construct
//! `TrackedEntity` via `From<PlainX>` impls that no longer exist; the
//! workspace's `insert<T: Entity + Serialize>(plain)` takes the plain
//! entity directly and serializes at the wire boundary. Fixtures
//! return plain entity types after Thread T's test rewrite re-enables
//! these modules.

#[cfg(any())]
#[path = "role.rs"]
pub mod role;

#[cfg(any())]
#[path = "team.rs"]
pub mod team;

#[cfg(any())]
#[path = "artifact_kind.rs"]
pub mod artifact_kind;

#[cfg(any())]
#[path = "task.rs"]
pub mod task;

#[cfg(any())]
#[path = "workflow.rs"]
pub mod workflow;

#[cfg(any())]
#[path = "reusable_workflow.rs"]
pub mod reusable_workflow;

#[cfg(any())]
#[path = "relay.rs"]
pub mod relay;

#[cfg(any())]
#[path = "hook.rs"]
pub mod hook;

#[cfg(any())]
#[path = "embedded_workflow.rs"]
pub mod embedded_workflow;
