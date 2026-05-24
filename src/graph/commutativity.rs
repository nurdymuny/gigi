//! Commutativity classifier for the dual adjacency operators
//! (catalog §1.1).
//!
//! ### What this answers
//!
//! Given principal `A_p` and auxiliary `A_a` adjacency operators
//! on the same node set, do they commute? Commuting operators
//! share an eigenbasis, so the query planner can reorder joins
//! safely; non-commuting forces a specific order.
//!
//! ### How we answer it (and what the Test-1 caveat taught us)
//!
//! The catalog's validation Test 1 caught a real failure mode:
//! the FIRST attempt at a non-commuting example used `{g, g⁻¹}`
//! as the generator set for S₃ where g was a 3-cycle. That set
//! is the *entire conjugacy class* of 3-cycles, and class sums
//! are CENTRAL in the group algebra ℂ[G] — so the adjacencies
//! commute, but for a reason that has NOTHING to do with the
//! Kähler structure.
//!
//! Vertex-transitivity alone is NOT enough. Centrality of the
//! generating sets (or abelianness of the underlying group) is
//! the load-bearing condition. The classifier here reflects that
//! in `CommutativityClass`, which records the WHY:
//!
//! - `Commute(Abelian)` — Cayley graph of an abelian group.
//! - `Commute(GeneratorSetsCentral)` — non-abelian but generators
//!   are unions of full conjugacy classes (the S₃ trap from Test 1).
//! - `Commute(NumericallyVerified)` — direct commutator check on
//!   the densified matrices found `[A_p, A_a] ≈ 0`.
//! - `NotCommute { max_entry }` — the largest |[A_p, A_a]_{ij}|,
//!   which is what the planner uses for ordering decisions.
//! - `Unknown` — graph too large for direct check and no group
//!   structure detected.
//!
//! Production GIGI usually lands in `NumericallyVerified` or
//! `NotCommute`, since field-index graphs aren't typically Cayley.
//! The other variants exist for the synthetic-test path and the
//! future case where we detect group structure (e.g. base space
//! is a Z/n × Z/m grid).
//!
//! References:
//! - `theory/kahler_upgrade/catalog.md §1.1` (the caveat is the
//!   "Caveat surfaced by testing" subsection)
//! - `theory/kahler_upgrade/IMPLEMENTATION_PLAN.md` L2.3
//! - `theory/kahler_upgrade/validation/validation_tests.py::test_1_kahler_commutativity`

use super::adjacency::{AuxiliaryAdjacency, PrincipalAdjacency, SparseAdjacency};

/// Threshold above which we skip the O(n³) direct matrix
/// commutator computation. Sized so a typical small bundle (a few
/// hundred records) still gets the exact check.
const DENSE_COMMUTATOR_NODE_LIMIT: usize = 256;

/// Threshold below which a commutator entry is treated as zero
/// for the purpose of declaring commutativity. The catalog Tests
/// pass at 1e-12; we use 1e-10 here to absorb a little extra noise
/// from the sparse → dense densification arithmetic.
const COMMUTATOR_ZERO_TOLERANCE: f64 = 1e-10;

/// The verdict of a commutativity check between principal and
/// auxiliary adjacency operators. Variants record the WHY in
/// addition to the YES/NO so the planner can act on it.
#[derive(Debug, Clone, PartialEq)]
pub enum CommutativityClass {
    /// Adjacencies commute. The contained `WhyCommute` distinguishes
    /// the rigorous reason from the merely numerical one.
    Commute(WhyCommute),

    /// Adjacencies do NOT commute. `max_entry` is the largest
    /// absolute commutator entry — useful as a magnitude
    /// indicator for how badly the ordering matters.
    NotCommute { max_entry: f64 },

    /// Graph too large for a direct check and we couldn't detect
    /// algebraic structure to apply the centrality fast path.
    /// Planner should treat this as "don't reorder."
    Unknown,
}

/// Why a pair of adjacency operators commute. Captures the
/// distinction the catalog's Test 1 was designed to surface.
#[derive(Debug, Clone, PartialEq)]
pub enum WhyCommute {
    /// The underlying group is abelian — every pair of generator
    /// sums commutes in ℂ[G] by definition. This is the only
    /// reason that's purely structural (no numerical check needed).
    /// Triggered when both adjacencies come from a known-abelian
    /// Cayley graph; we don't auto-detect from raw adjacency yet.
    Abelian,

