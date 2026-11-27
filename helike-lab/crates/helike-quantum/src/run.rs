//! Runners with the decision rules of the Python scripts, frozen as written there:
//! - `controls`: `run_helike_positive_cone_controls.py` (O(g²) coefficients within 0.03 of the exact values,
//!   identification weights ≥ 0.95);
//! - `convergence`: `run_helike_cone_convergence.py` (both identified gaps change by ≤ 0.003 from the lower to
//!   the higher cutoff at both couplings, weights ≥ 0.95; otherwise NOT_CONVERGED, or UNRESOLVED on refusal).
//!
//! Every point is solved with exact Kronecker moments (`ConeOperator::exact`) and cross-checked against the
//! independent Gauss–Hermite grid operator at order (Mq + 2) + extra + 2: the largest change of a Ritz value's
//! Rayleigh quotient when the grid operator is applied to the exact Ritz vectors.

use crate::model::Background;
use crate::operator::{ConeOperator, Mutation, Part};
use crate::ritz::{self, HelikeCompetitor, SolveSettings};
use serde::Serialize;
use std::time::Instant;

pub const CONTROL_S: f64 = 1.15;
pub const CONTROL_T: f64 = 3.0;
pub const CONTROL_G2: [f64; 2] = [0.004, 0.002];
/// exact O(g²) coefficients (Helike, competitor) that the controls must reproduce
pub const EXACT_COEFFICIENTS: [f64; 2] = [-0.5300, -2.7379];
pub const COEFFICIENT_TOL: f64 = 0.03;
pub const WEIGHT_MIN: f64 = 0.95;
pub const GAP_TOL: f64 = 0.003;

#[derive(Clone, Debug, Serialize)]
pub struct PointResult {
    pub s: f64,
    pub t: f64,
    pub g2: f64,
    #[serde(rename = "Mq")]
    pub mq: usize,
    pub dimension: usize,
    /// "exact" (Kronecker moments) for every solve made here
    pub moments: String,
    /// order per axis of the grid operator used for the cross-check
    pub cross_check_quadrature_order: usize,
    pub frequencies: [f64; 6],
    pub background_gradient: f64,
    pub result: HelikeCompetitor,
    /// max over returned levels of |θ − ⟨x, H_grid x⟩|: exact operator against the independent grid operator
    pub quadrature_cross_check: f64,
    /// max ‖Hx − θx‖ inside the finite trial space (Lanczos convergence), NOT a full-operator residual
    pub max_in_space_lanczos_residual: f64,
    pub applications: usize,
    pub seconds: f64,
}

pub fn solve_point(s: f64, t: f64, g2: f64, mq: usize, extra: usize, set: &SolveSettings) -> Result<PointResult, String> {
    solve_point_chart(s, t, g2, mq, extra, set, None)
}

/// `solve_point` in a supplied (dressed) chart, flux-method §5.10: the Hamiltonian is unchanged, the trial space and
/// the reference frame are the chart's, and the roles come from the chart file, not from ω_H.
pub fn solve_point_chart(s: f64, t: f64, g2: f64, mq: usize, extra: usize, set: &SolveSettings,
                         chart: Option<&crate::model::ChartFile>) -> Result<PointResult, String> {
    let t0 = Instant::now();
    if !(g2 > 0.0) {
        return Err("g^2 must be positive".into());
    }
    let bg = match chart {
        None => Background::maurer_cartan(s, t, g2.sqrt())?,
        Some(c) => Background::dressed(s, t, g2.sqrt(), c.g0, c.m, c.w)?,
    };
    let op = ConeOperator::exact(bg.clone(), mq, Mutation::None)?;
    let eigs = ritz::spectrum(&op, set);
    let result = match chart {
        None => ritz::helike_and_competitor(&op, &eigs, ritz::helike_frequency(s, t))?,
        Some(c) => ritz::helike_and_competitor_modes(&op, &eigs, &[c.roles[0], c.roles[1]], c.roles[2])?,
    };
    let fine = ConeOperator::new(bg.clone(), mq, extra + 2, Mutation::None)?;
    let hx = fine.apply(&eigs.vectors, Part::Full);
    let quadrature_cross_check = eigs
        .vectors
        .iter()
        .zip(&hx)
        .zip(&eigs.values)
        .map(|((x, h), th)| (x.iter().zip(h).map(|(a, b)| a * b).sum::<f64>() - th).abs())
        .fold(0.0, f64::max);
    Ok(PointResult {
        s,
        t,
        g2,
        mq,
        dimension: op.dim(),
        moments: op.moments().into(),
        cross_check_quadrature_order: fine.grid.q,
        frequencies: bg.w,
        background_gradient: bg.grad_norm,
        max_in_space_lanczos_residual: eigs.residuals.iter().cloned().fold(0.0, f64::max),
        applications: eigs.applications,
        result,
        quadrature_cross_check,
        seconds: t0.elapsed().as_secs_f64(),
    })
}

