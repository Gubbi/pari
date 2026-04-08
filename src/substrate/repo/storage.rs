//! [`RepoSubstrate`] — atomic filesystem persistence via LCA-based staging.

use std::{
    collections::HashSet,
    fs,
    io,
    path::{Path, PathBuf},
};

use crate::{
    schema::{
        entities::{
            hook::Hook,
            relay::Relay,
            role::Role,
            task::Task,
            team::Team,
            workflow::{
                ReviewStep, SharedWorkflow, Step, TrackedSharedWorkStepDefinition,
                TrackedSharedWorkflow, TrackedStep, TrackedWorkStepDefinition, TrackedWorkflow,
                Workflow, WorkStep, WorkStepDefinition,
            },
        },
        types::{Artifact, Extensions, HooksMap, Raci, TaskStateEntry, WorkflowStateEntry},
    },
    substrate::{
        changeset::{ChangeOp, ChangeSet, EntityData},
        SubstrateError,
    },
};

use super::{lca::lca, render};

pub struct RepoSubstrate {
    root: PathBuf,
}

impl RepoSubstrate {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl RepoSubstrate {
    pub fn atomic_persist(&self, changeset: &ChangeSet<'_>) -> Result<(), Vec<SubstrateError>> {
        if changeset.is_empty() {
            return Ok(());
        }

        let mut errors: Vec<SubstrateError> = Vec::new();

        // 1. Collect all (relative_file_path, content_or_delete) entries.
        let file_ops = collect_file_ops(changeset);

        // 2. Compute LCA across all affected file paths.
        let path_strs: Vec<&str> = file_ops.iter().map(|(p, _)| p.as_str()).collect();
        let lca_rel = lca(&path_strs);

        let lca_dir = self.root.join(&lca_rel);
        let part_dir = part_path(&self.root, &lca_rel);
        let old_dir = old_path(&self.root, &lca_rel);

        // 3a. Clean up any stale staging dirs left by a previously crashed persist.
        for stale in [&part_dir, &old_dir] {
            if stale.exists() {
                if let Err(e) = fs::remove_dir_all(stale) {
                    errors.push(err(stale, &e));
                    return Err(errors);
                }
            }
        }

        // 3. Stage directory.
        if let Err(e) = fs::create_dir_all(&part_dir) {
            errors.push(err(&part_dir, &e));
            return Err(errors);
        }

        // 4. Hard-link unchanged files from the existing LCA dir into staging.
        if lca_dir.exists() {
            let paths_to_overwrite: HashSet<&str> = file_ops
                .iter()
                .map(|(p, _)| p.as_str())
                .collect();
            if let Err(e) = hard_link_dir(&lca_dir, &part_dir, &self.root, &paths_to_overwrite) {
                errors.push(err(&lca_dir, &e));
                let _ = fs::remove_dir_all(&part_dir);
                return Err(errors);
            }
        }

        // 5. Write added/modified files; skip removed (omission = deletion).
        for (rel_path, op) in &file_ops {
            match op {
                FileOp::Write(content) => {
                    // File path relative to the LCA dir.
                    let rel_to_lca = strip_prefix(rel_path, &lca_rel);
                    let dest = part_dir.join(rel_to_lca);
                    if let Err(e) = fs::create_dir_all(dest.parent().unwrap()) {
                        errors.push(err(&dest, &e));
                        continue;
                    }
                    // Remove hard link first to avoid writing to the shared inode.
                    if dest.exists() {
                        let _ = fs::remove_file(&dest);
                    }
                    if let Err(e) = fs::write(&dest, content) {
                        errors.push(err(&dest, &e));
                    }
                }
                FileOp::Remove => {
                    // Removal: just don't hard-link it (already omitted above).
                }
            }
        }

        if !errors.is_empty() {
            let _ = fs::remove_dir_all(&part_dir);
            return Err(errors);
        }

        // 6. Atomic swap.
        if lca_dir.exists() {
            if let Err(e) = fs::rename(&lca_dir, &old_dir) {
                errors.push(err(&lca_dir, &e));
                let _ = fs::remove_dir_all(&part_dir);
                return Err(errors);
            }
        }
        if let Err(e) = fs::rename(&part_dir, &lca_dir) {
            errors.push(err(&part_dir, &e));
            // Best-effort restore.
            if old_dir.exists() {
                let _ = fs::rename(&old_dir, &lca_dir);
            }
            return Err(errors);
        }
        if old_dir.exists() {
            let _ = fs::remove_dir_all(&old_dir);
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// File operation collection
// ---------------------------------------------------------------------------

enum FileOp {
    Write(Vec<u8>),
    Remove,
}

/// Map each `EntityChange` to one or more `(relative_file_path, FileOp)` entries.
fn collect_file_ops(changeset: &ChangeSet<'_>) -> Vec<(String, FileOp)> {
    let mut ops: Vec<(String, FileOp)> = Vec::new();

    for change in &changeset.changes {
        match &change.op {
            ChangeOp::Removed => {
                for path in entity_file_paths(&change.path, &change.id, None) {
                    ops.push((path, FileOp::Remove));
                }
            }
            ChangeOp::Added(data) | ChangeOp::Modified { entity: data, .. } => {
                for (path, content) in render_entity(&change.path, &change.id, data) {
                    ops.push((path, FileOp::Write(content.into_bytes())));
                }
            }
        }
    }

    ops
}

/// Render an entity to one or more (relative_file_path, content) pairs.
fn render_entity(path: &str, id: &str, data: &EntityData<'_>) -> Vec<(String, String)> {
    match data {
        EntityData::Role(t) => {
            let plain = to_plain_role(t);
            vec![(format!("{path}/{id}.md"), render::render_role(&plain))]
        }
        EntityData::Hook(t) => {
            let plain = to_plain_hook(t);
            vec![(format!("{path}/{id}.md"), render::render_hook(&plain))]
        }
        EntityData::Team(t) => {
            let plain = to_plain_team(t);
            vec![(format!("{path}/{id}.md"), render::render_team(&plain))]
        }
        EntityData::Workflow(t) => {
            let plain = to_plain_workflow(t);
            let mut entries = vec![(format!("{path}/README.md"), render::render_workflow_readme(&plain))];
            // Template files for task steps are handled by Task EntityChange entries.
            entries
        }
        EntityData::SharedWorkflow(t) => {
            let plain = to_plain_shared_workflow(t);
            vec![(format!("{path}/README.md"), render::render_workflow_readme(&plain))]
        }
        EntityData::Task(t) => {
            let plain = to_plain_task(t);
            let mut entries = vec![(format!("{path}/README.md"), render::render_task_readme(&plain))];
            if let Some(template) = &plain.artifact.template {
                entries.push((
                    format!("{path}/{}.template.md", plain.artifact.name),
                    template.clone(),
                ));
            }
            entries
        }
        EntityData::Relay(t) => {
            let plain = to_plain_relay(t);
            vec![(format!("{path}/README.md"), render::render_relay_readme(&plain))]
        }
    }
}

/// File paths affected by a Removed entity (without content).
fn entity_file_paths(path: &str, id: &str, _data: Option<&EntityData<'_>>) -> Vec<String> {
    // For simplicity, match the same pattern as render_entity but return paths only.
    // We don't know the artifact name for removed Tasks, so we remove README.md only.
    vec![match path {
        p if p.starts_with("roles") || p.starts_with("teams") || p.starts_with("shared/hooks") => {
            format!("{path}/{id}.md")
        }
        _ => format!("{path}/README.md"),
    }]
}

// ---------------------------------------------------------------------------
// Tracked → Plain conversions (private, used only for rendering)
// ---------------------------------------------------------------------------

fn to_plain_role(t: &crate::schema::entities::role::TrackedRole) -> Role {
    Role {
        id: (*t.id).clone(),
        name: (*t.name).clone(),
        purpose: (*t.purpose).clone(),
        traits: (*t.traits).clone(),
        extensions: (*t.extensions).clone(),
    }
}

fn to_plain_hook(t: &crate::schema::entities::hook::TrackedHook) -> Hook {
    Hook {
        id: (*t.id).clone(),
        name: (*t.name).clone(),
        description: (*t.description).clone(),
        instructions: (*t.instructions).clone(),
        inputs: (*t.inputs).clone(),
        extensions: (*t.extensions).clone(),
    }
}

fn to_plain_team(t: &crate::schema::entities::team::TrackedTeam) -> Team {
    Team {
        id: (*t.id).clone(),
        name: (*t.name).clone(),
        description: (*t.description).clone(),
        members: (*t.members).clone(),
        include: (*t.include).clone(),
        import: (*t.import).clone(),
        extensions: (*t.extensions).clone(),
    }
}

fn to_plain_task(t: &crate::schema::entities::task::TrackedTask) -> Task {
    Task {
        id: (*t.id).clone(),
        name: (*t.name).clone(),
        description: (*t.description).clone(),
        purpose: (*t.purpose).clone(),
        instructions: (*t.instructions).clone(),
        criteria: (*t.criteria).clone(),
        accountability: (*t.accountability).clone(),
        artifact: (*t.artifact).clone(),
        states: (*t.states).clone(),
        hooks: (*t.hooks).clone(),
        guidance: (*t.guidance).clone(),
        extensions: (*t.extensions).clone(),
    }
}

fn to_plain_relay(t: &crate::schema::entities::relay::TrackedRelay) -> Relay {
    Relay {
        id: (*t.id).clone(),
        name: (*t.name).clone(),
        description: (*t.description).clone(),
        purpose: (*t.purpose).clone(),
        accountability: (*t.accountability).clone(),
        delegates_to: (*t.delegates_to).clone(),
        briefing: (*t.briefing).clone(),
        debriefing: (*t.debriefing).clone(),
        state_map: (*t.state_map).clone(),
        hooks: (*t.hooks).clone(),
        guidance: (*t.guidance).clone(),
        extensions: (*t.extensions).clone(),
    }
}

fn to_plain_review_step(t: &crate::schema::entities::workflow::TrackedReviewStep) -> ReviewStep {
    ReviewStep {
        id: (*t.id).clone(),
        approver: (*t.approver).clone(),
        on_reject: (*t.on_reject).clone(),
    }
}

fn to_plain_workflow(t: &TrackedWorkflow) -> Workflow {
    Workflow {
        id: (*t.id).clone(),
        name: (*t.name).clone(),
        description: (*t.description).clone(),
        purpose: (*t.purpose).clone(),
        accountability: (*t.accountability).clone(),
        steps: t.steps.values().map(|s| match s {
            TrackedStep::Work(ws) => Step::Work(WorkStep {
                depends_on: (*ws.depends_on).clone(),
                definition: match &ws.definition {
                    TrackedWorkStepDefinition::Task(task) => {
                        WorkStepDefinition::Task(to_plain_task(task))
                    }
                    TrackedWorkStepDefinition::Relay(relay) => {
                        WorkStepDefinition::Relay(to_plain_relay(relay))
                    }
                    TrackedWorkStepDefinition::Workflow(inner) => {
                        WorkStepDefinition::Workflow(Box::new(to_plain_workflow(inner)))
                    }
                },
            }),
            TrackedStep::Review(rs) => Step::Review(to_plain_review_step(rs)),
        }).collect(),
        states: (*t.states).clone(),
        hooks: (*t.hooks).clone(),
        guidance: (*t.guidance).clone(),
        extensions: (*t.extensions).clone(),
    }
}

fn to_plain_shared_workflow(t: &TrackedSharedWorkflow) -> SharedWorkflow {
    use crate::schema::entities::workflow::{SharedWorkStepDefinition, SharedWorkflow};
    SharedWorkflow {
        id: (*t.id).clone(),
        name: (*t.name).clone(),
        description: (*t.description).clone(),
        purpose: (*t.purpose).clone(),
        accountability: (*t.accountability).clone(),
        steps: t.steps.values().map(|s| match s {
            TrackedStep::Work(ws) => Step::Work(WorkStep {
                depends_on: (*ws.depends_on).clone(),
                definition: match &ws.definition {
                    TrackedSharedWorkStepDefinition::Task(task) => {
                        SharedWorkStepDefinition::Task(to_plain_task(task))
                    }
                    TrackedSharedWorkStepDefinition::SharedWorkflow(inner) => {
                        SharedWorkStepDefinition::SharedWorkflow(Box::new(
                            to_plain_shared_workflow(inner),
                        ))
                    }
                },
            }),
            TrackedStep::Review(rs) => Step::Review(to_plain_review_step(rs)),
        }).collect(),
        states: (*t.states).clone(),
        hooks: (*t.hooks).clone(),
        guidance: (*t.guidance).clone(),
        extensions: (*t.extensions).clone(),
    }
}

// ---------------------------------------------------------------------------
// Filesystem helpers
// ---------------------------------------------------------------------------

/// Recursively hard-link all files in `src_dir` into `dst_dir`.
/// Files whose repo-relative path is in `skip` are omitted — the caller will
/// write them fresh.
///
/// EXDEV is not a concern here: the staging dir is always a sibling of the
/// LCA dir, so source and destination are guaranteed to be on the same
/// filesystem.
fn hard_link_dir(
    src_dir: &Path,
    dst_dir: &Path,
    root: &Path,
    skip: &HashSet<&str>,
) -> io::Result<()> {
    for entry in fs::read_dir(src_dir)? {
        let entry = entry?;
        let src = entry.path();
        let dst = dst_dir.join(entry.file_name());
        if src.is_dir() {
            fs::create_dir_all(&dst)?;
            hard_link_dir(&src, &dst, root, skip)?;
        } else {
            let rel = src.strip_prefix(root).unwrap_or(&src);
            let rel_str = rel.to_string_lossy();
            if !skip.contains(rel_str.as_ref()) {
                fs::hard_link(&src, &dst)?;
            }
        }
    }
    Ok(())
}

/// Strip `prefix/` from `path`.  If prefix is empty, return `path` unchanged.
fn strip_prefix<'a>(path: &'a str, prefix: &str) -> &'a str {
    if prefix.is_empty() {
        path
    } else {
        path.strip_prefix(&format!("{prefix}/")).unwrap_or(path)
    }
}

/// Build the `.part` staging path for a given LCA relative path under root.
/// e.g. root=`/repo`, lca=`"roles"` → `/repo/roles.part`
///       root=`/repo`, lca=`""` → `/repo.part`
fn part_path(root: &Path, lca_rel: &str) -> PathBuf {
    suffixed_sibling(root, lca_rel, ".part")
}

fn old_path(root: &Path, lca_rel: &str) -> PathBuf {
    suffixed_sibling(root, lca_rel, ".old")
}

fn suffixed_sibling(root: &Path, lca_rel: &str, suffix: &str) -> PathBuf {
    if lca_rel.is_empty() {
        let name = root
            .file_name()
            .map(|n| format!("{}{suffix}", n.to_string_lossy()))
            .unwrap_or_else(|| format!("repo{suffix}"));
        root.parent().unwrap_or(root).join(name)
    } else {
        let name = lca_rel
            .rsplit('/')
            .next()
            .map(|n| format!("{n}{suffix}"))
            .unwrap_or_else(|| format!("dir{suffix}"));
        root.join(lca_rel).parent().unwrap_or(root).join(name)
    }
}

fn err(path: &Path, e: &impl std::fmt::Display) -> SubstrateError {
    SubstrateError {
        path: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fixtures::{role::minimal_role, team::minimal_team, workflow::minimal_workflow},
        schema::store::EntityStore,
    };
    use std::fs;

    // --- 12.1: RepoSubstrate::new tests ---

    #[test]
    fn new_accepts_arbitrary_path() {
        let tmp = tempdir();
        let substrate = RepoSubstrate::new(tmp.join("my-root"));
        drop(substrate);
        cleanup(tmp);
    }

    // --- 8.3: atomic_persist behaviour ---

    #[test]
    fn empty_changeset_is_noop() {
        let tmp = tempdir();
        let root = tmp.join("repo");
        let substrate = RepoSubstrate::new(&root);
        let store = EntityStore::new();
        let cs = store.collect_changes();
        substrate.atomic_persist(&cs).unwrap();
        // Root must not have been created for an empty changeset.
        assert!(!root.exists(), "root should not be created for empty changeset");
        cleanup(tmp);
    }

    #[test]
    fn initial_persist_creates_role_file() {
        let tmp = tempdir();
        let root = tmp.join("repo");
        let substrate = RepoSubstrate::new(&root);
        let mut store = EntityStore::new();
        store.insert_role(minimal_role("eng-lead"));
        let cs = store.collect_changes();
        substrate.atomic_persist(&cs).unwrap();
        assert!(root.join("roles/eng-lead.md").exists(), "roles/eng-lead.md should exist");
        cleanup(tmp);
    }

    #[test]
    fn initial_persist_creates_workflow_readme_and_task_readme() {
        let tmp = tempdir();
        let root = tmp.join("repo");
        let substrate = RepoSubstrate::new(&root);
        let mut store = EntityStore::new();
        store.insert_workflow(minimal_workflow("Initiative", "WriteProposal"));
        let cs = store.collect_changes();
        substrate.atomic_persist(&cs).unwrap();
        assert!(root.join("workflows/Initiative/README.md").exists());
        assert!(root.join("workflows/Initiative/WriteProposal/README.md").exists());
        cleanup(tmp);
    }

    #[test]
    fn no_part_dir_remains_after_successful_persist() {
        let tmp = tempdir();
        let root = tmp.join("repo");
        let substrate = RepoSubstrate::new(&root);
        let mut store = EntityStore::new();
        store.insert_role(minimal_role("eng-lead"));
        let cs = store.collect_changes();
        substrate.atomic_persist(&cs).unwrap();
        // No .part or .old remnants anywhere under tmp.
        assert!(!tmp.join("repo.part").exists());
        assert!(!tmp.join("repo.old").exists());
        assert!(!root.join("roles.part").exists());
        assert!(!root.join("roles.old").exists());
        cleanup(tmp);
    }

    #[test]
    fn second_persist_of_unrelated_entity_preserves_unchanged_file_inode() {
        let tmp = tempdir();
        let root = tmp.join("repo");
        let substrate = RepoSubstrate::new(&root);

        // First persist: role + team.
        let mut store = EntityStore::new();
        store.insert_role(minimal_role("eng-lead"));
        store.insert_team(minimal_team("platform-team"));
        let cs = store.collect_changes();
        substrate.atomic_persist(&cs).unwrap();
        store.reset_tracked();

        let team_path = root.join("teams/platform-team.md");
        let inode_before = inode(&team_path);

        // Second persist: modify only the role — team is untouched.
        {
            let role = store.get_role_mut("eng-lead").unwrap();
            *role.name = "Engineering Lead".to_string();
        }
        let cs2 = store.collect_changes();
        substrate.atomic_persist(&cs2).unwrap();

        // The team file was hard-linked from the original, so inode is unchanged.
        let inode_after = inode(&team_path);
        assert_eq!(
            inode_before, inode_after,
            "unchanged file should share inode (was hard-linked, not re-written)"
        );

        cleanup(tmp);
    }

    // --- 8.7: crash-safe staging (stale .part/.old cleanup) ---

    #[test]
    fn stale_part_dir_is_cleaned_up_before_new_persist() {
        let tmp = tempdir();
        let root = tmp.join("repo");
        let substrate = RepoSubstrate::new(&root);

        // First persist so roles/ exists.
        let mut store = EntityStore::new();
        store.insert_role(minimal_role("eng-lead"));
        let cs = store.collect_changes();
        substrate.atomic_persist(&cs).unwrap();
        store.reset_tracked();

        // Simulate a previous crash that left a stale roles.part/ dir.
        let stale_part = root.join("roles.part");
        fs::create_dir_all(&stale_part).unwrap();
        fs::write(stale_part.join("garbage.md"), "stale content from crashed persist").unwrap();
        assert!(stale_part.exists(), "precondition: stale .part dir exists");

        // Second persist into the same LCA — should clean up stale .part and succeed.
        store.insert_role(minimal_role("pm"));
        let cs2 = store.collect_changes();
        substrate.atomic_persist(&cs2).unwrap();

        assert!(!root.join("roles.part").exists(), "stale .part dir must be cleaned up");
        assert!(root.join("roles/pm.md").exists(), "new entity must be persisted");
        assert!(root.join("roles/eng-lead.md").exists(), "original entity must be preserved");
        cleanup(tmp);
    }

    #[test]
    fn stale_old_dir_is_cleaned_up_before_new_persist() {
        let tmp = tempdir();
        let root = tmp.join("repo");
        let substrate = RepoSubstrate::new(&root);

        // First persist so roles/ exists.
        let mut store = EntityStore::new();
        store.insert_role(minimal_role("eng-lead"));
        let cs = store.collect_changes();
        substrate.atomic_persist(&cs).unwrap();
        store.reset_tracked();

        // Simulate a previous crash that completed the first rename (lca → lca.old)
        // but died before deleting lca.old.
        let stale_old = root.join("roles.old");
        fs::create_dir_all(&stale_old).unwrap();
        fs::write(stale_old.join("stale.md"), "old content from crashed persist").unwrap();
        assert!(stale_old.exists(), "precondition: stale .old dir exists");

        // Next persist should clean up stale .old and succeed.
        {
            let role = store.get_role_mut("eng-lead").unwrap();
            *role.name = "Engineering Lead".to_string();
        }
        let cs2 = store.collect_changes();
        substrate.atomic_persist(&cs2).unwrap();

        assert!(!root.join("roles.old").exists(), "stale .old dir must be cleaned up");
        let content = fs::read_to_string(root.join("roles/eng-lead.md")).unwrap();
        assert!(content.contains("Engineering Lead"), "updated content must be present");
        cleanup(tmp);
    }

    #[test]
    fn second_persist_updates_modified_file_content() {
        let tmp = tempdir();
        let root = tmp.join("repo");
        let substrate = RepoSubstrate::new(&root);

        let mut store = EntityStore::new();
        store.insert_role(minimal_role("eng-lead"));
        let cs = store.collect_changes();
        substrate.atomic_persist(&cs).unwrap();
        store.reset_tracked();

        {
            let role = store.get_role_mut("eng-lead").unwrap();
            *role.name = "Engineering Lead".to_string();
        }
        let cs2 = store.collect_changes();
        substrate.atomic_persist(&cs2).unwrap();

        let content = fs::read_to_string(root.join("roles/eng-lead.md")).unwrap();
        assert!(content.contains("Engineering Lead"), "updated name should be in file");
        cleanup(tmp);
    }

    // --- helpers ---

    fn inode(path: &std::path::Path) -> u64 {
        use std::os::unix::fs::MetadataExt;
        fs::metadata(path).unwrap().ino()
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
