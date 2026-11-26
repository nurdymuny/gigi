//! The state itself (Controls scene spec §4, "the slow function"): f = ψ/ψ₀ on a two-dimensional slice of the chart's
//! normal coordinates z, over the contours of ψ₀². A trial state is P(z)·e^{−|z|²/2} with P = Σ_n C_n Π_a h_{n_a}(z_a)
//! in the orthonormal Hermite basis (helike-quantum's `hermite::normalized`), so the Gaussian cancels and
//! f = P_state / P_ground. On a slice through z_a, z_b with the other coordinates held at fixed values, P reduces
//! exactly to a small matrix A[p][q] h_p(x) h_q(y). Inputs: `helike cone-levels` (every Ritz vector) and
//! `helike cone-export` (the chirally mixed Helike pair) files. The values are DisplayGrade: drawn, never quoted.

use helike_quantum::hermite;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Deserialize)]
struct LevelsFile {
    #[serde(rename = "Mq")]
    mq: usize,
    g2: f64,
    w: Vec<u64>,
    basis: Vec<Vec<u8>>,
    coeff: Vec<u64>,
    coeff_shape: [usize; 2],
    levels: Vec<f64>,
}

#[derive(Deserialize)]
struct PairFile {
    basis: Vec<Vec<u8>>,
    coeff: Vec<u64>,
    coeff_shape: [usize; 2],
    mixing_re: Vec<u64>,
    mixing_im: Vec<u64>,
    level_indices: [usize; 4],
    compressed_charges: [f64; 2],
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum Which {
    /// Ritz level n of the levels file (real f)
    Level(usize),
    /// column j of the chirally mixed pair file (1 = helike_plus, 2 = helike_minus, 3 = competitor)
    Mixed(usize),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Slice {
    pub g2: f64,
    pub mq: usize,
    pub which: Which,
    pub axes: (usize, usize),
    pub extent: f64,
    pub res: usize,
    /// row-major res × res, y up: ψ₀² on the slice (Gaussian included), normalised to max 1
    pub density: Vec<f32>,
    /// f = ψ/ψ₀: real part and imaginary part (zero for a real level)
    pub f_re: Vec<f32>,
    pub f_im: Vec<f32>,
    /// winding number of f around the unit circle of the slice (charge read off the picture)
    pub winding: f64,
    pub energy: Option<f64>,
    /// the export's eigenvalues of the charge compression on the Helike pair (receipt values, ascending)
    pub charges: Option<[f64; 2]>,
    pub frequencies: Vec<f64>,
    pub source: String,
}

fn bits(v: &[u64]) -> Vec<f64> {
    v.iter().map(|b| f64::from_bits(*b)).collect()
}

/// A[p][q] for a coefficient vector (complex: re, im) on the slice through axes (a, b), others at `fixed`.
fn reduce(basis: &[Vec<u8>], c_re: &[f64], c_im: &[f64], a: usize, b: usize, fixed: &[f64; 6]) -> (Vec<Vec<f64>>, Vec<Vec<f64>>, usize) {
    let nmax = basis.iter().flat_map(|s| s.iter()).copied().max().unwrap_or(0) as usize;
    let hfix: Vec<Vec<f64>> = (0..6).map(|k| hermite::normalized(fixed[k], nmax)).collect();
    let mut are = vec![vec![0.0; nmax + 1]; nmax + 1];
    let mut aim = vec![vec![0.0; nmax + 1]; nmax + 1];
    for (r, st) in basis.iter().enumerate() {
        let mut fac = 1.0;
        for k in 0..6 {
            if k != a && k != b {
                fac *= hfix[k][st[k] as usize];
            }
        }
        if fac == 0.0 {
            continue;
        }
        let (p, q) = (st[a] as usize, st[b] as usize);
        are[p][q] += c_re[r] * fac;
        aim[p][q] += c_im[r] * fac;
    }
    (are, aim, nmax)
}

fn eval(a: &[Vec<f64>], hx: &[f64], hy: &[f64]) -> f64 {
    let mut s = 0.0;
    for (p, row) in a.iter().enumerate() {
        let mut t = 0.0;
        for (q, v) in row.iter().enumerate() {
            t += v * hy[q];
        }
        s += hx[p] * t;
    }
    s
}

pub struct Request<'a> {
    pub levels_path: &'a Path,
    pub pair_path: &'a Path,
    pub which: Which,
    pub axes: (usize, usize),
    pub fixed: [f64; 6],
    pub extent: f64,
    pub res: usize,
}

pub fn compute(req: &Request) -> Result<Slice, String> {
    let lv: LevelsFile = serde_json::from_str(&std::fs::read_to_string(req.levels_path).map_err(|e| format!("{}: {e}", req.levels_path.display()))?).map_err(|e| e.to_string())?;
    let (a, b) = req.axes;
    if a == b || a > 5 || b > 5 {
        return Err("choose two different axes".into());
    }
    let [n, k] = lv.coeff_shape;
    let lc = bits(&lv.coeff);
    let col = |c: usize| -> Vec<f64> { (0..n).map(|r| lc[r * k + c]).collect() };
    let zeros = vec![0.0; n];
    let ground = col(0);
    let mut charges = None;
    let (state_re, state_im, energy, source) = match req.which {
        Which::Level(l) => {
            if l >= k {
                return Err("no such level".into());
            }
            (col(l), zeros.clone(), Some(lv.levels[l] - lv.levels[0]), req.levels_path.display().to_string())
        }
        Which::Mixed(j) => {
            let pf: PairFile = serde_json::from_str(&std::fs::read_to_string(req.pair_path).map_err(|e| format!("{}: {e}", req.pair_path.display()))?).map_err(|e| e.to_string())?;
            if pf.basis != lv.basis {
                return Err("pair file and levels file use different bases".into());
            }
            let [pn, pk] = pf.coeff_shape;
            let pc = bits(&pf.coeff);
            let (mr, mi) = (bits(&pf.mixing_re), bits(&pf.mixing_im));
            // mixed = coeff @ mixing (the product the domain gate freezes)
            let mut re = vec![0.0; pn];
            let mut im = vec![0.0; pn];
            for r in 0..pn {
                for i in 0..pk {
                    re[r] += pc[r * pk + i] * mr[i * pk + j];
                    im[r] += pc[r * pk + i] * mi[i * pk + j];
                }
            }
            let li = pf.level_indices[j.min(3)];
            charges = Some(pf.compressed_charges);
            (re, im, Some(lv.levels[li] - lv.levels[0]), req.pair_path.display().to_string())
        }
    };
    let (g_re, _, nmax) = reduce(&lv.basis, &ground, &zeros, a, b, &req.fixed);
    let (s_re, s_im, _) = reduce(&lv.basis, &state_re, &state_im, a, b, &req.fixed);
    let res = req.res.max(16);
    let coord = |i: usize| -req.extent + 2.0 * req.extent * (i as f64 + 0.5) / res as f64;
    let hs: Vec<Vec<f64>> = (0..res).map(|i| hermite::normalized(coord(i), nmax)).collect();
    let mut density = vec![0f32; res * res];
    let mut f_re = vec![0f32; res * res];
    let mut f_im = vec![0f32; res * res];
    let mut dmax = 0f64;
    let others: f64 = (0..6).filter(|&q| q != a && q != b).map(|q| req.fixed[q] * req.fixed[q]).sum();
    for iy in 0..res {
        for ix in 0..res {
            let (x, y) = (coord(ix), coord(res - 1 - iy));
            let (hx, hy) = (&hs[ix], &hs[res - 1 - iy]);
            let p0 = eval(&g_re, hx, hy);
            let d = p0 * p0 * (-(x * x + y * y + others)).exp();
            let fr = eval(&s_re, hx, hy) / p0;
            let fi = eval(&s_im, hx, hy) / p0;
            density[iy * res + ix] = d as f32;
            f_re[iy * res + ix] = fr as f32;
            f_im[iy * res + ix] = fi as f32;
            dmax = dmax.max(d);
        }
    }
    for d in &mut density {
        *d /= dmax.max(1e-300) as f32;
    }
    // winding of f around the unit circle in the slice plane
    let steps = 720;
    let mut wind = 0.0;
    let mut prev: Option<f64> = None;
    for s in 0..=steps {
        let th = std::f64::consts::TAU * s as f64 / steps as f64;
        let (x, y) = (th.cos(), th.sin());
        let (hx, hy) = (hermite::normalized(x, nmax), hermite::normalized(y, nmax));
        let p0 = eval(&g_re, &hx, &hy);
        let ph = (eval(&s_im, &hx, &hy) / p0).atan2(eval(&s_re, &hx, &hy) / p0);
        if let Some(pp) = prev {
            let mut d = ph - pp;
            while d > std::f64::consts::PI {
                d -= std::f64::consts::TAU;
            }
            while d < -std::f64::consts::PI {
                d += std::f64::consts::TAU;
            }
            wind += d;
        }
        prev = Some(ph);
    }
    Ok(Slice {
        g2: lv.g2,
        mq: lv.mq,
        which: req.which,
        axes: req.axes,
        extent: req.extent,
        res,
        density,
        f_re,
        f_im,
        winding: wind / std::f64::consts::TAU,
        energy,
        charges,
        frequencies: bits(&lv.w),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hermite reduction is exact: a single basis state evaluates to the product of its 1-D functions.
    #[test]
    fn reduction_matches_direct_evaluation() {
        let basis = vec![vec![0u8, 0, 0, 0, 0, 0], vec![2, 1, 0, 2, 0, 0], vec![1, 0, 3, 0, 0, 1]];
        let c = vec![0.3, -0.7, 0.2];
        let fixed = [0.0, 0.0, 0.4, -0.3, 0.1, 0.25];
        let (a, _, nmax) = reduce(&basis, &c, &[0.0; 3], 0, 1, &fixed);
        let (x, y) = (0.37, -1.1);
        let direct: f64 = basis
            .iter()
            .zip(&c)
            .map(|(st, cc)| {
                let z = [x, y, fixed[2], fixed[3], fixed[4], fixed[5]];
                cc * (0..6).map(|k| hermite::normalized(z[k], nmax)[st[k] as usize]).product::<f64>()
            })
            .sum();
        let via = eval(&a, &hermite::normalized(x, nmax), &hermite::normalized(y, nmax));
        assert!((direct - via).abs() < 1e-14, "{direct} vs {via}");
    }
}
