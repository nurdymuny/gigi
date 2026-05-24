//! Principal + auxiliary adjacency operators for the dual-
//! adjacency upgrade (catalog §1.1).
//!
//! GIGI's existing `src/spectral.rs::field_index_graph` produces
//! ONE adjacency from the bundle's field-index graph (two records
//! adjacent iff they share an indexed field value). The Kähler
//! upgrade splits this into TWO: the **principal** adjacency over
//! one subset of indexed fields (the "structural" edges — primary/
//! foreign keys, parent/child relationships) and the **auxiliary**
//! adjacency over a different subset (the "twisted" edges —
//! computed columns, materialized cross-references). The commutativity
//! check between them is what the query planner uses for safe
//! join reordering (`commutativity.rs`).
//!
//! ### Representation
//!
//! Adjacency lives in a sparse symmetric matrix indexed by 64-bit
//! node IDs (we use `u64` because GIGI's `BasePoint` is `u64` and
//! we want zero-cost interop with the existing field-index graph).
//! For the foundational L2 milestone we keep the data structure
//! deliberately simple (a `HashMap<u64, Vec<u64>>`) — same shape
//! as `field_index_graph`'s output. Later layers can swap in a
//! CSR or bitmap representation if profiling demands it.
//!
//! ### Construction
//!
//! Two ways to build:
//! 1. **From a raw adjacency list** (`SparseAdjacency::from_pairs`):
//!    for tests, synthetic graphs, and the Cayley-graph examples
//!    in the validation suite.
//! 2. **From a bundle's field-index graph** restricted to a subset
//!    of fields (`PrincipalAdjacency::from_bundle_fields` /
//!    `AuxiliaryAdjacency::from_bundle_fields`): the production
//!    path. Reuses `src/spectral.rs` machinery for the bucket
//!    enumeration, then projects to only the requested fields.
//!
//! References:
//! - `theory/kahler_upgrade/catalog.md §1.1`
//! - `theory/kahler_upgrade/IMPLEMENTATION_PLAN.md` L2.2

use std::collections::{BTreeMap, BTreeSet};

/// Sparse symmetric adjacency, keyed by `u64` node IDs. Stored as
/// `BTreeMap<node, sorted Vec<neighbor>>` so iteration order is
/// deterministic — important for reproducible commutator
/// computations (small FP differences in different summation
/// orders would otherwise show up as spurious non-commutation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparseAdjacency {
    /// Sorted neighbor lists per node. Self-loops are not stored.
    pub(crate) edges: BTreeMap<u64, Vec<u64>>,
}

impl SparseAdjacency {
    /// Empty adjacency.
    pub fn new() -> Self {
        Self {
            edges: BTreeMap::new(),
        }
    }

    /// Build from an iterator of undirected (u, v) edge pairs.
    /// Each pair contributes both u→v and v→u so the result is
    /// symmetric. Duplicate edges are collapsed. Self-loops (u==v)
    /// are silently ignored — they don't carry adjacency
    /// information for our use case (we want the off-diagonal
    /// structure of A) and can confuse commutator arithmetic.
    pub fn from_pairs<I: IntoIterator<Item = (u64, u64)>>(pairs: I) -> Self {
        let mut tmp: BTreeMap<u64, BTreeSet<u64>> = BTreeMap::new();
        for (u, v) in pairs {
            if u == v {
                continue;
            }
            tmp.entry(u).or_default().insert(v);
            tmp.entry(v).or_default().insert(u);
        }
        let edges = tmp
            .into_iter()
            .map(|(k, set)| (k, set.into_iter().collect::<Vec<u64>>()))
            .collect();
        Self { edges }
    }

    /// Number of nodes (including isolated nodes that were
    /// explicitly inserted as empty entries — currently we only
    /// insert via `from_pairs` which always pairs, so all nodes
    /// have ≥ 1 neighbor; but the API is correct for the general
    /// case).
    pub fn node_count(&self) -> usize {
        self.edges.len()
    }

    /// Number of undirected edges (each pair counted once).
    pub fn edge_count(&self) -> usize {
        let total: usize = self.edges.values().map(|v| v.len()).sum();
        total / 2
    }

    /// Sorted iterator over node IDs. Deterministic order.
    pub fn nodes(&self) -> impl Iterator<Item = u64> + '_ {
        self.edges.keys().copied()
    }

    /// Neighbors of `node`, sorted. Empty slice if the node is
    /// absent.
    pub fn neighbors(&self, node: u64) -> &[u64] {
        self.edges.get(&node).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Densify to a row-major `n × n` float matrix using a stable
    /// node ordering (sorted). Each cell is 1.0 if the
    /// corresponding pair is adjacent, 0.0 otherwise. Returns the
    /// (matrix, node-order) pair so callers can map indices back
    /// to node IDs.
    ///
    /// O(n²) memory — use only when n is small (≤ a few hundred).
    /// For commutator computations on large graphs we switch to
    /// sample-based commutator probing in `commutativity.rs`.
    pub fn to_dense(&self) -> (Vec<f64>, Vec<u64>) {
        let node_order: Vec<u64> = self.nodes().collect();
        let n = node_order.len();
        let idx: BTreeMap<u64, usize> = node_order
            .iter()
            .copied()
            .enumerate()
            .map(|(i, v)| (v, i))
            .collect();
        let mut m = vec![0.0_f64; n * n];
        for (u, nbrs) in &self.edges {
            let i = idx[u];
            for v in nbrs {
                let j = idx[v];
                m[i * n + j] = 1.0;
            }
        }
        (m, node_order)
    }
}

