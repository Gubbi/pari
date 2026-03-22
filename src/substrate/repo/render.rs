//! Markdown + YAML-frontmatter renderers for each entity type.

use std::fmt::Write as FmtWrite;

use schemars::JsonSchema;
use serde::Serialize;
use serde_yaml::{Mapping, Value};

use crate::schema::{
    entities::{
        hook::Hook, relay::Relay, role::Role, task::Task, team::Team, workflow::WorkflowDef,
    },
    types::Extensions,
};

fn build_frontmatter(fm: &Mapping) -> String {
    let yaml = serde_yaml::to_string(fm).unwrap();
    format!("---\n{yaml}---\n")
}

fn push_extensions(fm: &mut Mapping, extensions: &Extensions) {
    let mut keys: Vec<_> = extensions.0.keys().collect();
    keys.sort();
    for k in keys {
        let v = &extensions.0[k];
        let yaml_val = serde_yaml::to_value(v).unwrap_or(Value::Null);
        fm.insert(Value::String(k.clone()), yaml_val);
    }
}

fn to_yaml_val<T: Serialize>(val: &T) -> Value {
    serde_yaml::to_value(val).unwrap_or(Value::Null)
}

// --- 13.2: render_role ---

pub fn render_role(role: &Role) -> String {
    let mut fm = Mapping::new();
    fm.insert(
        Value::String("id".to_string()),
        Value::String(role.id.to_string()),
    );
    push_extensions(&mut fm, &role.extensions);

    let mut out = build_frontmatter(&fm);
    let _ = write!(out, "\n# {}\n", role.name);
    let _ = write!(out, "\n## Purpose\n\n{}\n", role.purpose);
    if let Some(traits) = &role.traits {
        out.push_str("\n## Responsibilities\n\n");
        for t in traits {
            let _ = writeln!(out, "- {t}");
        }
    }
    out
}

// --- 13.4: render_hook ---

pub fn render_hook(hook: &Hook) -> String {
    let mut fm = Mapping::new();
    fm.insert(
        Value::String("id".to_string()),
        Value::String(hook.id.to_string()),
    );
    push_extensions(&mut fm, &hook.extensions);

    let mut out = build_frontmatter(&fm);
    let _ = write!(out, "\n# {}\n", hook.name);
    let _ = write!(out, "\n{}\n", hook.description);
    out.push_str("\n## Instructions\n\n");
    for instr in &hook.instructions {
        let _ = writeln!(out, "- {instr}");
    }
    out
}

// --- 13.6: render_team ---

pub fn render_team(team: &Team) -> String {
    let mut fm = Mapping::new();
    fm.insert(
        Value::String("id".to_string()),
        Value::String(team.id.to_string()),
    );
    if let Some(members) = &team.members {
        fm.insert(Value::String("members".to_string()), to_yaml_val(members));
    }
    if let Some(include) = &team.include {
        // Sort keys for deterministic output
        let sorted: std::collections::BTreeMap<_, _> = include.iter().collect();
        fm.insert(Value::String("include".to_string()), to_yaml_val(&sorted));
    }
    push_extensions(&mut fm, &team.extensions);

    let mut out = build_frontmatter(&fm);
    let _ = write!(out, "\n# {}\n", team.name);
    if let Some(desc) = &team.description {
        let _ = write!(out, "\n{desc}\n");
    }
    out
}

// --- 13.8: render_workflow_readme ---

pub fn render_workflow_readme<S>(workflow: &WorkflowDef<S>) -> String
where
    S: JsonSchema + Serialize,
{
    let mut fm = Mapping::new();
    fm.insert(
        Value::String("id".to_string()),
        Value::String(workflow.id.to_string()),
    );
    fm.insert(
        Value::String("accountability".to_string()),
        to_yaml_val(&workflow.accountability),
    );
    fm.insert(
        Value::String("steps".to_string()),
        to_yaml_val(&workflow.steps),
    );
    fm.insert(
        Value::String("states".to_string()),
        to_yaml_val(&workflow.states),
    );
    if let Some(hooks) = &workflow.hooks {
        fm.insert(Value::String("hooks".to_string()), to_yaml_val(hooks));
    }
    push_extensions(&mut fm, &workflow.extensions);

    let mut out = build_frontmatter(&fm);
    let _ = write!(out, "\n# {}\n", workflow.name);
    if let Some(desc) = &workflow.description {
        let _ = write!(out, "\n{desc}\n");
    }
    let _ = write!(out, "\n## Purpose\n\n{}\n", workflow.purpose);
    if let Some(guidance) = &workflow.guidance {
        let _ = write!(out, "\n## Guidance\n\n{guidance}\n");
    }
    out
}

