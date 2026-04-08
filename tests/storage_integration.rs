// Integration tests for RepoSubstrate::atomic_persist
// Verifies: full directory tree, valid YAML frontmatter, template files,
// incremental update behaviour, and ChangeSet integrity after failure.

use std::{collections::HashMap, fs};

use pari::{
    schema::{
        entities::{
            relay::Relay,
            role::Role,
            task::Task,
            team::Team,
            workflow::{
                ReviewStep, SharedWorkStepDefinition, SharedWorkflow,
                Step, WorkStep, WorkStepDefinition, Workflow,
            },
        },
        types::{Artifact, Extensions, Raci, StateMapEntry, TaskStateEntry, WorkflowStateEntry},
    },
    substrate::{repo::storage::RepoSubstrate, EntityStore},
};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

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

fn minimal_role(id: &str) -> Role {
    Role {
        id: id.into(),
        name: format!("{id} Name"),
        purpose: "Test purpose.".to_string(),
        traits: None,
        extensions: Extensions::default(),
    }
}

fn minimal_team(id: &str) -> Team {
    Team {
        id: id.into(),
        name: format!("{id} Name"),
        description: None,
        members: None,
        include: None,
        import: None,
        extensions: Extensions::default(),
    }
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
        vec![Step::<SharedWorkStepDefinition>::Work(WorkStep {
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
            Step::<SharedWorkStepDefinition>::Work(WorkStep {
                depends_on: None,
                definition: SharedWorkStepDefinition::Task(minimal_task("SharedTask", None)),
            }),
            Step::<SharedWorkStepDefinition>::Work(WorkStep {
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
    store.insert_role(role);
    store.insert_hook(hook);
    store.insert_team(team);
    store.insert_workflow(workflow);
    store.insert_shared_workflow(shared_workflow);
    store
}

/// Helper to build a Workflow (WorkflowDef<WorkStepDefinition>) inline.
#[allow(non_snake_case)]
fn WorkflowDef(id: &str, steps: Vec<Step<WorkStepDefinition>>) -> Workflow {
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

/// Helper to build a SharedWorkflow (WorkflowDef<SharedWorkStepDefinition>) inline.
#[allow(non_snake_case)]
fn WorkflowDef_shared(id: &str, steps: Vec<Step<SharedWorkStepDefinition>>) -> SharedWorkflow {
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

fn inode(path: &std::path::Path) -> u64 {
    use std::os::unix::fs::MetadataExt;
    fs::metadata(path).unwrap().ino()
}

// ---------------------------------------------------------------------------
// 9.1: Updated persist tests — now use atomic_persist(&ChangeSet)
// ---------------------------------------------------------------------------

#[test]
fn persist_full_store_creates_all_entity_files() {
    let tmp = tempdir();
    let root = tmp.join("repo");
    let substrate = RepoSubstrate::new(&root);
    let store = full_store();
    let cs = store.collect_changes();
    substrate.atomic_persist(&cs).unwrap();

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
    let store = full_store();
    let cs = store.collect_changes();
    substrate.atomic_persist(&cs).unwrap();

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
    let store = full_store();
    let cs = store.collect_changes();
    substrate.atomic_persist(&cs).unwrap();

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
    store.insert_workflow(wf);
    let cs = store.collect_changes();
    substrate.atomic_persist(&cs).unwrap();

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

// ---------------------------------------------------------------------------
// 9.2: End-to-end lifecycle: insert → collect → persist → reset → verify
// ---------------------------------------------------------------------------

#[test]
fn end_to_end_insert_persist_reset_verify_files_on_disk() {
    let tmp = tempdir();
    let root = tmp.join("repo");
    let substrate = RepoSubstrate::new(&root);

    let mut store = full_store();
    let cs = store.collect_changes();
    assert!(!cs.is_empty(), "store should have changes after insertions");

    substrate.atomic_persist(&cs).unwrap();
    drop(cs); // release borrow before reset_tracked

    store.reset_tracked();

    // After reset, collect_changes returns empty.
    let cs_after = store.collect_changes();
    assert!(
        cs_after.is_empty(),
        "collect_changes must return empty after reset_tracked"
    );

    // Files are on disk.
    assert!(root.join("roles/eng-lead.md").exists());
    assert!(root.join("teams/platform-team.md").exists());
    assert!(root.join("shared/hooks/UpdateJira.md").exists());
    assert!(root.join("workflows/Initiative/README.md").exists());
    assert!(root.join("shared/workflows/SharedInit/README.md").exists());

    cleanup(tmp);
}

// ---------------------------------------------------------------------------
// 9.3: Incremental update — only affected subtree changes (inode check)
// ---------------------------------------------------------------------------

#[test]
fn incremental_update_only_affected_subtree_changes() {
    let tmp = tempdir();
    let root = tmp.join("repo");
    let substrate = RepoSubstrate::new(&root);

    // First persist: role + team in separate directories.
    let mut store = EntityStore::new();
    store.insert_role(minimal_role("eng-lead"));
    store.insert_team(minimal_team("platform-team"));
    let cs = store.collect_changes();
    substrate.atomic_persist(&cs).unwrap();
    drop(cs);
    store.reset_tracked();

    let role_path = root.join("roles/eng-lead.md");
    let team_path = root.join("teams/platform-team.md");
    let role_inode_before = inode(&role_path);
    let team_inode_before = inode(&team_path);

    // Second persist: modify only the role — team is untouched.
    {
        let role = store.get_role_mut("eng-lead").unwrap();
        *role.name = "Engineering Lead".to_string();
    }
    let cs2 = store.collect_changes();
    substrate.atomic_persist(&cs2).unwrap();

    // Role file was rewritten — inode must differ.
    let role_inode_after = inode(&role_path);
    assert_ne!(
        role_inode_before, role_inode_after,
        "modified role file must have a new inode"
    );

    // Team file was hard-linked from the original — inode must be unchanged.
    let team_inode_after = inode(&team_path);
    assert_eq!(
        team_inode_before, team_inode_after,
        "unmodified team file must share inode (hard-linked, not re-written)"
    );

    // Content is correct.
    let content = fs::read_to_string(&role_path).unwrap();
    assert!(content.contains("Engineering Lead"));

    cleanup(tmp);
}

// ---------------------------------------------------------------------------
// 9.4: Failed persist — reset_tracked NOT called, ChangeSet remains intact
// ---------------------------------------------------------------------------

#[test]
fn failed_persist_leaves_changeset_intact_when_reset_not_called() {
    let tmp = tempdir();
    let root = tmp.join("repo");
    // Create root so we can make it read-only.
    fs::create_dir_all(&root).unwrap();
    let substrate = RepoSubstrate::new(&root);

    let mut store = EntityStore::new();
    store.insert_role(minimal_role("eng-lead"));
    let cs = store.collect_changes();
    assert_eq!(cs.len(), 1, "one change before persist");

    // Make root read-only so that creating roles.part/ inside it fails.
    make_readonly(&root);
    let result = substrate.atomic_persist(&cs);
    make_writable(&root); // restore before any assertions that might panic
    drop(cs);

    assert!(result.is_err(), "atomic_persist must fail with read-only root");

    // reset_tracked was NOT called — store still tracks the insertion.
    let cs2 = store.collect_changes();
    assert_eq!(
        cs2.len(),
        1,
        "ChangeSet must still contain 1 change after failed persist"
    );
    assert_eq!(cs2.changes[0].id, "eng-lead");

    cleanup(tmp);
}

#[cfg(unix)]
fn make_readonly(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o555)).unwrap();
}

#[cfg(unix)]
fn make_writable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}
