// 14.1: Integration tests for RepoSubstrate::persist
// Verifies: full directory tree, valid YAML frontmatter, template files.

use std::{collections::HashMap, fs};

use pari::{
    schema::{
        entities::{
            relay::Relay,
            role::Role,
            task::Task,
            team::Team,
            workflow::{
                ReviewStep, SharedStep, SharedWorkStep, SharedWorkStepDefinition, SharedWorkflow,
                Step, WorkStep, WorkStepDefinition, Workflow,
            },
        },
        types::{Artifact, Extensions, Raci, StateMapEntry, TaskStateEntry, WorkflowStateEntry},
    },
    substrate::{repo::storage::RepoSubstrate, EntityStore, Substrate},
};

fn raci() -> Raci {
    Raci {
        responsible: "eng-lead".to_string(),
        accountable: "pm".to_string(),
        consulted: vec![],
        informed: vec![],
    }
}

fn tempdir() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "pari-integ-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn cleanup(path: std::path::PathBuf) {
    let _ = fs::remove_dir_all(path);
}

fn minimal_task(id: &str, template: Option<&str>) -> Task {
    Task {
        id: id.into(),
        name: format!("{id} Name"),
        description: None,
        purpose: "Test purpose.".to_string(),
        instructions: vec!["Do the thing.".to_string()],
        criteria: vec!["Thing done.".to_string()],
        accountability: None,
        artifact: Artifact {
            name: "output".to_string(),
            template: template.map(str::to_string),
        },
        states: vec![
            TaskStateEntry {
                id: "Draft".to_string(),
                description: "In progress.".to_string(),
                semantic: None,
            },
            TaskStateEntry {
                id: "Done".to_string(),
                description: "Complete.".to_string(),
                semantic: Some(pari::schema::types::TaskSemantic::Complete),
            },
        ],
        hooks: None,
        guidance: None,
        extensions: Extensions::default(),
    }
}

fn minimal_relay(id: &str) -> Relay {
    let mut state_map = HashMap::new();
    state_map.insert(
        "Complete".to_string(),
        StateMapEntry {
            maps_to: "Done".to_string(),
            semantic: Some(pari::schema::types::RelayStateSemantic::Complete),
        },
    );
    Relay {
        id: id.into(),
        name: format!("{id} Relay"),
        description: None,
        purpose: "Relay purpose.".to_string(),
        accountability: None,
        delegates_to: "SomeSharedWorkflow".to_string(),
        briefing: None,
        debriefing: None,
        state_map,
        hooks: None,
        guidance: None,
        extensions: Extensions::default(),
    }
}

fn workflow_states() -> Vec<WorkflowStateEntry> {
    vec![
        WorkflowStateEntry {
            id: "Open".to_string(),
            description: "Open.".to_string(),
            semantic: None,
        },
        WorkflowStateEntry {
            id: "Done".to_string(),
            description: "Done.".to_string(),
            semantic: Some(pari::schema::types::WorkflowSemantic::Complete),
        },
    ]
}