// --- 13.10: render_task_readme ---

pub fn render_task_readme(task: &Task) -> String {
    let mut fm = Mapping::new();
    fm.insert(
        Value::String("id".to_string()),
        Value::String(task.id.to_string()),
    );
    fm.insert(
        Value::String("artifact".to_string()),
        to_yaml_val(&task.artifact),
    );
    fm.insert(
        Value::String("states".to_string()),
        to_yaml_val(&task.states),
    );
    if let Some(hooks) = &task.hooks {
        fm.insert(Value::String("hooks".to_string()), to_yaml_val(hooks));
    }
    push_extensions(&mut fm, &task.extensions);

    let mut out = build_frontmatter(&fm);
    let _ = write!(out, "\n# {}\n", task.name);
    if let Some(desc) = &task.description {
        let _ = write!(out, "\n{desc}\n");
    }
    let _ = write!(out, "\n## Purpose\n\n{}\n", task.purpose);
    out.push_str("\n## Steps\n\n");
    for step in &task.instructions {
        let _ = writeln!(out, "- {step}");
    }
    out.push_str("\n## Criteria\n\n");
    for criterion in &task.criteria {
        let _ = writeln!(out, "- {criterion}");
    }
    if let Some(guidance) = &task.guidance {
        let _ = write!(out, "\n## Guidance\n\n{guidance}\n");
    }
    out
}

// --- 13.12: render_relay_readme ---

