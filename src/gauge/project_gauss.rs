//! `PROJECT_GAUSS` — Tikhonov-regularized CG solver that projects an
//! E field onto the Gauss-constraint surface.
//!
//! Closes TDD-HAL-IV.3. The Gauss projector enforces
//! `||G_cov(E_clean)||_inf ≤ cg_tol` by finding a per-vertex Lie-
//! algebra Lagrange multiplier `λ ∈ ℝ^(V·3)` such that
//!
//! ```text
//!     E_clean = E_dirty − D_cov(U)^T · λ.
//! ```
//!
//! The normal equation is
//!
//! ```text
//!     L_cov(U) · λ = G_cov(E_dirty),
//!     L_cov(U) = D_cov(U) · D_cov(U)^T + tikhonov · I,
//! ```
//!
//! and we solve it with plain Hestenes–Stiefel CG. No preconditioner
//! (Bee's locked decision IV-E: buckyball cond(L_cov) ~ 16 needs none;
//! a Jacobi preconditioner is P1 future-tense).
//!
//! ── Operator definitions ──
//!
//! `D_cov(U)` is the per-vertex covariant divergence operator from
//! `gauss.rs`. Its action on an E field reads
//!
//! ```text
//!     [D_cov(U) · E][v] = Σ_{i: o_i=Forward} E[e_i].vec
//!                       - Σ_{i: o_i=Reverse} Ad(U[e_i]) E[e_i].vec.
//! ```
//!
//! Its transpose `D_cov(U)^T` distributes a per-vertex Lie vector
//! `λ` back to edges:
//!
//! ```text
//!     [D_cov(U)^T · λ][e] = λ[head(e)] - Ad(U[e])^T λ[tail(e)].
//! ```
//!
//! `Ad(U)^T = Ad(U^†)` because `Ad` is an SO(3) rotation, so the
//! transpose reduces to an Ad-sandwich with the conjugate quaternion.
//!
//! The composite matvec `apply_l_cov_matvec(U, v)` computes
//! `D_cov(U) · (D_cov(U)^T · v) + tikhonov · v` in two passes (edge
//! → vertex via `D_cov`, vertex → edge via `D_cov^T`), so each CG
//! iteration is O(E + V) on the buckyball.
//!
//! ── Group-erasure note ──
//!
//! SU(2)-only at launch. `project_gauss` dispatches on
//! `u.group()` and returns `UnsupportedGroup` for non-SU(2) inputs.
//! Future SU(3) ships a sibling `project_gauss_su3` with its own
//! `Ad` operator; future U(1) would skip the Ad action entirely
//! (`Ad ≡ id`).
//!
//! Reference: Halcyon Python `inertia_damping/buckyball_action.py`
//! `project_gauss_zero_cg`.

use super::e_field::SU2EField;
use super::error::GaugeFieldError;
use super::gauss::{compute_gauss_residual_covariant, max_inf_norm, VertexEdgeIncidence};
use super::group::Group;
use super::registry::GaugeFieldHandle;
use crate::lattice::{EdgeOrientation, Lattice};

/// CG knobs for `PROJECT_GAUSS`.
///
/// Locked decision IV-A: `Default` returns the Halcyon-production
/// defaults `{ tikhonov: 1e-14, cg_tol: 1e-10, cg_max_iter: 200 }`.
/// The 1e-12 spec default is reachable via an explicit struct literal
/// (`ProjectGaussConfig { tikhonov: 1e-12, .. }`).
#[derive(Debug, Clone, Copy)]
pub struct ProjectGaussConfig {
    /// Tikhonov regularization weight added to `D · D^T` before CG.
    /// Default `1e-14` matches Halcyon Python production.
    pub tikhonov: f64,
    /// CG relative-residual stopping tolerance on
    /// `||L · λ - G(E)||_2 / ||G(E)||_2`. Default `1e-10`.
    pub cg_tol: f64,
    /// Hard cap on CG iterations. Default `200`. Non-convergence is a
    /// diagnostic (`cg_did_not_converge = true`) — never a panic, never
    /// an `Err`.
    pub cg_max_iter: usize,
}

impl Default for ProjectGaussConfig {
    fn default() -> Self {
        Self {
            tikhonov: 1e-14,
            cg_tol: 1e-10,
            cg_max_iter: 200,
        }
    }
}

