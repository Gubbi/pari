//! [`Team`] entity — a named group of role handles with optional include/import composition.

use std::collections::{HashMap, HashSet, VecDeque};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::schema::{
    ids::TeamId,
    store::EntityStore,
    types::Extensions,
    validation::{is_kebab_case, is_valid_handle, validate_extensions, ValidationError},
};

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct TeamMember {
    #[schemars(regex(pattern = r"^@[a-z0-9._-]+$"))]
    pub handle: String,
    pub role: String,
}

#[derive(Serialize, Deserialize, JsonSchema, pari_macros::Tracked)]
#[schemars(deny_unknown_fields)]
pub struct Team {
    pub id: TeamId,
    pub name: String,
    pub description: Option<String>,
    pub members: Option<Vec<TeamMember>>,
    /// Map of `team_id` → `role_id`: all members of the referenced team are assigned this role.
    pub include: Option<HashMap<String, String>>,
    /// List of `team_ids` whose members carry their own roles from the source team.
    pub import: Option<Vec<String>>,
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl Team {
    /// Returns an iterator over all team ids directly referenced by this team
    /// via `include` (map keys) and `import` (list entries).
    pub fn get_refs(&self) -> impl Iterator<Item = &str> {
        self.include
            .iter()
            .flat_map(|m| m.keys().map(String::as_str))
            .chain(
                self.import
                    .iter()
                    .flat_map(|v| v.iter().map(String::as_str)),
            )
    }
}

impl TrackedTeam {
    /// Delegates to the underlying `include`/`import` fields via Deref.
    pub fn get_refs(&self) -> impl Iterator<Item = &str> {
        self.include
            .iter()
            .flat_map(|m| m.keys().map(String::as_str))
            .chain(
                self.import
                    .iter()
                    .flat_map(|v| v.iter().map(String::as_str)),
            )
    }
}

pub fn validate(team: &Team, ctx: &EntityStore) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    errors.extend(validate_structural(team));
    errors.extend(validate_cross_entity(team, ctx));
    errors.extend(validate_no_cycle(team, ctx));

    errors
}

fn validate_structural(team: &Team) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if !is_kebab_case(&team.id) {
        errors.push(ValidationError {
            path: "id".to_string(),
            message: format!("id must be kebab-case, got '{}'", team.id),
        });
    }

    errors.extend(validate_extensions(&team.extensions, "extensions"));

    // Handle format and uniqueness
    if let Some(members) = &team.members {
        let mut seen_handles: HashSet<&str> = HashSet::new();
        for (i, member) in members.iter().enumerate() {
            if !is_valid_handle(&member.handle) {
                errors.push(ValidationError {
                    path: format!("members[{i}].handle"),
                    message: format!("handle '{}' must match @[a-z0-9._-]+", member.handle),
                });
            }
            if !seen_handles.insert(member.handle.as_str()) {
                errors.push(ValidationError {
                    path: format!("members[{i}].handle"),
                    message: format!("duplicate handle '{}'", member.handle),
                });
            }
        }
    }

    errors
}

fn validate_cross_entity(team: &Team, ctx: &EntityStore) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // include: team_id keys and role_id values must exist
    if let Some(include) = &team.include {
        for (team_id, role_id) in include {
            if !ctx.has_team(team_id) {
                errors.push(ValidationError {
                    path: format!("include.{team_id}"),
                    message: format!("unknown team '{team_id}'"),
                });
            }
            if !ctx.has_role(role_id) {
                errors.push(ValidationError {
                    path: format!("include.{team_id}"),
                    message: format!("unknown role '{role_id}'"),
                });
            }
        }
    }

    // import: team_ids must exist
    if let Some(import) = &team.import {
        for (i, team_id) in import.iter().enumerate() {
            if !ctx.has_team(team_id) {
                errors.push(ValidationError {
                    path: format!("import[{i}]"),
                    message: format!("unknown team '{team_id}'"),
                });
            }
        }
    }

    // member roles must exist
    if let Some(members) = &team.members {
        for (i, member) in members.iter().enumerate() {
            if !ctx.has_role(&member.role) {
                errors.push(ValidationError {
                    path: format!("members[{i}].role"),
                    message: format!("unknown role '{}'", member.role),
                });
            }
        }
    }

    errors
}

