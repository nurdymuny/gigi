//! The sourced homogeneous model on colour singlets, and its harmonic background.
//!
//! Rows of the 3×3 matrix q are frame indices, columns colour. The potential is
//!     V(q) = ½ |ω∘q − g cof q|² − ½ Σ_k src_k |q_k|²
//! (`helike_berger_model.V_sourced` when ω = ω(t) = (√t, 1/√t, 1/√t) and src = (a, b, b) from `mc_sources`). On colour
//! singlets it depends on G = q qᵀ only:
//!     V(G) = Σ_k (ω_k² − src_k)/2 G_kk − g (Σω) √det G + (g²/2) e₂(G),
//! with e₂ the sum of principal 2×2 minors (sheet det q > 0). The kinetic cometric in the six entries
//! x = (G00, G11, G22, G01, G02, G12) is A_ij(G) = ∇_q x_i · ∇_q x_j, linear in G (`helike_condensate_pt.A_coeffs`).
//!
//! The background is a stationary point G0. Its normal frame is M = A0^{1/2} O with Oᵀ A0^{1/2} K A0^{1/2} O = diag(w²),
//! K the Hessian of V(x) at G0, exactly as `CondensatePT` builds it.

use faer::{Mat, Side};
use serde::{Deserialize, Serialize};

pub type M3 = [[f64; 3]; 3];
pub type M6 = [[f64; 6]; 6];

/// GKEYS of the Python code: x index -> (row, col) of G.
pub const GKEYS: [(usize, usize); 6] = [(0, 0), (1, 1), (2, 2), (0, 1), (0, 2), (1, 2)];

pub fn sym_index(k: usize, l: usize) -> usize {
    let (a, b) = if k <= l { (k, l) } else { (l, k) };
    GKEYS.iter().position(|&p| p == (a, b)).unwrap()
}

/// C[i][j][m] with A_ij(G) = Σ_m C[i][j][m] x_m.
pub fn a_coeffs() -> [[[f64; 6]; 6]; 6] {
    let mut c = [[[0.0; 6]; 6]; 6];
    for (i, &(k, l)) in GKEYS.iter().enumerate() {
        for (j, &(m, n)) in GKEYS.iter().enumerate() {
            for (a, b, cc, d) in [(k, m, l, n), (k, n, l, m), (l, m, k, n), (l, n, k, m)] {
                if a == b {
                    c[i][j][sym_index(cc, d)] += 1.0;
                }
            }
        }
    }
    c
}

/// The cometric A(x) as a 6×6 matrix.
pub fn a_at(x: &[f64; 6]) -> M6 {
    let c = a_coeffs();
    let mut a = [[0.0; 6]; 6];
    for i in 0..6 {
        for j in 0..6 {
            a[i][j] = (0..6).map(|m| c[i][j][m] * x[m]).sum();
        }
    }
    a
}

pub fn gram_entries(g: &M3) -> [f64; 6] {
    let mut x = [0.0; 6];
    for (i, &(k, l)) in GKEYS.iter().enumerate() {
        x[i] = g[k][l];
    }
    x
}

pub fn mat_from_vec(x: &[f64; 6]) -> M3 {
    let mut g = [[0.0; 3]; 3];
    for (i, &(k, l)) in GKEYS.iter().enumerate() {
        g[k][l] = x[i];
        g[l][k] = x[i];
    }
    g
}

fn cross(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

pub fn det3(q: &M3) -> f64 {
    q[0][0] * (q[1][1] * q[2][2] - q[1][2] * q[2][1]) - q[0][1] * (q[1][0] * q[2][2] - q[1][2] * q[2][0])
        + q[0][2] * (q[1][0] * q[2][1] - q[1][1] * q[2][0])
}

/// The model: helicities ω (one per frame row), flux coupling g, per-row quadratic sources.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Model {
    pub omega: [f64; 3],
    pub g: f64,
    pub sources: [f64; 3],
}

