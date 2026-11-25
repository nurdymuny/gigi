//! The exact SO(2) charge projector, by the norm bound of the quasimode construction registration (§2), not by a
//! finite angular sum.
//!
//! **The generator.** R_θ = diag(1, rot θ) acts by G ↦ R_θ G R_θᵀ; L = −i d/dθ at θ = 0, so v₊ = G01 + iG02 has
//! charge +1 and w₊ = (G11 − G22)/2 + iG12 has charge +2. In the log-Cholesky chart (r_i = e^{u_i}, a, b, c) the
//! rotation's velocity is the exact Laurent field
//!     u̇ = (0, −c e^{−u1}, c e^{−u1}, −b, a, e^{u1} − e^{2u2 − u1}),
//! the same field as `velocity` in `helike_quasimode.geometry` (tested here against J_u(L) u̇ = JG − GJ, and against
//! finite differences of the actual rotation of the saved trial function). So
//!     (L − m)ψ / f = −i Σ_i u̇_i (∂ψ/∂u_i)/f − m P
//! is a finite exponential-polynomial with four exponential groups, and its norm is enclosed by the same centred
//! moment sum as the residual (`residual::norm_squared`).
//!
//! **The bound.** For integer m on this single-valued representation, ‖(I − P_m)ψ‖ ≤ ‖(L − m)ψ‖. With η invariant
//! and genuine upper bounds t ≥ ‖(1 − η)ψ‖, c ≥ ‖(L − m)ψ‖, r ≥ ‖(H − E)ψ‖, ℓ ≥ ‖[H, η]ψ‖ for normalised ψ:
//! if t < 1 and q² = (1 − t)² − c² > 0, then ‖P_m ηψ‖ ≥ q and ‖(H − E)P_m ηψ‖ / ‖P_m ηψ‖ ≤ (r + ℓ)/q.
//! r and c are enclosed here; t and ℓ are the Arb receipt's upper endpoints (helike_cutoff_bounds.py, c31aab8e),
//! for the same frozen binary64 trial vector. E is the saved spectral centre, not the projected Rayleigh quotient.
//!
//! **What this does not do.** No separation hypothesis, no Temple bound, no gate verdict, no statement about the
//! level's identity beyond its exact charge sector.

use crate::interval::{Cx, Iv, ONE};
use crate::laurent::Poly;
use crate::residual::{
    self, accumulate, embed, finish_groups, physical_gradient, ChartIv, Group, Mutation, SavedControl, Space,
};
use crate::basis::State;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The rotation velocity u̇ as exact Laurent polynomials (key = exponents of e^{u0..u2}, powers of u3..u5).
pub fn charge_velocity(mutation: Mutation) -> [Poly; 6] {
    let m = |c: f64, k: [i32; 6]| Poly::mono(c, k);
    let mut dc = m(1.0, [0, 1, 0, 0, 0, 0]);
    if mutation != Mutation::DropGeneratorTerm {
        dc = dc.add(&m(-1.0, [0, -1, 2, 0, 0, 0]));
    }
    [
        Poly::zero(),
        m(-1.0, [0, -1, 0, 0, 0, 1]),
        m(1.0, [0, -1, 0, 0, 0, 1]),
        m(-1.0, [0, 0, 0, 0, 1, 0]),
        m(1.0, [0, 0, 0, 1, 0, 0]),
        dc,
    ]
}

