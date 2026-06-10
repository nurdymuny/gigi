//! WISH — the boundary-value extrapolation verb (Phase 3 of WISH_SPEC v0.1).
//!
//! WISH solves the geodesic BVP from a seed to a target on a Riemannian
//! substrate and returns one of three honest verdicts:
//!
//!   * `Granted` — a connecting geodesic within budget. SUDOKU `sat`.
//!   * `Unreachable` — no connecting path within budget; carries a
//!     frontier-truncation waypoint (the furthest in-budget node along
//!     the attempted candidate). SUDOKU `unsat: true`.
//!   * `Indeterminate` — singular configuration (conjugate locus) OR the
//!     solver did not converge / timed out. SUDOKU `unsat: null`. A
//!     timeout is reported as `Indeterminate`, NEVER as `Unreachable`:
//!     "I don't know" vs "no path exists" are different verdicts.
//!
//! Phase 3 (this file) ships:
//!   * The verdict-trichotomy types matching `WISH_SPEC_v0.1.md §5`.
//!   * The default **relaxation** solver (discrete-energy L-BFGS-style
//!     gradient descent with Armijo line search; 2-point Gauss-Legendre
//!     quadrature per segment so the chart-midpoint O(h²R) bias the GIGI
//!     team caught doesn't surface as a wrong-reason W1 failure).
//!   * The arc-length-parameterized Jacobi field integrator for low-dim
//!     conjugacy detection (W2 oracle on closed-form S²/CP¹).
//!   * Hard wall-clock cap that emits `Indeterminate { NonConvergence }`
//!     on timeout, with the server-side 50 ms floor pinned by config
//!     default to keep callers from manufacturing cheap `Indeterminate`
//!     verdicts.
//!
//! Phase 4 adds: capacity `C = τ/K` post-pass, `FrontierTruncation`
//! extraction, composition-stall detection on chain-rewishes.
//! Phase 5 adds: HTTP/GQL surfaces.

use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::imagine::provenance::{WishBlockReason, WishTargetProvenance};

// ─────────────────────────────────────────────────────────────────────────
// Spec §5 surface — types matching WISH_SPEC_v0.1.md exactly.
// ─────────────────────────────────────────────────────────────────────────

/// What the wish is aimed at — same shape as `WishTargetProvenance`,
/// but a request-side input rather than a per-record audit-trail field.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WishTarget {
    Coords(Vec<f64>),
    Record { bundle: String, record_id: String },
}

impl From<&WishTarget> for WishTargetProvenance {
    fn from(t: &WishTarget) -> Self {
        match t {
            WishTarget::Coords(c) => WishTargetProvenance::Coords(c.clone()),
            WishTarget::Record { bundle, record_id } => WishTargetProvenance::Record {
                bundle: bundle.clone(),
                record_id: record_id.clone(),
            },
        }
    }
}

/// Which solver to use. The spec defaults production to relaxation
/// (no exp-map Jacobian, robust through conjugacy); shooting is for
/// low-dim gates and explicit Jacobi analysis.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum SolverKind {
    /// Discrete-energy L-BFGS over interior nodes. Default.
    Relaxation { n_nodes: u32 },
    /// LM-damped Newton on the miss map `F(v) = exp_p(v) − x_1`,
    /// with a Jacobi-field side-integration for conjugacy detection.
    /// Reserved for low-dim gates (D ≲ 8); refuses on higher dim.
    Shooting,
}

impl Default for SolverKind {
    fn default() -> Self {
        Self::Relaxation { n_nodes: 32 }
    }
}

/// Why a wish came back as `Indeterminate`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum IndeterminateReason {
    /// The BVP is singular at this configuration (conjugate locus).
    /// In Phase 3 this is reported only by the shooting solver when
    /// the Jacobi field's perpendicular component zeros along the
    /// integration — the analytic special case W2 proves we handle
    /// correctly. Dim-lift to σ_min(J) monitoring at D ≥ 3 is a
    /// Phase 2 dim-lift dependency.
    ConjugateLocus { at_fraction: f64 },
    /// The solver did not converge within `max_iterations` or
    /// `max_solve_ms`, OR the energy stalled below `energy_tol` over
    /// the convergence window. The dominant high-dim trigger.
    NonConvergence { final_residual: f64 },
}

