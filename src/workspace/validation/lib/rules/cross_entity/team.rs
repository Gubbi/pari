use std::collections::{HashSet, VecDeque};

use crate::{
    entity::{
        entities::{role::Role, team::Team},
        AnyEntityRef, Entity, EntityRef,
    },
    error::primitive::PrimitiveError,
    workspace::Workspace,
};

/// BFS cycle detection for `include` edges.
///
/// A cycle exists if the current team's ref appears in the transitive
/// closure of `include` team-keys.
pub async fn no_include_cycle(
    workspace: &Workspace,
    self_ref: EntityRef<Team>,
    include: Vec<(EntityRef<Team>, EntityRef<Role>)>,
) -> Vec<PrimitiveError> {
    let seeds: Vec<String> = include
        .into_iter()
        .map(|(team, _role)| team.id().to_owned())
        .collect();
    if cycle_exists(self_ref.id(), seeds, |id| {
        fetch_include_neighbors(workspace, id)
    })
    .await
    {
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
    workspace: &Workspace,
    self_ref: EntityRef<Team>,
    import: Vec<EntityRef<Team>>,
) -> Vec<PrimitiveError> {
    let seeds: Vec<String> = import.into_iter().map(|t| t.id().to_owned()).collect();
    if cycle_exists(self_ref.id(), seeds, |id| {
        fetch_import_neighbors(workspace, id)
    })
    .await
    {
        vec![PrimitiveError::workflow_graph_inconsistency(
            "team import graph contains a cycle",
            "import_cycle",
        )]
    } else {
        vec![]
    }
}

/// Pure BFS over a graph defined by `fetch_neighbors`. Returns `true`
/// iff `self_id` appears in the transitive closure of `seeds`.
///
/// `fetch_neighbors` is the only I/O hop — production wrappers close
/// over a workspace and resolve a team to read its outgoing edges;
/// unit tests close over a static adjacency map.
async fn cycle_exists<F, Fut>(self_id: &str, seeds: Vec<String>, mut fetch_neighbors: F) -> bool
where
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<Output = Vec<String>>,
{
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = seeds.into_iter().collect();

    while let Some(id) = queue.pop_front() {
        if id == self_id {
            return true;
        }
        if !visited.insert(id.clone()) {
            continue;
        }
        for next_id in fetch_neighbors(id).await {
            queue.push_back(next_id);
        }
    }
    false
}

/// Resolve a team and project its `include` field to neighbour ids.
/// Missing teams or missing fields contribute no neighbours.
async fn fetch_include_neighbors(workspace: &Workspace, team_id: String) -> Vec<String> {
    let any_ref = AnyEntityRef::Team(EntityRef::<Team>::new(team_id));
    let tracked = match workspace.resolve_any(any_ref).await {
        Ok(t) => t,
        Err(_) => return vec![],
    };
    Team::extract(&tracked)
        .and_then(|t| t.include.get().cloned().flatten())
        .map(|nested| {
            nested
                .into_iter()
                .map(|(team, _role)| team.id().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// Resolve a team and project its `import` field to neighbour ids.
async fn fetch_import_neighbors(workspace: &Workspace, team_id: String) -> Vec<String> {
    let any_ref = AnyEntityRef::Team(EntityRef::<Team>::new(team_id));
    let tracked = match workspace.resolve_any(any_ref).await {
        Ok(t) => t,
        Err(_) => return vec![],
    };
    Team::extract(&tracked)
        .and_then(|t| t.import.get().cloned().flatten())
        .map(|nested| nested.into_iter().map(|t| t.id().to_owned()).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    //! Unit coverage for the pure BFS traversal in `cycle_exists`.
    //! Production callers close over a workspace; the tests close
    //! over a static `HashMap<id, Vec<id>>` adjacency map. Same
    //! algorithm exercised in either case.

    use std::collections::HashMap;

    use super::*;

    /// Build a `fetch_neighbors`-shaped closure backed by a static
    /// adjacency map. Missing keys yield no neighbours.
    fn neighbors_from(
        graph: HashMap<&'static str, Vec<&'static str>>,
    ) -> impl FnMut(String) -> std::future::Ready<Vec<String>> {
        move |id: String| {
            let neighbors = graph
                .get(id.as_str())
                .map(|v| v.iter().map(|s| s.to_string()).collect())
                .unwrap_or_default();
            std::future::ready(neighbors)
        }
    }

    fn graph(
        edges: &[(&'static str, &[&'static str])],
    ) -> HashMap<&'static str, Vec<&'static str>> {
        edges.iter().map(|(k, vs)| (*k, vs.to_vec())).collect()
    }

    fn seeds(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|s| (*s).to_string()).collect()
    }

    #[tokio::test]
    async fn no_seeds_no_cycle() {
        let g = neighbors_from(graph(&[]));
        assert!(!cycle_exists("self", seeds(&[]), g).await);
    }

    #[tokio::test]
    async fn unrelated_graph_no_cycle() {
        // self="a"; graph: b→c→d. None of b/c/d hit a.
        let g = neighbors_from(graph(&[("b", &["c"]), ("c", &["d"])]));
        assert!(!cycle_exists("a", seeds(&["b"]), g).await);
    }

    #[tokio::test]
    async fn direct_cycle_via_seeds_detected() {
        // self="a"; seed includes a → cycle hit on first pop.
        let g = neighbors_from(graph(&[]));
        assert!(cycle_exists("a", seeds(&["a"]), g).await);
    }

    #[tokio::test]
    async fn one_hop_cycle_detected() {
        // self="a"; seed b; b→a. Two pops in.
        let g = neighbors_from(graph(&[("b", &["a"])]));
        assert!(cycle_exists("a", seeds(&["b"]), g).await);
    }

    #[tokio::test]
    async fn multi_hop_cycle_detected() {
        // self="a"; b→c→d→a.
        let g = neighbors_from(graph(&[("b", &["c"]), ("c", &["d"]), ("d", &["a"])]));
        assert!(cycle_exists("a", seeds(&["b"]), g).await);
    }

    #[tokio::test]
    async fn diamond_no_cycle() {
        // self="a"; b→c, b→d, c→e, d→e. No path returns to a.
        let g = neighbors_from(graph(&[("b", &["c", "d"]), ("c", &["e"]), ("d", &["e"])]));
        assert!(!cycle_exists("a", seeds(&["b"]), g).await);
    }

    #[tokio::test]
    async fn diamond_with_back_edge_to_self_detected() {
        // self="a"; b→c, b→d, d→a — d's back-edge is the cycle.
        let g = neighbors_from(graph(&[("b", &["c", "d"]), ("c", &[]), ("d", &["a"])]));
        assert!(cycle_exists("a", seeds(&["b"]), g).await);
    }

    #[tokio::test]
    async fn visited_prevents_infinite_loop_on_cycle_not_involving_self() {
        // self="a"; b ↔ c (mutual). Without a visited set this would
        // loop forever; the BFS must terminate with no-cycle since
        // neither b nor c is "a".
        let g = neighbors_from(graph(&[("b", &["c"]), ("c", &["b"])]));
        assert!(!cycle_exists("a", seeds(&["b"]), g).await);
    }

    #[tokio::test]
    async fn missing_neighbor_treated_as_dead_end() {
        // self="a"; seed b, but b has no entry in the adjacency map
        // (mirrors the production case where workspace.resolve_any
        // fails). BFS should treat as no neighbours and terminate.
        let g = neighbors_from(graph(&[]));
        assert!(!cycle_exists("a", seeds(&["b"]), g).await);
    }

    #[tokio::test]
    async fn multiple_seeds_each_explored() {
        // self="a"; seeds [b, c]; only c→a triggers the cycle.
        let g = neighbors_from(graph(&[("b", &[]), ("c", &["a"])]));
        assert!(cycle_exists("a", seeds(&["b", "c"]), g).await);
    }
}