/// Build a minimal EntityStore covering all entity types and step variants.
fn full_store() -> EntityStore {
    let role = Role {
        id: "eng-lead".into(),
        name: "Engineering Lead".to_string(),
        purpose: "Drive technical direction.".to_string(),
        traits: Some(vec!["approver".to_string()]),
        extensions: Extensions::default(),
    };

    let hook = pari::schema::entities::hook::Hook {
        id: "UpdateJira".into(),
        name: "Update Jira".to_string(),
        description: "Updates Jira.".to_string(),
        instructions: vec!["Call the API.".to_string()],
        inputs: None,
        extensions: Extensions::default(),
    };

    let team = Team {
        id: "platform-team".into(),
        name: "Platform Team".to_string(),
        description: None,
        members: None,
        include: None,
        import: None,
        extensions: Extensions::default(),
    };

    // Workflow with: Task (with template), Relay, inline Workflow, ReviewStep
    let inline_wf = WorkflowDef(
        "InnerFlow",
        vec![Step::Work(WorkStep {
            depends_on: None,
            definition: WorkStepDefinition::Task(minimal_task("InnerTask", None)),
        })],
    );

    let workflow = Workflow {
        id: "Initiative".into(),
        name: "Initiative".to_string(),
        description: None,
        purpose: "Ship a new capability.".to_string(),
        accountability: raci(),
        steps: vec![
            Step::Work(WorkStep {
                depends_on: None,
                definition: WorkStepDefinition::Task(minimal_task(
                    "WriteProposal",
                    Some("# Proposal\n\nFill this in."),
                )),
            }),
            Step::Work(WorkStep {
                depends_on: None,
                definition: WorkStepDefinition::Relay(minimal_relay("LegalReview")),
            }),
            Step::Work(WorkStep {
                depends_on: None,
                definition: WorkStepDefinition::Workflow(Box::new(inline_wf)),
            }),
            Step::Review(ReviewStep {
                id: "Approve".to_string(),
                approver: "pm".to_string(),
                on_reject: "WriteProposal".to_string(),
            }),
        ],
        states: workflow_states(),
        hooks: None,
        guidance: None,
        extensions: Extensions::default(),
    };

    // SharedWorkflow with Task (no template) and inline SharedWorkflow
    let inner_shared = WorkflowDef_shared(
        "SharedInner",
        vec![SharedStep::Work(SharedWorkStep {
            depends_on: None,
            definition: SharedWorkStepDefinition::Task(minimal_task("SharedInnerTask", None)),
        })],
    );

    let shared_workflow = SharedWorkflow {
        id: "SharedInit".into(),
        name: "Shared Init".to_string(),
        description: None,
        purpose: "Shared setup.".to_string(),
        accountability: raci(),
        steps: vec![
            SharedStep::Work(SharedWorkStep {
                depends_on: None,
                definition: SharedWorkStepDefinition::Task(minimal_task("SharedTask", None)),
            }),
            SharedStep::Work(SharedWorkStep {
                depends_on: None,
                definition: SharedWorkStepDefinition::SharedWorkflow(Box::new(inner_shared)),
            }),
        ],
        states: workflow_states(),
        hooks: None,
        guidance: None,
        extensions: Extensions::default(),
    };

    let mut store = EntityStore::new();
    store.roles.insert(role.id.to_string(), role);
    store.hooks.insert(hook.id.to_string(), hook);
    store.teams.insert(team.id.to_string(), team);
    store.workflows.insert(workflow.id.to_string(), workflow);
    store
        .shared_workflows
        .insert(shared_workflow.id.to_string(), shared_workflow);
    store
}

/// Helper to build a Workflow (WorkflowDef<Step>) inline.
#[allow(non_snake_case)]
fn WorkflowDef(id: &str, steps: Vec<Step>) -> Workflow {
    Workflow {
        id: id.into(),
        name: format!("{id} Workflow"),
        description: None,
        purpose: "Inline workflow purpose.".to_string(),
        accountability: raci(),
        steps,
        states: workflow_states(),
        hooks: None,
        guidance: None,
        extensions: Extensions::default(),
    }
}

/// Helper to build a SharedWorkflow (WorkflowDef<SharedStep>) inline.
#[allow(non_snake_case)]
fn WorkflowDef_shared(id: &str, steps: Vec<SharedStep>) -> SharedWorkflow {
    SharedWorkflow {
        id: id.into(),
        name: format!("{id} Workflow"),
        description: None,
        purpose: "Inline shared workflow purpose.".to_string(),
        accountability: raci(),
        steps,
        states: workflow_states(),
        hooks: None,
        guidance: None,
        extensions: Extensions::default(),
    }
}

fn assert_valid_yaml(path: &std::path::Path) {
    let content = fs::read_to_string(path).unwrap_or_else(|_| panic!("{path:?} not readable"));
    let after_open = content
        .strip_prefix("---\n")
        .unwrap_or_else(|| panic!("{path:?} missing opening ---"));
    let end = after_open
        .find("\n---\n")
        .unwrap_or_else(|| panic!("{path:?} missing closing ---"));
    let fm = &after_open[..end];
    serde_yaml::from_str::<serde_yaml::Value>(fm)
        .unwrap_or_else(|e| panic!("{path:?} frontmatter is not valid YAML: {e}\n{fm}"));
}

