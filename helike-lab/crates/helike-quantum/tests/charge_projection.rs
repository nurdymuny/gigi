//! The exact SO(2) charge projector by its norm bound (quasimode construction registration §2), on the four saved
//! controls: enclosed charge defects ‖(L − m)ψ‖, an oracle that rotates the saved trial function itself, the
//! registered mutants, and the projected-sector residual bound with the Arb tail and leakage endpoints.

use helike_quantum::interval::Cx;
use helike_quantum::model::inv6;
use helike_quantum::projection::{self, ArbControl, ProjectionInputs};
use helike_quantum::residual::{self, Mutation, SavedControl, SavedStates};
use std::sync::OnceLock;

fn saved() -> &'static SavedStates {
    static S: OnceLock<SavedStates> = OnceLock::new();
    S.get_or_init(|| {
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../oracle/quasimode_states_e21087a1.json");
        residual::load_saved_states(std::path::Path::new(p)).expect("run oracle/export_quasimode_states.py first")
    })
}

fn inputs() -> &'static ProjectionInputs {
    static S: OnceLock<ProjectionInputs> = OnceLock::new();
    S.get_or_init(|| {
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../oracle/projection_inputs_c31aab8e.json");
        projection::load_projection_inputs(std::path::Path::new(p)).expect("run oracle/export_projection_inputs.py first")
    })
}

fn control(g2: f64, mq: usize) -> (&'static SavedControl, &'static ArbControl) {
    let s = saved().controls.iter().find(|c| c.g2 == g2 && c.mq == mq).expect("saved control");
    let a = inputs().controls.iter().find(|c| c.g2 == g2 && c.mq == mq).expect("arb control");
    (s, a)
}

fn frozen(a: &ArbControl) -> (Vec<f64>, Vec<f64>) {
    let f = |v: &[u64]| v.iter().map(|&b| f64::from_bits(b)).collect::<Vec<f64>>();
    (f(&a.trial_coeff_re), f(&a.trial_coeff_im))
}

fn bits(v: &[u64]) -> Vec<f64> {
    v.iter().map(|&b| f64::from_bits(b)).collect()
}

/// Orthonormal Hermite polynomial values h_0..h_n at x (h_n(x) = H_n(x)/√(2ⁿ n!)).
fn hermite(n: usize, x: f64) -> Vec<f64> {
    let mut h = vec![1.0; n + 1];
    if n >= 1 {
        h[1] = std::f64::consts::SQRT_2 * x;
    }
    for k in 1..n {
        h[k + 1] = (std::f64::consts::SQRT_2 * x * h[k] - (k as f64).sqrt() * h[k - 1]) / ((k + 1) as f64).sqrt();
    }
    h
}

/// Σ_n C_n Π_k h_{n_k}(√w_k z_k) for one column.
fn poly_at(states: &[[u8; 6]], coeff: &[(f64, f64)], ncol: usize, col: usize, w: &[f64; 6], z: &[f64; 6]) -> (f64, f64) {
    let deg = states.iter().flat_map(|s| s.iter()).copied().max().unwrap_or(0) as usize;
    let tables: Vec<Vec<f64>> = (0..6).map(|k| hermite(deg, w[k].sqrt() * z[k])).collect();
    let mut acc = (0.0, 0.0);
    for (i, s) in states.iter().enumerate() {
        let (re, im) = coeff[i * ncol + col];
        if re == 0.0 && im == 0.0 {
            continue;
        }
        let v: f64 = (0..6).map(|k| tables[k][s[k] as usize]).product();
        acc.0 += re * v;
        acc.1 += im * v;
    }
    acc
}

/// ψ(z) / (Π(w/π)^{1/4}) : the Gaussian and the density 1/√ν, times P. (The common constant cancels in ratios.)
fn psi(c: &SavedControl, coeff: &[(f64, f64)], col: usize, z: &[f64; 6]) -> (f64, f64) {
    let u0 = bits(&c.u0);
    let sv = bits(&c.s_mat);
    let w: [f64; 6] = bits(&c.w).try_into().unwrap();
    let u: Vec<f64> = (0..6).map(|i| u0[i] + (0..6).map(|k| sv[i * 6 + k] * z[k]).sum::<f64>()).collect();
    let gauss: f64 = (0..6).map(|k| -0.5 * w[k] * z[k] * z[k]).sum();
    let log_nu = 3.0 * u[0] + 2.0 * u[1] + u[2];
    let f = (gauss - 0.5 * log_nu).exp();
    let basis: Vec<[u8; 6]> = c.basis.iter().map(|b| b.as_slice().try_into().unwrap()).collect();
    let p = poly_at(&basis, coeff, c.centers.len(), col, &w, z);
    (f * p.0, f * p.1)
}