/// The groups of (L − m_j)ψ_j / f, one column per state.
pub fn build_charge(
    chart: &ChartIv,
    basis: &[State],
    w: [f64; 6],
    coeff: &[Cx],
    charges: &[i64],
    mutation: Mutation,
) -> Result<(Space, Vec<Group>), String> {
    let ncol = charges.len();
    // first derivatives raise the degree by one, the polynomial velocity components by one more
    let (space, p) = embed(w, basis, coeff, ncol, 2)?;
    let first = physical_gradient(&space, chart, &p, mutation != Mutation::DropDensityDerivative)?;
    let mut groups: BTreeMap<[i32; 3], Vec<Cx>> = BTreeMap::new();
    let mut g0 = p.clone();
    for row in g0.chunks_mut(ncol) {
        for (j, v) in row.iter_mut().enumerate() {
            let m = if mutation == Mutation::ReverseCharge { -charges[j] } else { charges[j] };
            *v = v.scale(Iv::int(-m));
        }
    }
    groups.insert([0, 0, 0], g0);
    let velocity = charge_velocity(mutation);
    for (i, vi) in velocity.iter().enumerate() {
        if vi.0.is_empty() {
            continue;
        }
        // −i · ∂ψ/∂u_i
        let rotated: Vec<Cx> = first[i].iter().map(|v| Cx::new(v.im, -v.re)).collect();
        for (key, &c) in &vi.0 {
            accumulate(&space, chart, &mut groups, *key, &rotated, Iv::point(c))?;
        }
    }
    Ok((space, finish_groups(chart, groups)))
}

#[derive(Clone, Debug, Serialize)]
pub struct SectorBound {
    /// q with ‖P_m ηψ‖ ≥ q (normalised ψ)
    pub q_lower: f64,
    /// (r + ℓ)/q ≥ ‖(H − E)P_m ηψ‖ / ‖P_m ηψ‖
    pub residual_upper: f64,
}

/// The registration's §2 inequality on genuine upper bounds (all for normalised ψ). Refuses rather than returns a
/// number when an input is invalid or the projected norm has no positive lower bound.
pub fn sector_bound(t: f64, ell: f64, r: f64, c: f64) -> Result<SectorBound, String> {
    for (name, v) in [("tail", t), ("leakage", ell), ("residual", r), ("charge", c)] {
        if !(v.is_finite() && v >= 0.0) {
            return Err(format!("{name} bound must be finite and nonnegative"));
        }
    }
    if !(t < 1.0) {
        return Err("tail bound must be below one".into());
    }
    let q2 = (ONE - Iv::point(t)).sqr() - Iv::point(c).sqr();
    if !(q2.lo > 0.0) {
        return Err("projected norm has no positive lower bound".into());
    }
    let q = Iv::point(q2.lo).sqrt().lo;
    let bound = (Iv::point(r) + Iv::point(ell)) / Iv::point(q);
    Ok(SectorBound { q_lower: q, residual_upper: bound.hi })
}

// ---------------------------------------------------------------------------------------------------------------
// Inputs from the Arb record (oracle/export_projection_inputs.py) and the per-control runner.