#[derive(Clone, Debug, Serialize)]
pub struct Controls {
    pub status: String,
    pub scope: String,
    #[serde(rename = "Mq")]
    pub mq: usize,
    pub rows: Vec<PointResult>,
    pub coefficients: Vec<[f64; 2]>,
    pub extrapolated_coefficients: [f64; 2],
    pub coefficient_errors: [f64; 2],
    pub coefficient_pass: bool,
    pub weight_pass: bool,
}

/// The two weak-coupling controls at one cutoff (s = 1.15, t = 3, g² = 0.004, 0.002).
pub fn controls(mq: usize, extra: usize, set: &SolveSettings) -> Result<Controls, String> {
    let wh = ritz::helike_frequency(CONTROL_S, CONTROL_T);
    let mut rows = vec![];
    let mut coefficients = vec![];
    for g2 in CONTROL_G2 {
        let r = solve_point(CONTROL_S, CONTROL_T, g2, mq, extra, set)?;
        let w = r.frequencies;
        let wc = w
            .iter()
            .cloned()
            .filter(|&x| w.iter().filter(|&&y| (y - x).abs() < 1e-6).count() == 1)
            .fold(f64::INFINITY, f64::min);
        coefficients.push([(r.result.helike.gap - wh) / g2, (r.result.competitor.gap - wc) / g2]);
        rows.push(r);
    }
    let ex = [2.0 * coefficients[1][0] - coefficients[0][0], 2.0 * coefficients[1][1] - coefficients[0][1]];
    let err = [(ex[0] - EXACT_COEFFICIENTS[0]).abs(), (ex[1] - EXACT_COEFFICIENTS[1]).abs()];
    let coefficient_pass = err[0].max(err[1]) <= COEFFICIENT_TOL;
    let weight_pass = rows.iter().all(|r| r.result.helike.weight > WEIGHT_MIN && r.result.competitor.weight > WEIGHT_MIN);
    Ok(Controls {
        status: if coefficient_pass && weight_pass { "PASS_CONTROLS" } else { "FAIL_CONTROLS" }.into(),
        scope: "interior trial controls, no physical boundary certificate".into(),
        mq,
        rows,
        coefficients,
        extrapolated_coefficients: ex,
        coefficient_errors: err,
        coefficient_pass,
        weight_pass,
    })
}