/// Per-call diagnostics from a single `project_gauss` invocation.
#[derive(Debug, Clone)]
pub struct ProjectGaussDiagnostics {
    /// CG iterations consumed. Equal to `cg_max_iter` on non-
    /// convergence; strictly less on a clean exit.
    pub cg_iterations: usize,
    /// Final relative residual `||L · λ - G(E)||_2 / ||G(E)||_2`
    /// observed at exit (NaN-safe: 0.0 when the input residual is
    /// already zero).
    pub cg_residual_final: f64,
    /// `true` iff CG hit `cg_max_iter` without reaching `cg_tol`. The
    /// projector still writes back the partial `E_clean`; downstream
    /// code reads this flag and decides whether to abort the
    /// surrounding leapfrog step.
    pub cg_did_not_converge: bool,
    /// `||G_cov(E_dirty)||_inf` measured before the projection. Useful
    /// for the IV.10 acceptance bound (`< 1e-3` energy drift) and the
    /// SHOW E_FIELD diagnostics row.
    pub initial_gauss_residual_inf: f64,
    /// `||G_cov(E_clean)||_inf` measured after the projection. The
    /// production-canonical target is `< 1e-9` on the buckyball.
    pub final_gauss_residual_inf: f64,
}

/// Project `e_dirty` onto the Gauss-constraint surface.
///
/// Algorithm (mirrors Halcyon `project_gauss_zero_cg`):
///
/// 1. Compute `b = G_cov(E_dirty)` via the existing covariant
///    divergence operator.
/// 2. Solve `L_cov(U) · λ = b` by unpreconditioned CG with relative-
///    residual stopping `||r||_2 / ||b||_2 ≤ cg_tol` or
///    `cg_max_iter` exhausted.
/// 3. Subtract `D_cov(U)^T · λ` from `E_dirty` in place (q0=0
///    invariant re-enforced on every write).
/// 4. Recompute `G_cov(E_clean)` for the diagnostics row.
///
/// Group dispatch: `u.group()` must be `Group::SU2`. Non-SU(2) inputs
/// return `Err(UnsupportedGroup(_))`. The E field's group is
/// implicitly SU(2) by the `&mut SU2EField` signature.
///
/// `inc` is the vertex-edge incidence the caller hoisted out of any
/// sweep loop (see `build_vertex_edge_incidence`). The projector does
/// NOT recompute it.
pub fn project_gauss(
    e_dirty: &mut SU2EField,
    u: &dyn GaugeFieldHandle,
    lat: &Lattice,
    inc: &VertexEdgeIncidence,
    config: ProjectGaussConfig,
) -> Result<ProjectGaussDiagnostics, GaugeFieldError> {
    if u.group() != Group::SU2 {
        return Err(GaugeFieldError::UnsupportedGroup(u.group()));
    }

    // 1. b = G_cov(E_dirty) — flatten (V, 3) into a single Vec<f64>
    //    so the CG inner loop sees a plain dense vector.
    let b_vec = compute_gauss_residual_covariant(u, e_dirty, lat, inc)?;
    let initial_gauss_residual_inf = max_inf_norm(&b_vec);
    let b = flatten(&b_vec);

    // 2. CG: L · λ = b. No preconditioner (locked decision IV-E).
    let (lambda, cg_iterations, cg_residual_final, cg_did_not_converge) =
        cg_solve(u, lat, inc, &b, config);

    // 3. E_clean = E_dirty − D_cov(U)^T · λ — in place. The q0=0
    //    invariant is forced on every write by `SU2EField::write_element_q`.
    let lambda_vec = unflatten(&lambda, lat.n_vertices);
    apply_d_cov_transpose_subtract(e_dirty, u, lat, inc, &lambda_vec);

    // 4. Final residual for the diagnostics row.
    let final_residual = compute_gauss_residual_covariant(u, e_dirty, lat, inc)?;
    let final_gauss_residual_inf = max_inf_norm(&final_residual);

    Ok(ProjectGaussDiagnostics {
        cg_iterations,
        cg_residual_final,
        cg_did_not_converge,
        initial_gauss_residual_inf,
        final_gauss_residual_inf,
    })
}

// ─────────────────────────── CG inner loop ───────────────────────────