    /// Generators are central in ℂ[G] — typically because each
    /// generator set is a union of full conjugacy classes (their
    /// sum is fixed by conjugation, hence central). The S₃
    /// example from Test 1 lives here: it commutes for the
    /// "wrong" reason from the catalog's perspective, so the
    /// planner should know that's why.
    GeneratorSetsCentral,

    /// Direct matrix commutator computed and found ≈ 0 within
    /// `COMMUTATOR_ZERO_TOLERANCE`. The default reason in
    /// production where bundles don't carry explicit group
    /// structure.
    NumericallyVerified,
}

/// Compute the commutativity class of (principal, auxiliary).
///
/// Decision tree:
/// 1. If either adjacency has more than `DENSE_COMMUTATOR_NODE_LIMIT`
///    nodes, return `Unknown` (planner won't reorder).
/// 2. Build dense matrices `P, A` on the shared node order (union
///    of both node sets, sorted).
/// 3. Compute `‖[P, A]‖_∞` (max absolute entry of `P·A − A·P`).
/// 4. If ≤ tolerance → `Commute(NumericallyVerified)`. Else
///    `NotCommute { max_entry }`.
///
/// The `WhyCommute::Abelian` and `WhyCommute::GeneratorSetsCentral`
/// variants are produced by `commute_with_hint` (callers who know
/// the group structure pass it in explicitly). The structure-free
/// path collapses every commuting case to `NumericallyVerified`.
pub fn commute(
    principal: &PrincipalAdjacency,
    auxiliary: &AuxiliaryAdjacency,
) -> CommutativityClass {
    commute_inner(&principal.adj, &auxiliary.adj, None)
}

/// Hint the classifier with a known algebraic reason for
/// commutation. Useful in tests (where we KNOW we built a Cayley
/// graph of an abelian group, say) and for the future planner
/// path where we detect base-space group structure at bundle
/// construction time.
pub fn commute_with_hint(
    principal: &PrincipalAdjacency,
    auxiliary: &AuxiliaryAdjacency,
    hint: WhyCommute,
) -> CommutativityClass {
    commute_inner(&principal.adj, &auxiliary.adj, Some(hint))
}

fn commute_inner(
    p: &SparseAdjacency,
    a: &SparseAdjacency,
    hint: Option<WhyCommute>,
) -> CommutativityClass {
    // Build the shared node order (union of both adjacencies' nodes).
    let mut all_nodes: std::collections::BTreeSet<u64> = p.nodes().collect();
    all_nodes.extend(a.nodes());
    let node_order: Vec<u64> = all_nodes.into_iter().collect();
    let n = node_order.len();

    if n == 0 {
        // Vacuously commute — there's nothing to fail on.
        return CommutativityClass::Commute(hint.unwrap_or(WhyCommute::NumericallyVerified));
    }

    if n > DENSE_COMMUTATOR_NODE_LIMIT {
        // If the caller knew the structure, honor the hint anyway —
        // for abelian/central cases the verdict doesn't depend on
        // the matrix size.
        if let Some(why) = hint {
            return CommutativityClass::Commute(why);
        }
        return CommutativityClass::Unknown;
    }

    // Densify both adjacencies on the common node order.
    let idx: std::collections::BTreeMap<u64, usize> = node_order
        .iter()
        .copied()
        .enumerate()
        .map(|(i, v)| (v, i))
        .collect();
    let p_dense = densify_on(p, &idx, n);
    let a_dense = densify_on(a, &idx, n);

    // Commutator [P, A] = P·A − A·P.
    let pa = matmul(&p_dense, &a_dense, n);
    let ap = matmul(&a_dense, &p_dense, n);
    let mut max_entry = 0.0_f64;
    for k in 0..(n * n) {
        let v = (pa[k] - ap[k]).abs();
        if v > max_entry {
            max_entry = v;
        }
    }

    if max_entry <= COMMUTATOR_ZERO_TOLERANCE {
        // Hint dominates when it makes a stronger structural claim.
        CommutativityClass::Commute(hint.unwrap_or(WhyCommute::NumericallyVerified))
    } else {
        CommutativityClass::NotCommute { max_entry }
    }
}

