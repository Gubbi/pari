//! [`RepoSubstrate`] — atomic filesystem persistence via sibling `.part/` directory.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    schema::entities::workflow::{Step, WorkStepDefinition},
    substrate::{
        repo::render::{
            render_hook, render_relay_readme, render_role, render_task_readme, render_team,
            render_workflow_readme,
        },
        EntityStore, Substrate, SubstrateError,
    },
};

pub struct RepoSubstrate {
    root: PathBuf,
}

impl RepoSubstrate {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl Substrate for RepoSubstrate {
    fn persist(&self, store: &EntityStore) -> Result<(), Vec<SubstrateError>> {
        // Write to a sibling .part/ directory, then atomically rename on success.
        let part = sibling_part_dir(&self.root);

        // If a stale .part/ remains from a prior crash, remove it.
        if part.exists() {
            let _ = fs::remove_dir_all(&part);
        }

        // Always create the .part/ directory up front.
        if let Err(e) = fs::create_dir_all(&part) {
            return Err(vec![SubstrateError {
                path: part.to_string_lossy().into_owned(),
                message: format!("failed to create temp directory: {e}"),
            }]);
        }

        let mut errors: Vec<SubstrateError> = Vec::new();

        // --- collect all (relative_path, content) pairs ---
        let mut files: Vec<(PathBuf, String)> = Vec::new();

        for role in store.roles.values() {
            files.push((
                PathBuf::from(format!("roles/{}.md", role.id)),
                render_role(role),
            ));
        }

        for team in store.teams.values() {
            files.push((
                PathBuf::from(format!("teams/{}.md", team.id)),
                render_team(team),
            ));
        }

        for hook in store.hooks.values() {
            files.push((
                PathBuf::from(format!("shared/hooks/{}.md", hook.id)),
                render_hook(hook),
            ));
        }

        for workflow in store.shared_workflows.values() {
            let wf_dir = format!("shared/workflows/{}", workflow.id);
            files.push((
                PathBuf::from(format!("{wf_dir}/README.md")),
                render_workflow_readme(workflow),
            ));
            collect_shared_step_files(&wf_dir, &workflow.steps, &mut files);
        }

        for workflow in store.workflows.values() {
            let wf_dir = format!("workflows/{}", workflow.id);
            files.push((
                PathBuf::from(format!("{wf_dir}/README.md")),
                render_workflow_readme(workflow),
            ));
            collect_step_files(&wf_dir, &workflow.steps, &mut files);
        }

        // --- write all files to .part/ directory ---
        for (rel, content) in &files {
            let dest = part.join(rel);
            if let Some(parent) = dest.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    errors.push(SubstrateError {
                        path: rel.to_string_lossy().into_owned(),
                        message: format!("failed to create directory: {e}"),
                    });
                    continue;
                }
            }
            if let Err(e) = fs::write(&dest, content) {
                errors.push(SubstrateError {
                    path: rel.to_string_lossy().into_owned(),
                    message: format!("failed to write file: {e}"),
                });
            }
        }

        if !errors.is_empty() {
            let _ = fs::remove_dir_all(&part);
            return Err(errors);
        }

        // --- atomic rename ---
        if self.root.exists() {
            if let Err(e) = fs::remove_dir_all(&self.root) {
                let _ = fs::remove_dir_all(&part);
                return Err(vec![SubstrateError {
                    path: self.root.to_string_lossy().into_owned(),
                    message: format!("failed to remove old root: {e}"),
                }]);
            }
        }

        if let Err(e) = fs::rename(&part, &self.root) {
            let _ = fs::remove_dir_all(&part);
            return Err(vec![SubstrateError {
                path: self.root.to_string_lossy().into_owned(),
                message: format!("failed to rename temp dir to root: {e}"),
            }]);
        }

        Ok(())
    }
}

fn sibling_part_dir(root: &Path) -> PathBuf {
    let name = root.file_name().map_or_else(
        || ".part".to_string(),
        |n| format!("{}.part", n.to_string_lossy()),
    );
    match root.parent() {
        Some(parent) => parent.join(name),
        None => PathBuf::from(name),
    }
}

