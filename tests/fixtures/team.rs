//! Canonical [`Team`] sample data for tests.
//!
//! Each function returns a fully-formed [`TrackedEntity`] with a name
//! that reads at the call site. Variants compose internally; callers
//! see only the named result.

use pari::{
    entities::{
        role::Role,
        team::{Team, TeamMember, TrackedTeam},
    },
    entity::{EntityRef, TrackedEntity},
};

/// Bare team with required fields populated; no roster, no composition.
pub fn a_minimal_team(id: &str) -> TrackedEntity {
    team(
        id,
        "Minimal Team",
        Some("A team for tests."),
        None,
        None,
        None,
    )
}

/// Team whose roster references existing roles.
///
/// `members` is a list of `(handle, role_id)` pairs; each `role_id` is
/// resolved to a top-level `EntityRef<Role>`.
pub fn a_team_with_members(id: &str, members: &[(&str, &str)]) -> TrackedEntity {
    let members = members
        .iter()
        .map(|(handle, role_id)| TeamMember {
            handle: (*handle).to_string(),
            role: EntityRef::new(*role_id),
        })
        .collect();
    team(
        id,
        "Engineering Team",
        Some("A team with a roster."),
        Some(members),
        None,
        None,
    )
}

/// Team that composes other teams via `include` and `import`.
///
/// `includes` maps `(team_id, role_id)` so the included team's role
/// resolves to a concrete handle. `imports` is a flat list of team ids.
pub fn a_team_with_composition(
    id: &str,
    includes: &[(&str, &str)],
    imports: &[&str],
) -> TrackedEntity {
    let include = includes
        .iter()
        .map(|(team_id, role_id)| {
            (
                EntityRef::<Team>::new(*team_id),
                EntityRef::<Role>::new(*role_id),
            )
        })
        .collect::<Vec<_>>();
    let import = imports
        .iter()
        .map(|team_id| EntityRef::<Team>::new(*team_id))
        .collect();
    team(
        id,
        "Composed Team",
        Some("A team that composes others."),
        None,
        Some(include),
        Some(import),
    )
}

fn team(
    id: &str,
    name: &str,
    description: Option<&str>,
    members: Option<Vec<TeamMember>>,
    include: Option<Vec<(EntityRef<Team>, EntityRef<Role>)>>,
    import: Option<Vec<EntityRef<Team>>>,
) -> TrackedEntity {
    TrackedEntity::from_team(TrackedTeam::from(Team {
        entity_ref: EntityRef::new(id),
        name: name.to_string(),
        description: description.map(str::to_string),
        members,
        include,
        import,
        extensions: Default::default(),
    }))
}