/// The chart point of R_θ G R_θᵀ, for G at chart point z.
fn rotate(c: &SavedControl, z: &[f64; 6], theta: f64) -> [f64; 6] {
    let u0 = bits(&c.u0);
    let sv = bits(&c.s_mat);
    let s: [[f64; 6]; 6] = std::array::from_fn(|i| std::array::from_fn(|j| sv[i * 6 + j]));
    let u: Vec<f64> = (0..6).map(|i| u0[i] + (0..6).map(|k| s[i][k] * z[k]).sum::<f64>()).collect();
    let l = [[u[0].exp(), 0.0, 0.0], [u[3], u[1].exp(), 0.0], [u[4], u[5], u[2].exp()]];
    let g: [[f64; 3]; 3] = std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|m| l[i][m] * l[j][m]).sum()));
    let (co, si) = (theta.cos(), theta.sin());
    let r = [[1.0, 0.0, 0.0], [0.0, co, -si], [0.0, si, co]];
    let rg: [[f64; 3]; 3] = std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|m| r[i][m] * g[m][j]).sum()));
    let gr: [[f64; 3]; 3] = std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|m| rg[i][m] * r[j][m]).sum()));
    // Cholesky
    let mut lp = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..=i {
            let t: f64 = gr[i][j] - (0..j).map(|k| lp[i][k] * lp[j][k]).sum::<f64>();
            lp[i][j] = if i == j { t.sqrt() } else { t / lp[j][j] };
        }
    }
    let up = [lp[0][0].ln(), lp[1][1].ln(), lp[2][2].ln(), lp[1][0], lp[2][0], lp[2][1]];
    let x = inv6(&s);
    std::array::from_fn(|i| (0..6).map(|k| x[i][k] * (up[k] - u0[k])).sum())
}

#[test]
fn the_charge_operator_is_the_derivative_of_the_actual_rotation() {
    let (c, a) = control(0.004, 6);
    let (fre, fim) = frozen(a);
    let coeff: Vec<(f64, f64)> = fre.iter().zip(&fim).map(|(x, y)| (*x, *y)).collect();
    let cx: Vec<Cx> = residual::trial_coefficients(c, Some((&fre, &fim))).unwrap();
    let (chart, w) = residual::saved_chart(c).unwrap();
    let basis = residual::saved_basis(c).unwrap();
    // charge 0 for every column: the groups are Lψ / f
    let (space, groups) = projection::build_charge(&chart, &basis, w, &cx, &[0, 0, 0, 0], Mutation::None).unwrap();
    let points: [[f64; 6]; 3] = [
        [0.0; 6],
        std::array::from_fn(|k| 0.6 / w[k].sqrt() * if k % 2 == 0 { 1.0 } else { -1.0 }),
        std::array::from_fn(|k| [0.3, -0.8, 0.5, 0.9, -0.4, 0.7][k] / w[k].sqrt()),
    ];
    let ncol = 4;
    let (mut largest, mut worst) = ([0.0f64; 4], [0.0f64; 4]);
    for z in &points {
        // group sum Σ exp(α · z) Q(z), at interval midpoints
        let gauss: f64 = (0..6).map(|k| -0.5 * w[k] * z[k] * z[k]).sum();
        let u: Vec<f64> = (0..6).map(|i| chart.u0[i] + (0..6).map(|k| chart.s[i][k] * z[k]).sum::<f64>()).collect();
        let f = (gauss - 0.5 * (3.0 * u[0] + 2.0 * u[1] + u[2])).exp();
        for col in 0..ncol {
            let mut lpsi = (0.0, 0.0);
            for g in &groups {
                let e = (0..6).map(|k| g.alpha[k].mid() * z[k]).sum::<f64>().exp();
                let q: Vec<(f64, f64)> = g.coeffs.iter().map(|v| (v.re.mid(), v.im.mid())).collect();
                let (re, im) = poly_at(&space.states, &q, ncol, col, &w, z);
                lpsi.0 += e * re;
                lpsi.1 += e * im;
            }
            // −i dψ/dθ by Richardson-extrapolated central differences of the actual rotation
            let d = |h: f64| {
                let (p, m) = (psi(c, &coeff, col, &rotate(c, z, h)), psi(c, &coeff, col, &rotate(c, z, -h)));
                ((p.0 - m.0) / (2.0 * h), (p.1 - m.1) / (2.0 * h))
            };
            let (d1, d2) = (d(2e-3), d(1e-3));
            let dpsi = ((4.0 * d2.0 - d1.0) / 3.0, (4.0 * d2.1 - d1.1) / 3.0);
            let oracle = (dpsi.1 / f, -dpsi.0 / f);
            let err = (lpsi.0 - oracle.0).abs().max((lpsi.1 - oracle.1).abs());
            largest[col] = largest[col].max(lpsi.0.abs().max(lpsi.1.abs()));
            worst[col] = worst[col].max(err);
        }
    }
    // relative to the largest |Lψ/f| seen in the column: the charge-0 states have |Lψ| ~ 1e-3 |ψ| (their defect),
    // the Helike states ~ 2|ψ|; the Helike states have a node at the centre, so the maximum is taken over the points
    for col in 0..ncol {
        eprintln!("column {col}: max |L psi / f| {:.3e}, max |groups - rotation| {:.3e}", largest[col], worst[col]);
        assert!(largest[col] > 1e-6, "column {col}: vacuous comparison");
        assert!(worst[col] < 1e-6 * largest[col], "column {col}: the groups are not the rotation's derivative");
    }
}

