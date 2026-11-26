//! Ritz levels, reference weights and the Helike / competitor identification (port of `ConeRitz.spectrum`,
//! `classify`, `reference_amplitudes`, `helike_and_competitor`).

use crate::grid::SlabGrid;
use crate::lanczos::{self, Eigs};
use crate::model::*;
use crate::operator::{ConeOperator, Part};
use rayon::prelude::*;
use serde::Serialize;

pub const BOUNDARY_STATUS: &str = "UNRESOLVED_PHYSICAL_REALIZATION";
pub const SCOPE: &str = "positive-cone interior trial values; no boundary or sector convergence certificate";

/// Solver settings recorded with every result.
#[derive(Clone, Debug, Serialize)]
pub struct SolveSettings {
    pub nev: usize,
    pub block: usize,
    pub tol: f64,
    pub max_dim: usize,
}

impl Default for SolveSettings {
    fn default() -> Self {
        SolveSettings { nev: 12, block: 4, tol: 1e-10, max_dim: 1600 }
    }
}

/// The lowest levels of H by block Lanczos, started from the harmonic ground state and single-quantum states.
pub fn spectrum(op: &ConeOperator, set: &SolveSettings) -> Eigs {
    let n = op.dim();
    if n <= set.nev + set.block || n <= 64 {
        return dense_spectrum(op, set.nev);
    }
    let start: Vec<Vec<f64>> = (0..set.block.min(n))
        .map(|k| {
            let mut v = vec![0.0; n];
            v[k] = 1.0;
            v
        })
        .collect();
    lanczos::lowest(n, set.nev, set.block, &start, set.tol, set.max_dim, |vs| op.apply(vs, Part::Full))
}

/// Dense eigen-decomposition (oracle path for small trial spaces).
pub fn dense_spectrum(op: &ConeOperator, nev: usize) -> Eigs {
    let n = op.dim();
    let h = op.dense(Part::Full);
    let m = faer::Mat::from_fn(n, n, |i, j| 0.5 * (h[i * n + j] + h[j * n + i]));
    let e = m.self_adjoint_eigen(faer::Side::Lower).expect("dense eigen");
    let s = e.S().column_vector();
    let u = e.U();
    let k = nev.min(n);
    let values: Vec<f64> = (0..k).map(|c| s[c]).collect();
    let vectors: Vec<Vec<f64>> = (0..k).map(|c| (0..n).map(|i| u[(i, c)]).collect()).collect();
    let residuals = (0..k)
        .map(|c| {
            let r: f64 = (0..n)
                .map(|i| {
                    let hv: f64 = (0..n).map(|j| h[i * n + j] * vectors[c][j]).sum();
                    (hv - values[c] * vectors[c][i]).powi(2)
                })
                .sum();
            r.sqrt()
        })
        .collect();
    Eigs { values, vectors, residuals, applications: n, krylov_dimension: n }
}

/// Weight of a vector on the single-quantum states of each mode.
pub fn classify(op: &ConeOperator, v: &[f64]) -> [f64; 6] {
    let mut w = [0.0; 6];
    for (i, s) in op.basis.iter().enumerate() {
        if s.iter().map(|&x| x as usize).sum::<usize>() == 1 {
            let k = s.iter().position(|&x| x == 1).unwrap();
            w[k] += v[i] * v[i];
        }
    }
    w
}

