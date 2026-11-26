//! M8 physical overlaps between states at two points (docs/research/helike-M8-weighted-geometry-registration-
//! 2026-11-26.md, A3.1; the Rust port of `helike_m8_overlap.overlap_matrix`, which stays as the reference).
//!
//! The Cholesky coordinates u = (log r, a, b, c) are global and each point's chart is affine, u = u0 + S z. With
//! chi = sqrt(rho) Psi and rho dz = dmu, rho_A / rho_B = |det S_A / det S_B| is constant, so
//!   <Psi_a(A), Psi_b(B)> = sqrt|det S_B / det S_A| * int chi_a(z_A(z_B)) chi_b(z_B) dz_B,
//!   z_A = S_A^{-1} (u0_B - u0_A + S_B z_B).
//! On B's Gauss–Hermite grid (x = sqrt(w) z, weights normalised to 1 per axis):
//!   O[a][b] = sum_nodes W p_a(x_A) p_b(x_B) e^{(|x_B|^2 - |x_A|^2)/2} pi^3 N_A N_B / prod sqrt(w_B) * jac,
//!   N = prod (w / pi)^{1/4}.
//! B's values stream from `SlabGrid` one slab at a time; A's states are evaluated directly at the mapped points.

use crate::basis::State;
use crate::grid::SlabGrid;
use crate::hermite;
use rayon::prelude::*;
use serde_json::Value;

/// One `cone-levels` export: the chart, the frequencies, the basis and every Ritz vector.
pub struct Levels {
    pub u0: [f64; 6],
    pub s: [[f64; 6]; 6],
    pub w: [f64; 6],
    pub basis: Vec<State>,
    /// N x k, row-major
    pub coeff: Vec<f64>,
    pub k: usize,
    pub mq: usize,
}

fn f64s(v: &Value) -> Result<Vec<f64>, String> {
    v.as_array()
        .ok_or("expected an array of bit patterns")?
        .iter()
        .map(|x| x.as_u64().map(f64::from_bits).ok_or_else(|| "expected u64 bits".to_string()))
        .collect()
}

impl Levels {
    pub fn from_json(text: &str) -> Result<Self, String> {
        let e: Value = serde_json::from_str(text).map_err(|x| x.to_string())?;
        if e["schema"] != "helike-lab/cone-levels-export/v1" || e["moments"] != "exact" {
            return Err("not an exact-moment cone-levels export".into());
        }
        let u0v = f64s(&e["u0"])?;
        let sv = f64s(&e["S"])?;
        let wv = f64s(&e["w"])?;
        let shape = e["coeff_shape"].as_array().ok_or("coeff_shape")?;
        let (n, k) = (shape[0].as_u64().ok_or("n")? as usize, shape[1].as_u64().ok_or("k")? as usize);
        let coeff = f64s(&e["coeff"])?;
        let basis: Vec<State> = e["basis"]
            .as_array()
            .ok_or("basis")?
            .iter()
            .map(|b| {
                let a = b.as_array().unwrap();
                let mut s = [0u8; 6];
                for j in 0..6 {
                    s[j] = a[j].as_u64().unwrap() as u8;
                }
                s
            })
            .collect();
        if u0v.len() != 6 || sv.len() != 36 || wv.len() != 6 || coeff.len() != n * k || basis.len() != n {
            return Err("inconsistent export shapes".into());
        }
        let mut s = [[0.0; 6]; 6];
        for i in 0..6 {
            for j in 0..6 {
                s[i][j] = sv[i * 6 + j];
            }
        }
        Ok(Levels {
            u0: u0v.try_into().unwrap(),
            s,
            w: wv.try_into().unwrap(),
            basis,
            coeff,
            k,
            mq: e["Mq"].as_u64().ok_or("Mq")? as usize,
        })
    }
}

/// LU solve of a 6x6 system with partial pivoting; returns (inverse, |det|).
fn inverse6(a: &[[f64; 6]; 6]) -> Result<([[f64; 6]; 6], f64), String> {
    let mut m = *a;
    let mut inv = [[0.0; 6]; 6];
    for i in 0..6 {
        inv[i][i] = 1.0;
    }
    let mut det = 1.0;
    for c in 0..6 {
        let p = (c..6).max_by(|&i, &j| m[i][c].abs().partial_cmp(&m[j][c].abs()).unwrap()).unwrap();
        if m[p][c] == 0.0 {
            return Err("singular chart matrix".into());
        }
        if p != c {
            m.swap(p, c);
            inv.swap(p, c);
        }
        det *= m[c][c];
        let d = m[c][c];
        for j in 0..6 {
            m[c][j] /= d;
            inv[c][j] /= d;
        }
        for i in 0..6 {
            if i != c {
                let f = m[i][c];
                if f != 0.0 {
                    for j in 0..6 {
                        m[i][j] -= f * m[c][j];
                        inv[i][j] -= f * inv[c][j];
                    }
                }
            }
        }
    }
    Ok((inv, det.abs()))
}