/// Unpreconditioned Hestenes–Stiefel CG for `L · λ = b`.
///
/// Stopping rule: `||r||_2 ≤ cg_tol · ||b||_2` (relative residual) or
/// `cg_max_iter` exhausted. Returns `(lambda, iter, rel_residual,
/// did_not_converge)`.
fn cg_solve(
    u: &dyn GaugeFieldHandle,
    lat: &Lattice,
    inc: &VertexEdgeIncidence,
    b: &[f64],
    config: ProjectGaussConfig,
) -> (Vec<f64>, usize, f64, bool) {
    let n = b.len();
    let b_norm = dot(b, b).sqrt();
    let mut x = vec![0.0_f64; n];

    // r = b − L · x. With x = 0 → r = b.
    let mut r = b.to_vec();
    let mut p = r.clone();
    let mut rs_old = dot(&r, &r);

    if b_norm == 0.0 {
        // Already satisfied — zero λ is the answer; no iterations.
        return (x, 0, 0.0, false);
    }
    let tol_abs = config.cg_tol * b_norm;
    let tol_sq = tol_abs * tol_abs;

    if rs_old <= tol_sq {
        // Initial residual already below tol (degenerate input).
        return (x, 0, rs_old.sqrt() / b_norm, false);
    }

    let mut iter = 0;
    let mut rel_residual = 1.0_f64;
    let mut did_not_converge = true;
    while iter < config.cg_max_iter {
        iter += 1;
        let ap = apply_l_cov_matvec(u, lat, inc, &p, config.tikhonov);
        let p_ap = dot(&p, &ap);
        if p_ap == 0.0 {
            // Numerical breakdown — emit current x and bail.
            rel_residual = rs_old.sqrt() / b_norm;
            break;
        }
        let alpha = rs_old / p_ap;
        for i in 0..n {
            x[i] += alpha * p[i];
            r[i] -= alpha * ap[i];
        }
        let rs_new = dot(&r, &r);
        rel_residual = rs_new.sqrt() / b_norm;
        if rs_new <= tol_sq {
            did_not_converge = false;
            break;
        }
        let beta = rs_new / rs_old;
        for i in 0..n {
            p[i] = r[i] + beta * p[i];
        }
        rs_old = rs_new;
    }

    (x, iter, rel_residual, did_not_converge)
}

#[inline]
fn dot(a: &[f64], b: &[f64]) -> f64 {
    debug_assert_eq!(a.len(), b.len());
    let mut s = 0.0_f64;
    for i in 0..a.len() {
        s += a[i] * b[i];
    }
    s
}

#[inline]
fn flatten(v: &[[f64; 3]]) -> Vec<f64> {
    let mut out = Vec::with_capacity(v.len() * 3);
    for row in v {
        out.push(row[0]);
        out.push(row[1]);
        out.push(row[2]);
    }
    out
}

#[inline]
fn unflatten(v: &[f64], n_vertices: usize) -> Vec<[f64; 3]> {
    debug_assert_eq!(v.len(), 3 * n_vertices);
    let mut out = vec![[0.0_f64; 3]; n_vertices];
    for vid in 0..n_vertices {
        out[vid][0] = v[3 * vid];
        out[vid][1] = v[3 * vid + 1];
        out[vid][2] = v[3 * vid + 2];
    }
    out
}

// ─────────────────────────── Operator helpers ───────────────────────────

/// `D_cov(U)^T · λ → per-edge Lie row`. For an edge `e = (a, b)`
/// (tail `a`, head `b`):
///
/// ```text
///     [D_cov(U)^T λ][e] = λ[b] − Ad(U_e)^T λ[a]
///                       = λ[b] − Ad(U_e^†)  λ[a].
/// ```
///
/// This is the transpose of the per-vertex divergence operator in
/// `gauss.rs`. Used in two places: (1) the CG matvec
/// `apply_l_cov_matvec` (forward sweep, applies `D` to the result),
/// (2) `apply_d_cov_transpose_subtract` which writes the final
/// `E_clean = E_dirty − D^T λ` back into the SU(2) E field.
fn apply_d_cov_transpose_per_edge(
    u: &dyn GaugeFieldHandle,
    lat: &Lattice,
    lambda: &[[f64; 3]],
) -> Vec<[f64; 3]> {
    let u_buf = u.as_dense_buffer();
    let mut out = vec![[0.0_f64; 3]; lat.n_edges()];
    for eid in 0..lat.n_edges() {
        let (a, b) = lat.edges[eid];
        let la = lambda[a];
        let lb = lambda[b];
        let base = u_buf.repr_dim * eid;
        // Conjugate quaternion for Ad(U^†): (q0, -q1, -q2, -q3).
        let u0 = u_buf.data[base];
        let u1 = -u_buf.data[base + 1];
        let u2 = -u_buf.data[base + 2];
        let u3 = -u_buf.data[base + 3];
        let ad = ad_action_su2([u0, u1, u2, u3], la);
        out[eid][0] = lb[0] - ad[0];
        out[eid][1] = lb[1] - ad[1];
        out[eid][2] = lb[2] - ad[2];
    }
    out
}

/// Apply `D_cov(U)` to a per-edge Lie buffer `e_rows`, returning the
/// per-vertex divergence. Mirrors `compute_gauss_residual_covariant`
/// but reads from a raw `Vec<[f64; 3]>` (the intermediate `D^T λ`
/// inside CG), not an `EFieldHandle`.
fn apply_d_cov_per_vertex(
    u: &dyn GaugeFieldHandle,
    lat: &Lattice,
    inc: &VertexEdgeIncidence,
    e_rows: &[[f64; 3]],
) -> Vec<[f64; 3]> {
    let u_buf = u.as_dense_buffer();
    let mut out = vec![[0.0_f64; 3]; lat.n_vertices];
    for vid in 0..lat.n_vertices {
        let mut g = [0.0_f64; 3];
        for &(eid, orient) in &inc[vid] {
            let ev = e_rows[eid];
            match orient {
                EdgeOrientation::Forward => {
                    g[0] += ev[0];
                    g[1] += ev[1];
                    g[2] += ev[2];
                }
                EdgeOrientation::Reverse => {
                    let base = u_buf.repr_dim * eid;
                    let u0 = u_buf.data[base];
                    let u1 = u_buf.data[base + 1];
                    let u2 = u_buf.data[base + 2];
                    let u3 = u_buf.data[base + 3];
                    let r = ad_action_su2([u0, u1, u2, u3], ev);
                    g[0] -= r[0];
                    g[1] -= r[1];
                    g[2] -= r[2];
                }
            }
        }
        out[vid] = g;
    }
    out
}

