//! Workspace layer — caller-facing async API.
//!
//! [`Workspace`] is the bounded session of entity work over a
//! [`Dispatcher`](crate::store::Dispatcher) into the store layer.
//! [`XViewer<'ws, T>`](XViewer) wraps a tracked entity bound to that
//! session for read-side use; per-entity `XDelegate` types (generated
//! by `#[derive(Entity)]`) handle the mutation lifecycle.
//!
//! Anyone can construct a workspace; multiple workspaces over the same
//! store coexist. The type↔erased conversion at the workspace↔store
//! boundary is handled here — downstream layers see only `AnyEntityRef`
//! and `TrackedEntity`.
//!
//! See `docs/design/layers/workspace.md` for the L3 design.

mod viewer;
#[allow(clippy::module_inception)]
mod workspace;

pub use viewer::XViewer;
pub use workspace::Workspace;