/// O[a * kB + b] = <Psi_a(A), Psi_b(B)> (signed) on B's grid of the given order.
pub fn overlap(a: &Levels, b: &Levels, order: usize) -> Result<Vec<f64>, String> {
    let (ka, kb) = (a.k, b.k);
    let (sa_inv, det_a) = inverse6(&a.s)?;
    let (_, det_b) = inverse6(&b.s)?;
    // z_A = T z_B + c
    let mut t = [[0.0; 6]; 6];
    let mut c = [0.0; 6];
    for i in 0..6 {
        for j in 0..6 {
            t[i][j] = (0..6).map(|m| sa_inv[i][m] * b.s[m][j]).sum();
        }
        c[i] = (0..6).map(|m| sa_inv[i][m] * (b.u0[m] - a.u0[m])).sum();
    }
    let pi = std::f64::consts::PI;
    let norm = |w: &[f64; 6]| w.iter().map(|x| (x / pi).powf(0.25)).product::<f64>();
    let fac0 = pi.powi(3) * norm(&a.w) * norm(&b.w) / b.w.iter().map(|x| x.sqrt()).product::<f64>() * (det_b / det_a).sqrt();
    let grid = SlabGrid::new(&b.basis, b.mq + 1, order);
    let q = grid.q;
    let cols_b: Vec<Vec<f64>> = (0..kb).map(|c| (0..b.basis.len()).map(|r| b.coeff[r * kb + c]).collect()).collect();
    let refs: Vec<&[f64]> = cols_b.iter().map(|v| v.as_slice()).collect();
    let nn_a = a.mq + 1;
    let slabs: Vec<(usize, usize)> = (0..q).flat_map(|i| (0..q).map(move |j| (i, j))).collect();
    let partial: Vec<Vec<f64>> = slabs
        .par_iter()
        .map(|&(q0, q1)| {
            let mut sc = grid.scratch(kb);
            let mut o = vec![0.0; ka * kb];
            let mut va = vec![0.0; ka];
            let mut tab = vec![0.0; 6 * nn_a];
            grid.process(&refs, q0, q1, &mut sc, None, |q2, vals| {
                for p in 0..q * q * q {
                    let [q3, q4, q5] = grid.decode3(p);
                    let idx = [q0, q1, q2, q3, q4, q5];
                    let mut xb = [0.0; 6];
                    let mut wgt = 1.0;
                    for j in 0..6 {
                        xb[j] = grid.nodes[idx[j]];
                        wgt *= grid.weights[idx[j]];
                    }
                    let mut xa = [0.0; 6];
                    let mut r2b = 0.0;
                    let mut r2a = 0.0;
                    for i in 0..6 {
                        let mut za = c[i];
                        for j in 0..6 {
                            za += t[i][j] * xb[j] / b.w[j].sqrt();
                        }
                        xa[i] = za * a.w[i].sqrt();
                        r2a += xa[i] * xa[i];
                        r2b += xb[i] * xb[i];
                    }
                    let f = wgt * (0.5 * (r2b - r2a)).exp() * fac0;
                    if f == 0.0 {
                        continue;
                    }
                    for j in 0..6 {
                        let h = hermite::normalized(xa[j], a.mq);
                        tab[j * nn_a..(j + 1) * nn_a].copy_from_slice(&h);
                    }
                    va.fill(0.0);
                    for (r, st) in a.basis.iter().enumerate() {
                        let v = tab[st[0] as usize]
                            * tab[nn_a + st[1] as usize]
                            * tab[2 * nn_a + st[2] as usize]
                            * tab[3 * nn_a + st[3] as usize]
                            * tab[4 * nn_a + st[4] as usize]
                            * tab[5 * nn_a + st[5] as usize];
                        let row = &a.coeff[r * ka..(r + 1) * ka];
                        for (x, &cf) in va.iter_mut().zip(row) {
                            *x += v * cf;
                        }
                    }
                    let vb = &vals[p * kb..(p + 1) * kb];
                    for ia in 0..ka {
                        let s = f * va[ia];
                        for ib in 0..kb {
                            o[ia * kb + ib] += s * vb[ib];
                        }
                    }
                }
            });
            o
        })
        .collect();
    let mut out = vec![0.0; ka * kb];
    for o in partial {
        for (x, y) in out.iter_mut().zip(o) {
            *x += y;
        }
    }
    Ok(out)
}
