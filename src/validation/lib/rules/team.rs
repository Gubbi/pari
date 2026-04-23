use super::{
    super::schema::{AnyCrossEntityRule, AnyStructuralRule, ValidationSchema},
    structural::{
        primitives::{kebab_case_id, non_empty_str, opt_non_empty_str, x_prefix_keys},
        team::unique_member_handles,
    },
};
use crate::entity::entities::team::{Team, TrackedTeam};

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
    cross_entity.insert(
        "members",
        vec![crate::ref_check_rule!(TrackedTeam, members)],
    );
    cross_entity.insert(
        "include",
        vec![crate::ref_check_rule!(TrackedTeam, include)],
    );
    cross_entity.insert("import", vec![crate::ref_check_rule!(TrackedTeam, import)]);

    ValidationSchema {
        structural,
        semantic: std::collections::HashMap::new(),
        cross_entity,
    }
}