/// Overlaps of trial vectors with the original linear-Gram harmonic single-quantum projectors, by a Gauss–Hermite
/// rule of the given order in each chart coordinate. Returns a 6 × k matrix (row-major, rows = modes).
pub fn reference_amplitudes(op: &ConeOperator, vectors: &[Vec<f64>], order: usize) -> Result<Vec<f64>, String> {
    if !(2..=28).contains(&order) || vectors.is_empty() || vectors.len() > 12 {
        return Err("overlap resource contract requires order 2..28 and 1..12 vectors".into());
    }
    let k = vectors.len();
    let grid = SlabGrid::new(&op.basis, op.mq + 1, order);
    let ch = &op.chart;
    let inv_m = inv6(&ch.m);
    let x0 = gram_entries(&ch.g0);
    let logdet = 8f64.ln() + logdet6(&ch.s) - logdet6(&ch.m);
    let w = op.bg.w;
    let zq: Vec<Vec<f64>> = (0..6).map(|a| grid.nodes.iter().map(|x| x / w[a].sqrt()).collect()).collect();
    let slabs: Vec<(usize, usize)> = (0..order).flat_map(|a| (0..order).map(move |b| (a, b))).collect();
    let res: Result<Vec<f64>, String> = slabs
        .par_iter()
        .map(|&(q0, q1)| {
            let mut sc = grid.scratch(k);
            let refs: Vec<&[f64]> = vectors.iter().map(|v| &v[..]).collect();
            let mut amp = vec![0.0; 6 * k];
            let mut err: Option<String> = None;
            grid.process(&refs, q0, q1, &mut sc, None, |q2, vals| {
                for p in 0..vals.len() / k {
                    let [q3, q4, q5] = grid.decode3(p);
                    let qs = [q0, q1, q2, q3, q4, q5];
                    let mut z = [0.0; 6];
                    let mut wt = 1.0;
                    for a in 0..6 {
                        z[a] = zq[a][qs[a]];
                        wt *= grid.weights[qs[a]];
                    }
                    let u = ch.coordinates(&z);
                    let pt = ch.point_from_u(&u);
                    let mut y = [0.0; 6];
                    for i in 0..6 {
                        for j in 0..6 {
                            y[i] += inv_m[i][j] * (pt.x[j] - x0[j]);
                        }
                    }
                    let quad: f64 = (0..6).map(|a| w[a] * (y[a] * y[a] - z[a] * z[a])).sum();
                    let log_ratio = 0.5 * (logdet + 4.0 * u[0] + 3.0 * u[1] + 2.0 * u[2] - quad);
                    if !log_ratio.is_finite() || log_ratio > 600.0 {
                        err = Some("unresolved reference-overlap numerical range".to_string());
                        return;
                    }
                    let measure = wt * log_ratio.exp();
                    for a in 0..6 {
                        let target = y[a] * (2.0 * w[a]).sqrt() * measure;
                        for c in 0..k {
                            amp[a * k + c] += target * vals[p * k + c];
                        }
                    }
                }
            });
            if let Some(e) = err {
                return Err(e);
            }
            Ok(amp)
        })
        .try_reduce(|| vec![0.0; 6 * k], |mut a, b| {
            for (x, y) in a.iter_mut().zip(&b) {
                *x += y;
            }
            Ok(a)
        });
    res
}