pub fn render_relay_readme(relay: &Relay) -> String {
    let mut fm = Mapping::new();
    fm.insert(
        Value::String("id".to_string()),
        Value::String(relay.id.to_string()),
    );
    fm.insert(
        Value::String("delegates_to".to_string()),
        Value::String(relay.delegates_to.clone()),
    );
    fm.insert(
        Value::String("state_map".to_string()),
        to_yaml_val(&relay.state_map),
    );
    push_extensions(&mut fm, &relay.extensions);

    let mut out = build_frontmatter(&fm);
    let _ = write!(out, "\n# {}\n", relay.name);
    if let Some(desc) = &relay.description {
        let _ = write!(out, "\n{desc}\n");
    }
    let _ = write!(out, "\n## Purpose\n\n{}\n", relay.purpose);
    if let Some(briefing) = &relay.briefing {
        let _ = write!(out, "\n## Briefing\n\n{briefing}\n");
    }
    if let Some(debriefing) = &relay.debriefing {
        let _ = write!(out, "\n## Debriefing\n\n{debriefing}\n");
    }
    if let Some(guidance) = &relay.guidance {
        let _ = write!(out, "\n## Guidance\n\n{guidance}\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::schema::{
        entities::{
            team::TeamMember,
            workflow::{
                ReviewStep, SharedWorkStepDefinition, Step, WorkStep,
                WorkStepDefinition,
            },
        },
        types::{Artifact, Raci, StateMapEntry, TaskStateEntry, WorkflowStateEntry},
    };

    fn extract_frontmatter(s: &str) -> &str {
        // Returns content between the first --- and second ---
        let after_open = s.strip_prefix("---\n").expect("missing opening ---");
        let end = after_open.find("\n---\n").expect("missing closing ---");
        &after_open[..end]
    }

    fn raci() -> Raci {
        Raci {
            responsible: "eng-lead".to_string(),
            accountable: "pm".to_string(),
            consulted: vec![],
            informed: vec![],
        }
    }

    // -----------------------------------------------------------------------
    // 13.1 + 13.2: render_role
    // -----------------------------------------------------------------------

    fn base_role() -> Role {
        Role {
            id: "eng-lead".into(),
            name: "Engineering Lead".to_string(),
            purpose: "Drive technical direction.".to_string(),
            traits: None,
            extensions: Extensions::default(),
        }
    }

    #[test]
    fn render_role_frontmatter_contains_id() {
        let out = render_role(&base_role());
        let fm = extract_frontmatter(&out);
        assert!(fm.contains("id: eng-lead"), "fm: {fm}");
    }

    #[test]
    fn render_role_frontmatter_is_valid_yaml() {
        let out = render_role(&base_role());
        let fm = extract_frontmatter(&out);
        serde_yaml::from_str::<serde_yaml::Value>(fm).expect("frontmatter must be valid YAML");
    }

    #[test]
    fn render_role_body_has_name_and_purpose() {
        let out = render_role(&base_role());
        assert!(out.contains("# Engineering Lead"), "out: {out}");
        assert!(out.contains("## Purpose"), "out: {out}");
        assert!(out.contains("Drive technical direction."), "out: {out}");
    }

    #[test]
    fn render_role_with_traits_has_responsibilities() {
        let role = Role {
            traits: Some(vec!["approver".to_string(), "reviewer".to_string()]),
            ..base_role()
        };
        let out = render_role(&role);
        assert!(out.contains("## Responsibilities"), "out: {out}");
        assert!(out.contains("- approver"), "out: {out}");
        assert!(out.contains("- reviewer"), "out: {out}");
    }

    #[test]
    fn render_role_without_traits_omits_responsibilities() {
        let out = render_role(&base_role());
        assert!(!out.contains("## Responsibilities"), "out: {out}");
    }

    #[test]
    fn render_role_extensions_appear_in_frontmatter() {
        let mut map = HashMap::new();
        map.insert("x-owner".to_string(), serde_json::json!("platform"));
        let role = Role {
            extensions: Extensions(map),
            ..base_role()
        };
        let out = render_role(&role);
        let fm = extract_frontmatter(&out);
        assert!(fm.contains("x-owner"), "fm: {fm}");
        assert!(fm.contains("platform"), "fm: {fm}");
    }

    // -----------------------------------------------------------------------
    // 13.3 + 13.4: render_hook
    // -----------------------------------------------------------------------

    fn base_hook() -> Hook {
        Hook {
            id: "UpdateJiraStatus".into(),
            name: "Update Jira Status".to_string(),
            description: "Updates the Jira issue status.".to_string(),
            instructions: vec!["Call the Jira API with the new status.".to_string()],
            inputs: None,
            extensions: Extensions::default(),
        }
    }

    #[test]
    fn render_hook_frontmatter_contains_id() {
        let out = render_hook(&base_hook());
        let fm = extract_frontmatter(&out);
        assert!(fm.contains("id: UpdateJiraStatus"), "fm: {fm}");
    }

    #[test]
    fn render_hook_frontmatter_is_valid_yaml() {
        let out = render_hook(&base_hook());
        let fm = extract_frontmatter(&out);
        serde_yaml::from_str::<serde_yaml::Value>(fm).expect("frontmatter must be valid YAML");
    }

    #[test]
    fn render_hook_body_has_name_and_instructions() {
        let out = render_hook(&base_hook());
        assert!(out.contains("# Update Jira Status"), "out: {out}");
        assert!(out.contains("## Instructions"), "out: {out}");
        assert!(out.contains("- Call the Jira API"), "out: {out}");
    }

    #[test]
    fn render_hook_body_has_description_paragraph() {
        let out = render_hook(&base_hook());
        assert!(out.contains("Updates the Jira issue status."), "out: {out}");
    }

    #[test]
    fn render_hook_extensions_in_frontmatter() {
        let mut map = HashMap::new();
        map.insert("x-version".to_string(), serde_json::json!("2"));
        let hook = Hook {
            extensions: Extensions(map),
            ..base_hook()
        };
        let out = render_hook(&hook);
        let fm = extract_frontmatter(&out);
        assert!(fm.contains("x-version"), "fm: {fm}");
    }

    // -----------------------------------------------------------------------
    // 13.5 + 13.6: render_team
    // -----------------------------------------------------------------------

    fn base_team() -> Team {
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

    #[test]
    fn render_team_frontmatter_contains_id() {
        let out = render_team(&base_team());
        let fm = extract_frontmatter(&out);
        assert!(fm.contains("id: platform-team"), "fm: {fm}");
    }

    #[test]
    fn render_team_frontmatter_is_valid_yaml() {
        let out = render_team(&base_team());
        let fm = extract_frontmatter(&out);
        serde_yaml::from_str::<serde_yaml::Value>(fm).expect("frontmatter must be valid YAML");
    }

    #[test]
    fn render_team_body_has_name() {
        let out = render_team(&base_team());
        assert!(out.contains("# Platform Team"), "out: {out}");
    }

    #[test]
    fn render_team_with_members_has_members_in_frontmatter() {
        let team = Team {
            members: Some(vec![TeamMember {
                handle: "@alice".to_string(),
                role: "eng-lead".to_string(),
            }]),
            ..base_team()
        };
        let out = render_team(&team);
        let fm = extract_frontmatter(&out);
        assert!(fm.contains("members"), "fm: {fm}");
        assert!(fm.contains("@alice"), "fm: {fm}");
    }

    #[test]
    fn render_team_without_members_omits_members_from_frontmatter() {
        let out = render_team(&base_team());
        let fm = extract_frontmatter(&out);
        assert!(!fm.contains("members"), "fm should not have members: {fm}");
    }

    #[test]
    fn render_team_with_include_has_include_in_frontmatter() {
        let mut include = HashMap::new();
        include.insert("backend-team".to_string(), "eng-lead".to_string());
        let team = Team {
            include: Some(include),
            ..base_team()
        };
        let out = render_team(&team);
        let fm = extract_frontmatter(&out);
        assert!(fm.contains("include"), "fm: {fm}");
        assert!(fm.contains("backend-team"), "fm: {fm}");
    }

    #[test]
    fn render_team_without_include_omits_include_from_frontmatter() {
        let out = render_team(&base_team());
        let fm = extract_frontmatter(&out);
        assert!(!fm.contains("include"), "fm should not have include: {fm}");
    }

    #[test]
    fn render_team_optional_description_in_body() {
        let team = Team {
            description: Some("The core platform squad.".to_string()),
            ..base_team()
        };
        let out = render_team(&team);
        assert!(out.contains("The core platform squad."), "out: {out}");
    }

    #[test]
    fn render_team_without_description_omits_it() {
        let out = render_team(&base_team());
        // Should not have any stray paragraph beyond the name
        let body_after_name = out.split("# Platform Team").nth(1).unwrap();
        assert!(
            !body_after_name.trim().starts_with('\n') || !body_after_name.contains("The core"),
            "out: {out}"
        );
    }

    // -----------------------------------------------------------------------
    // 13.7 + 13.8: render_workflow_readme
    // -----------------------------------------------------------------------

    fn workflow_states() -> Vec<WorkflowStateEntry> {
        use crate::schema::types::WorkflowSemantic;
        vec![
            WorkflowStateEntry {
                id: "InProgress".to_string(),
                description: "Work is ongoing.".to_string(),
                semantic: None,
            },
            WorkflowStateEntry {
                id: "Done".to_string(),
                description: "Work is complete.".to_string(),
                semantic: Some(WorkflowSemantic::Complete),
            },
        ]
    }

    fn base_workflow() -> crate::schema::entities::workflow::Workflow {
        use crate::schema::entities::workflow::Workflow;
        Workflow {
            id: "Initiative".into(),
            name: "Initiative".to_string(),
            description: None,
            purpose: "Ship a new capability.".to_string(),
            accountability: raci(),
            steps: vec![Step::Work(WorkStep {
                depends_on: None,
                definition: WorkStepDefinition::Task(base_task()),
            })],
            states: workflow_states(),
            hooks: None,
            guidance: None,
            extensions: Extensions::default(),
        }
    }

    fn base_task() -> Task {
        Task {
            id: "WriteProposal".into(),
            name: "Write Proposal".to_string(),
            description: None,
            purpose: "Document the approach.".to_string(),
            instructions: vec!["Write the proposal doc.".to_string()],
            criteria: vec!["Proposal approved.".to_string()],
            accountability: None,
            artifact: Artifact {
                name: "proposal".to_string(),
                template: None,
            },
            states: vec![
                TaskStateEntry {
                    id: "Draft".to_string(),
                    description: "Being written.".to_string(),
                    semantic: None,
                },
                TaskStateEntry {
                    id: "Approved".to_string(),
                    description: "Approved by stakeholders.".to_string(),
                    semantic: Some(crate::schema::types::TaskSemantic::Complete),
                },
            ],
            hooks: None,
            guidance: None,
            extensions: Extensions::default(),
        }
    }

    #[test]
    fn render_workflow_frontmatter_has_id_accountability_steps_states() {
        let out = render_workflow_readme(&base_workflow());
        let fm = extract_frontmatter(&out);
        let parsed: serde_yaml::Value =
            serde_yaml::from_str(fm).expect("frontmatter must be valid YAML");
        assert!(parsed.get("id").is_some(), "missing id in: {fm}");
        assert!(
            parsed.get("accountability").is_some(),
            "missing accountability in: {fm}"
        );
        assert!(parsed.get("steps").is_some(), "missing steps in: {fm}");
        assert!(parsed.get("states").is_some(), "missing states in: {fm}");
    }

    #[test]
    fn render_workflow_frontmatter_is_valid_yaml() {
        let out = render_workflow_readme(&base_workflow());
        let fm = extract_frontmatter(&out);
        serde_yaml::from_str::<serde_yaml::Value>(fm).expect("frontmatter must be valid YAML");
    }

    #[test]
    fn render_workflow_body_has_name_and_purpose() {
        let out = render_workflow_readme(&base_workflow());
        assert!(out.contains("# Initiative"), "out: {out}");
        assert!(out.contains("## Purpose"), "out: {out}");
        assert!(out.contains("Ship a new capability."), "out: {out}");
    }

    #[test]
    fn render_workflow_review_step_in_steps_frontmatter_only() {
        // ReviewStep appears in the serialized steps YAML; no directory is created for it
        // (directory creation is the responsibility of persist(), not render)
        use crate::schema::entities::workflow::Workflow;
        let wf = Workflow {
            steps: vec![
                Step::Work(WorkStep {
                    depends_on: None,
                    definition: WorkStepDefinition::Task(base_task()),
                }),
                Step::Review(ReviewStep {
                    id: "Approve".to_string(),
                    approver: "pm".to_string(),
                    on_reject: "WriteProposal".to_string(),
                }),
            ],
            ..base_workflow()
        };
        let out = render_workflow_readme(&wf);
        let fm = extract_frontmatter(&out);
        // ReviewStep data is present in the serialized steps
        assert!(
            fm.contains("Approve"),
            "ReviewStep id should appear in steps fm: {fm}"
        );
        assert!(
            fm.contains("approver"),
            "ReviewStep approver key should appear in steps fm: {fm}"
        );
    }

    #[test]
    fn render_workflow_hooks_in_frontmatter_when_present() {
        use crate::schema::types::{HookInvocation, HookInvocationValue};
        let mut hooks = HashMap::new();
        hooks.insert(
            "on_complete".to_string(),
            HookInvocationValue::Single(HookInvocation::Bare("UpdateJira".to_string())),
        );
        let wf = crate::schema::entities::workflow::Workflow {
            hooks: Some(hooks),
            ..base_workflow()
        };
        let out = render_workflow_readme(&wf);
        let fm = extract_frontmatter(&out);
        assert!(fm.contains("hooks"), "fm: {fm}");
        assert!(fm.contains("on_complete"), "fm: {fm}");
    }

    #[test]
    fn render_workflow_guidance_section_when_present() {
        let wf = crate::schema::entities::workflow::Workflow {
            guidance: Some("Follow the process carefully.".to_string()),
            ..base_workflow()
        };
        let out = render_workflow_readme(&wf);
        assert!(out.contains("## Guidance"), "out: {out}");
        assert!(out.contains("Follow the process carefully."), "out: {out}");
    }

    #[test]
    fn render_workflow_no_guidance_section_when_absent() {
        let out = render_workflow_readme(&base_workflow());
        assert!(!out.contains("## Guidance"), "out: {out}");
    }

    #[test]
    fn render_shared_workflow_works() {
        use crate::schema::entities::workflow::SharedWorkflow;
        let sw = SharedWorkflow {
            id: "SharedInit".into(),
            name: "Shared Init".to_string(),
            description: None,
            purpose: "Shared setup.".to_string(),
            accountability: raci(),
            steps: vec![Step::<SharedWorkStepDefinition>::Work(WorkStep {
                depends_on: None,
                definition: SharedWorkStepDefinition::Task(base_task()),
            })],
            states: workflow_states(),
            hooks: None,
            guidance: None,
            extensions: Extensions::default(),
        };
        let out = render_workflow_readme(&sw);
        let fm = extract_frontmatter(&out);
        assert!(fm.contains("id: SharedInit"), "fm: {fm}");
        assert!(out.contains("# Shared Init"), "out: {out}");
    }

    // -----------------------------------------------------------------------
    // 13.9 + 13.10: render_task_readme
    // -----------------------------------------------------------------------

    #[test]
    fn render_task_frontmatter_has_id_artifact_states() {
        let out = render_task_readme(&base_task());
        let fm = extract_frontmatter(&out);
        let parsed: serde_yaml::Value =
            serde_yaml::from_str(fm).expect("frontmatter must be valid YAML");
        assert!(parsed.get("id").is_some(), "missing id: {fm}");
        assert!(parsed.get("artifact").is_some(), "missing artifact: {fm}");
        assert!(parsed.get("states").is_some(), "missing states: {fm}");
    }

    #[test]
    fn render_task_frontmatter_is_valid_yaml() {
        let out = render_task_readme(&base_task());
        let fm = extract_frontmatter(&out);
        serde_yaml::from_str::<serde_yaml::Value>(fm).expect("frontmatter must be valid YAML");
    }

    #[test]
    fn render_task_body_has_steps_and_criteria() {
        let out = render_task_readme(&base_task());
        assert!(out.contains("## Steps"), "out: {out}");
        assert!(out.contains("- Write the proposal doc."), "out: {out}");
        assert!(out.contains("## Criteria"), "out: {out}");
        assert!(out.contains("- Proposal approved."), "out: {out}");
    }

    #[test]
    fn render_task_guidance_section_when_present() {
        let task = Task {
            guidance: Some("Ask your team lead if unsure.".to_string()),
            ..base_task()
        };
        let out = render_task_readme(&task);
        assert!(out.contains("## Guidance"), "out: {out}");
        assert!(out.contains("Ask your team lead if unsure."), "out: {out}");
    }

    #[test]
    fn render_task_no_guidance_section_when_absent() {
        let out = render_task_readme(&base_task());
        assert!(!out.contains("## Guidance"), "out: {out}");
    }

    #[test]
    fn render_task_hooks_in_frontmatter_when_present() {
        use crate::schema::types::{HookInvocation, HookInvocationValue};
        let mut hooks = HashMap::new();
        hooks.insert(
            "on_complete".to_string(),
            HookInvocationValue::Single(HookInvocation::Bare("Notify".to_string())),
        );
        let task = Task {
            hooks: Some(hooks),
            ..base_task()
        };
        let out = render_task_readme(&task);
        let fm = extract_frontmatter(&out);
        assert!(fm.contains("hooks"), "fm: {fm}");
        assert!(fm.contains("on_complete"), "fm: {fm}");
    }

    #[test]
    fn render_task_artifact_template_in_frontmatter() {
        // When artifact.template is set, it appears in the artifact frontmatter block.
        // Writing the actual .template.md file is the responsibility of persist().
        let task = Task {
            artifact: Artifact {
                name: "proposal".to_string(),
                template: Some("# Proposal Template\n\nFill this in.".to_string()),
            },
            ..base_task()
        };
        let out = render_task_readme(&task);
        let fm = extract_frontmatter(&out);
        let parsed: serde_yaml::Value =
            serde_yaml::from_str(fm).expect("frontmatter must be valid YAML");
        let artifact = parsed
            .get("artifact")
            .expect("artifact must be in frontmatter");
        assert!(
            artifact.get("template").is_some(),
            "artifact.template missing: {artifact:?}"
        );
    }

    // -----------------------------------------------------------------------
    // 13.11 + 13.12: render_relay_readme
    // -----------------------------------------------------------------------

    fn base_relay() -> Relay {
        let mut state_map = HashMap::new();
        state_map.insert(
            "Complete".to_string(),
            StateMapEntry {
                maps_to: "Done".to_string(),
                semantic: Some(crate::schema::types::RelayStateSemantic::Complete),
            },
        );
        Relay {
            id: "LegalReview".into(),
            name: "Legal Review".to_string(),
            description: None,
            purpose: "Get legal sign-off.".to_string(),
            accountability: None,
            delegates_to: "LegalWorkflow".to_string(),
            briefing: None,
            debriefing: None,
            state_map,
            hooks: None,
            guidance: None,
            extensions: Extensions::default(),
        }
    }

    #[test]
    fn render_relay_frontmatter_has_id_delegates_to_state_map() {
        let out = render_relay_readme(&base_relay());
        let fm = extract_frontmatter(&out);
        let parsed: serde_yaml::Value =
            serde_yaml::from_str(fm).expect("frontmatter must be valid YAML");
        assert!(parsed.get("id").is_some(), "missing id: {fm}");
        assert!(
            parsed.get("delegates_to").is_some(),
            "missing delegates_to: {fm}"
        );
        assert!(parsed.get("state_map").is_some(), "missing state_map: {fm}");
    }

    #[test]
    fn render_relay_frontmatter_is_valid_yaml() {
        let out = render_relay_readme(&base_relay());
        let fm = extract_frontmatter(&out);
        serde_yaml::from_str::<serde_yaml::Value>(fm).expect("frontmatter must be valid YAML");
    }

    #[test]
    fn render_relay_body_has_name_and_purpose() {
        let out = render_relay_readme(&base_relay());
        assert!(out.contains("# Legal Review"), "out: {out}");
        assert!(out.contains("## Purpose"), "out: {out}");
        assert!(out.contains("Get legal sign-off."), "out: {out}");
    }

    #[test]
    fn render_relay_briefing_section_when_present() {
        let relay = Relay {
            briefing: Some("Provide context on the deal.".to_string()),
            ..base_relay()
        };
        let out = render_relay_readme(&relay);
        assert!(out.contains("## Briefing"), "out: {out}");
        assert!(out.contains("Provide context on the deal."), "out: {out}");
    }

    #[test]
    fn render_relay_no_briefing_when_absent() {
        let out = render_relay_readme(&base_relay());
        assert!(!out.contains("## Briefing"), "out: {out}");
    }

    #[test]
    fn render_relay_debriefing_section_when_present() {
        let relay = Relay {
            debriefing: Some("Summarize the legal outcome.".to_string()),
            ..base_relay()
        };
        let out = render_relay_readme(&relay);
        assert!(out.contains("## Debriefing"), "out: {out}");
        assert!(out.contains("Summarize the legal outcome."), "out: {out}");
    }

    #[test]
    fn render_relay_no_debriefing_when_absent() {
        let out = render_relay_readme(&base_relay());
        assert!(!out.contains("## Debriefing"), "out: {out}");
    }

    #[test]
    fn render_relay_guidance_section_when_present() {
        let relay = Relay {
            guidance: Some("Escalate if blocked.".to_string()),
            ..base_relay()
        };
        let out = render_relay_readme(&relay);
        assert!(out.contains("## Guidance"), "out: {out}");
        assert!(out.contains("Escalate if blocked."), "out: {out}");
    }

    #[test]
    fn render_relay_no_guidance_when_absent() {
        let out = render_relay_readme(&base_relay());
        assert!(!out.contains("## Guidance"), "out: {out}");
    }
}