/// Trust-envelope + solver configuration. Defaults match
/// `WISH_SPEC_v0.1.md §5`, with the GIGI-team review's `max_solve_ms`
/// floor pinned in `effective_max_solve_ms()` rather than the field
/// itself so the field stays declarative.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WishConfig {
    // ── trust envelope (shared with WALK) ──
    pub max_imagined_curvature: f64,
    pub max_accumulated_holonomy: f64,
    pub max_arc_length: f64,
    pub sudoku_preflight_target: bool,

    // ── solver (§3) ──
    pub solver: SolverKind,
    pub max_iterations: u32,
    pub max_solve_ms: u32,
    pub grad_tol: f64,
    pub energy_tol: f64,
    pub residual_tol: f64,

    // ── frontier waypoint + composition (§6) — used by Phase 4 ──
    pub return_waypoint_on_unreachable: bool,
    pub min_progress_per_wish: f64,
    pub materialize_on_grant: bool,
}

impl Default for WishConfig {
    fn default() -> Self {
        Self {
            max_imagined_curvature: 4.0,        // K(CP¹ FS) ceiling
            max_accumulated_holonomy: 0.5,
            max_arc_length: 4.0,
            sudoku_preflight_target: true,
            solver: SolverKind::default(),
            max_iterations: 200,
            max_solve_ms: 250,
            grad_tol: 1e-6,
            energy_tol: 1e-8,
            residual_tol: 1e-6,
            return_waypoint_on_unreachable: true,
            min_progress_per_wish: 0.05,
            materialize_on_grant: false,
        }
    }
}

/// Server-side floor on `max_solve_ms`, per the GIGI-team review:
/// a caller can't set a sub-millisecond cap and manufacture cheap
/// `Indeterminate` verdicts to dodge the real solve.
pub const MAX_SOLVE_MS_FLOOR: u32 = 50;

impl WishConfig {
    /// The actually-enforced wall-clock budget — never below the floor.
    pub fn effective_max_solve_ms(&self) -> u32 {
        self.max_solve_ms.max(MAX_SOLVE_MS_FLOOR)
    }
}

/// What WISH returns. The three-variant trichotomy is the verb's
/// load-bearing contract; no fourth state, no silent default.
#[derive(Clone, Debug)]
pub enum WishOutcome {
    Granted {
        /// Path nodes in chart coords (N+1 entries, N = `n_nodes`).
        path: Vec<Vec<f64>>,
        arc_length: f64,
        integrated_curvature: f64,
        /// `C = τ / K`. Populated by Phase 4; Phase 3 reports `f64::NAN`.
        capacity: f64,
        accumulated_holonomy: f64,
        solver_iterations: u32,
        final_grad_norm: f64,
    },
    Unreachable {
        /// Phase 4 populates this. Phase 3 always returns Granted or
        /// Indeterminate; the variant exists so consumers can pattern-
        /// match against the full trichotomy from day one.
        frontier_waypoint: Vec<f64>,
        reached_fraction: f64,
        blocked_by: WishBlockReason,
        capacity_to_waypoint: f64,
    },
    Indeterminate {
        reason: IndeterminateReason,
    },
}

#[derive(thiserror::Error, Clone, Debug)]
pub enum WishError {
    #[error("seed and target have different dimensions: seed={seed_dim}, target={target_dim}")]
    DimMismatch { seed_dim: usize, target_dim: usize },
    #[error("Phase 3 only supports dim = 2; got dim = {dim}")]
    UnsupportedDim { dim: usize },
    #[error("target rejected by SUDOKU preflight: {detail}")]
    TargetConstraintViolation { detail: String },
}

// ─────────────────────────────────────────────────────────────────────────
// 2D metric trait + closed-form impls for the toy validation manifolds.
// The full BundleStore-backed metric surfaces are a Phase 4/5 wiring;
// Phase 3 exercises the solver against the W-math closed forms first.
// ─────────────────────────────────────────────────────────────────────────

/// 2-dimensional Riemannian metric for the wish solver. The conformal-
/// factor form `g = exp(2*phi) * delta` covers S², T², CP¹ stereographic
/// charts — the manifolds W-math validated against.
pub trait WishMetric2D: Sync {
    /// Conformal factor `exp(2*phi(x, y))`.
    fn exp2phi(&self, p: [f64; 2]) -> f64;
    /// Closed-form gradient of `exp(2*phi)`. Returned analytically to
    /// honor the spec's accuracy contract (Marcella-team review note
    /// #1: finite differences pollute the per-cell signal).
    fn grad_exp2phi(&self, p: [f64; 2]) -> [f64; 2];
    /// Scalar (Gaussian) curvature `K(x, y)`. Used by Phase 4's K
    /// integration and by the budget gates.
    fn scalar_curvature(&self, p: [f64; 2]) -> f64;
}

/// Unit S² in stereographic chart: `exp(2*phi) = 4 / (1 + r²)²`, K = 1.
#[derive(Clone, Copy, Debug)]
pub struct S2Stereographic;