impl Model {
    /// The Berger sourced model on the Maurer–Cartan line at (s, t, g): ω(t), sources (a, b, b) with
    /// a = (1−s)(t−2s), b = (1−s)((1−s)/t − s).
    pub fn berger(s: f64, t: f64, g: f64) -> Self {
        let a = (1.0 - s) * (t - 2.0 * s);
        let b = (1.0 - s) * ((1.0 - s) / t - s);
        Model { omega: [t.sqrt(), 1.0 / t.sqrt(), 1.0 / t.sqrt()], g, sources: [a, b, b] }
    }

    /// V_sourced(q).
    pub fn potential(&self, q: &M3) -> f64 {
        let cof = [cross(&q[1], &q[2]), cross(&q[2], &q[0]), cross(&q[0], &q[1])];
        let mut v = 0.0;
        for k in 0..3 {
            for c in 0..3 {
                let f = self.omega[k] * q[k][c] - self.g * cof[k][c];
                v += 0.5 * f * f - 0.5 * self.sources[k] * q[k][c] * q[k][c];
            }
        }
        v
    }

    /// ∇_x V and the Hessian K of V as a function of x = G entries (sheet det q > 0).
    pub fn gradient_hessian(&self, x: &[f64; 6]) -> ([f64; 6], M6) {
        let d = x[0] * x[1] * x[2] + 2.0 * x[3] * x[4] * x[5] - x[0] * x[5] * x[5] - x[1] * x[4] * x[4] - x[2] * x[3] * x[3];
        assert!(d > 0.0, "background Gram matrix must be positive definite");
        let dd = [
            x[1] * x[2] - x[5] * x[5],
            x[0] * x[2] - x[4] * x[4],
            x[0] * x[1] - x[3] * x[3],
            2.0 * x[4] * x[5] - 2.0 * x[2] * x[3],
            2.0 * x[3] * x[5] - 2.0 * x[1] * x[4],
            2.0 * x[3] * x[4] - 2.0 * x[0] * x[5],
        ];
        let mut hd = [[0.0; 6]; 6];
        let mut set = |i: usize, j: usize, v: f64| {
            hd[i][j] = v;
            hd[j][i] = v;
        };
        set(0, 1, x[2]);
        set(0, 2, x[1]);
        set(1, 2, x[0]);
        set(0, 5, -2.0 * x[5]);
        set(1, 4, -2.0 * x[4]);
        set(2, 3, -2.0 * x[3]);
        set(3, 3, -2.0 * x[2]);
        set(4, 4, -2.0 * x[1]);
        set(5, 5, -2.0 * x[0]);
        set(3, 4, 2.0 * x[5]);
        set(3, 5, 2.0 * x[4]);
        set(4, 5, 2.0 * x[3]);
        let sd = d.sqrt();
        let cubic = self.g * self.omega.iter().sum::<f64>();
        let g2 = self.g * self.g;
        // e2 = x0x1 + x0x2 + x1x2 - x3^2 - x4^2 - x5^2
        let de2 = [x[1] + x[2], x[0] + x[2], x[0] + x[1], -2.0 * x[3], -2.0 * x[4], -2.0 * x[5]];
        let mut grad = [0.0; 6];
        let mut hess = [[0.0; 6]; 6];
        for i in 0..6 {
            let lin = if i < 3 { 0.5 * (self.omega[i] * self.omega[i] - self.sources[i]) } else { 0.0 };
            grad[i] = lin - cubic * dd[i] / (2.0 * sd) + 0.5 * g2 * de2[i];
            for j in 0..6 {
                let hsq = hd[i][j] / (2.0 * sd) - dd[i] * dd[j] / (4.0 * d * sd);
                let he2 = match (i, j) {
                    (0, 1) | (1, 0) | (0, 2) | (2, 0) | (1, 2) | (2, 1) => 1.0,
                    (3, 3) | (4, 4) | (5, 5) => -2.0,
                    _ => 0.0,
                };
                hess[i][j] = -cubic * hsq + 0.5 * g2 * he2;
            }
        }
        (grad, hess)
    }
}

pub fn to_mat6(a: &M6) -> Mat<f64> {
    Mat::from_fn(6, 6, |i, j| a[i][j])
}