#[derive(Clone, Debug, Deserialize)]
pub struct ArbState {
    pub name: String,
    pub charge: i64,
    pub tail_norm_upper: f64,
    pub leakage_norm_upper: f64,
    pub tail_norm_upper_dyadic: serde_json::Value,
    pub leakage_norm_upper_dyadic: serde_json::Value,
    pub reference_charge_sq: BTreeMap<String, f64>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ArbControl {
    pub g2: f64,
    #[serde(rename = "Mq")]
    pub mq: usize,
    pub trial_file: String,
    pub trial_sha256: String,
    pub trial_coeff_re: Vec<u64>,
    pub trial_coeff_im: Vec<u64>,
    pub states: Vec<ArbState>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ProjectionInputs {
    pub schema: String,
    pub states_file_sha256: String,
    pub cutoff_receipt_sha256: String,
    pub cutoff_source_revision: String,
    pub construction_receipt_sha256: String,
    pub controls: Vec<ArbControl>,
}

pub fn load_projection_inputs(path: &std::path::Path) -> Result<ProjectionInputs, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let s: ProjectionInputs = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if s.schema != "helike-lab/projection-inputs/v1" {
        return Err("unexpected projection-input schema".into());
    }
    Ok(s)
}

#[derive(Clone, Debug, Serialize)]
pub struct SectorState {
    pub name: String,
    pub charge: i64,
    pub center: f64,
    pub norm_sq: [f64; 2],
    /// ‖(L − m)ψ‖² (Gaussian measure, unnormalised)
    pub charge_sq: [f64; 2],
    /// c = ‖(L − m)ψ‖ / ‖ψ‖, upper
    pub charge_upper: f64,
    /// the construction receipt's quadrature charge_sq at orders 10 and 12 (diagnostics, not enclosures)
    pub reference_charge_sq: BTreeMap<String, f64>,
    pub reference_charge_inside: BTreeMap<String, bool>,
    /// r = ‖(H − E)ψ‖ / ‖ψ‖ for the hulled trial coefficients
    pub r_lower: f64,
    pub r_upper: f64,
    /// Arb upper endpoints (receipt c31aab8e), rounded upward to binary64
    pub tail_norm_upper: f64,
    pub leakage_norm_upper: f64,
    pub sector: Result<SectorBound, String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SectorControl {
    pub g2: f64,
    #[serde(rename = "Mq")]
    pub mq: usize,
    pub trial_file: String,
    pub trial_sha256: String,
    pub charge_workspace_degree: usize,
    pub charge_group_count: usize,
    pub residual_group_count: usize,
    pub states: Vec<SectorState>,
    pub seconds: f64,
}

fn bits(v: &[u64]) -> Vec<f64> {
    v.iter().map(|&b| f64::from_bits(b)).collect()
}

/// Charge defects ‖(L − m)ψ‖² and norms ‖ψ‖² for the given coefficients.
pub fn charge_defects(
    c: &SavedControl,
    coeff: &[Cx],
    charges: &[i64],
    mutation: Mutation,
) -> Result<(Vec<Iv>, Vec<Iv>, usize, usize), String> {
    let (chart, w) = residual::saved_chart(c)?;
    let basis = residual::saved_basis(c)?;
    let (space, groups) = build_charge(&chart, &basis, w, coeff, charges, mutation)?;
    let (values, _) = residual::norm_squared(&space, &groups, mutation)?;
    let ncol = charges.len();
    let rows = basis.len();
    let norms = (0..ncol)
        .map(|j| residual::tree_sum(&(0..rows).map(|n| coeff[n * ncol + j].norm_sq()).collect::<Vec<_>>()))
        .collect();
    Ok((values, norms, space.degree, groups.len()))
}

/// Everything for one control: residual and charge enclosures of the frozen trial, and the sector bound.
pub fn project_control(saved: &SavedControl, arb: &ArbControl, mutation: Mutation) -> Result<SectorControl, String> {
    let t0 = std::time::Instant::now();
    if (saved.g2, saved.mq) != (arb.g2, arb.mq) {
        return Err("control mismatch between saved states and Arb inputs".into());
    }
    let names: Vec<&str> = arb.states.iter().map(|s| s.name.as_str()).collect();
    if names != saved.names.iter().map(|s| s.as_str()).collect::<Vec<_>>() {
        return Err("state order differs between saved states and Arb inputs".into());
    }
    let (fre, fim) = (bits(&arb.trial_coeff_re), bits(&arb.trial_coeff_im));
    let coeff = residual::trial_coefficients(saved, Some((&fre, &fim)))?;
    let charges: Vec<i64> = arb.states.iter().map(|s| s.charge).collect();
    let (charge_sq, norms, degree, ngroups) = charge_defects(saved, &coeff, &charges, mutation)?;
    let res = residual::enclose_control_with(saved, Some((&fre, &fim)), Mutation::None)?;
    let states = arb
        .states
        .iter()
        .enumerate()
        .map(|(j, a)| {
            let (v, nn) = (charge_sq[j], norms[j]);
            let c_up = (Iv::point(v.hi.max(0.0)) / Iv::point(nn.lo)).sqrt().hi;
            let rs = &res.states[j];
            let inside = a.reference_charge_sq.iter().map(|(k, &x)| (k.clone(), v.contains(x))).collect();
            SectorState {
                name: a.name.clone(),
                charge: a.charge,
                center: rs.center,
                norm_sq: [nn.lo, nn.hi],
                charge_sq: [v.lo, v.hi],
                charge_upper: c_up,
                reference_charge_sq: a.reference_charge_sq.clone(),
                reference_charge_inside: inside,
                r_lower: rs.r_lower,
                r_upper: rs.r_upper,
                tail_norm_upper: a.tail_norm_upper,
                leakage_norm_upper: a.leakage_norm_upper,
                sector: sector_bound(a.tail_norm_upper, a.leakage_norm_upper, rs.r_upper, c_up),
            }
        })
        .collect();
    Ok(SectorControl {
        g2: saved.g2,
        mq: saved.mq,
        trial_file: arb.trial_file.clone(),
        trial_sha256: arb.trial_sha256.clone(),
        charge_workspace_degree: degree,
        charge_group_count: ngroups,
        residual_group_count: res.group_count,
        states,
        seconds: t0.elapsed().as_secs_f64(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::jacobian_u;

    /// J_u(L) u̇ = JG − GJ with J the generator of R_θ (independent of the Laurent field's derivation).
    #[test]
    fn velocity_is_the_rotation_generator() {
        let v = charge_velocity(Mutation::None);
        let points: [[f64; 6]; 3] = [[0.3, -0.2, 0.5, 0.7, -1.1, 0.4], [1.0, 0.1, -0.6, -0.3, 0.2, -0.9], [0.0; 6]];
        for (k, u) in points.iter().enumerate() {
            let (r0, r1, r2) = (u[0].exp(), u[1].exp(), u[2].exp());
            let (a, b, c) = (u[3], u[4], u[5]);
            let l = [[r0, 0.0, 0.0], [a, r1, 0.0], [b, c, r2]];
            let g: [[f64; 3]; 3] = std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|m| l[i][m] * l[j][m]).sum()));
            // J e1 = e2, J e2 = −e1
            let jm = [[0.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]];
            let mul = |x: &[[f64; 3]; 3], y: &[[f64; 3]; 3]| -> [[f64; 3]; 3] {
                std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|m| x[i][m] * y[m][j]).sum()))
            };
            let (jg, gj) = (mul(&jm, &g), mul(&g, &jm));
            let dg = |i: usize, j: usize| jg[i][j] - gj[i][j];
            let target = [dg(0, 0), dg(1, 1), dg(2, 2), dg(0, 1), dg(0, 2), dg(1, 2)];
            let ud: Vec<f64> = v.iter().map(|p| p.eval(&[r0, r1, r2, a, b, c])).collect();
            let ju = jacobian_u(&l);
            for i in 0..6 {
                let got: f64 = (0..6).map(|m| ju[i][m] * ud[m]).sum();
                assert!((got - target[i]).abs() < 1e-12 * (1.0 + target[i].abs()), "point {k}, row {i}: {got} vs {}", target[i]);
            }
        }
        // the dropped-term fault is not the generator
        let bad = charge_velocity(Mutation::DropGeneratorTerm);
        assert!((bad[5].eval(&[1.0, 1.0, 1.0, 0.0, 0.0, 0.0]) - v[5].eval(&[1.0, 1.0, 1.0, 0.0, 0.0, 0.0])).abs() > 0.5);
    }

    #[test]
    fn sector_bound_arithmetic_and_refusals() {
        let b = sector_bound(0.0, 0.0, 0.1, 0.0).unwrap();
        assert!(b.q_lower <= 1.0 && b.q_lower > 1.0 - 1e-15);
        assert!(b.residual_upper >= 0.1 && b.residual_upper <= 0.1 * (1.0 + 1e-14));
        // (r + ℓ)/√((1 − t)² − c²), 3-4-5: q = 0.8 at t = 0, c = 0.6
        let b = sector_bound(0.0, 0.02, 0.06, 0.6).unwrap();
        assert!(b.q_lower <= 0.8 && b.q_lower > 0.8 * (1.0 - 1e-15));
        assert!(b.residual_upper >= 0.1 && b.residual_upper < 0.1 * (1.0 + 1e-14));
        assert!(sector_bound(0.5, 0.0, 0.1, 0.5).is_err(), "q² = 0 must refuse");
        assert!(sector_bound(1.0, 0.0, 0.1, 0.0).is_err());
        assert!(sector_bound(0.0, f64::NAN, 0.1, 0.0).is_err());
        assert!(sector_bound(0.0, 0.0, -1e-30, 0.0).is_err());
    }
}
