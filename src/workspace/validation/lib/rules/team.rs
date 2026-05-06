use super::{
    super::schema::{AnyCrossEntityRule, AnyStructuralRule, ValidationSchema},
    structural::{
        primitives::{
            kebab_case_id, non_empty_list, non_empty_str, opt_non_empty_str, x_prefix_keys,
        },
        team::{include_structural, members_structural},
    },
};
use crate::{
    entity::entities::team::{Team, TrackedTeam},
    validation::lib::rules::cross_entity::team::{no_import_cycle, no_include_cycle},
    workspace::XViewer,
};

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
                .map(|v| members_structural(v))
                .unwrap_or_default()
        })],
    );
    structural.insert(
        "include",
        vec![Box::new(|e: &TrackedTeam| {
            e.include
                .get()
                .map(|v| include_structural(v))
                .unwrap_or_default()
        })],
    );
    structural.insert(
        "import",
        vec![Box::new(|e: &TrackedTeam| {
            e.import
                .get()
                .map(|opt_list| {
                    opt_list
                        .as_ref()
                        .map(|l| non_empty_list(l.as_slice()))
                        .unwrap_or_default()
                })
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
    cross_entity.insert("members", vec![crate::ref_check_rule!(Team, members)]);
    cross_entity.insert(
        "include",
        vec![
            crate::ref_check_rule!(Team, include),
            Box::new(|viewer: &XViewer<'_, Team>| {
                let self_ref = viewer.tracked().entity_ref.clone();
                let include = viewer
                    .tracked()
                    .include
                    .get()
                    .and_then(|v| v.as_ref())
                    .cloned()
                    .unwrap_or_default();
                let workspace = viewer.workspace();
                Box::pin(async move { no_include_cycle(workspace, self_ref, include).await })
            }),
        ],
    );
    cross_entity.insert(
        "import",
        vec![
            crate::ref_check_rule!(Team, import),
            Box::new(|viewer: &XViewer<'_, Team>| {
                let self_ref = viewer.tracked().entity_ref.clone();
                let import = viewer
                    .tracked()
                    .import
                    .get()
                    .and_then(|v| v.as_ref())
                    .cloned()
                    .unwrap_or_default();
                let workspace = viewer.workspace();
                Box::pin(async move { no_import_cycle(workspace, self_ref, import).await })
            }),
        ],
    );

    ValidationSchema {
        structural,
        semantic: std::collections::HashMap::new(),
        cross_entity,
    }
}