/// Recursively collect (`relative_path`, content) pairs for a `Workflow`'s steps.
fn collect_step_files(
    parent_dir: &str,
    steps: &[crate::schema::entities::workflow::Step],
    files: &mut Vec<(PathBuf, String)>,
) {
    for step in steps {
        match step {
            Step::Review(_) => {
                // ReviewStep: no directory created; represented only in parent README frontmatter.
            }
            Step::Work(ws) => match &ws.definition {
                WorkStepDefinition::Task(task) => {
                    let dir = format!("{parent_dir}/{}", task.id);
                    files.push((
                        PathBuf::from(format!("{dir}/README.md")),
                        render_task_readme(task),
                    ));
                    if let Some(template_content) = &task.artifact.template {
                        files.push((
                            PathBuf::from(format!("{dir}/{}.template.md", task.artifact.name)),
                            template_content.clone(),
                        ));
                    }
                }
                WorkStepDefinition::Relay(relay) => {
                    let dir = format!("{parent_dir}/{}", relay.id);
                    files.push((
                        PathBuf::from(format!("{dir}/README.md")),
                        render_relay_readme(relay),
                    ));
                }
                WorkStepDefinition::Workflow(wf) => {
                    let dir = format!("{parent_dir}/{}", wf.id);
                    files.push((
                        PathBuf::from(format!("{dir}/README.md")),
                        render_workflow_readme(wf),
                    ));
                    collect_step_files(&dir, &wf.steps, files);
                }
            },
        }
    }
}

