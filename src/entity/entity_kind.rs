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