/// Validates that team does not form a circular chain through include/import.
/// Uses BFS over team references via `RepoContext`. Since no existing team in
/// `RepoContext` can reference the incoming team, a cycle can only occur if the
/// incoming team's id appears in the reachable set starting from its own references.
fn validate_no_cycle(team: &Team, ctx: &EntityStore) -> Vec<ValidationError> {
    let mut to_visit: VecDeque<String> = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();

    // Seed with the incoming team's direct references
    if let Some(include) = &team.include {
        for team_id in include.keys() {
            to_visit.push_back(team_id.clone());
        }
    }
    if let Some(import) = &team.import {
        for team_id in import {
            to_visit.push_back(team_id.clone());
        }
    }

    while let Some(current) = to_visit.pop_front() {
        if team.id == current {
            return vec![ValidationError {
                path: "include/import".to_string(),
                message: format!("circular reference: team '{}' forms a cycle", team.id),
            }];
        }
        if visited.insert(current.clone()) {
            if let Some(current_team) = ctx.get_team(&current) {
                for r in current_team.get_refs() {
                    to_visit.push_back(r.to_string());
                }
            }
        }
    }

    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{store::EntityStore, types::Extensions};

    fn base_ctx() -> EntityStore {
        let mut ctx = EntityStore::new();
        ctx.insert_role(crate::schema::entities::role::Role {
            id: "eng-lead".into(),
            name: "Engineering Lead".to_string(),
            purpose: "test".to_string(),
            traits: None,
            extensions: Extensions::default(),
        });
        ctx.insert_role(crate::schema::entities::role::Role {
            id: "pm".into(),
            name: "Product Manager".to_string(),
            purpose: "test".to_string(),
            traits: None,
            extensions: Extensions::default(),
        });
        ctx.insert_team(Team {
            id: "backend-team".into(),
            name: "Backend Team".to_string(),
            description: None,
            members: None,
            include: None,
            import: None,
            extensions: Extensions::default(),
        });
        ctx.insert_team(Team {
            id: "qa-team".into(),
            name: "QA Team".to_string(),
            description: None,
            members: None,
            include: None,
            import: None,
            extensions: Extensions::default(),
        });
        ctx
    }

    fn valid_team() -> Team {
        Team {
            id: "platform-team".into(),
            name: "Platform Team".to_string(),
            description: None,
            members: None,
            include: None,
            import: None,
            extensions: Extensions::default(),
        }
    }

    // --- 10.1: Team structural validator tests ---

    #[test]
    fn valid_team_passes_structural_validation() {
        let errors = validate_structural(&valid_team());
        assert!(errors.is_empty());
    }

    #[test]
    fn team_camel_case_id_fails() {
        let team = Team {
            id: "PlatformTeam".into(),
            ..valid_team()
        };
        let errors = validate_structural(&team);
        assert!(!errors.is_empty());
        assert_eq!(errors[0].path, "id");
    }

    #[test]
    fn team_member_valid_handle_with_dot() {
        let team = Team {
            members: Some(vec![TeamMember {
                handle: "@alice.smith".to_string(),
                role: "eng-lead".to_string(),
            }]),
            ..valid_team()
        };
        let errors = validate_structural(&team);
        assert!(errors.is_empty());
    }

    #[test]
    fn team_member_handle_without_at_fails() {
        let team = Team {
            members: Some(vec![TeamMember {
                handle: "alice".to_string(),
                role: "eng-lead".to_string(),
            }]),
            ..valid_team()
        };
        let errors = validate_structural(&team);
        assert!(!errors.is_empty());
        assert!(errors[0].path.contains("handle"));
    }

    #[test]
    fn team_member_handle_uppercase_fails() {
        let team = Team {
            members: Some(vec![TeamMember {
                handle: "@Alice".to_string(),
                role: "eng-lead".to_string(),
            }]),
            ..valid_team()
        };
        let errors = validate_structural(&team);
        assert!(!errors.is_empty());
    }

    #[test]
    fn team_duplicate_handle_fails() {
        let team = Team {
            members: Some(vec![
                TeamMember {
                    handle: "@alice".to_string(),
                    role: "eng-lead".to_string(),
                },
                TeamMember {
                    handle: "@alice".to_string(),
                    role: "pm".to_string(),
                },
            ]),
            ..valid_team()
        };
        let errors = validate_structural(&team);
        assert!(!errors.is_empty());
        assert!(errors
            .iter()
            .any(|e| e.message.contains("duplicate handle")));
    }

    #[test]
    fn team_unique_handles_passes() {
        let team = Team {
            members: Some(vec![
                TeamMember {
                    handle: "@alice".to_string(),
                    role: "eng-lead".to_string(),
                },
                TeamMember {
                    handle: "@bob".to_string(),
                    role: "pm".to_string(),
                },
            ]),
            ..valid_team()
        };
        let errors = validate_structural(&team);
        assert!(errors.is_empty());
    }

    // --- 10.3: Team cross-entity validator tests ---

    #[test]
    fn team_valid_include_and_import() {
        let ctx = base_ctx();
        let mut include = HashMap::new();
        include.insert("backend-team".to_string(), "eng-lead".to_string());
        let team = Team {
            include: Some(include),
            import: Some(vec!["qa-team".to_string()]),
            ..valid_team()
        };
        let errors = validate_cross_entity(&team, &ctx);
        assert!(errors.is_empty());
    }

    #[test]
    fn team_unknown_team_in_include_fails() {
        let ctx = base_ctx();
        let mut include = HashMap::new();
        include.insert("unknown-team".to_string(), "eng-lead".to_string());
        let team = Team {
            include: Some(include),
            ..valid_team()
        };
        let errors = validate_cross_entity(&team, &ctx);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.message.contains("unknown team")));
    }

    #[test]
    fn team_unknown_role_in_include_fails() {
        let ctx = base_ctx();
        let mut include = HashMap::new();
        include.insert("backend-team".to_string(), "unknown-role".to_string());
        let team = Team {
            include: Some(include),
            ..valid_team()
        };
        let errors = validate_cross_entity(&team, &ctx);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.message.contains("unknown role")));
    }

    #[test]
    fn team_unknown_team_in_import_fails() {
        let ctx = base_ctx();
        let team = Team {
            import: Some(vec!["ghost-team".to_string()]),
            ..valid_team()
        };
        let errors = validate_cross_entity(&team, &ctx);
        assert!(!errors.is_empty());
        assert!(errors[0].path.contains("import[0]"));
    }

    #[test]
    fn team_member_unknown_role_fails() {
        let ctx = base_ctx();
        let team = Team {
            members: Some(vec![TeamMember {
                handle: "@alice".to_string(),
                role: "unknown-role".to_string(),
            }]),
            ..valid_team()
        };
        let errors = validate_cross_entity(&team, &ctx);
        assert!(!errors.is_empty());
        assert!(errors[0].path.contains("members[0].role"));
    }

    /// Conflict precedence is documented behaviour (members > import > include,
    /// last import wins). Structural validation does not enforce precedence —
    /// it only checks referential integrity. The precedence rule is applied at
    /// runtime when building the effective team membership.
    #[test]
    fn team_member_overrides_import_is_documented_not_validated() {
        // Both the direct member and the imported team's member share a handle.
        // This is allowed at definition time — conflict resolution happens at runtime.
        let ctx = base_ctx();
        let team = Team {
            members: Some(vec![TeamMember {
                handle: "@alice".to_string(),
                role: "pm".to_string(),
            }]),
            import: Some(vec!["backend-team".to_string()]),
            ..valid_team()
        };
        let errors = validate(&team, &ctx);
        // No referential-integrity error, even if handles overlap at runtime
        assert!(!errors.iter().any(|e| e.message.contains("duplicate")));
    }

    // --- 10.5: Team circular reference tests ---

    #[test]
    fn team_no_cycle_passes() {
        let ctx = base_ctx();
        let mut include = HashMap::new();
        include.insert("backend-team".to_string(), "eng-lead".to_string());
        let team = Team {
            include: Some(include),
            ..valid_team()
        };
        let errors = validate_no_cycle(&team, &ctx);
        assert!(errors.is_empty());
    }

    #[test]
    fn team_self_reference_via_include_fails() {
        let ctx = base_ctx();
        // The incoming team's id exists in include — self-reference
        // Note: it won't be in ctx.team_ids since it's incoming
        let mut include = HashMap::new();
        include.insert("platform-team".to_string(), "eng-lead".to_string());
        let team = Team {
            id: "platform-team".into(),
            include: Some(include),
            ..valid_team()
        };
        let errors = validate_no_cycle(&team, &ctx);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("circular reference"));
    }

    // --- 8.2: Team extensions validation tests ---

    #[test]
    fn team_x_prefixed_extension_passes() {
        let mut map = std::collections::HashMap::new();
        map.insert("x-cost-center".to_string(), serde_json::json!("eng"));
        let team = Team {
            extensions: Extensions(map),
            ..valid_team()
        };
        let errors = validate_structural(&team);
        assert!(errors.is_empty());
    }

    #[test]
    fn team_non_x_extension_key_fails() {
        let mut map = std::collections::HashMap::new();
        map.insert("cost-center".to_string(), serde_json::json!("eng"));
        let team = Team {
            extensions: Extensions(map),
            ..valid_team()
        };
        let errors = validate_structural(&team);
        assert!(!errors.is_empty());
        assert!(errors[0].path.contains("extensions"));
        assert!(errors[0].message.contains("x-"));
    }

    #[test]
    fn team_transitive_cycle_fails() {
        let mut ctx = base_ctx();
        // team-b's refs include the incoming team (team-x)
        // With EntityStore, we insert team-b as a full Team whose import contains "team-x"
        ctx.insert_team(Team {
            id: "team-b".into(),
            name: "Team B".to_string(),
            description: None,
            members: None,
            include: None,
            import: Some(vec!["team-x".to_string()]),
            extensions: Extensions::default(),
        });

        // Incoming team "team-x" imports "team-b"
        let team = Team {
            id: "team-x".into(),
            name: "Team X".to_string(),
            description: None,
            members: None,
            include: None,
            import: Some(vec!["team-b".to_string()]),
            extensions: Extensions::default(),
        };
        let errors = validate_no_cycle(&team, &ctx);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("circular reference"));
    }

    // --- Team::get_refs tests ---

    #[test]
    fn get_refs_yields_include_keys() {
        let mut include = HashMap::new();
        include.insert("backend-team".to_string(), "eng-lead".to_string());
        let team = Team {
            include: Some(include),
            ..valid_team()
        };
        let refs: Vec<&str> = team.get_refs().collect();
        assert_eq!(refs, vec!["backend-team"]);
    }

    #[test]
    fn get_refs_yields_import_entries() {
        let team = Team {
            import: Some(vec!["design-team".to_string()]),
            ..valid_team()
        };
        let refs: Vec<&str> = team.get_refs().collect();
        assert_eq!(refs, vec!["design-team"]);
    }

    #[test]
    fn get_refs_yields_both_include_and_import() {
        let mut include = HashMap::new();
        include.insert("backend-team".to_string(), "eng-lead".to_string());
        let team = Team {
            include: Some(include),
            import: Some(vec!["design-team".to_string()]),
            ..valid_team()
        };
        let mut refs: Vec<&str> = team.get_refs().collect();
        refs.sort();
        assert!(refs.contains(&"backend-team"));
        assert!(refs.contains(&"design-team"));
        assert_eq!(refs.len(), 2);
    }

    #[test]
    fn get_refs_yields_nothing_for_team_with_no_references() {
        let team = valid_team();
        let refs: Vec<&str> = team.get_refs().collect();
        assert!(refs.is_empty());
    }
}
