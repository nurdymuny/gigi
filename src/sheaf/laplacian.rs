//! Sheaf Laplacian construction and Schur complement solver.
//!
//! Implements §1.2–§1.4 of the GIGI Sheaf Completion Spec:
//! - Build the sheaf Laplacian L_F from adjacency graph + restriction maps
//! - Partition into observed (o) and missing (m) vertices
//! - Solve x̂_m = -L_mm^{-1} L_mo x_o via Cholesky factorization
//! - Derive confidence from (L_mm^{-1})_{vv}
//!
//! The Laplacian is real, symmetric, positive semi-definite. For a connected
//! subgraph with at least one observed vertex, L_mm is positive definite
//! and the completion is unique.

use nalgebra::{DMatrix, DVector};
use std::collections::HashMap;

/// An edge in the adjacency graph with its weight and restriction map coefficient.
#[derive(Debug, Clone)]
pub struct SheafEdge {
    /// Index of the source vertex in the local vertex list.
    pub src: usize,
    /// Index of the target vertex in the local vertex list.
    pub tgt: usize,
    /// Edge weight (from adjacency declaration).
    pub weight: f64,
    /// Restriction map coefficient: F_{tgt←src}.
    /// For identity restriction maps, this is 1.0.
    /// For Transform adjacency, this is the linearized coefficient.
    pub restriction: f64,
}

/// Classification of a vertex as observed or missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexKind {
    Observed,
    Missing,
}

/// A local sheaf problem: a small subgraph around a query vertex.
#[derive(Debug)]
pub struct SheafProblem {
    /// Number of vertices in the local subgraph.
    pub n_vertices: usize,
    /// Edges with weights and restriction maps.
    pub edges: Vec<SheafEdge>,
    /// Classification of each vertex.
    pub kinds: Vec<VertexKind>,
    /// Observed values (keyed by vertex index). Missing vertices have no entry.
    pub observed: HashMap<usize, f64>,
}

/// Result of sheaf completion for a single missing vertex.
#[derive(Debug, Clone)]
pub struct CompletionResult {
    /// The completed (predicted) value.
    pub value: f64,
    /// Confidence score: 1 / (1 + (L_mm^{-1})_{vv}).
    pub confidence: f64,
    /// The diagonal element (L_mm^{-1})_{vv} — raw uncertainty.
    pub inv_diag: f64,
}

/// Result when L_mm is singular — some directions are undetermined.
#[derive(Debug, Clone)]
pub struct UndeterminedDirection {
    /// Index of the undetermined vertex in the local problem.
    pub vertex_idx: usize,
    /// The null eigenvector component for this vertex.
    pub null_component: f64,
}

/// Full result of solving a sheaf completion problem.
#[derive(Debug)]
pub struct SheafSolution {
    /// Completed values for each missing vertex.
    pub completions: Vec<(usize, CompletionResult)>,
    /// Undetermined directions (if L_mm is singular).
    pub undetermined: Vec<UndeterminedDirection>,
    /// Empirical residual variance s² at observed vertices.
    pub residual_variance: f64,
}

/// Build the sheaf Laplacian matrix from edges.
///
/// L_F[i,i] = Σ_{j~i} w_{ij} * F_{ij}²
/// L_F[i,j] = -w_{ij} * F_{ij}    (for adjacent i,j)
///
/// For identity restriction maps (F=1), this reduces to the standard
/// weighted graph Laplacian.
pub fn build_laplacian(n: usize, edges: &[SheafEdge]) -> DMatrix<f64> {
    let mut l = DMatrix::zeros(n, n);
    for e in edges {
        let w = e.weight;
        let f = e.restriction;
        // Off-diagonal: -w * f
        l[(e.src, e.tgt)] -= w * f;
        l[(e.tgt, e.src)] -= w * f;
        // Diagonal contributions
        l[(e.src, e.src)] += w * f * f;
        l[(e.tgt, e.tgt)] += w;
    }
    l
}

