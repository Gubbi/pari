//! Structural and cross-entity validation schema for [`Team`].

use super::{
    kebab_case_id, non_empty_str, unique_by, x_prefix_keys, AnyCrossEntityRule, AnyStructuralRule,
    RuleViolation, ValidationSchema,
};
use crate::entities::team::{Team, TeamMember, TrackedTeam};

fn opt_non_empty_str(value: &Option<String>) -> Vec<RuleViolation> {
    match value {
        None => vec![],
        Some(s) => non_empty_str(s),
    }
}

fn unique_member_handles(value: &Option<Vec<TeamMember>>) -> Vec<RuleViolation> {
    match value {
        None => vec![],
        Some(members) => unique_by(members, |m| m.handle.clone()),
    }
}

pub fn team_validation_schema() -> ValidationSchema<Team> {
    let mut structural: std::collections::HashMap<&'static str, Vec<AnyStructuralRule<Team>>> =
        std::collections::HashMap::new();

    structural.insert(
        "entity_ref",
        vec![Box::new(|e: &TrackedTeam| kebab_case_id(&e.entity_ref))],
    );

    structural.insert(
        "name",
        vec![Box::new(|e: &TrackedTeam| {
            e.name.get().map(|v| non_empty_str(v)).unwrap_or_default()
        })],
    );

    structural.insert(
        "description",
        vec![Box::new(|e: &TrackedTeam| {
            e.description
                .get()
                .map(|v| opt_non_empty_str(v))
                .unwrap_or_default()
        })],
    );

    structural.insert(
        "members",
        vec![Box::new(|e: &TrackedTeam| {
            e.members
                .get()
                .map(|v| unique_member_handles(v))
                .unwrap_or_default()
        })],
    );

    structural.insert(
        "extensions",
        vec![Box::new(|e: &TrackedTeam| {
            e.extensions
                .get()
                .map(|v| x_prefix_keys(v))
                .unwrap_or_default()
        })],
    );

    let mut cross_entity: std::collections::HashMap<&'static str, Vec<AnyCrossEntityRule<Team>>> =
        std::collections::HashMap::new();

    // Stubs: member_roles_exist, include_teams_exist, no_include_cycle, no_import_cycle
    cross_entity.insert(
        "members",
        vec![Box::new(|_e: &TrackedTeam| Box::pin(async { vec![] }))],
    );

    cross_entity.insert(
        "include",
        vec![
            Box::new(|_e: &TrackedTeam| Box::pin(async { vec![] })),
            Box::new(|_e: &TrackedTeam| Box::pin(async { vec![] })),
        ],
    );

    cross_entity.insert(
        "import",
        vec![Box::new(|_e: &TrackedTeam| Box::pin(async { vec![] }))],
    );

    ValidationSchema {
        structural,
        semantic: std::collections::HashMap::new(),
        cross_entity,
    }
}