impl Default for SparseAdjacency {
    fn default() -> Self {
        Self::new()
    }
}

/// The **principal** adjacency operator on a bundle. Wraps a
/// `SparseAdjacency` for type-distinct dispatch in
/// `commutativity::commute(p, a)`. The wrapping is intentional —
/// it makes argument order at call sites self-documenting and
/// catches "I swapped principal/auxiliary" bugs at compile time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalAdjacency {
    pub adj: SparseAdjacency,
}

impl PrincipalAdjacency {
    pub fn new(adj: SparseAdjacency) -> Self {
        Self { adj }
    }

    pub fn from_pairs<I: IntoIterator<Item = (u64, u64)>>(pairs: I) -> Self {
        Self::new(SparseAdjacency::from_pairs(pairs))
    }
}

/// The **auxiliary** adjacency operator on a bundle. See
/// `PrincipalAdjacency` for why this is a distinct newtype.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuxiliaryAdjacency {
    pub adj: SparseAdjacency,
}

impl AuxiliaryAdjacency {
    pub fn new(adj: SparseAdjacency) -> Self {
        Self { adj }
    }

    pub fn from_pairs<I: IntoIterator<Item = (u64, u64)>>(pairs: I) -> Self {
        Self::new(SparseAdjacency::from_pairs(pairs))
    }
}

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Positive: from_pairs symmetrizes and dedups.
    #[test]
    fn from_pairs_symmetric_and_deduped() {
        // Triangle {0,1,2} with one redundant pair and one
        // self-loop (must be ignored).
        let a = SparseAdjacency::from_pairs(vec![
            (0, 1),
            (1, 0), // duplicate of (0,1)
            (1, 2),
            (0, 2),
            (3, 3), // self-loop — ignored
        ]);
        assert_eq!(a.node_count(), 3);
        assert_eq!(a.edge_count(), 3);
        assert_eq!(a.neighbors(0), &[1, 2]);
        assert_eq!(a.neighbors(1), &[0, 2]);
        assert_eq!(a.neighbors(2), &[0, 1]);
        // Node 3 was a self-loop only — should not appear.
        let empty: &[u64] = &[];
        assert_eq!(a.neighbors(3), empty);
    }

    /// Positive: densify produces a symmetric 0/1 matrix in the
    /// declared node order.
    #[test]
    fn dense_is_symmetric_and_zero_one() {
        let a = SparseAdjacency::from_pairs(vec![(0, 1), (1, 2)]);
        let (m, order) = a.to_dense();
        assert_eq!(order, vec![0, 1, 2]);
        // Expected:
        //     0 1 0
        //     1 0 1
        //     0 1 0
        assert_eq!(m, vec![0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0]);

        // Symmetric.
        let n = order.len();
        for i in 0..n {
            for j in 0..n {
                assert_eq!(m[i * n + j], m[j * n + i]);
            }
        }
    }

    /// Positive: an empty adjacency densifies to an empty
    /// matrix without panicking (n = 0 corner case).
    #[test]
    fn empty_adjacency_densifies_cleanly() {
        let a = SparseAdjacency::new();
        let (m, order) = a.to_dense();
        assert!(m.is_empty());
        assert!(order.is_empty());
        assert_eq!(a.node_count(), 0);
        assert_eq!(a.edge_count(), 0);
    }

    /// Positive: PrincipalAdjacency and AuxiliaryAdjacency wrap
    /// the same underlying SparseAdjacency but are distinct types.
    /// Compile-time check via the type system — runtime sanity that
    /// the wrappers preserve the adjacency.
    #[test]
    fn principal_and_auxiliary_preserve_adjacency() {
        let pairs = vec![(0, 1), (1, 2), (2, 3)];
        let p = PrincipalAdjacency::from_pairs(pairs.clone());
        let a = AuxiliaryAdjacency::from_pairs(pairs);
        assert_eq!(p.adj.node_count(), 4);
        assert_eq!(a.adj.node_count(), 4);
        assert_eq!(p.adj.edge_count(), 3);
        assert_eq!(a.adj.edge_count(), 3);
        // Same underlying data → adjacency lists agree.
        assert_eq!(p.adj.neighbors(1), a.adj.neighbors(1));
    }
}