/// Solve the sheaf completion problem via Schur complement.
///
/// Given L_F partitioned as:
///   [L_oo  L_om]
///   [L_mo  L_mm]
///
/// The optimal completion is: x̂_m = -L_mm^{-1} L_mo x_o
/// Confidence: 1 / (1 + (L_mm^{-1})_{ii})
pub fn solve(problem: &SheafProblem) -> SheafSolution {
    let n = problem.n_vertices;
    let l = build_laplacian(n, &problem.edges);

    // Partition vertices into observed and missing
    let mut obs_indices: Vec<usize> = Vec::new();
    let mut miss_indices: Vec<usize> = Vec::new();
    for (i, kind) in problem.kinds.iter().enumerate() {
        match kind {
            VertexKind::Observed => obs_indices.push(i),
            VertexKind::Missing => miss_indices.push(i),
        }
    }

    let n_o = obs_indices.len();
    let n_m = miss_indices.len();

    if n_m == 0 {
        return SheafSolution {
            completions: vec![],
            undetermined: vec![],
            residual_variance: 0.0,
        };
    }

    // Extract L_mm, L_mo sub-matrices
    let mut l_mm = DMatrix::zeros(n_m, n_m);
    let mut l_mo = DMatrix::zeros(n_m, n_o);

    for (mi, &i) in miss_indices.iter().enumerate() {
        for (mj, &j) in miss_indices.iter().enumerate() {
            l_mm[(mi, mj)] = l[(i, j)];
        }
        for (oj, &j) in obs_indices.iter().enumerate() {
            l_mo[(mi, oj)] = l[(i, j)];
        }
    }

    // Build observed value vector
    let mut x_o = DVector::zeros(n_o);
    for (oj, &j) in obs_indices.iter().enumerate() {
        x_o[oj] = *problem.observed.get(&j).unwrap_or(&0.0);
    }

    // Compute residual variance s² from observed vertices
    let residual_variance = compute_residual_variance(&l, &obs_indices, &x_o);

    // Try Cholesky decomposition of L_mm
    let epsilon = 1e-12;
    match l_mm.clone().cholesky() {
        Some(chol) => {
            // L_mm is positive definite — unique solution
            let rhs = -(l_mo * &x_o);
            let x_m = chol.solve(&rhs);

            // Get L_mm^{-1} diagonal for confidence
            let l_mm_inv = chol.inverse();

            let completions = miss_indices
                .iter()
                .enumerate()
                .map(|(mi, &vertex_idx)| {
                    let inv_diag = l_mm_inv[(mi, mi)];
                    let confidence = 1.0 / (1.0 + inv_diag);
                    (
                        vertex_idx,
                        CompletionResult {
                            value: x_m[mi],
                            confidence,
                            inv_diag,
                        },
                    )
                })
                .collect();

            SheafSolution {
                completions,
                undetermined: vec![],
                residual_variance,
            }
        }
        None => {
            // L_mm is singular — eigendecompose to find null directions
            let eigen = l_mm.symmetric_eigen();
            let eigenvalues = &eigen.eigenvalues;
            let eigenvectors = &eigen.eigenvectors;

            // Find null and non-null eigenvalues
            let mut null_indices = Vec::new();
            let mut nonnull_indices = Vec::new();
            for i in 0..n_m {
                if eigenvalues[i].abs() < epsilon {
                    null_indices.push(i);
                } else {
                    nonnull_indices.push(i);
                }
            }

            // Build pseudoinverse on the non-null subspace
            let mut l_mm_pinv = DMatrix::zeros(n_m, n_m);
            for &i in &nonnull_indices {
                let ev = eigenvectors.column(i);
                l_mm_pinv += (1.0 / eigenvalues[i]) * (&ev * ev.transpose());
            }

            let rhs = -(l_mo * &x_o);
            let x_m = &l_mm_pinv * &rhs;

            // Completions from non-null subspace
            let completions = miss_indices
                .iter()
                .enumerate()
                .filter(|(mi, _)| {
                    // Check if this vertex is constrained (not purely in null space)
                    let inv_diag = l_mm_pinv[(*mi, *mi)];
                    inv_diag.abs() > epsilon || x_m[*mi].abs() > epsilon
                })
                .map(|(mi, &vertex_idx)| {
                    let inv_diag = l_mm_pinv[(mi, mi)];
                    let confidence = if inv_diag.abs() < epsilon {
                        0.0
                    } else {
                        1.0 / (1.0 + inv_diag)
                    };
                    (
                        vertex_idx,
                        CompletionResult {
                            value: x_m[mi],
                            confidence,
                            inv_diag,
                        },
                    )
                })
                .collect();

            // Identify undetermined directions
            let undetermined = null_indices
                .iter()
                .flat_map(|&null_idx| {
                    let ev = eigenvectors.column(null_idx);
                    miss_indices
                        .iter()
                        .enumerate()
                        .filter(move |(mi, _)| ev[*mi].abs() > epsilon)
                        .map(move |(mi, &vertex_idx)| UndeterminedDirection {
                            vertex_idx,
                            null_component: ev[mi],
                        })
                })
                .collect();

            SheafSolution {
                completions,
                undetermined,
                residual_variance,
            }
        }
    }
}

