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
    if !(g2 > 0.0) {
        return Err("g^2 must be positive".into());
    }
    let g = g2.sqrt();
    let bg = Background::maurer_cartan(s, t, g)?;
    let op = ConeOperator::exact(bg.clone(), mq, OpMutation::None)?;
    let eigs = ritz::spectrum(&op, set);
    let wh = ritz::helike_frequency(s, t);
    let result = ritz::helike_and_competitor(&op, &eigs, wh)?;
    let k = eigs.vectors.len();
    // the pair is chosen with the same resolved quadrature order as the identification (flux-grid runner note)
    let amps = ritz::reference_amplitudes(&op, &eigs.vectors, result.overlap_orders[1])?;
    let hel: Vec<usize> = (0..6).filter(|&j| (bg.w[j] - wh).abs() < 1e-6).collect();
    if hel.len() != 2 {
        return Err("Helike doublet not identified".into());
    }
    let weight = |c: usize| hel.iter().map(|&m| amps[m * k + c].powi(2)).sum::<f64>();
    let mut order: Vec<usize> = (1..k).collect();
    order.sort_by(|&a, &b| weight(b).partial_cmp(&weight(a)).unwrap());
    let mut pair = [order[0], order[1]];
    pair.sort();
    let comp = result.competitor.level_index;
    if pair.iter().any(|&c| weight(c) < 0.95) || !pair.contains(&result.helike.level_index) {
        return Err("unresolved Helike partner identification".into());
    }
    let idx = [0, pair[0], pair[1], comp];
    if result.competitor.weight < 0.95 || idx.iter().collect::<std::collections::BTreeSet<_>>().len() != 4 {
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