/// Recursively collect (`relative_path`, content) pairs for a `SharedWorkflow`'s steps.
fn collect_shared_step_files(
    parent_dir: &str,
    steps: &[crate::schema::entities::workflow::SharedStep],
    files: &mut Vec<(PathBuf, String)>,
) {
    use crate::schema::entities::workflow::{SharedStep, SharedWorkStepDefinition};
    for step in steps {
        match step {
            SharedStep::Review(_) => {}
            SharedStep::Work(ws) => match &ws.definition {
                SharedWorkStepDefinition::Task(task) => {
                    let dir = format!("{parent_dir}/{}", task.id);
                    files.push((
                        PathBuf::from(format!("{dir}/README.md")),
                        render_task_readme(task),
                    ));
                    if let Some(template_content) = &task.artifact.template {
                        files.push((
                            PathBuf::from(format!("{dir}/{}.template.md", task.artifact.name)),
                            template_content.clone(),
                        ));
                    }
                }
                SharedWorkStepDefinition::SharedWorkflow(wf) => {
                    let dir = format!("{parent_dir}/{}", wf.id);
                    files.push((
                        PathBuf::from(format!("{dir}/README.md")),
                        render_workflow_readme(wf),
                    ));
                    collect_shared_step_files(&dir, &wf.steps, files);
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        schema::{
            entities::workflow::{ReviewStep, Step, WorkStep, WorkStepDefinition, Workflow},
            types::{Artifact, Extensions, Raci, TaskStateEntry, WorkflowStateEntry},
        },
        substrate::Substrate,
    };

    fn raci() -> Raci {
        Raci {
            responsible: "eng-lead".to_string(),
            accountable: "pm".to_string(),
            consulted: vec![],
            informed: vec![],
        }
    }

    fn minimal_task(id: &str) -> crate::schema::entities::task::Task {
        crate::schema::entities::task::Task {
            id: id.into(),
            name: format!("{id} Name"),
            description: None,
            purpose: "Test purpose.".to_string(),
            instructions: vec!["Do the thing.".to_string()],
            criteria: vec!["Thing done.".to_string()],
            accountability: None,
            artifact: Artifact {
                name: "output".to_string(),
                template: None,
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
                    semantic: Some(crate::schema::types::TaskSemantic::Complete),
                },
            ],
            hooks: None,
            guidance: None,
            extensions: Extensions::default(),
        }
    }

    fn minimal_workflow(id: &str, task_id: &str) -> Workflow {
        Workflow {
            id: id.into(),
            name: format!("{id} Workflow"),
            description: None,
            purpose: "Test workflow.".to_string(),
            accountability: raci(),
            steps: vec![Step::Work(WorkStep {
                depends_on: None,
                definition: WorkStepDefinition::Task(minimal_task(task_id)),
            })],
            states: vec![
                WorkflowStateEntry {
                    id: "Open".to_string(),
                    description: "Open.".to_string(),
                    semantic: None,
                },
                WorkflowStateEntry {
                    id: "Closed".to_string(),
                    description: "Closed.".to_string(),
                    semantic: Some(crate::schema::types::WorkflowSemantic::Complete),
                },
            ],
            hooks: None,
            guidance: None,
            extensions: Extensions::default(),
        }
    }

    fn empty_store() -> EntityStore {
        EntityStore::new()
    }

    // --- 12.1: RepoSubstrate::new and persist tests ---

    #[test]
    fn new_accepts_arbitrary_path() {
        let tmp = tempdir();
        let substrate = RepoSubstrate::new(tmp.join("my-root"));
        drop(substrate);
        cleanup(tmp);
    }

    #[test]
    fn persist_empty_store_creates_root_directory() {
        let tmp = tempdir();
        let root = tmp.join("repo");
        let substrate = RepoSubstrate::new(&root);
        substrate.persist(&empty_store()).unwrap();
        assert!(root.exists(), "root should be created");
        cleanup(tmp);
    }

    #[test]
    fn persist_no_part_dir_remains_on_success() {
        let tmp = tempdir();
        let root = tmp.join("repo");
        let part = tmp.join("repo.part");
        let substrate = RepoSubstrate::new(&root);
        substrate.persist(&empty_store()).unwrap();
        assert!(!part.exists(), ".part dir should be removed after success");
        cleanup(tmp);
    }

    #[test]
    fn persist_role_written_as_flat_file() {
        let tmp = tempdir();
        let root = tmp.join("repo");
        let substrate = RepoSubstrate::new(&root);
        let role = crate::schema::entities::role::Role {
            id: "eng-lead".into(),
            name: "Engineering Lead".to_string(),
            purpose: "Drive technical direction.".to_string(),
            traits: None,
            extensions: Extensions::default(),
        };
        let mut store = empty_store();
        store.roles.insert(role.id.to_string(), role);
        substrate.persist(&store).unwrap();
        let path = root.join("roles/eng-lead.md");
        assert!(path.exists(), "roles/eng-lead.md should exist");
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("id: eng-lead"));
        cleanup(tmp);
    }

    #[test]
    fn persist_workflow_written_as_directory_with_readme() {
        let tmp = tempdir();
        let root = tmp.join("repo");
        let substrate = RepoSubstrate::new(&root);
        let wf = minimal_workflow("Initiative", "WriteProposal");
        let mut store = empty_store();
        store.workflows.insert(wf.id.to_string(), wf);
        substrate.persist(&store).unwrap();
        assert!(root.join("workflows/Initiative/README.md").exists());
        cleanup(tmp);
    }

    #[test]
    fn persist_embedded_task_written_under_workflow_dir() {
        let tmp = tempdir();
        let root = tmp.join("repo");
        let substrate = RepoSubstrate::new(&root);
        let wf = minimal_workflow("Initiative", "WriteProposal");
        let mut store = empty_store();
        store.workflows.insert(wf.id.to_string(), wf);
        substrate.persist(&store).unwrap();
        assert!(root
            .join("workflows/Initiative/WriteProposal/README.md")
            .exists());
        cleanup(tmp);
    }

    #[test]
    fn persist_review_step_creates_no_directory() {
        let tmp = tempdir();
        let root = tmp.join("repo");
        let substrate = RepoSubstrate::new(&root);
        let mut wf = minimal_workflow("Initiative", "WriteProposal");
        wf.steps.push(Step::Review(ReviewStep {
            id: "Approve".to_string(),
            approver: "pm".to_string(),
            on_reject: "WriteProposal".to_string(),
        }));
        let mut store = empty_store();
        store.workflows.insert(wf.id.to_string(), wf);
        substrate.persist(&store).unwrap();
        assert!(!root.join("workflows/Initiative/Approve").exists());
        cleanup(tmp);
    }

    #[test]
    fn persist_task_with_template_writes_template_file() {
        let tmp = tempdir();
        let root = tmp.join("repo");
        let substrate = RepoSubstrate::new(&root);
        let mut task = minimal_task("WriteProposal");
        task.artifact = Artifact {
            name: "proposal".to_string(),
            template: Some("# Proposal\n\nFill this in.".to_string()),
        };
        let wf = Workflow {
            steps: vec![Step::Work(WorkStep {
                depends_on: None,
                definition: WorkStepDefinition::Task(task),
            })],
            ..minimal_workflow("Initiative", "WriteProposal")
        };
        let mut store = empty_store();
        store.workflows.insert(wf.id.to_string(), wf);
        substrate.persist(&store).unwrap();
        let template_path = root.join("workflows/Initiative/WriteProposal/proposal.template.md");
        assert!(template_path.exists(), "proposal.template.md should exist");
        let content = fs::read_to_string(&template_path).unwrap();
        assert!(content.contains("# Proposal"));
        cleanup(tmp);
    }

    fn tempdir() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "pari-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn cleanup(path: std::path::PathBuf) {
        let _ = fs::remove_dir_all(&path);
    }
}