pub fn from_mat6(a: &Mat<f64>) -> M6 {
    let mut o = [[0.0; 6]; 6];
    for i in 0..6 {
        for j in 0..6 {
            o[i][j] = a[(i, j)];
        }
    }
    o
}

/// Symmetric eigen-decomposition of a 6×6 matrix: (ascending values, orthonormal columns).
pub fn eigh6(a: &M6) -> ([f64; 6], M6) {
    let m = Mat::from_fn(6, 6, |i, j| 0.5 * (a[i][j] + a[j][i]));
    let e = m.self_adjoint_eigen(Side::Lower).expect("6x6 eigen");
    let s = e.S().column_vector();
    let mut vals = [0.0; 6];
    for k in 0..6 {
        vals[k] = s[k];
    }
    (vals, from_mat6(&e.U().to_owned()))
}

pub fn matmul6(a: &M6, b: &M6) -> M6 {
    let mut c = [[0.0; 6]; 6];
    for i in 0..6 {
        for k in 0..6 {
            let aik = a[i][k];
            for j in 0..6 {
                c[i][j] += aik * b[k][j];
            }
        }
    }
    c
}

pub fn transpose6(a: &M6) -> M6 {
    let mut c = [[0.0; 6]; 6];
    for i in 0..6 {
        for j in 0..6 {
            c[i][j] = a[j][i];
        }
    }
    c
}

/// Inverse of a 6×6 matrix by Gauss–Jordan elimination with partial pivoting.
pub fn inv6(a: &M6) -> M6 {
    let mut m = *a;
    let mut inv = [[0.0; 6]; 6];
    for i in 0..6 {
        inv[i][i] = 1.0;
    }
    for c in 0..6 {
        let p = (c..6).max_by(|&i, &j| m[i][c].abs().partial_cmp(&m[j][c].abs()).unwrap()).unwrap();
        m.swap(c, p);
        inv.swap(c, p);
        let piv = m[c][c];
        assert!(piv != 0.0 && piv.is_finite(), "singular matrix");
        for k in 0..6 {
            m[c][k] /= piv;
            inv[c][k] /= piv;
        }
        for r in 0..6 {
            if r != c {
                let f = m[r][c];
                if f != 0.0 {
                    for k in 0..6 {
                        m[r][k] -= f * m[c][k];
                        inv[r][k] -= f * inv[c][k];
                    }
                }
            }
        }
    }
    inv
}

/// log |det| of a 6×6 matrix.
pub fn logdet6(a: &M6) -> f64 {
    // Gaussian elimination with partial pivoting
    let mut m = *a;
    let mut ld = 0.0;
    for c in 0..6 {
        let p = (c..6).max_by(|&i, &j| m[i][c].abs().partial_cmp(&m[j][c].abs()).unwrap()).unwrap();
        m.swap(c, p);
        let piv = m[c][c];
        assert!(piv != 0.0, "singular matrix");
        ld += piv.abs().ln();
        for r in c + 1..6 {
            let f = m[r][c] / piv;
            for k in c..6 {
                m[r][k] -= f * m[c][k];
            }
        }
    }
    ld
}

pub fn cond6_sym(a: &M6) -> f64 {
    let (v, _) = eigh6(a);
    let lo = v.iter().cloned().fold(f64::INFINITY, |x, y| x.min(y.abs()));
    let hi = v.iter().cloned().fold(0.0, |x: f64, y| x.max(y.abs()));
    hi / lo
}

/// A stationary background G0 with its harmonic normal frame.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Background {
    pub model: Model,
    pub g0: M3,
    /// normal frame: G − G0 = M z (in x entries)
    pub m: M6,
    /// harmonic frequencies, ascending
    pub w: [f64; 6],
    /// max |∂V/∂x| at G0 (must vanish for a stationary background)
    pub grad_norm: f64,
}