/// CG matvec: `apply_l_cov_matvec(U, λ) = D_cov(U) · D_cov(U)^T · λ
/// + tikhonov · λ`. Flat in/out (`Vec<f64>` of length `3 · V`) so the
/// inner CG loop stays on a dense vector.
fn apply_l_cov_matvec(
    u: &dyn GaugeFieldHandle,
    lat: &Lattice,
    inc: &VertexEdgeIncidence,
    lambda_flat: &[f64],
    tikhonov: f64,
) -> Vec<f64> {
    let lambda = unflatten(lambda_flat, lat.n_vertices);
    let dt_lambda = apply_d_cov_transpose_per_edge(u, lat, &lambda);
    let l_lambda = apply_d_cov_per_vertex(u, lat, inc, &dt_lambda);
    let mut out = flatten(&l_lambda);
    for i in 0..out.len() {
        out[i] += tikhonov * lambda_flat[i];
    }
    out
}

/// Final write-back: `E_clean[e] = E_dirty[e] − D_cov(U)^T λ[e]`. The
/// q0=0 invariant is forced on every write through
/// `SU2EField::write_element_q`.
fn apply_d_cov_transpose_subtract(
    e: &mut SU2EField,
    u: &dyn GaugeFieldHandle,
    lat: &Lattice,
    _inc: &VertexEdgeIncidence,
    lambda: &[[f64; 3]],
) {
    let dt_lambda = apply_d_cov_transpose_per_edge(u, lat, lambda);
    for eid in 0..lat.n_edges() {
        let row = e.read_element_q(eid);
        let new_row = [
            0.0,
            row[1] - dt_lambda[eid][0],
            row[2] - dt_lambda[eid][1],
            row[3] - dt_lambda[eid][2],
        ];
        e.write_element_q(eid, new_row);
    }
}

/// SU(2) adjoint action on the Lie algebra: `Ad(U) v = U · (0, v) ·
/// U^†`. Local mirror of `gauss::ad_action_su2` (private there); kept
/// in-module so the CG matvec doesn't reach across visibility for a
/// single inline helper.
#[inline]
fn ad_action_su2(u: [f64; 4], v: [f64; 3]) -> [f64; 3] {
    let u0 = u[0];
    let u1 = u[1];
    let u2 = u[2];
    let u3 = u[3];
    // Step 1: a = u · (0, v).
    let s = u1 * v[0] + u2 * v[1] + u3 * v[2];
    let cx = u2 * v[2] - u3 * v[1];
    let cy = u3 * v[0] - u1 * v[2];
    let cz = u1 * v[1] - u2 * v[0];
    let a0 = -s;
    let a1 = u0 * v[0] - cx;
    let a2 = u0 * v[1] - cy;
    let a3 = u0 * v[2] - cz;
    // Step 2: c = a · u^†, u^† = (u0, -u1, -u2, -u3).
    let bx = a2 * u3 - a3 * u2;
    let by = a3 * u1 - a1 * u3;
    let bz = a1 * u2 - a2 * u1;
    let c1 = -a0 * u1 + u0 * a1 + bx;
    let c2 = -a0 * u2 + u0 * a2 + by;
    let c3 = -a0 * u3 + u0 * a3 + bz;
    [c1, c2, c3]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `ProjectGaussConfig::default()` returns the Halcyon-production
    /// defaults per locked decision IV-A. Direct unit smoke — the
    /// load-bearing integration check lives in
    /// `tests/gauge_project_gauss_unit.rs`.
    #[test]
    fn default_config_is_halcyon_production() {
        let cfg = ProjectGaussConfig::default();
        assert_eq!(cfg.tikhonov, 1e-14);
        assert_eq!(cfg.cg_tol, 1e-10);
        assert_eq!(cfg.cg_max_iter, 200);
    }

    /// `flatten` / `unflatten` round-trip is the identity.
    #[test]
    fn flatten_unflatten_round_trip() {
        let v: Vec<[f64; 3]> = vec![[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]];
        let f = flatten(&v);
        let u = unflatten(&f, 3);
        assert_eq!(v, u);
    }
}