#[derive(Clone, Debug, Serialize)]
pub struct Change {
    pub g2: f64,
    pub helike: f64,
    pub competitor: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Convergence {
    pub decision: String,
    pub rule: String,
    pub ladder: [usize; 2],
    pub gap_tol: f64,
    pub weight_min: f64,
    pub rows: Vec<Result<PointResult, String>>,
    pub changes: Vec<Change>,
}

/// Basis convergence of the controls from `ladder[0]` to `ladder[1]` at the frozen 0.003 rule.
pub fn convergence(ladder: [usize; 2], extra: usize, set: &SolveSettings) -> Convergence {
    let mut rows = vec![];
    let mut changes = vec![];
    let mut decision = "CONVERGED".to_string();
    for g2 in CONTROL_G2 {
        let a = solve_point(CONTROL_S, CONTROL_T, g2, ladder[0], extra, set);
        let b = solve_point(CONTROL_S, CONTROL_T, g2, ladder[1], extra, set);
        match (&a, &b) {
            (Ok(ra), Ok(rb)) => {
                let wmin = [&ra.result, &rb.result]
                    .iter()
                    .flat_map(|r| [r.helike.weight, r.competitor.weight])
                    .fold(f64::INFINITY, f64::min);
                if wmin < WEIGHT_MIN {
                    if decision != "UNRESOLVED" {
                        decision = "UNRESOLVED".into();
                    }
                } else {
                    let ch = Change {
                        g2,
                        helike: (rb.result.helike.gap - ra.result.helike.gap).abs(),
                        competitor: (rb.result.competitor.gap - ra.result.competitor.gap).abs(),
                    };
                    if ch.helike.max(ch.competitor) > GAP_TOL && decision == "CONVERGED" {
                        decision = "NOT_CONVERGED".into();
                    }
                    changes.push(ch);
                }
            }
            _ => decision = "UNRESOLVED".into(),
        }
        rows.push(a);
        rows.push(b);
    }
    Convergence {
        decision,
        rule: "CONVERGED if both identified gaps change by at most 0.003 between the two cutoffs at both couplings and \
               every identification weight is at least 0.95; NOT_CONVERGED if a change exceeds 0.003; UNRESOLVED if \
               identification or numerics refuse. Interior trial family only; not a boundary or spectral certificate."
            .into(),
        ladder,
        gap_tol: GAP_TOL,
        weight_min: WEIGHT_MIN,
        rows,
        changes,
    }
}


/// Heat-signal monomials (registration amendment 1), as exponents over the roles [h1, h2, c, x, s1, s2].
pub const HEAT_MONOMIALS: [[u8; 6]; 22] = [
    [1, 0, 0, 0, 0, 0], [0, 1, 0, 0, 0, 0], [0, 0, 1, 0, 0, 0], [0, 0, 0, 1, 0, 0],
    [1, 0, 1, 0, 0, 0], [0, 1, 1, 0, 0, 0], [1, 0, 0, 1, 0, 0], [0, 1, 0, 1, 0, 0],
    [0, 0, 0, 0, 2, 0], [0, 0, 0, 0, 0, 2], [0, 0, 0, 0, 1, 1], [2, 0, 0, 0, 0, 0], [0, 2, 0, 0, 0, 0],
    [0, 0, 2, 0, 0, 0], [0, 0, 0, 2, 0, 0], [0, 0, 1, 1, 0, 0],
    [3, 0, 0, 0, 0, 0], [1, 2, 0, 0, 0, 0], [2, 1, 0, 0, 0, 0], [0, 3, 0, 0, 0, 0], [2, 0, 1, 0, 0, 0], [0, 2, 1, 0, 0, 0],
];

#[derive(Clone, Debug, Serialize)]
pub struct HeatOrder {
    pub order: usize,
    /// 7 x k reference amplitudes (rows 0..5 the bare single quanta per mode index, row 6 the vacuum)
    pub reference_amplitudes: Vec<f64>,
    /// nm x k monomial-source overlaps, nm x nm Gram, nm G-components, k <G|psi_c>
    pub d: Vec<f64>,
    pub n: Vec<f64>,
    pub p: Vec<f64>,
    pub s: Vec<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct HeatResult {
    pub s: f64,
    pub t: f64,
    pub g2: f64,
    #[serde(rename = "Mq")]
    pub mq: usize,
    pub frequencies: [f64; 6],
    /// mode indices of [h1, h2, c, x, s1, s2]
    pub roles: [usize; 6],
    /// monomial exponents over the roles (registration order)
    pub monomials: Vec<[u8; 6]>,
    pub levels: Vec<f64>,
    pub identification: HelikeCompetitor,
    pub fine: HeatOrder,
    pub coarse: HeatOrder,
    pub seconds: f64,
}

/// One heat-signal point: solve, identify at the resolved orders, then the monomial-source overlaps and the bare
/// reference amplitudes at both resolved orders (fine for the signal, coarse for the T4 quadrature seam).
pub fn heat_point(s: f64, t: f64, g2: f64, mq: usize, set: &SolveSettings) -> Result<HeatResult, String> {
    let t0 = Instant::now();
    let bg = Background::maurer_cartan(s, t, g2.sqrt())?;
    let op = ConeOperator::exact(bg.clone(), mq, Mutation::None)?;
    let eigs = ritz::spectrum(&op, set);
    let identification = ritz::helike_and_competitor(&op, &eigs, ritz::helike_frequency(s, t))?;
    let roles = ritz::mode_roles(&bg.w, ritz::helike_frequency(s, t))?;
    // exponents over mode indices, from exponents over roles
    let monos: Vec<[u8; 6]> = HEAT_MONOMIALS
        .iter()
        .map(|e| {
            let mut m = [0u8; 6];
            for (r, &mode) in roles.iter().enumerate() {
                m[mode] += e[r];
            }
            m
        })
        .collect();
    let one = |order: usize| -> Result<HeatOrder, String> {
        let reference_amplitudes = ritz::reference_amplitudes_with_vacuum(&op, &eigs.vectors, order)?;
        let (d, n, p, sv) = ritz::source_overlap_terms(&op, &eigs.vectors, order, &monos)?;
        Ok(HeatOrder { order, reference_amplitudes, d, n, p, s: sv })
    };
    let fine = one(identification.overlap_orders[1])?;
    let coarse = one(identification.overlap_orders[0])?;
    Ok(HeatResult {
        s, t, g2, mq, frequencies: bg.w, roles, monomials: HEAT_MONOMIALS.to_vec(), levels: eigs.values.clone(),
        identification, fine, coarse, seconds: t0.elapsed().as_secs_f64(),
    })
}