impl Background {
    /// The background on the Maurer–Cartan line: q_bg = diag(x, y, y)/(2g) with (x, y) = (2s/√t, 2s).
    pub fn maurer_cartan(s: f64, t: f64, g: f64) -> Result<Self, String> {
        if !(s.is_finite() && t.is_finite() && g.is_finite()) || t <= 1.0 || g <= 0.0 || !(1.0 < s && s < (t + 2.0) / (t + 1.0)) {
            return Err("positive coupling and stable registered-background regime required (t > 1, g > 0, 1 < s < (t+2)/(t+1))".into());
        }
        let (x, y) = (2.0 * s / t.sqrt(), 2.0 * s);
        let d = [x / (2.0 * g), y / (2.0 * g), y / (2.0 * g)];
        let g0 = [[d[0] * d[0], 0.0, 0.0], [0.0, d[1] * d[1], 0.0], [0.0, 0.0, d[2] * d[2]]];
        Self::from_stationary(Model::berger(s, t, g), g0)
    }

    /// Any stationary G0 of the model: the normal frame is computed as in `CondensatePT`.
    pub fn from_stationary(model: Model, g0: M3) -> Result<Self, String> {
        let x0 = gram_entries(&g0);
        let (grad, k) = model.gradient_hessian(&x0);
        let grad_norm = grad.iter().fold(0.0f64, |a, &v| a.max(v.abs()));
        let a0 = a_at(&x0);
        let (wa, ua) = eigh6(&a0);
        if wa.iter().any(|&v| v <= 0.0) {
            return Err("cometric at the background is not positive definite".into());
        }
        let mut sq = [[0.0; 6]; 6];
        for i in 0..6 {
            for j in 0..6 {
                sq[i][j] = (0..6).map(|c| ua[i][c] * wa[c].sqrt() * ua[j][c]).sum();
            }
        }
        let wkw = matmul6(&matmul6(&sq, &k), &sq);
        let (w2, o) = eigh6(&wkw);
        if w2.iter().any(|&v| !(v > 0.0)) {
            return Err("background has no stable harmonic normal frame".into());
        }
        let mut w = [0.0; 6];
        for c in 0..6 {
            w[c] = w2[c].sqrt();
        }
        Ok(Background { model, g0, m: matmul6(&sq, &o), w, grad_norm })
    }

    /// Flux-method §5.10: the model on the Maurer–Cartan line, with a supplied (dressed) chart instead of the bare
    /// normal frame. The Hamiltonian is unchanged; only the trial space moves.
    pub fn dressed(s: f64, t: f64, g: f64, g0: M3, m: M6, w: [f64; 6]) -> Result<Self, String> {
        let bare = Self::maurer_cartan(s, t, g)?;
        if w.iter().any(|&v| !(v > 0.0 && v.is_finite())) {
            return Err("dressed widths must be positive and finite".into());
        }
        Ok(Self::from_parts(bare.model, g0, m, w))
    }

