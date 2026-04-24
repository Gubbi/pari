//! The one-and-only `entity_registry!` invocation.
//!
//! Adding a new entity type is a one-line edit in this file plus a
//! `#[derive(Entity)]` on the plain struct. The registry generates
//! [`EntityKind`](crate::entity::EntityKind),
//! [`AnyEntityRef`](crate::entity::AnyEntityRef), the type-erased
//! [`TrackedEntity`](crate::entity::TrackedEntity) wrapper, and the
//! cross-layer dispatch impls (`store` persist/checkout helpers,
//! `substrate` load strategy and schema trait, `validation` dispatch). See
//! the L3 design doc at `docs/design/layers/entities.md` for the full map
//! of what each layer consumes from this macro.

use super::{EntityRef, NoParent, WorkflowParent};
use crate::entity::entities::{
    artifact_kind::{ArtifactKind, TrackedArtifactKind},
    hook::{Hook, TrackedHook},
    relay::{Relay, TrackedRelay},
    role::{Role, TrackedRole},
    task::{Task, TrackedTask},
    team::{Team, TrackedTeam},
    workflow::{
        EmbeddedWorkflow, ReusableWorkflow, TrackedEmbeddedWorkflow, TrackedReusableWorkflow,
        TrackedWorkflow, Workflow,
    },
};

pari_macros::entity_registry! {
    Role             => NoParent,
    Hook             => NoParent,
    Team             => NoParent,
    Workflow         => NoParent,
    ReusableWorkflow => NoParent,
    ArtifactKind     => NoParent,
    Task             => WorkflowParent,
    Relay            => WorkflowParent,
    EmbeddedWorkflow => WorkflowParent,
}
