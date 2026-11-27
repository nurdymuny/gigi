//! Export of a Rust-solved cone point as quasimode trial states, for the per-point domain-validity gate of the
//! flux-grid method amendment (docs/research/helike-flux-method-amendment-2026-11-25.md, §3.3).
//!
//! - **Selection,** exactly as `run_helike_quasimode.select`: the ground state; the two levels (excluding the ground)
//!   with the largest weight on Helike's harmonic doublet, each ≥ 0.95, one of them being the identified Helike level;
//!   the identified competitor (weight ≥ 0.95). Centres are Ritz values plus the energy shift, the Helike pair
//!   averaged.
//! - **Chiral mixing,** as `helike_quasimode.chiral_frame`, but from exact linear charge moments rather than a
//!   quadrature grid: the 2×2 compression ⟨v_a, L v_b⟩ on the Helike pair by polarization of ⟨x, Lx⟩ for
//!   x ∈ {v₁, v₂, (v₁+v₂)/√2, (v₁+iv₂)/√2}. The positive-charge eigenvector becomes helike_plus. The mixing is only a
//!   choice of trial vector: the rigorous charge bound later certifies whatever it is.
//! - Everything that defines the trial (chart u0, S, frequencies, basis, coefficients, mixing, centres) is exported
//!   as IEEE-754 bit patterns, so the Arb and interval stages evaluate exactly the same functions.

use crate::interval::{Cx, Iv};
use crate::model::Background;
use crate::operator::{ConeOperator, Mutation as OpMutation};
use crate::residual::{self, tree_sum, ChartIv, Mutation};
use crate::ritz::{self, SolveSettings};
use crate::{projection, temple};
use serde::Serialize;

pub const SCHEMA: &str = "helike-lab/cone-state-export/v1";
pub const NAMES: [&str; 4] = ["ground", "helike_plus", "helike_minus", "competitor"];