impl WishMetric2D for S2Stereographic {
    fn exp2phi(&self, p: [f64; 2]) -> f64 {
        let r2 = p[0] * p[0] + p[1] * p[1];
        let s = 1.0 + r2;
        4.0 / (s * s)
    }
    fn grad_exp2phi(&self, p: [f64; 2]) -> [f64; 2] {
        let r2 = p[0] * p[0] + p[1] * p[1];
        let s = 1.0 + r2;
        let c = -16.0 / (s * s * s);
        [c * p[0], c * p[1]]
    }
    fn scalar_curvature(&self, _p: [f64; 2]) -> f64 {
        1.0
    }
}

/// Flat T² (any chart): exp(2*phi) ≡ 1, K ≡ 0.
#[derive(Clone, Copy, Debug)]
pub struct T2Flat;

impl WishMetric2D for T2Flat {
    fn exp2phi(&self, _p: [f64; 2]) -> f64 {
        1.0
    }
    fn grad_exp2phi(&self, _p: [f64; 2]) -> [f64; 2] {
        [0.0, 0.0]
    }
    fn scalar_curvature(&self, _p: [f64; 2]) -> f64 {
        0.0
    }
}

/// CP¹ Fubini-Study in stereographic chart: `exp(2*phi) = 1 / (1 + r²)²`, K = 4.
#[derive(Clone, Copy, Debug)]
pub struct CP1FubiniStudy;

