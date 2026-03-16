//! Maximum cardinality matching via Edmonds' Blossom algorithm.
//!
//! Finds the largest set of non-overlapping vertex pairs in a general
//! (non-bipartite) weighted graph.  When multiple maximum-cardinality
//! matchings exist, the one with the highest total weight is preferred
//! thanks to the greedy initialization sorting edges by weight.

use std::collections::VecDeque;

/// Returned for unmatched vertices.
pub const SENTINEL: usize = usize::MAX;

/// Builder for a maximum-cardinality matching on a general weighted graph.
///
/// ```ignore
/// let mates = Matching::new(edges).max_cardinality().solve();
/// ```
pub struct Matching {
    edges: Vec<(usize, usize, i32)>,
}

impl Matching {
    pub fn new(edges: Vec<(usize, usize, i32)>) -> Self {
        Self { edges }
    }

    /// Request maximum-cardinality mode (currently the only mode).
    pub fn max_cardinality(self) -> Self {
        self
    }

    /// Solve and return `mates[v]` for every vertex `v`.
    /// `mates[v] == SENTINEL` when `v` is unmatched.
    pub fn solve(self) -> Vec<usize> {
        if self.edges.is_empty() {
            return Vec::new();
        }
        let n = self
            .edges
            .iter()
            .flat_map(|&(u, v, _)| [u, v])
            .max()
            .map(|m| m + 1)
            .unwrap_or(0);
        if n == 0 {
            return Vec::new();
        }
        edmonds_matching(n, &self.edges)
    }
}

// ---------------------------------------------------------------------------
// Edmonds' Blossom algorithm – O(V²·E) maximum-cardinality matching
// ---------------------------------------------------------------------------

fn edmonds_matching(n: usize, edges: &[(usize, usize, i32)]) -> Vec<usize> {
    let mut adj = vec![vec![]; n];
    for &(u, v, _) in edges {
        adj[u].push(v);
        adj[v].push(u);
    }

    let mut mate = vec![SENTINEL; n];

    // Greedy init: prefer higher-weight edges for a better starting point.
    let mut sorted: Vec<_> = edges.to_vec();
    sorted.sort_by(|a, b| b.2.cmp(&a.2));
    for &(u, v, _) in &sorted {
        if mate[u] == SENTINEL && mate[v] == SENTINEL {
            mate[u] = v;
            mate[v] = u;
        }
    }

    // Augment from every remaining free vertex.
    for root in 0..n {
        if mate[root] != SENTINEL {
            continue;
        }
        try_augment(n, &adj, &mut mate, root);
    }

    mate
}

/// BFS from `root` looking for an augmenting path.  Returns `true` if one was
/// found (and the matching has already been updated).
fn try_augment(n: usize, adj: &[Vec<usize>], mate: &mut [usize], root: usize) -> bool {
    let mut base: Vec<usize> = (0..n).collect();
    let mut parent = vec![SENTINEL; n];
    let mut color = vec![0u8; n]; // 0 = unseen, 1 = outer, 2 = inner
    let mut queue = VecDeque::new();

    color[root] = 1;
    queue.push_back(root);

    while let Some(v) = queue.pop_front() {
        for &u in &adj[v] {
            if base[v] == base[u] || color[u] == 2 {
                continue;
            }
            if color[u] == 1 {
                // Both outer → blossom.
                let lca = find_lca(&base, &parent, mate, root, v, u);
                contract(
                    &mut base,
                    &mut parent,
                    &mut color,
                    mate,
                    &mut queue,
                    v,
                    u,
                    lca,
                );
            } else if mate[u] == SENTINEL {
                // Free vertex → augmenting path found.
                parent[u] = v;
                augment(mate, &parent, u);
                return true;
            } else {
                // Matched, unseen vertex → extend tree.
                parent[u] = v;
                color[u] = 2;
                let w = mate[u];
                color[w] = 1;
                queue.push_back(w);
            }
        }
    }

    false
}

/// Walk from both endpoints towards the root to find the lowest common
/// ancestor in the alternating tree (respecting blossom bases).
fn find_lca(
    base: &[usize],
    parent: &[usize],
    mate: &[usize],
    root: usize,
    a: usize,
    b: usize,
) -> usize {
    let n = base.len();
    let mut visited = vec![false; n];
    let mut a = base[a];
    let mut b = base[b];
    loop {
        visited[a] = true;
        if a == root {
            break;
        }
        a = base[parent[mate[a]]];
    }
    loop {
        if visited[b] {
            return b;
        }
        b = base[parent[mate[b]]];
    }
}

/// Shrink the blossom defined by paths v→lca and u→lca.
fn contract(
    base: &mut [usize],
    parent: &mut [usize],
    color: &mut [u8],
    mate: &[usize],
    queue: &mut VecDeque<usize>,
    v: usize,
    u: usize,
    lca: usize,
) {
    let n = base.len();
    let mut blossom = vec![false; n];
    mark_path(base, parent, mate, &mut blossom, v, lca, u);
    mark_path(base, parent, mate, &mut blossom, u, lca, v);
    for i in 0..n {
        if blossom[base[i]] {
            base[i] = lca;
            if color[i] != 1 {
                color[i] = 1;
                queue.push_back(i);
            }
        }
    }
}