#[derive(Clone, Debug, Serialize)]
pub struct StateExport {
    pub schema: &'static str,
    pub s: f64,
    pub t: f64,
    pub g2: f64,
    pub g: f64,
    #[serde(rename = "Mq")]
    pub mq: usize,
    pub moments: &'static str,
    pub u0: Vec<u64>,
    #[serde(rename = "S")]
    pub s_mat: Vec<u64>,
    pub w: Vec<u64>,
    #[serde(rename = "G0")]
    pub g0: Vec<u64>,
    #[serde(rename = "M")]
    pub m: Vec<u64>,
    pub basis: Vec<Vec<u8>>,
    /// N × 4 real, row-major, columns in NAMES order before mixing
    pub coeff: Vec<u64>,
    pub coeff_shape: [usize; 2],
    /// 4 × 4 complex, row-major
    pub mixing_re: Vec<u64>,
    pub mixing_im: Vec<u64>,
    pub centers: Vec<u64>,
    pub names: [&'static str; 4],
    pub level_indices: [usize; 4],
    pub pair_weights: [f64; 2],
    pub competitor_weight: f64,
    /// eigenvalues of the 2×2 charge compression on the Helike pair (expected near ∓2)
    pub compressed_charges: [f64; 2],
    pub energy_shift: f64,
    pub levels: Vec<f64>,
    pub max_in_space_lanczos_residual: f64,
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// ⟨x_j, L x_j⟩ / ‖x_j‖² for up to four complex columns (exact linear charge moments, interval midpoints).
fn charge_expectations(chart: &ChartIv, basis: &[crate::basis::State], w: [f64; 6], cols: &[Cx], ncol: usize) -> Result<Vec<f64>, String> {
    let zeros = vec![0i64; ncol];
    let (space, groups) = projection::build_charge(chart, basis, w, cols, &zeros, Mutation::None)?;
    let (_, p) = residual::embed(w, basis, cols, ncol, 2)?;
    let s = temple::linear_moment(&space, &p, &groups, Mutation::None)?;
    let rows = basis.len();
    Ok((0..ncol)
        .map(|j| {
            let n = tree_sum(&(0..rows).map(|r| cols[r * ncol + j].norm_sq()).collect::<Vec<_>>());
            (s[j] / n).mid()
        })
        .collect())
}

/// The chiral mixing of the Helike pair: (C, compressed charges ascending).
pub fn chiral_mixing(chart: &ChartIv, basis: &[crate::basis::State], w: [f64; 6], v1: &[f64], v2: &[f64]) -> Result<([[(f64, f64); 4]; 4], [f64; 2]), String> {
    let r = Iv::point(0.5).sqrt();
    let n = basis.len();
    let mut cols = vec![Cx::new(Iv::point(0.0), Iv::point(0.0)); n * 4];
    for k in 0..n {
        let (a, b) = (Iv::point(v1[k]), Iv::point(v2[k]));
        cols[k * 4] = Cx::real(a);
        cols[k * 4 + 1] = Cx::real(b);
        cols[k * 4 + 2] = Cx::real((a + b) * r);
        cols[k * 4 + 3] = Cx::new(a * r, b * r);
    }
    let d = charge_expectations(chart, basis, w, &cols, 4)?;
    let (l11, l22) = (d[0], d[1]);
    let re12 = d[2] - 0.5 * (l11 + l22);
    let im12 = 0.5 * (l11 + l22) - d[3];
    // eigen-decomposition of [[l11, b], [conj b, l22]], b = re12 + i im12
    let half = 0.5 * (l11 - l22);
    let rad = (half * half + re12 * re12 + im12 * im12).sqrt();
    let mean = 0.5 * (l11 + l22);
    let (e_lo, e_hi) = (mean - rad, mean + rad);
    if !(e_lo < 0.0 && 0.0 < e_hi) {
        return Err("Helike partners do not straddle charge zero".into());
    }
    if (re12 * re12 + im12 * im12).sqrt() < 1e-12 {
        return Err("Helike pair is already diagonal in charge; mixing undetermined".into());
    }
    // eigenvector of λ: (b, λ − l11), normalised
    let vec_of = |lam: f64| -> [(f64, f64); 2] {
        let (x0, x1) = ((re12, im12), (lam - l11, 0.0));
        let nrm = (x0.0 * x0.0 + x0.1 * x0.1 + x1.0 * x1.0).sqrt();
        [(x0.0 / nrm, x0.1 / nrm), (x1.0 / nrm, 0.0)]
    };
    let (plus, minus) = (vec_of(e_hi), vec_of(e_lo));
    let mut c = [[(0.0, 0.0); 4]; 4];
    c[0][0] = (1.0, 0.0);
    c[3][3] = (1.0, 0.0);
    c[1][1] = plus[0];
    c[2][1] = plus[1];
    c[1][2] = minus[0];
    c[2][2] = minus[1];
    Ok((c, [e_lo, e_hi]))
}

/// Solve one point with exact moments and export its four trial states.
pub fn export_point(s: f64, t: f64, g2: f64, mq: usize, set: &SolveSettings) -> Result<StateExport, String> {
    export_point_with(s, t, g2, mq, set, None, Selection::Quasimode)
}

/// Which states the export selects.
/// - `Quasimode` (the certificate path, unchanged): the two levels with the largest Helike weight, each w~ >= 0.95,
///   and the identified competitor with w~ >= 0.95.
/// - `Grid` (flux-method §5.10, for the grid's domain gate): exactly the states the grid identified, under the grid's
///   own rule w~ >= 0.9: the identified Helike level, its charge partner from the charge compression, and the
///   identified competitor. No second, stricter identity rule.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Selection {
    Quasimode,
    Grid,
}

pub const GRID_WEIGHT_MIN: f64 = 0.9;
pub const QUASIMODE_WEIGHT_MIN: f64 = 0.95;

/// The charge partner of level h among the Ritz levels: the level b maximising |Im<v_h, L v_b>| (polarization, as in
/// `export_levels`). Returns (b, |Im|).
fn charge_partner(chart: &ChartIv, basis: &[crate::basis::State], w: [f64; 6], vecs: &[Vec<f64>], h: usize, exclude: &[usize])
    -> Result<(usize, f64), String> {
    let n = basis.len();
    let r = Iv::point(0.5).sqrt();
    let cands: Vec<usize> = (1..vecs.len()).filter(|&b| b != h && !exclude.contains(&b)).collect();
    let mut cols_all: Vec<(usize, Option<usize>)> = vec![(h, None)];
    for &b in &cands {
        cols_all.push((b, None));
        cols_all.push((h, Some(b)));
    }
    let mut d = Vec::with_capacity(cols_all.len());
    for chunk in cols_all.chunks(4) {
        let nc = chunk.len();
        let mut cols = vec![Cx::new(Iv::point(0.0), Iv::point(0.0)); n * nc];
        for row in 0..n {
            for (c, &(a, b)) in chunk.iter().enumerate() {
                cols[row * nc + c] = match b {
                    None => Cx::real(Iv::point(vecs[a][row])),
                    Some(b) => Cx::new(Iv::point(vecs[a][row]) * r, Iv::point(vecs[b][row]) * r),
                };
            }
        }
        d.extend(charge_expectations(chart, basis, w, &cols, nc)?);
    }
    let dh = d[0];
    let mut best = (usize::MAX, 0.0f64);
    for (i, &b) in cands.iter().enumerate() {
        let (db, dhb) = (d[1 + 2 * i], d[2 + 2 * i]);
        let im = (0.5 * (dh + db) - dhb).abs();
        if im > best.1 {
            best = (b, im);
        }
    }
    if best.0 == usize::MAX {
        return Err("no charge partner candidate".into());
    }
    Ok(best)
}

/// Solve one point (optionally in a dressed chart) and export its four trial states under the given selection.
pub fn export_point_with(s: f64, t: f64, g2: f64, mq: usize, set: &SolveSettings,
                         chart: Option<&crate::model::ChartFile>, selection: Selection) -> Result<StateExport, String> {
    if !(g2 > 0.0) {
        return Err("g^2 must be positive".into());
    }
    let g = g2.sqrt();
    let bg = match chart {
        None => Background::maurer_cartan(s, t, g)?,
        Some(c) => Background::dressed(s, t, g, c.g0, c.m, c.w)?,
    };
    let op = ConeOperator::exact(bg.clone(), mq, OpMutation::None)?;
    let eigs = ritz::spectrum(&op, set);
    let wh = ritz::helike_frequency(s, t);
    let (result, hel) = match chart {
        None => {
            let hel: Vec<usize> = (0..6).filter(|&j| (bg.w[j] - wh).abs() < 1e-6).collect();
            (ritz::helike_and_competitor(&op, &eigs, wh)?, hel)
        }
        Some(c) => (ritz::helike_and_competitor_modes(&op, &eigs, &[c.roles[0], c.roles[1]], c.roles[2])?,
                    vec![c.roles[0], c.roles[1]]),
    };
    let k = eigs.vectors.len();
    // the pair is chosen with the same resolved quadrature order as the identification (flux-grid runner note)
    let amps = ritz::reference_amplitudes(&op, &eigs.vectors, result.overlap_orders[1])?;
    if hel.len() != 2 {
        return Err("Helike doublet not identified".into());
    }
    // dressed weights (amendment 5.2): relative to the ground state's vacuum weight Z
    let z = result.vacuum_weight;
    let weight = |c: usize| hel.iter().map(|&m| amps[m * k + c].powi(2)).sum::<f64>() / z;
    let comp = result.competitor.level_index;
    let pair = match selection {
        Selection::Quasimode => {
            let mut order: Vec<usize> = (1..k).collect();
            order.sort_by(|&a, &b| weight(b).partial_cmp(&weight(a)).unwrap());
            let mut pair = [order[0], order[1]];
            pair.sort();
            if pair.iter().any(|&c| weight(c) < QUASIMODE_WEIGHT_MIN) || !pair.contains(&result.helike.level_index) {
                return Err("unresolved Helike partner identification".into());
            }
            if result.competitor.dressed_weight < QUASIMODE_WEIGHT_MIN {
                return Err("ambiguous state selection".into());
            }
            pair
        }
        Selection::Grid => {
            let h = result.helike.level_index;
            if result.helike.dressed_weight < GRID_WEIGHT_MIN || result.competitor.dressed_weight < GRID_WEIGHT_MIN {
                return Err(format!("not identified by the grid rule w~ >= {GRID_WEIGHT_MIN}: Helike {:.4}, competitor {:.4}",
                                   result.helike.dressed_weight, result.competitor.dressed_weight));
            }
            let chart_iv = ChartIv::new(op.chart.u0, op.chart.s)?;
            let (b, q) = charge_partner(&chart_iv, &op.basis, bg.w, &eigs.vectors, h, &[comp])?;
            if q <= 0.5 {
                return Err(format!("Helike's charge partner is not resolved (|Im<v_h, L v_b>| = {q:.3})"));
            }
            let mut pair = [h, b];
            pair.sort();
            pair
        }
    };
    let idx = [0, pair[0], pair[1], comp];
    if idx.iter().collect::<std::collections::BTreeSet<_>>().len() != 4 {
        return Err("ambiguous state selection".into());
    }
    let n = op.dim();
    let mut coeff = vec![0.0; n * 4];
    for (j, &c) in idx.iter().enumerate() {
        for r in 0..n {
            coeff[r * 4 + j] = eigs.vectors[c][r];
        }
    }
    let mut centers: Vec<f64> = idx.iter().map(|&c| eigs.values[c] + op.energy_shift).collect();
    let mid = 0.5 * (centers[1] + centers[2]);
    centers[1] = mid;
    centers[2] = mid;
    let chart = ChartIv::new(op.chart.u0, op.chart.s)?;
    let (mix, charges) = chiral_mixing(&chart, &op.basis, bg.w, &eigs.vectors[pair[0]], &eigs.vectors[pair[1]])?;
    let flat = |f: &dyn Fn((f64, f64)) -> f64| -> Vec<u64> { mix.iter().flatten().map(|&z| f(z).to_bits()).collect() };
    Ok(StateExport {
        schema: SCHEMA,
        s,
        t,
        g2,
        g,
        mq,
        moments: op.moments(),
        u0: bits(&op.chart.u0),
        s_mat: bits(&op.chart.s.iter().flatten().cloned().collect::<Vec<_>>()),
        w: bits(&bg.w),
        g0: bits(&bg.g0.iter().flatten().cloned().collect::<Vec<_>>()),
        m: bits(&bg.m.iter().flatten().cloned().collect::<Vec<_>>()),
        basis: op.basis.iter().map(|st| st.to_vec()).collect(),
        coeff: bits(&coeff),
        coeff_shape: [n, 4],
        mixing_re: flat(&|z| z.0),
        mixing_im: flat(&|z| z.1),
        centers: bits(&centers),
        names: NAMES,
        level_indices: idx,
        pair_weights: [weight(pair[0]), weight(pair[1])],
        competitor_weight: result.competitor.weight,
        compressed_charges: charges,
        energy_shift: op.energy_shift,
        levels: eigs.values.clone(),
        max_in_space_lanczos_residual: eigs.residuals.iter().cloned().fold(0.0, f64::max),
    })
}

// ------------------------------------------------------------------------------------------------------------------
// M8 (docs/research/helike-M8-weighted-geometry-registration-2026-11-26.md): every Ritz level at a point, so the
// carrier can be followed by physical overlap and crossings verified. Report-only data; it decides nothing here.

pub const LEVELS_SCHEMA: &str = "helike-lab/cone-levels-export/v1";

#[derive(Clone, Debug, Serialize)]
pub struct LevelsExport {
    pub schema: &'static str,
    pub s: f64,
    pub t: f64,
    pub g2: f64,
    pub g: f64,
    #[serde(rename = "Mq")]
    pub mq: usize,
    pub moments: &'static str,
    pub u0: Vec<u64>,
    #[serde(rename = "S")]
    pub s_mat: Vec<u64>,
    pub w: Vec<u64>,
    #[serde(rename = "G0")]
    pub g0: Vec<u64>,
    #[serde(rename = "M")]
    pub m: Vec<u64>,
    pub basis: Vec<Vec<u8>>,
    /// N × k real, row-major: every Ritz vector
    pub coeff: Vec<u64>,
    pub coeff_shape: [usize; 2],
    pub energy_shift: f64,
    /// Ritz values (without the energy shift), ascending
    pub levels: Vec<f64>,
    /// true in-space residual norms ‖Hx − θx‖; for the finite symmetric matrix |θ − λ| ≤ ‖Hx − θx‖ (the recorded
    /// per-level numerical uncertainty; it excludes floating-point error in assembling H)
    pub in_space_residuals: Vec<f64>,
    pub uncertainty_method: &'static str,
    /// k × k, row-major: Im <v_a, L v_b> (antisymmetric). A charge-±m pair (a, b) has |Im| ≈ m.
    pub charge_im: Vec<f64>,
    /// <v_a, L v_a>, expected 0 for real vectors (a check)
    pub charge_diag: Vec<f64>,
    /// [h1, h2, c, x, s1, s2] mode indices (heat-signal amendment 1)
    pub roles: [usize; 6],
    /// §5.3 dressed overlap per level on Helike's doublet and on the competitor's mode (resolved fine order)
    pub dressed_overlap_helike: Vec<Option<f64>>,
    pub dressed_overlap_competitor: Vec<Option<f64>>,
    pub identification: ritz::HelikeCompetitor,
}

/// Solve one point with exact moments and export every Ritz level (M8).
pub fn export_levels(s: f64, t: f64, g2: f64, mq: usize, set: &SolveSettings) -> Result<LevelsExport, String> {
    export_levels_chart(s, t, g2, mq, set, None)
}

/// `export_levels` in a supplied (dressed) chart (flux-method §5.10); the roles come from the chart file.
pub fn export_levels_chart(s: f64, t: f64, g2: f64, mq: usize, set: &SolveSettings,
                           chart: Option<&crate::model::ChartFile>) -> Result<LevelsExport, String> {
    if !(g2 > 0.0) {
        return Err("g^2 must be positive".into());
    }
    let g = g2.sqrt();
    let bg = match chart {
        None => Background::maurer_cartan(s, t, g)?,
        Some(c) => Background::dressed(s, t, g, c.g0, c.m, c.w)?,
    };
    let op = ConeOperator::exact(bg.clone(), mq, OpMutation::None)?;
    let eigs = ritz::spectrum(&op, set);
    let (identification, roles) = match chart {
        None => {
            let wh = ritz::helike_frequency(s, t);
            (ritz::helike_and_competitor(&op, &eigs, wh)?, ritz::mode_roles(&bg.w, wh)?)
        }
        Some(c) => (ritz::helike_and_competitor_modes(&op, &eigs, &[c.roles[0], c.roles[1]], c.roles[2])?, c.roles),
    };
    let k = eigs.vectors.len();
    let n = op.dim();
    let mut coeff = vec![0.0; n * k];
    for c in 0..k {
        for r in 0..n {
            coeff[r * k + c] = eigs.vectors[c][r];
        }
    }
    // the charge compression over all levels: Im <v_a, L v_b> by polarization (L is Hermitian and imaginary in the real
    // basis, so <v_a, L v_a> = 0 and the a-b block is [[0, i m], [-i m, 0]] with charges -|m|, +|m|). Pairs are found
    // from this coupling, not from an energy tolerance: the trial space splits Helike's exact pair slightly.
    let chart = ChartIv::new(op.chart.u0, op.chart.s)?;
    let r = Iv::point(0.5).sqrt();
    let pairs: Vec<(usize, usize)> = (0..k).flat_map(|a| ((a + 1)..k).map(move |b| (a, b))).collect();
    // columns: the k real vectors, then (v_a + i v_b)/sqrt 2 for every pair; the charge routine takes four at a time
    let column = |j: usize, row: usize| -> Cx {
        if j < k {
            Cx::real(Iv::point(eigs.vectors[j][row]))
        } else {
            let (a, b) = pairs[j - k];
            Cx::new(Iv::point(eigs.vectors[a][row]) * r, Iv::point(eigs.vectors[b][row]) * r)
        }
    };
    let total = k + pairs.len();
    let mut d = Vec::with_capacity(total);
    for start in (0..total).step_by(4) {
        let nc = (total - start).min(4);
        let mut cols = vec![Cx::new(Iv::point(0.0), Iv::point(0.0)); n * nc];
        for row in 0..n {
            for c in 0..nc {
                cols[row * nc + c] = column(start + c, row);
            }
        }
        d.extend(charge_expectations(&chart, &op.basis, bg.w, &cols, nc)?);
    }
    let mut charge_im = vec![0.0; k * k];
    for (j, &(a, b)) in pairs.iter().enumerate() {
        let im = 0.5 * (d[a] + d[b]) - d[k + j];
        charge_im[a * k + b] = im;
        charge_im[b * k + a] = -im;
    }
    let charge_diag: Vec<f64> = d[..k].to_vec();
    let terms = ritz::dressed_overlap_terms(&op, &eigs.vectors, identification.overlap_orders[1]).ok();
    let per_level = |modes: &[usize]| -> Vec<Option<f64>> {
        (0..k).map(|c| terms.as_ref().and_then(|t| ritz::dressed_overlap_weight(t, k, modes, c))).collect()
    };
    Ok(LevelsExport {
        schema: LEVELS_SCHEMA,
        s,
        t,
        g2,
        g,
        mq,
        moments: op.moments(),
        u0: bits(&op.chart.u0),
        s_mat: bits(&op.chart.s.iter().flatten().cloned().collect::<Vec<_>>()),
        w: bits(&bg.w),
        g0: bits(&bg.g0.iter().flatten().cloned().collect::<Vec<_>>()),
        m: bits(&bg.m.iter().flatten().cloned().collect::<Vec<_>>()),
        basis: op.basis.iter().map(|st| st.to_vec()).collect(),
        coeff: bits(&coeff),
        coeff_shape: [n, k],
        energy_shift: op.energy_shift,
        levels: eigs.values.clone(),
        in_space_residuals: eigs.residuals.clone(),
        uncertainty_method: "in-space residual norm |Hx - theta x| (bound for the finite symmetric matrix; excludes assembly rounding)",
        charge_im,
        charge_diag,
        roles,
        dressed_overlap_helike: per_level(&[roles[0], roles[1]]),
        dressed_overlap_competitor: per_level(&[roles[2]]),
        identification,
    })
}