fn reference_check(s: &projection::SectorState) -> (f64, f64) {
    // distance from the enclosure to each quadrature reference, relative to the value
    let d = |x: f64| if s.charge_sq[0] <= x && x <= s.charge_sq[1] { 0.0 } else { (s.charge_sq[0] - x).max(x - s.charge_sq[1]) / x };
    (d(s.reference_charge_sq["10"]), d(s.reference_charge_sq["12"]))
}

#[test]
fn every_control_projects_with_a_positive_sector_norm() {
    for (g2, mq) in [(0.004, 6), (0.004, 8), (0.002, 6), (0.002, 8)] {
        let (c, a) = control(g2, mq);
        let r = projection::project_control(c, a, Mutation::None).unwrap();
        assert_eq!(r.charge_group_count, 4);
        for s in &r.states {
            let (d10, d12) = reference_check(s);
            let w = s.charge_sq[1] - s.charge_sq[0];
            eprintln!(
                "g2 {g2} Mq {mq} {:<13} m {:+} c^2 [{:.12e}, {:.12e}] rel width {:.1e}  ref10 off {:.1e} ref12 off {:.1e}  r [{:.7}, {:.7}]  q {:.15}  R {:.7}",
                s.name, s.charge, s.charge_sq[0], s.charge_sq[1], w / s.charge_sq[0], d10, d12, s.r_lower, s.r_upper,
                s.sector.as_ref().map(|b| b.q_lower).unwrap_or(f64::NAN),
                s.sector.as_ref().map(|b| b.residual_upper).unwrap_or(f64::NAN)
            );
            // useful, not merely valid (the smallest defects, ~4e-8, lose a few digits to cancellation)
            assert!(s.charge_sq[0] > 0.0 && w < 1e-4 * s.charge_sq[0], "{}: charge enclosure too wide", s.name);
            // the quadrature is not exact (the velocity carries exponentials); it must still agree closely
            assert!(d10.min(d12) < 1e-4, "{}: no quadrature reference near the enclosure", s.name);
            assert!((s.norm_sq[0] - 1.0).abs() < 1e-11 && (s.norm_sq[1] - 1.0).abs() < 1e-11);
            let b = s.sector.as_ref().expect("sector bound");
            assert!(b.q_lower > 0.99 && b.residual_upper >= s.r_upper);
        }
    }
}

#[test]
fn every_charge_fault_moves_the_reference_outside() {
    let (c, a) = control(0.004, 6);
    let (fre, fim) = frozen(a);
    let cx = residual::trial_coefficients(c, Some((&fre, &fim))).unwrap();
    let charges = [0, 2, -2, 0];
    let (clean, _, _, _) = projection::charge_defects(c, &cx, &charges, Mutation::None).unwrap();
    let refs: Vec<f64> = a.states.iter().map(|s| s.reference_charge_sq["12"]).collect();
    for (j, v) in clean.iter().enumerate() {
        assert!((v.mid() - refs[j]).abs() < 1e-6 * refs[j], "clean enclosure far from the reference");
    }
    for m in [Mutation::ReverseCharge, Mutation::DropDensityDerivative, Mutation::DropGeneratorTerm, Mutation::NoConjugate] {
        let (bad, _, _, _) = projection::charge_defects(c, &cx, &charges, m).unwrap();
        let caught = bad.iter().zip(&refs).zip(&clean).any(|((b, r), k)| !b.contains(*r) && (b.mid() - r).abs() > 100.0 * k.width().max(1e-6 * r));
        assert!(caught, "{m:?} was not caught");
    }
}

#[test]
fn the_sector_bound_refuses_without_a_positive_projected_norm() {
    let (c, a) = control(0.004, 6);
    let (fre, fim) = frozen(a);
    let cx = residual::trial_coefficients(c, Some((&fre, &fim))).unwrap();
    // targeting the wrong sector makes c ≈ ‖(L ∓ 2)ψ‖ ≈ 4 and the bound must refuse
    let (bad, norms, _, _) = projection::charge_defects(c, &cx, &[0, -2, 2, 0], Mutation::None).unwrap();
    let c_up = (bad[1].hi / norms[1].lo).sqrt();
    assert!(c_up > 3.0);
    assert!(projection::sector_bound(0.0, 0.0, 0.05, c_up).is_err());
}
