use std::collections::{HashMap, HashSet, VecDeque};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::schema::context::RepoContext;
use crate::schema::validation::{is_kebab_case, is_valid_handle, ValidationError};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct TeamMember {
    #[schemars(regex(pattern = r"^@[a-z0-9._-]+$"))]
    pub handle: String,
    pub role: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Team {
    #[schemars(regex(pattern = r"^[a-z][a-z0-9-]*$"))]
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub members: Option<Vec<TeamMember>>,
    /// Map of team_id → role_id: all members of the referenced team are assigned this role.
    pub include: Option<HashMap<String, String>>,
    /// List of team_ids whose members carry their own roles from the source team.
    pub import: Option<Vec<String>>,
}

pub fn validate(team: &Team, ctx: &RepoContext) -> Vec<ValidationError> {
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

    // Handle format and uniqueness
    if let Some(members) = &team.members {
        let mut seen_handles: HashSet<&str> = HashSet::new();
        for (i, member) in members.iter().enumerate() {
            if !is_valid_handle(&member.handle) {
                errors.push(ValidationError {
                    path: format!("members[{}].handle", i),
                    message: format!(
                        "handle '{}' must match @[a-z0-9._-]+",
                        member.handle
                    ),
                });
            }
            if !seen_handles.insert(member.handle.as_str()) {
                errors.push(ValidationError {
                    path: format!("members[{}].handle", i),
                    message: format!("duplicate handle '{}'", member.handle),
                });
            }
        }
    }

    errors
}

fn validate_cross_entity(team: &Team, ctx: &RepoContext) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // include: team_id keys and role_id values must exist
    if let Some(include) = &team.include {
        for (team_id, role_id) in include {
            if !ctx.has_team(team_id) {
                errors.push(ValidationError {
                    path: format!("include.{}", team_id),
                    message: format!("unknown team '{}'", team_id),
                });
            }
            if !ctx.has_role(role_id) {
                errors.push(ValidationError {
                    path: format!("include.{}", team_id),
                    message: format!("unknown role '{}'", role_id),
                });
            }
        }
    }

    // import: team_ids must exist
    if let Some(import) = &team.import {
        for (i, team_id) in import.iter().enumerate() {
            if !ctx.has_team(team_id) {
                errors.push(ValidationError {
                    path: format!("import[{}]", i),
                    message: format!("unknown team '{}'", team_id),
                });
            }
        }
    }

    // member roles must exist
    if let Some(members) = &team.members {
        for (i, member) in members.iter().enumerate() {
            if !ctx.has_role(&member.role) {
                errors.push(ValidationError {
                    path: format!("members[{}].role", i),
                    message: format!("unknown role '{}'", member.role),
                });
            }
        }
    }

    errors
}

/// Validates that team does not form a circular chain through include/import.
/// Uses BFS over team references via RepoContext. Since no existing team in
/// RepoContext can reference the incoming team, a cycle can only occur if the
/// incoming team's id appears in the reachable set starting from its own references.
fn validate_no_cycle(team: &Team, ctx: &RepoContext) -> Vec<ValidationError> {
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
        if current == team.id {
            return vec![ValidationError {
                path: "include/import".to_string(),
                message: format!(
                    "circular reference: team '{}' forms a cycle",
                    team.id
                ),
            }];
        }
        if visited.insert(current.clone()) {
            if let Some(refs) = ctx.get_team_refs(&current) {
                for r in refs {
                    to_visit.push_back(r.clone());
                }
            }
        }
    }

    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::context::RepoContext;

    fn base_ctx() -> RepoContext {
        let mut ctx = RepoContext::new();
        ctx.role_ids.insert("eng-lead".to_string());
        ctx.role_ids.insert("pm".to_string());
        ctx.team_ids.insert("backend-team".to_string());
        ctx.team_ids.insert("qa-team".to_string());
        ctx
    }

    fn valid_team() -> Team {
        Team {
            id: "platform-team".to_string(),
            name: "Platform Team".to_string(),
            description: None,
            members: None,
            include: None,
            import: None,
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
            id: "PlatformTeam".to_string(),
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
        assert!(errors.iter().any(|e| e.message.contains("duplicate handle")));
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
            id: "platform-team".to_string(),
            include: Some(include),
            ..valid_team()
        };
        let errors = validate_no_cycle(&team, &ctx);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("circular reference"));
    }

    #[test]
    fn team_transitive_cycle_fails() {
        let mut ctx = base_ctx();
        // team-b's refs include the incoming team (team-x)
        let mut b_refs = std::collections::HashSet::new();
        b_refs.insert("team-x".to_string());
        ctx.team_direct_refs.insert("team-b".to_string(), b_refs);
        ctx.team_ids.insert("team-b".to_string());

        // Incoming team "team-x" imports "team-b"
        let team = Team {
            id: "team-x".to_string(),
            name: "Team X".to_string(),
            description: None,
            members: None,
            include: None,
            import: Some(vec!["team-b".to_string()]),
        };
        let errors = validate_no_cycle(&team, &ctx);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("circular reference"));
    }
}