impl WishMetric2D for CP1FubiniStudy {
    fn exp2phi(&self, p: [f64; 2]) -> f64 {
        let r2 = p[0] * p[0] + p[1] * p[1];
        let s = 1.0 + r2;
        1.0 / (s * s)
    }
    fn grad_exp2phi(&self, p: [f64; 2]) -> [f64; 2] {
        let r2 = p[0] * p[0] + p[1] * p[1];
        let s = 1.0 + r2;
        let c = -4.0 / (s * s * s);
        [c * p[0], c * p[1]]
    }
    fn scalar_curvature(&self, _p: [f64; 2]) -> f64 {
        4.0
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Discrete geodesic energy with GL-2 quadrature (matches wish_validation.py).
// ─────────────────────────────────────────────────────────────────────────

const GL2_S_MINUS: f64 = 0.5 - 0.288_675_134_594_812_9; // = 1/(2*sqrt(3))
const GL2_S_PLUS: f64 = 0.5 + 0.288_675_134_594_812_9;

/// Total energy of a discrete path through `nodes`. Each segment uses
/// 2-point Gauss-Legendre quadrature on `exp(2*phi)` so the segment
/// integral is `O(h⁴)` per segment — small enough that the global
/// `O(h²)` chord-discretization error dominates, NOT a midpoint-rule
/// `O(h²)` quadrature bias.
fn segment_energy<M: WishMetric2D + ?Sized>(metric: &M, p: [f64; 2], q: [f64; 2]) -> f64 {
    let d = [q[0] - p[0], q[1] - p[1]];
    let d2 = d[0] * d[0] + d[1] * d[1];
    if d2 < 1e-30 {
        return 0.0;
    }
    let v_minus = [p[0] + GL2_S_MINUS * d[0], p[1] + GL2_S_MINUS * d[1]];
    let v_plus = [p[0] + GL2_S_PLUS * d[0], p[1] + GL2_S_PLUS * d[1]];
    0.5 * (metric.exp2phi(v_minus) + metric.exp2phi(v_plus)) * d2
}

/// Analytic gradient of `segment_energy` w.r.t. `(p, q)`. Returns
/// `(dE/dp, dE/dq)` as a pair of 2-vectors.
///
/// Derivation: with `S = f(v_-) + f(v_+)`, `D = (q-p)·(q-p)`,
/// `E_seg = 0.5 * S * D`. Then
///   dE/dq = 0.5 * D * (s_-·∇f_- + s_+·∇f_+) + S * (q - p)
///   dE/dp = 0.5 * D * ((1-s_-)·∇f_- + (1-s_+)·∇f_+) - S * (q - p)
/// where `v_s = p + s*(q-p)` so `∂v_s/∂q = s·I`, `∂v_s/∂p = (1-s)·I`.
fn segment_energy_grad<M: WishMetric2D + ?Sized>(
    metric: &M,
    p: [f64; 2],
    q: [f64; 2],
) -> (f64, [f64; 2], [f64; 2]) {
    let d = [q[0] - p[0], q[1] - p[1]];
    let d2 = d[0] * d[0] + d[1] * d[1];
    if d2 < 1e-30 {
        return (0.0, [0.0, 0.0], [0.0, 0.0]);
    }
    let v_minus = [p[0] + GL2_S_MINUS * d[0], p[1] + GL2_S_MINUS * d[1]];
    let v_plus = [p[0] + GL2_S_PLUS * d[0], p[1] + GL2_S_PLUS * d[1]];
    let f_minus = metric.exp2phi(v_minus);
    let f_plus = metric.exp2phi(v_plus);
    let gf_minus = metric.grad_exp2phi(v_minus);
    let gf_plus = metric.grad_exp2phi(v_plus);
    let s = f_minus + f_plus;
    let e_seg = 0.5 * s * d2;

    let coef_q = 0.5 * d2;
    let coef_p = 0.5 * d2;
    let de_dq = [
        coef_q * (GL2_S_MINUS * gf_minus[0] + GL2_S_PLUS * gf_plus[0]) + s * d[0],
        coef_q * (GL2_S_MINUS * gf_minus[1] + GL2_S_PLUS * gf_plus[1]) + s * d[1],
    ];
    let de_dp = [
        coef_p
            * ((1.0 - GL2_S_MINUS) * gf_minus[0] + (1.0 - GL2_S_PLUS) * gf_plus[0])
            - s * d[0],
        coef_p
            * ((1.0 - GL2_S_MINUS) * gf_minus[1] + (1.0 - GL2_S_PLUS) * gf_plus[1])
            - s * d[1],
    ];
    (e_seg, de_dp, de_dq)
}

/// Total energy and gradient w.r.t. the interior nodes `z` (flattened).
fn total_energy_grad<M: WishMetric2D + ?Sized>(
    metric: &M,
    seed: [f64; 2],
    target: [f64; 2],
    z: &[f64],
    n_nodes: usize,
) -> (f64, Vec<f64>) {
    let d_state = 2;
    debug_assert_eq!(z.len(), (n_nodes - 1) * d_state);

    let mut path = vec![[0.0_f64; 2]; n_nodes + 1];
    path[0] = seed;
    path[n_nodes] = target;
    for i in 0..(n_nodes - 1) {
        path[i + 1] = [z[2 * i], z[2 * i + 1]];
    }

    let mut grad_path = vec![[0.0_f64; 2]; n_nodes + 1];
    let mut total = 0.0;
    for i in 0..n_nodes {
        let (e_seg, de_dp, de_dq) = segment_energy_grad(metric, path[i], path[i + 1]);
        total += e_seg;
        grad_path[i][0] += de_dp[0];
        grad_path[i][1] += de_dp[1];
        grad_path[i + 1][0] += de_dq[0];
        grad_path[i + 1][1] += de_dq[1];
    }

    let mut grad_z = vec![0.0_f64; (n_nodes - 1) * d_state];
    for i in 0..(n_nodes - 1) {
        grad_z[2 * i] = grad_path[i + 1][0];
        grad_z[2 * i + 1] = grad_path[i + 1][1];
    }
    (total, grad_z)
}

fn norm(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

// ─────────────────────────────────────────────────────────────────────────
// Relaxation solver — gradient descent with Armijo backtracking line search.
// The Python toy validation used scipy L-BFGS-B; here we use a hand-rolled
// gradient descent because (a) we control precision absolutely, (b) the
// 2D toy state is tiny (62-126 variables at N=32-64) so the convergence-
// rate hit vs L-BFGS doesn't matter, and (c) no external optimization dep.
// ─────────────────────────────────────────────────────────────────────────

/// Solve the geodesic BVP from `seed` to `target` on `metric` via the
/// relaxation method. Returns the verdict trichotomy.
///
/// Phase 3 contract:
///   * Granted: gradient norm fell below `grad_tol`, the budget gates
///     (curvature ceiling / arc length / holonomy) all pass. Budget
///     checks live in Phase 4 — Phase 3 returns Granted unconditionally
///     on convergence. (Capacity also lives in Phase 4.)
///   * Indeterminate { NonConvergence }: gradient norm did not fall
///     below `grad_tol` within `max_iterations` OR `max_solve_ms`.
///   * Unreachable is NEVER returned by Phase 3.
pub fn relaxation_solve<M: WishMetric2D + ?Sized>(
    metric: &M,
    seed: [f64; 2],
    target: [f64; 2],
    config: &WishConfig,
) -> WishOutcome {
    let n_nodes = match config.solver {
        SolverKind::Relaxation { n_nodes } => n_nodes as usize,
        SolverKind::Shooting => {
            // Defer to shooting; not implemented in Phase 3 beyond the
            // Jacobi field for W2. Caller should pick a different solver
            // for now.
            return WishOutcome::Indeterminate {
                reason: IndeterminateReason::NonConvergence {
                    final_residual: f64::INFINITY,
                },
            };
        }
    };
    let d_state = 2;
    debug_assert!(n_nodes >= 2);

    // Chord initialization.
    let mut z = vec![0.0_f64; (n_nodes - 1) * d_state];
    for i in 0..(n_nodes - 1) {
        let s = (i + 1) as f64 / n_nodes as f64;
        z[2 * i] = (1.0 - s) * seed[0] + s * target[0];
        z[2 * i + 1] = (1.0 - s) * seed[1] + s * target[1];
    }

    let max_ms = config.effective_max_solve_ms() as u128;
    let start = Instant::now();
    let mut iter: u32 = 0;
    let mut last_e = f64::INFINITY;
    let mut energy_stall_count = 0u32;

    let (mut e, mut grad) = total_energy_grad(metric, seed, target, &z, n_nodes);
    // Polak-Ribière nonlinear conjugate gradient: descent direction
    // `d_{k+1} = -g_{k+1} + beta * d_k` with
    // `beta = max(0, (g_{k+1}·(g_{k+1} - g_k)) / (g_k · g_k))`.
    // Restart to steepest descent every `n` iterations or when `beta`
    // is reset to 0. Reaches gtol=1e-6 on curved 62-var BVPs in a few
    // hundred iters where pure steepest descent stalls at ~1e-5.
    let mut direction: Vec<f64> = grad.iter().map(|g| -g).collect();
    let restart_every = z.len();
    let mut final_grad_norm;

    loop {
        final_grad_norm = norm(&grad);
        if final_grad_norm < config.grad_tol {
            break;
        }
        if iter >= config.max_iterations {
            return WishOutcome::Indeterminate {
                reason: IndeterminateReason::NonConvergence {
                    final_residual: final_grad_norm,
                },
            };
        }
        if start.elapsed().as_millis() > max_ms {
            return WishOutcome::Indeterminate {
                reason: IndeterminateReason::NonConvergence {
                    final_residual: final_grad_norm,
                },
            };
        }
        // Armijo backtracking line search along `direction`.
        let mut alpha = 1.0;
        let armijo_c = 1e-4;
        let g_dot_d: f64 = grad.iter().zip(direction.iter()).map(|(g, d)| g * d).sum();
        if g_dot_d >= 0.0 {
            // Direction isn't a descent direction (numerical fluke);
            // restart to steepest descent.
            direction = grad.iter().map(|g| -g).collect();
            continue;
        }
        let mut new_z = z.clone();
        let mut new_e = e;
        let mut accepted = false;
        for _ in 0..40 {
            for k in 0..z.len() {
                new_z[k] = z[k] + alpha * direction[k];
            }
            new_e = total_energy_grad(metric, seed, target, &new_z, n_nodes).0;
            if new_e <= e + armijo_c * alpha * g_dot_d {
                accepted = true;
                break;
            }
            alpha *= 0.5;
        }
        if !accepted {
            return WishOutcome::Indeterminate {
                reason: IndeterminateReason::NonConvergence {
                    final_residual: final_grad_norm,
                },
            };
        }
        let (e_next, grad_next) = total_energy_grad(metric, seed, target, &new_z, n_nodes);
        // Energy-stall detection (per spec §3.2).
        if (last_e - e_next).abs() / e_next.abs().max(1e-12) < config.energy_tol {
            energy_stall_count += 1;
            if energy_stall_count >= 8 {
                return WishOutcome::Indeterminate {
                    reason: IndeterminateReason::NonConvergence {
                        final_residual: norm(&grad_next),
                    },
                };
            }
        } else {
            energy_stall_count = 0;
        }
        // Polak-Ribière β with non-negative clamp.
        let g_old_dot: f64 = grad.iter().map(|g| g * g).sum();
        let beta: f64 = if g_old_dot < 1e-30 {
            0.0
        } else {
            let num: f64 = grad_next
                .iter()
                .zip(grad.iter())
                .map(|(gn, go)| gn * (gn - go))
                .sum();
            (num / g_old_dot).max(0.0)
        };
        // Periodic restart to steepest descent.
        let beta_eff = if iter % restart_every as u32 == 0 { 0.0 } else { beta };
        for k in 0..direction.len() {
            direction[k] = -grad_next[k] + beta_eff * direction[k];
        }
        last_e = e_next;
        z = new_z;
        e = e_next;
        grad = grad_next;
        iter += 1;
    }

    // Assemble the granted path.
    let mut path: Vec<Vec<f64>> = Vec::with_capacity(n_nodes + 1);
    path.push(seed.to_vec());
    for i in 0..(n_nodes - 1) {
        path.push(vec![z[2 * i], z[2 * i + 1]]);
    }
    path.push(target.to_vec());

    // Compute arc length τ via GL-2 quadrature on sqrt(exp(2*phi)).
    let mut tau = 0.0;
    let mut k_int = 0.0;
    for i in 0..n_nodes {
        let p = [path[i][0], path[i][1]];
        let q = [path[i + 1][0], path[i + 1][1]];
        let d = [q[0] - p[0], q[1] - p[1]];
        let d2 = d[0] * d[0] + d[1] * d[1];
        if d2 < 1e-30 {
            continue;
        }
        let seg_len = d2.sqrt();
        for s in [GL2_S_MINUS, GL2_S_PLUS] {
            let v = [p[0] + s * d[0], p[1] + s * d[1]];
            tau += 0.5 * metric.exp2phi(v).sqrt() * seg_len;
            k_int += 0.5 * metric.scalar_curvature(v).abs() * seg_len;
        }
    }

    WishOutcome::Granted {
        path,
        arc_length: tau,
        integrated_curvature: k_int,
        capacity: f64::NAN, // Phase 4
        accumulated_holonomy: k_int, // for 2D, holonomy = K integrated
        solver_iterations: iter,
        final_grad_norm,
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Jacobi field — arc-length-parameterized scalar ODE J'' + K·J = 0 with
// J(0) = 0, J'(0) = 1. Returns the s-values and J-values; the first
// sign change of J marks the conjugate point. Used by W2.
// ─────────────────────────────────────────────────────────────────────────

/// Integrate the perpendicular Jacobi scalar `J(s)` along a geodesic on
/// which the scalar curvature is given by `K_along_geodesic(s)`. Returns
/// the s-values and J-values for inspection.
///
/// On a manifold of constant K the closed-form solution is
///   `K > 0`: `J(s) = sin(sqrt(K)·s) / sqrt(K)`,  first zero at s = π/sqrt(K).
///   `K = 0`: `J(s) = s`,                          no zeros.
///   `K < 0`: `J(s) = sinh(sqrt(-K)·s) / sqrt(-K)`, no zeros.
///
/// The first sign change of J marks the conjugate point along this
/// geodesic; the W2 oracle calls this with `K = 1` (S²) and `K = 4`
/// (CP¹) and verifies the zero lands at π and π/2 respectively.
pub fn jacobi_field_arc_length<F: Fn(f64) -> f64>(
    k_along_geodesic: F,
    s_end: f64,
    n_steps: usize,
) -> (Vec<f64>, Vec<f64>) {
    let h = s_end / n_steps as f64;
    let mut state = [0.0_f64, 1.0]; // (J, J')
    let mut ss = Vec::with_capacity(n_steps + 1);
    let mut js = Vec::with_capacity(n_steps + 1);
    ss.push(0.0);
    js.push(0.0);
    let f = |s_val: f64, st: [f64; 2]| -> [f64; 2] {
        let k = k_along_geodesic(s_val);
        [st[1], -k * st[0]]
    };
    for i in 0..n_steps {
        let s_now = i as f64 * h;
        let k1 = f(s_now, state);
        let k2 = f(
            s_now + 0.5 * h,
            [state[0] + 0.5 * h * k1[0], state[1] + 0.5 * h * k1[1]],
        );
        let k3 = f(
            s_now + 0.5 * h,
            [state[0] + 0.5 * h * k2[0], state[1] + 0.5 * h * k2[1]],
        );
        let k4 = f(s_now + h, [state[0] + h * k3[0], state[1] + h * k3[1]]);
        state[0] += (h / 6.0) * (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]);
        state[1] += (h / 6.0) * (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]);
        ss.push((i + 1) as f64 * h);
        js.push(state[0]);
    }
    (ss, js)
}

/// Find the first index i > 0 where `J[i-1] > 0 && J[i] <= 0`. Returns
/// the arc-length value at that index, or `None` if no zero crossing
/// is found within the integration range. The conjugate-point oracle
/// for W2: on S², this should land within ~1e-2 of π for an integrator
/// with `n_steps >= 4000`.
pub fn first_jacobi_zero(ss: &[f64], js: &[f64]) -> Option<f64> {
    for i in 1..js.len() {
        if js[i - 1] > 0.0 && js[i] <= 0.0 {
            return Some(ss[i]);
        }
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────
// Tests — Rust ports of W1, W2, W5 from `wish_validation.py`.
// W3 (capacity monotonicity) and W4 (waypoint + composition) land in
// Phase 4 alongside the capacity / frontier-truncation logic.
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn open_budgets() -> WishConfig {
        // Tests use loose budgets so the W-math closed-form geodesics
        // aren't reclassified Unreachable by the trust-envelope ceilings.
        // grad_tol = 1e-5 is the honest floor for a hand-rolled Polak-
        // Ribière CG solver with Armijo backtracking (vs the 1e-6
        // scipy L-BFGS-B floor the Python validation used). The
        // discretization-rate test below verifies the SOLVER is
        // correct, not just terminating; that's the real W1(b) check.
        let mut c = WishConfig {
            max_imagined_curvature: 1e9,
            max_accumulated_holonomy: 1e9,
            max_arc_length: 1e9,
            max_iterations: 5000,
            max_solve_ms: 60_000,
            grad_tol: 1e-5,
            energy_tol: 1e-12,
            ..Default::default()
        };
        c.solver = SolverKind::Relaxation { n_nodes: 32 };
        c
    }

    // ─── W1 ───────────────────────────────────────────────────────────────

    #[test]
    fn w1_solver_converges_on_flat_t2() {
        // The straight-line chord init IS the geodesic on T²; the solver
        // should converge in zero iterations and return arc_length equal
        // to the Euclidean distance.
        let m = T2Flat;
        let cfg = open_budgets();
        let out = relaxation_solve(&m, [0.0, 0.0], [0.6, 0.4], &cfg);
        match out {
            WishOutcome::Granted { arc_length, final_grad_norm, .. } => {
                let exp = (0.6_f64 * 0.6 + 0.4 * 0.4).sqrt();
                assert!(
                    (arc_length - exp).abs() < 1e-9,
                    "T² arc length {} vs analytic {}",
                    arc_length,
                    exp
                );
                assert!(final_grad_norm < 1e-6, "grad_norm={}", final_grad_norm);
            }
            other => panic!("expected Granted on flat T², got {:?}", variant_name(&other)),
        }
    }

    #[test]
    fn w1_solver_converges_on_s2_stereographic() {
        let m = S2Stereographic;
        let cfg = open_budgets();
        let out = relaxation_solve(&m, [0.1, 0.0], [0.5, 0.3], &cfg);
        let arc_length = match &out {
            WishOutcome::Granted { arc_length, .. } => *arc_length,
            WishOutcome::Indeterminate { reason } => {
                panic!("expected Granted on S², got Indeterminate: {:?}", reason)
            }
            other => panic!("expected Granted on S², got {:?}", variant_name(other)),
        };
        // Closed-form arc length: angle between embedded points.
        let p0 = stereo_to_embed([0.1, 0.0]);
        let p1 = stereo_to_embed([0.5, 0.3]);
        let cos_a = (p0[0] * p1[0] + p0[1] * p1[1] + p0[2] * p1[2]).clamp(-1.0, 1.0);
        let arc_cf = cos_a.acos();
        let rel = (arc_length - arc_cf).abs() / arc_cf;
        assert!(rel < 5e-4, "S² arc length {} vs {} (rel {})", arc_length, arc_cf, rel);
    }

    #[test]
    fn w1_oh_squared_discretization_decay_on_s2() {
        // Per Fable's GL-2 fix: discretization error should decay at
        // rate O(h²) when N doubles. The Python validation observed
        // ratios of exactly 4.00 on S². We check that ratios live in
        // [2.0, 8.0] (the same envelope the Python W1 uses).
        let m = S2Stereographic;
        let p0 = stereo_to_embed([0.1, 0.0]);
        let p1 = stereo_to_embed([0.5, 0.3]);
        let cos_a = (p0[0] * p1[0] + p0[1] * p1[1] + p0[2] * p1[2]).clamp(-1.0, 1.0);
        let arc_cf = cos_a.acos();
        let mut residuals = Vec::new();
        for n in [8u32, 16, 32, 64] {
            let mut cfg = open_budgets();
            cfg.solver = SolverKind::Relaxation { n_nodes: n };
            // gtol = 1e-7 is below the expected discretization error
            // at N=8 (~2e-6) and N=16 (~5e-7), so the residual the test
            // measures is discretization-dominated, not solver-floor-
            // dominated. Tighter is unreliable for a hand-rolled CG.
            cfg.grad_tol = 1e-7;
            cfg.energy_tol = 1e-14;
            cfg.max_iterations = 20_000;
            let out = relaxation_solve(&m, [0.1, 0.0], [0.5, 0.3], &cfg);
            match out {
                WishOutcome::Granted { arc_length, .. } => {
                    residuals.push((arc_length - arc_cf).abs() / arc_cf);
                }
                _ => {
                    // At high N the solver may hit gtol floor; record as
                    // NaN and rely on the lower-N ratios.
                    residuals.push(f64::NAN);
                }
            }
        }
        // First two ratios: r[0]/r[1] and r[1]/r[2]. Both should sit near 4.
        let r1 = residuals[0] / residuals[1];
        let r2 = residuals[1] / residuals[2];
        assert!(
            (2.0..=8.0).contains(&r1) && (2.0..=8.0).contains(&r2),
            "discretization ratios outside [2,8] envelope: r1={}, r2={}, residuals={:?}",
            r1,
            r2,
            residuals
        );
    }

    // ─── W2 ───────────────────────────────────────────────────────────────

    #[test]
    fn w2_jacobi_field_zero_at_pi_on_s2_constant_k() {
        // S²(K=1): J(s) = sin(s), first zero at s = π.
        let (ss, js) = jacobi_field_arc_length(|_s| 1.0, 4.0, 4000);
        let zero = first_jacobi_zero(&ss, &js).expect("conjugate point on S²");
        assert!(
            (zero - std::f64::consts::PI).abs() < 0.02,
            "S² Jacobi zero at {} (expected {})",
            zero,
            std::f64::consts::PI
        );
    }

    #[test]
    fn w2_jacobi_field_zero_at_pi_over_two_on_cp1() {
        // CP¹(K=4): J(s) = sin(2s)/2, first zero at s = π/2.
        let (ss, js) = jacobi_field_arc_length(|_s| 4.0, 4.0, 4000);
        let zero = first_jacobi_zero(&ss, &js).expect("conjugate point on CP¹");
        let exp = std::f64::consts::FRAC_PI_2;
        assert!(
            (zero - exp).abs() < 0.02,
            "CP¹ Jacobi zero at {} (expected {})",
            zero,
            exp
        );
    }

    #[test]
    fn w2_flat_t2_has_no_conjugate_point() {
        // K=0: J(s) = s, never zero.
        let (ss, js) = jacobi_field_arc_length(|_s| 0.0, 4.0, 4000);
        let zero = first_jacobi_zero(&ss, &js);
        assert!(
            zero.is_none(),
            "T² flat should have no conjugate point in [0, 4]; found {:?}",
            zero
        );
    }

    #[test]
    fn w2_ill_conditioned_relaxation_returns_indeterminate() {
        let m = S2Stereographic;
        let cfg = WishConfig {
            max_iterations: 2,
            max_solve_ms: 60_000,
            solver: SolverKind::Relaxation { n_nodes: 32 },
            ..Default::default()
        };
        let out = relaxation_solve(&m, [0.1, 0.0], [0.5, 0.3], &cfg);
        match out {
            WishOutcome::Indeterminate {
                reason: IndeterminateReason::NonConvergence { .. },
            } => {}
            other => panic!(
                "expected Indeterminate{{NonConvergence}}, got {:?}",
                variant_name(&other)
            ),
        }
    }

    // ─── W5 ───────────────────────────────────────────────────────────────

    #[test]
    fn w5_iteration_cap_returns_indeterminate_not_partial_grant() {
        // max_iterations = 1: solver cannot converge on a curved BVP.
        // Must NOT return a Granted with a partial path.
        let m = S2Stereographic;
        let cfg = WishConfig {
            max_iterations: 1,
            max_solve_ms: 60_000,
            solver: SolverKind::Relaxation { n_nodes: 64 },
            ..Default::default()
        };
        let out = relaxation_solve(&m, [0.1, 0.0], [0.5, 0.3], &cfg);
        match out {
            WishOutcome::Indeterminate { .. } => {}
            other => panic!(
                "max_iterations=1 must produce Indeterminate, got {:?}",
                variant_name(&other)
            ),
        }
    }

    #[test]
    fn w5_max_solve_ms_floor_is_enforced() {
        // The GIGI-team review's 50 ms floor: a caller setting
        // max_solve_ms = 1 should still get at least 50 ms of compute
        // budget. We don't observe the floor directly here (timing is
        // flaky in CI); we check `effective_max_solve_ms()` reports it.
        let cfg = WishConfig {
            max_solve_ms: 1,
            ..Default::default()
        };
        assert_eq!(cfg.effective_max_solve_ms(), MAX_SOLVE_MS_FLOOR);
        let cfg = WishConfig {
            max_solve_ms: 500,
            ..Default::default()
        };
        assert_eq!(cfg.effective_max_solve_ms(), 500);
    }

    // ─── support ──────────────────────────────────────────────────────────

    fn stereo_to_embed(p: [f64; 2]) -> [f64; 3] {
        let s = 1.0 + p[0] * p[0] + p[1] * p[1];
        [2.0 * p[0] / s, 2.0 * p[1] / s, (p[0] * p[0] + p[1] * p[1] - 1.0) / s]
    }

    fn variant_name(o: &WishOutcome) -> &'static str {
        match o {
            WishOutcome::Granted { .. } => "Granted",
            WishOutcome::Unreachable { .. } => "Unreachable",
            WishOutcome::Indeterminate { .. } => "Indeterminate",
        }
    }
}
