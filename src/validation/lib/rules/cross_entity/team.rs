use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    entity::{
        entities::{role::Role, team::Team},
        AnyEntityRef, Entity, EntityRef,
    },
    error::primitive::PrimitiveError,
    workspace::EntityClient,
};

/// BFS cycle detection for `include` edges.
///
/// A cycle exists if the current team's ref appears in the transitive
/// closure of `include` keys.
pub async fn no_include_cycle(
    self_ref: EntityRef<Team>,
    include: HashMap<EntityRef<Team>, EntityRef<Role>>,
) -> Vec<PrimitiveError> {
    if cycle_exists_include(self_ref, include).await {
        vec![PrimitiveError::workflow_graph_inconsistency(
            "team include graph contains a cycle",
            "include_cycle",
        )]
    } else {
        vec![]
    }
}

/// BFS cycle detection for `import` edges.
pub async fn no_import_cycle(
    self_ref: EntityRef<Team>,
    import: Vec<EntityRef<Team>>,
) -> Vec<PrimitiveError> {
    if cycle_exists_import(self_ref, import).await {
        vec![PrimitiveError::workflow_graph_inconsistency(
            "team import graph contains a cycle",
            "import_cycle",
        )]
    } else {
        vec![]
    }
}

async fn cycle_exists_include(
    self_ref: EntityRef<Team>,
    include: HashMap<EntityRef<Team>, EntityRef<Role>>,
) -> bool {
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<EntityRef<Team>> = include.into_keys().collect();

    while let Some(team_ref) = queue.pop_front() {
        let id = team_ref.id().to_owned();
        if id == self_ref.id() {
            return true;
        }
        if !visited.insert(id) {
            continue;
        }
        let any_ref = AnyEntityRef::Team(team_ref);
        let tracked = match EntityClient::resolve(any_ref).await {
            Ok(t) => t,
            Err(_) => continue,
        };
        if let Some(t) = Team::extract(&tracked) {
            if let Some(Some(nested_include)) = t.include.get() {
                for next_ref in nested_include.keys() {
                    queue.push_back(next_ref.clone());
                }
            }
        }
    }
    false
}

async fn cycle_exists_import(self_ref: EntityRef<Team>, import: Vec<EntityRef<Team>>) -> bool {
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<EntityRef<Team>> = import.into_iter().collect();

    while let Some(team_ref) = queue.pop_front() {
        let id = team_ref.id().to_owned();
        if id == self_ref.id() {
            return true;
        }
        if !visited.insert(id) {
            continue;
        }
        let any_ref = AnyEntityRef::Team(team_ref);
        let tracked = match EntityClient::resolve(any_ref).await {
            Ok(t) => t,
            Err(_) => continue,
        };
        if let Some(t) = Team::extract(&tracked) {
            if let Some(Some(nested_import)) = t.import.get() {
                for next_ref in nested_import {
                    queue.push_back(next_ref.clone());
                }
            }
        }
    }
    false
}
