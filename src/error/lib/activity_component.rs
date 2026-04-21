/// Identifies the subsystem component at a layer boundary where an activity error originated.
///
/// Each variant corresponds to an orchestration-visible component — the granularity that the
/// orchestrating layer sees when a pure lib component returns a `PrimitiveError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityComponent {
    // substrate/repo
    RepoSubstrate,
    RepoCodec,
    RepoExecutor,
    RepoLocationResolver,
    // substrate/in_memory
    InMemoryCodec,
    InMemoryExecutor,
    // store
    EntityServer,
    // validation
    ValidationRunner,
}

impl std::fmt::Display for ActivityComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ActivityComponent::RepoSubstrate => "repo_substrate",
            ActivityComponent::RepoCodec => "repo_codec",
            ActivityComponent::RepoExecutor => "repo_executor",
            ActivityComponent::RepoLocationResolver => "repo_location_resolver",
            ActivityComponent::InMemoryCodec => "in_memory_codec",
            ActivityComponent::InMemoryExecutor => "in_memory_executor",
            ActivityComponent::EntityServer => "entity_server",
            ActivityComponent::ValidationRunner => "validation_runner",
        };
        f.write_str(s)
    }
}