#[derive(Clone, Debug, Serialize)]
pub struct Channel {
    pub gap: f64,
    pub weight: f64,
    pub log_basis_weight: f64,
    pub level_index: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct HelikeCompetitor {
    #[serde(rename = "E0")]
    pub e0: f64,
    pub scope: String,
    pub boundary_status: String,
    pub energy_shift: f64,
    pub reference_weight_error: f64,
    pub overlap_orders: [usize; 2],
    pub helike: Channel,
    pub competitor: Channel,
    pub levels: Vec<f64>,
    /// ‖Hx − θx‖ of the Ritz pairs INSIDE the finite trial space (Lanczos convergence). These are not the
    /// full-operator residuals ‖(H − E)ψ‖ of the physical quasimode certificate, which are orders of magnitude larger.
    pub in_space_lanczos_residuals: Vec<f64>,
}

/// Helike's charge-2 doublet frequency on the Maurer–Cartan line.
pub fn helike_frequency(s: f64, t: f64) -> f64 {
    (2.0 * s * (1.0 + 2.0 / t - s)).sqrt()
}

/// Identify Helike (the doublet at ω_H) and the competitor (the lowest isolated mode) among the lowest levels by
/// their weight on the harmonic single-quantum projectors, exactly as the Python reference does.
pub fn helike_and_competitor(op: &ConeOperator, eigs: &Eigs, wh: f64) -> Result<HelikeCompetitor, String> {
    let w = op.bg.w;
    let hel: Vec<usize> = (0..6).filter(|&j| (w[j] - wh).abs() < 1e-6).collect();
    let single: Vec<usize> = (0..6)
        .filter(|&j| !hel.contains(&j) && (0..6).filter(|&m| (w[m] - w[j]).abs() < 1e-6).count() == 1)
        .collect();
    if hel.len() != 2 || single.is_empty() || eigs.values.len() < 2 {
        return Err("ambiguous harmonic reference channel".into());
    }
    let comp = *single.iter().min_by(|&&a, &&b| w[a].partial_cmp(&w[b]).unwrap()).unwrap();
    let k = eigs.vectors.len();
    // The overlap integrand is not polynomial in the chart, so Gauss–Hermite order must be raised until consecutive
    // orders agree to the frozen tolerance 1e-6. The orders (10, 12) resolve the g² = 0.002 / 0.004 controls; the
    // stronger couplings need more. The tolerance is unchanged; only the order ladder is extended (flux-grid runner
    // note, 26 Nov). The first resolved pair is used, so the controls reproduce exactly.
    let error_of = |coarse: &[f64], fine: &[f64]| {
        let mut error: f64 = 0.0;
        for i in 0..6 * k {
            error = error.max((coarse[i] * coarse[i] - fine[i] * fine[i]).abs());
        }
        for c in 0..k {
            let a: f64 = hel.iter().map(|&m| coarse[m * k + c].powi(2)).sum();
            let b: f64 = hel.iter().map(|&m| fine[m * k + c].powi(2)).sum();
            error = error.max((a - b).abs());
        }
        error
    };
    let mut coarse = reference_amplitudes(op, &eigs.vectors, 10)?;
    let mut orders = [10, 12];
    let mut fine = reference_amplitudes(op, &eigs.vectors, 12)?;
    let mut error = error_of(&coarse, &fine);
    while error > 1e-6 && orders[1] < 28 {
        orders = [orders[1], orders[1] + 2];
        coarse = fine;
        fine = reference_amplitudes(op, &eigs.vectors, orders[1])?;
        error = error_of(&coarse, &fine);
    }
    let max_norm = (0..k).map(|c| (0..6).map(|m| fine[m * k + c].powi(2)).sum::<f64>()).fold(0.0, f64::max);
    if error > 1e-6 || max_norm > 1.0 + 1e-6 {
        return Err(format!("reference weights unresolved at frozen overlap tolerance through orders {orders:?}: {error}"));
    }
    let chan = |modes: &[usize]| {
        let weights: Vec<f64> = (0..k).map(|c| modes.iter().map(|&m| fine[m * k + c].powi(2)).sum()).collect();
        // first maximum over levels 1.. (numpy argmax)
        let mut best = 1;
        for c in 2..k {
            if weights[c] > weights[best] {
                best = c;
            }
        }
        let cl = classify(op, &eigs.vectors[best]);
        Channel {
            gap: eigs.values[best] - eigs.values[0],
            weight: weights[best],
            log_basis_weight: modes.iter().map(|&m| cl[m]).sum(),
            level_index: best,
        }
    };
    Ok(HelikeCompetitor {
        e0: eigs.values[0],
        scope: SCOPE.into(),
        boundary_status: BOUNDARY_STATUS.into(),
        energy_shift: op.energy_shift,
        reference_weight_error: error,
        overlap_orders: orders,
        helike: chan(&hel),
        competitor: chan(&[comp]),
        levels: eigs.values.clone(),
        in_space_lanczos_residuals: eigs.residuals.clone(),
    })
}