/// Compute the empirical residual variance s² at observed vertices.
/// s² = (1/n_o) Σ_i (Σ_j L_ij x_j)² for i ∈ observed
fn compute_residual_variance(
    l: &DMatrix<f64>,
    obs_indices: &[usize],
    x_o: &DVector<f64>,
) -> f64 {
    if obs_indices.is_empty() {
        return 1.0;
    }
    let n = l.nrows();
    // Build full x vector (missing = 0 for residual computation)
    let mut x_full = DVector::zeros(n);
    for (oj, &j) in obs_indices.iter().enumerate() {
        x_full[j] = x_o[oj];
    }
    let residuals = l * &x_full;
    let sum_sq: f64 = obs_indices.iter().map(|&i| residuals[i].powi(2)).sum();
    sum_sq / obs_indices.len() as f64
}

/// Compute the coherence interval C ∈ [τ/(K+σ_K), τ/(K-σ_K)] per §1.7.
///
/// Returns None if σ_K ≥ K (insufficient confidence).
pub fn coherence_interval(tau: f64, k: f64, sigma_k: f64) -> Option<(f64, f64)> {
    if sigma_k >= k {
        return None; // Insufficient confidence
    }
    let c_lo = tau / (k + sigma_k);
    let c_hi = tau / (k - sigma_k);
    Some((c_lo, c_hi))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SHEAF-1: Laplacian is symmetric positive semi-definite ──

    #[test]
    fn sheaf_1_laplacian_symmetric_psd() {
        // Triangle graph with unit weights and identity restriction maps
        let edges = vec![
            SheafEdge { src: 0, tgt: 1, weight: 1.0, restriction: 1.0 },
            SheafEdge { src: 1, tgt: 2, weight: 1.0, restriction: 1.0 },
            SheafEdge { src: 0, tgt: 2, weight: 1.0, restriction: 1.0 },
        ];
        let l = build_laplacian(3, &edges);

        // Symmetry
        for i in 0..3 {
            for j in 0..3 {
                assert!(
                    (l[(i, j)] - l[(j, i)]).abs() < 1e-12,
                    "L[{i},{j}] = {} ≠ L[{j},{i}] = {}",
                    l[(i, j)],
                    l[(j, i)]
                );
            }
        }

        // PSD: all eigenvalues ≥ 0
        let eigen = l.symmetric_eigen();
        for (i, ev) in eigen.eigenvalues.iter().enumerate() {
            assert!(*ev >= -1e-10, "eigenvalue {i} = {ev} should be ≥ 0");
        }

        // At least one zero eigenvalue (constant vector is in kernel)
        let min_ev = eigen.eigenvalues.iter().cloned().fold(f64::MAX, f64::min);
        assert!(min_ev.abs() < 1e-10, "smallest eigenvalue should be ~0, got {min_ev}");
    }

    // ── SHEAF-2: Complete a single missing vertex on a path graph ──

    #[test]
    fn sheaf_2_single_completion_path() {
        //  v0(obs=1.0) --w=1-- v1(miss) --w=1-- v2(obs=3.0)
        // With identity restriction maps, the optimal completion at v1 is the
        // weighted average = 2.0 (midpoint of 1.0 and 3.0).
        let problem = SheafProblem {
            n_vertices: 3,
            edges: vec![
                SheafEdge { src: 0, tgt: 1, weight: 1.0, restriction: 1.0 },
                SheafEdge { src: 1, tgt: 2, weight: 1.0, restriction: 1.0 },
            ],
            kinds: vec![VertexKind::Observed, VertexKind::Missing, VertexKind::Observed],
            observed: [(0, 1.0), (2, 3.0)].into_iter().collect(),
        };

        let solution = solve(&problem);
        assert_eq!(solution.completions.len(), 1);
        let (idx, result) = &solution.completions[0];
        assert_eq!(*idx, 1);
        assert!((result.value - 2.0).abs() < 1e-10, "should be 2.0, got {}", result.value);
        assert!(result.confidence > 0.0 && result.confidence < 1.0);
        assert!(solution.undetermined.is_empty());
    }

    // ── SHEAF-3: Weighted edges bias the completion ──

    #[test]
    fn sheaf_3_weighted_completion() {
        //  v0(obs=1.0) --w=3-- v1(miss) --w=1-- v2(obs=5.0)
        // Heavier weight to v0 → completion biased toward 1.0.
        // L_mm = [3+1] = [4], L_mo = [-3, -1]
        // x̂ = -4^{-1} * [-3*1 + -1*5] = -(-8)/4 = 2.0
        let problem = SheafProblem {
            n_vertices: 3,
            edges: vec![
                SheafEdge { src: 0, tgt: 1, weight: 3.0, restriction: 1.0 },
                SheafEdge { src: 1, tgt: 2, weight: 1.0, restriction: 1.0 },
            ],
            kinds: vec![VertexKind::Observed, VertexKind::Missing, VertexKind::Observed],
            observed: [(0, 1.0), (2, 5.0)].into_iter().collect(),
        };

        let solution = solve(&problem);
        let (_, result) = &solution.completions[0];
        // x̂ = (3*1 + 1*5)/(3+1) = 8/4 = 2.0
        assert!((result.value - 2.0).abs() < 1e-10, "should be 2.0, got {}", result.value);
    }

    // ── SHEAF-4: Confidence increases with more neighbors ──

    #[test]
    fn sheaf_4_confidence_increases_with_neighbors() {
        // v1 missing, connected to 2 observed neighbors
        let problem_2 = SheafProblem {
            n_vertices: 3,
            edges: vec![
                SheafEdge { src: 0, tgt: 1, weight: 1.0, restriction: 1.0 },
                SheafEdge { src: 1, tgt: 2, weight: 1.0, restriction: 1.0 },
            ],
            kinds: vec![VertexKind::Observed, VertexKind::Missing, VertexKind::Observed],
            observed: [(0, 1.0), (2, 1.0)].into_iter().collect(),
        };

        // v1 missing, connected to 4 observed neighbors (star graph)
        let problem_4 = SheafProblem {
            n_vertices: 5,
            edges: vec![
                SheafEdge { src: 0, tgt: 1, weight: 1.0, restriction: 1.0 },
                SheafEdge { src: 1, tgt: 2, weight: 1.0, restriction: 1.0 },
                SheafEdge { src: 1, tgt: 3, weight: 1.0, restriction: 1.0 },
                SheafEdge { src: 1, tgt: 4, weight: 1.0, restriction: 1.0 },
            ],
            kinds: vec![
                VertexKind::Observed,
                VertexKind::Missing,
                VertexKind::Observed,
                VertexKind::Observed,
                VertexKind::Observed,
            ],
            observed: [(0, 1.0), (2, 1.0), (3, 1.0), (4, 1.0)].into_iter().collect(),
        };

        let sol_2 = solve(&problem_2);
        let sol_4 = solve(&problem_4);
        let conf_2 = sol_2.completions[0].1.confidence;
        let conf_4 = sol_4.completions[0].1.confidence;
        assert!(
            conf_4 > conf_2,
            "4 neighbors should give higher confidence than 2: {conf_4} vs {conf_2}"
        );
    }

    // ── SHEAF-5: Zero-valued completion has nonzero confidence ──

    #[test]
    fn sheaf_5_zero_valued_completion() {
        // All neighbors have value 0.0 — completion should be 0.0 with HIGH confidence
        let problem = SheafProblem {
            n_vertices: 4,
            edges: vec![
                SheafEdge { src: 0, tgt: 1, weight: 1.0, restriction: 1.0 },
                SheafEdge { src: 1, tgt: 2, weight: 1.0, restriction: 1.0 },
                SheafEdge { src: 1, tgt: 3, weight: 1.0, restriction: 1.0 },
            ],
            kinds: vec![
                VertexKind::Observed,
                VertexKind::Missing,
                VertexKind::Observed,
                VertexKind::Observed,
            ],
            observed: [(0, 0.0), (2, 0.0), (3, 0.0)].into_iter().collect(),
        };

        let solution = solve(&problem);
        let (_, result) = &solution.completions[0];
        assert!(result.value.abs() < 1e-10, "completion should be 0.0, got {}", result.value);
        assert!(
            result.confidence > 0.5,
            "zero-valued completion should have high confidence, got {}",
            result.confidence
        );
    }

    // ── SHEAF-6: Disconnected missing vertex → undetermined ──

    #[test]
    fn sheaf_6_disconnected_missing() {
        // v0(obs) -- v1(miss), v2(miss) is isolated (no edges)
        let problem = SheafProblem {
            n_vertices: 3,
            edges: vec![
                SheafEdge { src: 0, tgt: 1, weight: 1.0, restriction: 1.0 },
            ],
            kinds: vec![VertexKind::Observed, VertexKind::Missing, VertexKind::Missing],
            observed: [(0, 5.0)].into_iter().collect(),
        };

        let solution = solve(&problem);
        // v1 should be completable (connected to v0)
        // v2 should be undetermined (disconnected)
        assert!(
            !solution.undetermined.is_empty(),
            "disconnected vertex should produce undetermined directions"
        );
        // Check that v2 is in the undetermined list
        let v2_undetermined = solution
            .undetermined
            .iter()
            .any(|u| u.vertex_idx == 2);
        assert!(v2_undetermined, "v2 (disconnected) should be undetermined");
    }

    // ── SHEAF-7: Non-identity restriction map (Transform adjacency) ──

    #[test]
    fn sheaf_7_transform_restriction_map() {
        // v0(obs=100.0) --w=1,F=0.1-- v1(miss)
        // With restriction F=0.1: the sheaf says v1 ≈ F * v0 = 10.0
        // L_mm = [0.1² * 1] = [0.01]  (weight * F²)
        // L_mo = [-0.1 * 1] = [-0.1]  (weight * F)
        // x̂ = -0.01^{-1} * (-0.1 * 100) = -(1/0.01) * (-10) = 1000... wait let me recalculate
        //
        // Actually for non-identity F:
        // The Laplacian for edge (i,j) with weight w and restriction F_{j←i}:
        // L[i,i] += w * F²,  L[j,j] += w,  L[i,j] -= w*F, L[j,i] -= w*F
        //
        // For src=0, tgt=1, w=1, F=0.1:
        // L[0,0] += 1 * 0.01 = 0.01
        // L[1,1] += 1
        // L[0,1] -= 1 * 0.1 = -0.1
        // L[1,0] -= 0.1
        //
        // With v0 observed, v1 missing:
        // L_mm = [1.0]
        // L_mo = [-0.1]
        // x̂ = -1.0^{-1} * (-0.1 * 100) = 10.0
        let problem = SheafProblem {
            n_vertices: 2,
            edges: vec![
                SheafEdge { src: 0, tgt: 1, weight: 1.0, restriction: 0.1 },
            ],
            kinds: vec![VertexKind::Observed, VertexKind::Missing],
            observed: [(0, 100.0)].into_iter().collect(),
        };

        let solution = solve(&problem);
        let (_, result) = &solution.completions[0];
        assert!(
            (result.value - 10.0).abs() < 1e-8,
            "Transform F=0.1: expected 10.0, got {}",
            result.value
        );
    }

    // ── SHEAF-8: Coherence interval ──

    #[test]
    fn sheaf_8_coherence_interval() {
        // τ=10, K=5, σ_K=1 → C ∈ [10/6, 10/4] = [1.667, 2.5]
        let interval = coherence_interval(10.0, 5.0, 1.0);
        assert!(interval.is_some());
        let (lo, hi) = interval.unwrap();
        assert!((lo - 10.0 / 6.0).abs() < 1e-10);
        assert!((hi - 10.0 / 4.0).abs() < 1e-10);
        assert!(lo < hi);

        // σ_K ≥ K → insufficient confidence
        let none = coherence_interval(10.0, 5.0, 5.0);
        assert!(none.is_none(), "σ_K = K should return None");
        let none2 = coherence_interval(10.0, 5.0, 10.0);
        assert!(none2.is_none(), "σ_K > K should return None");
    }

    // ── SHEAF-9: Multiple missing vertices are jointly solved ──

    #[test]
    fn sheaf_9_multiple_missing() {
        // Path: v0(obs=0) -- v1(miss) -- v2(miss) -- v3(obs=6)
        // Equal weights, identity restriction maps.
        // This is a linear interpolation: v1 ≈ 2, v2 ≈ 4
        let problem = SheafProblem {
            n_vertices: 4,
            edges: vec![
                SheafEdge { src: 0, tgt: 1, weight: 1.0, restriction: 1.0 },
                SheafEdge { src: 1, tgt: 2, weight: 1.0, restriction: 1.0 },
                SheafEdge { src: 2, tgt: 3, weight: 1.0, restriction: 1.0 },
            ],
            kinds: vec![
                VertexKind::Observed,
                VertexKind::Missing,
                VertexKind::Missing,
                VertexKind::Observed,
            ],
            observed: [(0, 0.0), (3, 6.0)].into_iter().collect(),
        };

        let solution = solve(&problem);
        assert_eq!(solution.completions.len(), 2);

        // Sort by vertex index
        let mut comps: Vec<_> = solution.completions.clone();
        comps.sort_by_key(|(idx, _)| *idx);

        let v1_val = comps[0].1.value;
        let v2_val = comps[1].1.value;
        assert!(
            (v1_val - 2.0).abs() < 1e-8,
            "v1 should be ~2.0, got {v1_val}"
        );
        assert!(
            (v2_val - 4.0).abs() < 1e-8,
            "v2 should be ~4.0, got {v2_val}"
        );
    }

    // ── SHEAF-10: Laplacian diagonal matches expected formula ──

    #[test]
    fn sheaf_10_laplacian_diagonal() {
        // Star graph: center (v0) connected to v1, v2, v3 with weights 2, 3, 5
        let edges = vec![
            SheafEdge { src: 0, tgt: 1, weight: 2.0, restriction: 1.0 },
            SheafEdge { src: 0, tgt: 2, weight: 3.0, restriction: 1.0 },
            SheafEdge { src: 0, tgt: 3, weight: 5.0, restriction: 1.0 },
        ];
        let l = build_laplacian(4, &edges);

        // L[0,0] should be sum of weights = 2+3+5 = 10
        assert!((l[(0, 0)] - 10.0).abs() < 1e-12);
        // L[1,1] = 2 (only connected to v0 with weight 2)
        assert!((l[(1, 1)] - 2.0).abs() < 1e-12);
        // L[2,2] = 3
        assert!((l[(2, 2)] - 3.0).abs() < 1e-12);
        // L[3,3] = 5
        assert!((l[(3, 3)] - 5.0).abs() < 1e-12);
        // Off-diagonal: L[0,1] = -2
        assert!((l[(0, 1)] - (-2.0)).abs() < 1e-12);
    }
}