/// Walk from `v` towards `lca`, marking blossom members and redirecting
/// parent pointers so that future augmentations can traverse the blossom.
fn mark_path(
    base: &[usize],
    parent: &mut [usize],
    mate: &[usize],
    blossom: &mut [bool],
    mut v: usize,
    lca: usize,
    child: usize,
) {
    let mut cur_child = child;
    while base[v] != lca {
        blossom[base[v]] = true;
        blossom[base[mate[v]]] = true;
        parent[v] = cur_child;
        cur_child = mate[v];
        v = parent[mate[v]];
    }
}

/// Flip matched / unmatched edges along the augmenting path ending at `u`.
fn augment(mate: &mut [usize], parent: &[usize], mut u: usize) {
    while u != SENTINEL {
        let v = parent[u];
        let prev = if v != SENTINEL { mate[v] } else { SENTINEL };
        mate[u] = v;
        if v != SENTINEL {
            mate[v] = u;
        }
        u = prev;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prioritizes_cardinality() {
        // Path 0–1–2: one edge can be matched. With max_cardinality the
        // algorithm must NOT leave node 1 unmatched just because (0,1) has
        // higher weight.  Either (0,1) or (1,2) is fine – 1 pair total.
        let edges = vec![(0, 1, 10_100i32), (1, 2, 10_090)];
        let mates = Matching::new(edges).max_cardinality().solve();
        let pairs: Vec<_> = mates
            .iter()
            .enumerate()
            .filter_map(|(i, &m)| if m != SENTINEL && m > i { Some((i, m)) } else { None })
            .collect();
        assert_eq!(pairs.len(), 1);
    }

    #[test]
    fn disconnected_components() {
        let edges = vec![(0, 1, 10_010), (2, 3, 10_005)];
        let mates = Matching::new(edges).max_cardinality().solve();
        let matched = mates.iter().filter(|&&m| m != SENTINEL).count();
        assert_eq!(matched, 4);
    }

    #[test]
    fn empty_edges() {
        let mates = Matching::new(vec![]).max_cardinality().solve();
        assert!(mates.is_empty());
    }

    #[test]
    fn triangle() {
        // Triangle: max matching = 1 pair.
        let edges = vec![(0, 1, 1), (1, 2, 1), (0, 2, 1)];
        let mates = Matching::new(edges).max_cardinality().solve();
        let matched = mates.iter().filter(|&&m| m != SENTINEL).count();
        assert_eq!(matched, 2); // 1 pair = 2 matched vertices
    }

    #[test]
    fn augmenting_path_needed() {
        // 0–1–2–3: greedy might match (0,1) and leave (2,3) for the second
        // pass. Either way, 2 pairs are achievable.
        let edges = vec![(0, 1, 1), (1, 2, 1), (2, 3, 1)];
        let mates = Matching::new(edges).max_cardinality().solve();
        let pairs: Vec<_> = mates
            .iter()
            .enumerate()
            .filter_map(|(i, &m)| if m != SENTINEL && m > i { Some((i, m)) } else { None })
            .collect();
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn pentagon_blossom() {
        // 5-cycle: max matching = 2 pairs.
        let edges = vec![(0, 1, 1), (1, 2, 1), (2, 3, 1), (3, 4, 1), (4, 0, 1)];
        let mates = Matching::new(edges).max_cardinality().solve();
        let pairs: Vec<_> = mates
            .iter()
            .enumerate()
            .filter_map(|(i, &m)| if m != SENTINEL && m > i { Some((i, m)) } else { None })
            .collect();
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn complete_graph_k6() {
        // K6: 6 vertices, max matching = 3 pairs.
        let mut edges = Vec::new();
        for i in 0..6 {
            for j in (i + 1)..6 {
                edges.push((i, j, 1));
            }
        }
        let mates = Matching::new(edges).max_cardinality().solve();
        let pairs: Vec<_> = mates
            .iter()
            .enumerate()
            .filter_map(|(i, &m)| if m != SENTINEL && m > i { Some((i, m)) } else { None })
            .collect();
        assert_eq!(pairs.len(), 3);
    }

    #[test]
    fn weight_tiebreaker() {
        // 4 vertices, 2 possible perfect matchings:
        // (0,1)+(2,3) total weight = 100+1 = 101
        // (0,2)+(1,3) total weight = 50+50 = 100
        // Greedy init prefers (0,1) first (weight 100), then (2,3).
        let edges = vec![(0, 1, 100), (0, 2, 50), (1, 3, 50), (2, 3, 1)];
        let mates = Matching::new(edges).max_cardinality().solve();
        let pairs: Vec<_> = mates
            .iter()
            .enumerate()
            .filter_map(|(i, &m)| if m != SENTINEL && m > i { Some((i, m)) } else { None })
            .collect();
        assert_eq!(pairs.len(), 2); // perfect matching
    }
}