    /// A background given in full (for parity runs that reuse another implementation's frame).
    pub fn from_parts(model: Model, g0: M3, m: M6, w: [f64; 6]) -> Self {
        let (grad, _) = model.gradient_hessian(&gram_entries(&g0));
        Background { model, g0, m, w, grad_norm: grad.iter().fold(0.0f64, |a, &v| a.max(v.abs())) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_and_hessian_match_finite_differences() {
        let m = Model::berger(1.15, 3.0, 0.07);
        let x = [3.0, 4.0, 5.0, 0.3, -0.2, 0.4];
        let v = |x: &[f64; 6]| {
            let gm = mat_from_vec(x);
            // V(G) on the positive sheet
            let d = det3(&gm);
            let e2 = x[0] * x[1] + x[0] * x[2] + x[1] * x[2] - x[3] * x[3] - x[4] * x[4] - x[5] * x[5];
            (0..3).map(|k| 0.5 * (m.omega[k] * m.omega[k] - m.sources[k]) * x[k]).sum::<f64>()
                - m.g * m.omega.iter().sum::<f64>() * d.sqrt()
                + 0.5 * m.g * m.g * e2
        };
        let (g, h) = m.gradient_hessian(&x);
        let e = 1e-5;
        for i in 0..6 {
            let mut p = x;
            let mut q = x;
            p[i] += e;
            q[i] -= e;
            assert!(((v(&p) - v(&q)) / (2.0 * e) - g[i]).abs() < 1e-7);
            let (gp, _) = m.gradient_hessian(&p);
            let (gq, _) = m.gradient_hessian(&q);
            for j in 0..6 {
                assert!(((gp[j] - gq[j]) / (2.0 * e) - h[i][j]).abs() < 1e-6, "{i},{j}");
            }
        }
    }

    #[test]
    fn potential_in_q_matches_potential_in_g() {
        let m = Model::berger(1.15, 3.0, 0.07);
        let q = [[1.3, 0.2, -0.1], [0.4, 0.9, 0.3], [-0.2, 0.5, 1.1]];
        let mut g = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                g[i][j] = (0..3).map(|c| q[i][c] * q[j][c]).sum();
            }
        }
        let x = gram_entries(&g);
        let e2 = x[0] * x[1] + x[0] * x[2] + x[1] * x[2] - x[3] * x[3] - x[4] * x[4] - x[5] * x[5];
        let vg = (0..3).map(|k| 0.5 * (m.omega[k] * m.omega[k] - m.sources[k]) * x[k]).sum::<f64>()
            - m.g * m.omega.iter().sum::<f64>() * det3(&q)
            + 0.5 * m.g * m.g * e2;
        assert!((m.potential(&q) - vg).abs() < 1e-12);
    }

    #[test]
    fn maurer_cartan_background_is_stationary_with_helike_doublet() {
        let (s, t) = (1.15, 3.0);
        let b = Background::maurer_cartan(s, t, 0.004f64.sqrt()).unwrap();
        assert!(b.grad_norm < 1e-9, "{}", b.grad_norm);
        let wh = (2.0 * s * (1.0 + 2.0 / t - s)).sqrt();
        assert_eq!(b.w.iter().filter(|&&w| (w - wh).abs() < 1e-6).count(), 2);
    }
}

/// A dressed chart file (flux-method §5.10): schema "helike-lab/dressed-chart/v1", with bit patterns for G0 (3x3),
/// M (6x6) and w (6), and the role map [h1, h2, c, x, s1, s2] as chart-mode indices.
pub struct ChartFile {
    pub g0: M3,
    pub m: M6,
    pub w: [f64; 6],
    pub roles: [usize; 6],
}

impl ChartFile {
    pub fn read(path: &std::path::Path) -> Result<Self, String> {
        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        if v["schema"] != "helike-lab/dressed-chart/v1" {
            return Err("not a dressed-chart file".into());
        }
        let bits = |k: &str, n: usize| -> Result<Vec<f64>, String> {
            let a = v[k].as_array().ok_or(format!("missing {k}"))?;
            if a.len() != n {
                return Err(format!("{k} must have {n} entries"));
            }
            a.iter().map(|x| x.as_u64().map(f64::from_bits).ok_or(format!("{k}: expected u64 bits"))).collect()
        };
        let (g, m, w) = (bits("G0", 9)?, bits("M", 36)?, bits("w", 6)?);
        let r = v["roles"].as_array().ok_or("missing roles")?;
        if r.len() != 6 {
            return Err("roles must list [h1, h2, c, x, s1, s2]".into());
        }
        let mut roles = [0usize; 6];
        for (i, x) in r.iter().enumerate() {
            roles[i] = x.as_u64().ok_or("roles: expected integers")? as usize;
        }
        let mut sorted = roles;
        sorted.sort();
        if sorted != [0, 1, 2, 3, 4, 5] {
            return Err("roles must be a permutation of the six chart modes".into());
        }
        let mut g0 = [[0.0; 3]; 3];
        let mut mm = [[0.0; 6]; 6];
        for i in 0..3 {
            for j in 0..3 {
                g0[i][j] = g[i * 3 + j];
            }
        }
        for i in 0..6 {
            for j in 0..6 {
                mm[i][j] = m[i * 6 + j];
            }
        }
        Ok(ChartFile { g0, m: mm, w: w.try_into().unwrap(), roles })
    }
}