fn densify_on(
    adj: &SparseAdjacency,
    idx: &std::collections::BTreeMap<u64, usize>,
    n: usize,
) -> Vec<f64> {
    let mut m = vec![0.0_f64; n * n];
    for (u, nbrs) in &adj.edges {
        let i = idx[u];
        for v in nbrs {
            // Some auxiliary adjacencies may reference nodes that
            // exist only in the principal set; the union node order
            // covers them, so this index lookup is always defined.
            let j = idx[v];
            m[i * n + j] = 1.0;
        }
    }
    m
}

fn matmul(a: &[f64], b: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![0.0_f64; n * n];
    for i in 0..n {
        for j in 0..n {
            let mut s = 0.0_f64;
            for k in 0..n {
                s += a[i * n + k] * b[k * n + j];
            }
            out[i * n + j] = s;
        }
    }
    out
}

// ── Tests (port of validation_tests.py::test_1, with the S₃ trap
//         baked in as a first-class test case) ─────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Build the Cayley graph of Z/n × Z/n with generators
    // {(±1, 0), (0, ±1)}, partitioned into principal (axial X) and
    // auxiliary (axial Y) shifts. Abelian → adjacencies must
    // commute.
    fn z4_x_z4_axial() -> (PrincipalAdjacency, AuxiliaryAdjacency) {
        let n: i64 = 4;
        let node = |i: i64, j: i64| -> u64 {
            let ii = (i.rem_euclid(n)) as u64;
            let jj = (j.rem_euclid(n)) as u64;
            ii * (n as u64) + jj
        };
        let mut p_edges = Vec::new();
        let mut a_edges = Vec::new();
        for i in 0..n {
            for j in 0..n {
                let here = node(i, j);
                // Principal: ±x shifts.
                p_edges.push((here, node(i + 1, j)));
                p_edges.push((here, node(i - 1, j)));
                // Auxiliary: ±y shifts.
                a_edges.push((here, node(i, j + 1)));
                a_edges.push((here, node(i, j - 1)));
            }
        }
        (
            PrincipalAdjacency::from_pairs(p_edges),
            AuxiliaryAdjacency::from_pairs(a_edges),
        )
    }

    // S₃ as a 6-element group, indexed 0..6 via permutation
    // ordering. Helper to build a Cayley graph from a single
    // generator (right-multiplication by g).
    fn s3_cayley_single(g: [usize; 3]) -> Vec<(u64, u64)> {
        use std::collections::BTreeMap;
        let mut perms = Vec::new();
        for a in 0..3 {
            for b in 0..3 {
                for c in 0..3 {
                    let p = [a, b, c];
                    let mut seen = [false; 3];
                    let mut ok = true;
                    for &x in &p {
                        if seen[x] {
                            ok = false;
                            break;
                        }
                        seen[x] = true;
                    }
                    if ok {
                        perms.push(p);
                    }
                }
            }
        }
        perms.sort();
        let idx: BTreeMap<[usize; 3], u64> = perms
            .iter()
            .enumerate()
            .map(|(i, p)| (*p, i as u64))
            .collect();
        let mut edges = Vec::new();
        for p in &perms {
            // x ∘ g: apply g first, then x. Result[i] = x[g[i]].
            let xg = [p[g[0]], p[g[1]], p[g[2]]];
            edges.push((idx[p], idx[&xg]));
        }
        edges
    }

    /// Positive: Z/4 × Z/4 axial Cayley graph — abelian, so
    /// adjacencies commute. Maps to the FIRST half of
    /// `validation_tests.py::test_1`.
    #[test]
    fn z4_x_z4_axial_adjacencies_commute() {
        let (p, a) = z4_x_z4_axial();
        let result = commute(&p, &a);
        match result {
            CommutativityClass::Commute(_) => { /* good */ }
            other => panic!("expected Commute for abelian Cayley, got {:?}", other),
        }
    }

    /// Negative: S₃ Cayley graph with two single (NON-central)
    /// transpositions g_trans_1 = (0 1) and g_trans_2 = (1 2).
    /// These don't commute in the group AND aren't full conjugacy
    /// classes, so the adjacencies must NOT commute. Max
    /// commutator entry should be exactly 1.0 (matches Python
    /// test 1).
    #[test]
    fn s3_single_transpositions_do_not_commute() {
        let g_trans_1 = [1, 0, 2]; // swap 0,1
        let g_trans_2 = [0, 2, 1]; // swap 1,2

        let p = PrincipalAdjacency::from_pairs(s3_cayley_single(g_trans_1));
        let a = AuxiliaryAdjacency::from_pairs(s3_cayley_single(g_trans_2));

        match commute(&p, &a) {
            CommutativityClass::NotCommute { max_entry } => {
                assert!(
                    (max_entry - 1.0).abs() < 1e-12,
                    "expected max_entry == 1.0 for S₃ single transpositions, got {}",
                    max_entry
                );
            }
            other => panic!(
                "expected NotCommute for S₃ single transpositions, got {:?}",
                other
            ),
        }
    }

    /// THE TRAP from catalog Test 1. Using `{g_cyc, g_cyc⁻¹}` as
    /// the auxiliary generator set in S₃ — that's the ENTIRE
    /// conjugacy class of 3-cycles, and class sums are central in
    /// ℂ[G]. The adjacencies commute, but for the WRONG reason
    /// (centrality of the generating set, NOT Kähler structure).
    /// The classifier must:
    /// 1. Return Commute (the math is right — they DO commute).
    /// 2. When given the WhyCommute::GeneratorSetsCentral HINT,
    ///    preserve that reason so the planner knows why.
    /// This is the test that ensures the planner can't be fooled
    /// into treating this as a real Kähler-style commutation.
    #[test]
    fn s3_full_3cycle_class_commutes_via_centrality_trap() {
        let g_cyc = [1, 2, 0]; // 3-cycle (0 1 2)
        let g_cyc_inv = [2, 0, 1]; // inverse 3-cycle

        // Principal: a single non-central transposition.
        let p = PrincipalAdjacency::from_pairs(s3_cayley_single([1, 0, 2]));
        // Auxiliary: BOTH 3-cycles = the full conjugacy class.
        let mut aux_edges = s3_cayley_single(g_cyc);
        aux_edges.extend(s3_cayley_single(g_cyc_inv));
        let a = AuxiliaryAdjacency::from_pairs(aux_edges);

        // The auxiliary adjacency, being a class sum, commutes
        // with EVERYTHING in ℂ[G]. Bare classifier sees this as
        // numerically verified — true but doesn't capture the
        // structural reason.
        match commute(&p, &a) {
            CommutativityClass::Commute(WhyCommute::NumericallyVerified) => { /* good */ }
            other => panic!(
                "expected Commute(NumericallyVerified) for S₃ class-sum trap, got {:?}",
                other
            ),
        }

        // When the caller passes the structural hint, the
        // classifier surfaces it — this is what the query planner
        // would see when the bundle declares group structure.
        match commute_with_hint(&p, &a, WhyCommute::GeneratorSetsCentral) {
            CommutativityClass::Commute(WhyCommute::GeneratorSetsCentral) => { /* good */ }
            other => panic!(
                "expected Commute(GeneratorSetsCentral) when hinted, got {:?}",
                other
            ),
        }
    }

    /// Edge: empty adjacencies commute vacuously.
    #[test]
    fn empty_adjacencies_commute_vacuously() {
        let p = PrincipalAdjacency::new(SparseAdjacency::new());
        let a = AuxiliaryAdjacency::new(SparseAdjacency::new());
        match commute(&p, &a) {
            CommutativityClass::Commute(_) => { /* good */ }
            other => panic!("expected Commute on empty, got {:?}", other),
        }
    }

    /// Edge: small triangle on disjoint generators — both have
    /// edges {(0,1), (1,2)} so they're identical, must commute
    /// (anything commutes with itself).
    #[test]
    fn identical_adjacencies_commute_trivially() {
        let edges = vec![(0u64, 1u64), (1, 2)];
        let p = PrincipalAdjacency::from_pairs(edges.clone());
        let a = AuxiliaryAdjacency::from_pairs(edges);
        match commute(&p, &a) {
            CommutativityClass::Commute(_) => { /* good */ }
            other => panic!("identical adjacencies should commute, got {:?}", other),
        }
    }
}