#[test]
fn persist_full_store_creates_all_entity_files() {
    let tmp = tempdir();
    let root = tmp.join("repo");
    let substrate = RepoSubstrate::new(&root);
    substrate.persist(&full_store()).unwrap();

    // Flat files
    assert!(root.join("roles/eng-lead.md").exists(), "roles/eng-lead.md");
    assert!(
        root.join("teams/platform-team.md").exists(),
        "teams/platform-team.md"
    );
    assert!(
        root.join("shared/hooks/UpdateJira.md").exists(),
        "shared/hooks/UpdateJira.md"
    );

    // Workflow directory tree
    assert!(root.join("workflows/Initiative/README.md").exists());
    assert!(root
        .join("workflows/Initiative/WriteProposal/README.md")
        .exists());
    assert!(root
        .join("workflows/Initiative/LegalReview/README.md")
        .exists());
    assert!(root
        .join("workflows/Initiative/InnerFlow/README.md")
        .exists());
    assert!(root
        .join("workflows/Initiative/InnerFlow/InnerTask/README.md")
        .exists());

    // ReviewStep must NOT have a directory
    assert!(
        !root.join("workflows/Initiative/Approve").exists(),
        "ReviewStep must not have dir"
    );

    // SharedWorkflow directory tree
    assert!(root.join("shared/workflows/SharedInit/README.md").exists());
    assert!(root
        .join("shared/workflows/SharedInit/SharedTask/README.md")
        .exists());
    assert!(root
        .join("shared/workflows/SharedInit/SharedInner/README.md")
        .exists());
    assert!(root
        .join("shared/workflows/SharedInit/SharedInner/SharedInnerTask/README.md")
        .exists());

    cleanup(tmp);
}

#[test]
fn persist_all_frontmatter_is_valid_yaml() {
    let tmp = tempdir();
    let root = tmp.join("repo");
    let substrate = RepoSubstrate::new(&root);
    substrate.persist(&full_store()).unwrap();

    let files = [
        "roles/eng-lead.md",
        "teams/platform-team.md",
        "shared/hooks/UpdateJira.md",
        "workflows/Initiative/README.md",
        "workflows/Initiative/WriteProposal/README.md",
        "workflows/Initiative/LegalReview/README.md",
        "workflows/Initiative/InnerFlow/README.md",
        "shared/workflows/SharedInit/README.md",
        "shared/workflows/SharedInit/SharedTask/README.md",
    ];

    for f in &files {
        assert_valid_yaml(&root.join(f));
    }

    cleanup(tmp);
}

#[test]
fn persist_template_file_created_when_artifact_template_set() {
    let tmp = tempdir();
    let root = tmp.join("repo");
    let substrate = RepoSubstrate::new(&root);
    substrate.persist(&full_store()).unwrap();

    let template_path = root.join("workflows/Initiative/WriteProposal/output.template.md");
    assert!(template_path.exists(), "output.template.md should exist");
    let content = fs::read_to_string(&template_path).unwrap();
    assert!(
        content.contains("# Proposal"),
        "template content should be written"
    );

    cleanup(tmp);
}

#[test]
fn persist_no_template_file_when_artifact_template_absent() {
    let tmp = tempdir();
    let root = tmp.join("repo");
    let substrate = RepoSubstrate::new(&root);

    let wf = Workflow {
        id: "Simple".into(),
        name: "Simple".to_string(),
        description: None,
        purpose: "Simple.".to_string(),
        accountability: raci(),
        steps: vec![Step::Work(WorkStep {
            depends_on: None,
            definition: WorkStepDefinition::Task(minimal_task("Task1", None)),
        })],
        states: workflow_states(),
        hooks: None,
        guidance: None,
        extensions: Extensions::default(),
    };
    let mut store = EntityStore::new();
    store.workflows.insert(wf.id.to_string(), wf);

    substrate.persist(&store).unwrap();

    let task_dir = root.join("workflows/Simple/Task1");
    assert!(task_dir.join("README.md").exists());
    // No template file
    let template = task_dir.join("output.template.md");
    assert!(
        !template.exists(),
        "template file should not exist when artifact.template is None"
    );

    cleanup(tmp);
}
