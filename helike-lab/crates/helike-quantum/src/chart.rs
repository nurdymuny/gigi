//! The positive Gram-cone chart (`helike_positive_cone.ConeChart`).
//!
//! G = L Lᵀ with L lower triangular, diag L = exp(u0..u2), offdiagonal (L10, L20, L21) = (u3, u4, u5), and
//! u = u0 + S z with S = J_u(L0)^{-1} M. Every z ∈ ℝ⁶ lands strictly inside the positive cone. The measure is
//! log ν = log 8 + log|det S| + (3, 2, 1, 0, 0, 0)·u, and the cometric in z is B = J⁻¹ A(G) J⁻ᵀ with J = J_u(L) S.

use crate::model::*;
use serde::{Deserialize, Serialize};

pub const MEASURE_POWERS: [f64; 6] = [3.0, 2.0, 1.0, 0.0, 0.0, 0.0];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConeChart {
    pub g0: M3,
    pub m: M6,
    pub u0: [f64; 6],
    pub s: M6,
    pub inv_s: M6,
    pub ell: [f64; 6],
    pub log_scale: f64,
    #[serde(skip, default = "a_coeff_list")]
    a_list: Vec<(usize, usize, usize, f64)>,
}

fn a_coeff_list() -> Vec<(usize, usize, usize, f64)> {
    let c = a_coeffs();
    let mut out = vec![];
    for i in 0..6 {
        for j in 0..6 {
            for m in 0..6 {
                if c[i][j][m] != 0.0 {
                    out.push((i, j, m, c[i][j][m]));
                }
            }
        }
    }
    out
}

/// Cholesky factor of a symmetric positive definite 3×3 matrix; refuses (never projects) otherwise.
pub fn positive_factor(g: &M3) -> Result<M3, String> {
    for i in 0..3 {
        for j in 0..3 {
            if !g[i][j].is_finite() || g[i][j] != g[j][i] {
                return Err("G must be a finite symmetric 3 by 3 matrix".into());
            }
        }
    }
    let mut l = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..=i {
            let s: f64 = g[i][j] - (0..j).map(|k| l[i][k] * l[j][k]).sum::<f64>();
            if i == j {
                if !(s > 0.0) {
                    return Err("G must be positive definite; projection is forbidden".into());
                }
                l[i][i] = s.sqrt();
            } else {
                l[i][j] = s / l[j][j];
            }
        }
    }
    let gm = g_eigs(g);
    let cond = gm[2] / gm[0];
    if !(cond <= 1e12) {
        return Err("Gram chart is numerically ill-conditioned".into());
    }
    Ok(l)
}

/// Eigenvalues of a symmetric 3×3 matrix, ascending.
fn g_eigs(g: &M3) -> [f64; 3] {
    let m = faer::Mat::from_fn(3, 3, |i, j| g[i][j]);
    let e = m.self_adjoint_eigenvalues(faer::Side::Lower).expect("3x3 eigenvalues");
    [e[0], e[1], e[2]]
}

/// J_u(L): derivative of x = (G00, G11, G22, G01, G02, G12) with respect to u = (log r0, log r1, log r2, a, b, c).
pub fn jacobian_u(l: &M3) -> M6 {
    let (r0, r1, r2) = (l[0][0], l[1][1], l[2][2]);
    let (a, b, c) = (l[1][0], l[2][0], l[2][1]);
    [
        [2.0 * r0 * r0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 2.0 * r1 * r1, 0.0, 2.0 * a, 0.0, 0.0],
        [0.0, 0.0, 2.0 * r2 * r2, 0.0, 2.0 * b, 2.0 * c],
        [r0 * a, 0.0, 0.0, r0, 0.0, 0.0],
        [r0 * b, 0.0, 0.0, 0.0, r0, 0.0],
        [0.0, r1 * c, 0.0, b, a, r1],
    ]
}

/// Solve J_u x = y (the triangular structure: G00→u0, G01→u3, G02→u4, G11→u1, G12→u5, G22→u2).
#[inline]
pub fn solve_ju(r: &[f64; 3], a: f64, b: f64, c: f64, y: &[f64; 6]) -> [f64; 6] {
    let mut x = [0.0; 6];
    x[0] = y[0] / (2.0 * r[0] * r[0]);
    x[3] = (y[3] - r[0] * a * x[0]) / r[0];
    x[4] = (y[4] - r[0] * b * x[0]) / r[0];
    x[1] = (y[1] - 2.0 * a * x[3]) / (2.0 * r[1] * r[1]);
    x[5] = (y[5] - r[1] * c * x[1] - b * x[3] - a * x[4]) / r[1];
    x[2] = (y[2] - 2.0 * b * x[4] - 2.0 * c * x[5]) / (2.0 * r[2] * r[2]);
    x
}

/// Solve J_uᵀ x = y (x indexed by G entries).
#[inline]
pub fn solve_ju_t(r: &[f64; 3], a: f64, b: f64, c: f64, y: &[f64; 6]) -> [f64; 6] {
    let mut x = [0.0; 6];
    x[2] = y[2] / (2.0 * r[2] * r[2]);
    x[5] = (y[5] - 2.0 * c * x[2]) / r[1];
    x[1] = (y[1] - r[1] * c * x[5]) / (2.0 * r[1] * r[1]);
    x[4] = (y[4] - 2.0 * b * x[2] - a * x[5]) / r[0];
    x[3] = (y[3] - 2.0 * a * x[1] - b * x[5]) / r[0];
    x[0] = (y[0] - r[0] * a * x[3] - r[0] * b * x[4]) / (2.0 * r[0] * r[0]);
    x
}

/// Geometry at one chart point, everything the operator needs.
pub struct Point {
    pub l: M3,
    pub r: [f64; 3],
    pub x: [f64; 6],
    pub u: [f64; 6],
}

impl ConeChart {
    pub fn new(g0: &M3, m: &M6) -> Result<Self, String> {
        let l = positive_factor(g0)?;
        for row in m {
            if row.iter().any(|v| !v.is_finite()) {
                return Err("normal frame must be finite and nonsingular".into());
            }
        }
        let u0 = [l[0][0].ln(), l[1][1].ln(), l[2][2].ln(), l[1][0], l[2][0], l[2][1]];
        let s = matmul6(&inv6(&jacobian_u(&l)), m);
        let inv_s = inv6(&s);
        let mut ell = [0.0; 6];
        for j in 0..6 {
            ell[j] = (0..6).map(|i| s[i][j] * MEASURE_POWERS[i]).sum();
        }
        let log_scale = 8f64.ln() + logdet6(&s);
        Ok(ConeChart { g0: *g0, m: *m, u0, s, inv_s, ell, log_scale, a_list: a_coeff_list() })
    }

    pub fn coordinates(&self, z: &[f64; 6]) -> [f64; 6] {
        let mut u = self.u0;
        for i in 0..6 {
            for j in 0..6 {
                u[i] += self.s[i][j] * z[j];
            }
        }
        u
    }

    pub fn point_from_u(&self, u: &[f64; 6]) -> Point {
        let r = [u[0].exp(), u[1].exp(), u[2].exp()];
        let (a, b, c) = (u[3], u[4], u[5]);
        let l = [[r[0], 0.0, 0.0], [a, r[1], 0.0], [b, c, r[2]]];
        let x = [
            r[0] * r[0],
            a * a + r[1] * r[1],
            b * b + c * c + r[2] * r[2],
            r[0] * a,
            r[0] * b,
            a * b + r[1] * c,
        ];
        Point { l, r, x, u: *u }
    }

    pub fn factor(&self, z: &[f64; 6]) -> M3 {
        self.point_from_u(&self.coordinates(z)).l
    }

    pub fn gram(&self, z: &[f64; 6]) -> M3 {
        mat_from_vec(&self.point_from_u(&self.coordinates(z)).x)
    }

    pub fn jacobian(&self, z: &[f64; 6]) -> M6 {
        matmul6(&jacobian_u(&self.factor(z)), &self.s)
    }

    pub fn log_measure(&self, z: &[f64; 6]) -> f64 {
        let u = self.coordinates(z);
        self.log_scale + (0..6).map(|i| MEASURE_POWERS[i] * u[i]).sum::<f64>()
    }

    /// The cometric B(z) = J⁻¹ A(G) J⁻ᵀ, dense (for checks).
    pub fn metric(&self, z: &[f64; 6]) -> M6 {
        let p = self.point_from_u(&self.coordinates(z));
        let inv = inv6(&self.jacobian(z));
        matmul6(&matmul6(&inv, &a_at(&p.x)), &transpose6(&inv))
    }

    /// B(z) v without forming B: inv_S · J_u⁻¹ · A(G) · J_u⁻ᵀ · inv_Sᵀ · v.
    #[inline]
    pub fn apply_metric(&self, p: &Point, v: &[f64; 6]) -> [f64; 6] {
        let mut t = [0.0; 6];
        for i in 0..6 {
            for j in 0..6 {
                t[i] += self.inv_s[j][i] * v[j];
            }
        }
        let y = solve_ju_t(&p.r, p.u[3], p.u[4], p.u[5], &t);
        let mut ay = [0.0; 6];
        for &(i, j, m, c) in &self.a_list {
            ay[i] += c * p.x[m] * y[j];
        }
        let x = solve_ju(&p.r, p.u[3], p.u[4], p.u[5], &ay);
        let mut out = [0.0; 6];
        for i in 0..6 {
            for j in 0..6 {
                out[i] += self.inv_s[i][j] * x[j];
            }
        }
        out
    }

    /// The cometric in the u coordinates, C = J_u⁻¹ A(G) J_u⁻ᵀ, applied to v (B = S⁻¹ C S⁻ᵀ). This is the
    /// `laurent_model` metric of the Python code, contracted with the u-derivatives D_i.
    #[inline]
    pub fn apply_cometric_u(&self, p: &Point, v: &[f64; 6]) -> [f64; 6] {
        let y = solve_ju_t(&p.r, p.u[3], p.u[4], p.u[5], v);
        let mut ay = [0.0; 6];
        for &(i, j, m, c) in &self.a_list {
            ay[i] += c * p.x[m] * y[j];
        }
        solve_ju(&p.r, p.u[3], p.u[4], p.u[5], &ay)
    }

    /// A(G) at a point, as a dense 6×6 matrix (to reuse across several vectors).
    #[inline]
    pub fn cometric_a(&self, p: &Point) -> M6 {
        let mut a = [[0.0; 6]; 6];
        for &(i, j, m, c) in &self.a_list {
            a[i][j] += c * p.x[m];
        }
        a
    }

    /// C v = J_u⁻¹ A J_u⁻ᵀ v with A precomputed by `cometric_a`.
    #[inline]
    pub fn apply_cometric_u_with(&self, p: &Point, a: &M6, v: &[f64; 6]) -> [f64; 6] {
        let y = solve_ju_t(&p.r, p.u[3], p.u[4], p.u[5], v);
        let mut ay = [0.0; 6];
        for i in 0..6 {
            ay[i] = a[i][0] * y[0] + a[i][1] * y[1] + a[i][2] * y[2] + a[i][3] * y[3] + a[i][4] * y[4] + a[i][5] * y[5];
        }
        solve_ju(&p.r, p.u[3], p.u[4], p.u[5], &ay)
    }

    /// The inverse chart: z for a positive definite G.
    pub fn inverse(&self, g: &M3) -> Result<[f64; 6], String> {
        let l = positive_factor(g)?;
        let u = [l[0][0].ln(), l[1][1].ln(), l[2][2].ln(), l[1][0], l[2][0], l[2][1]];
        let mut z = [0.0; 6];
        for i in 0..6 {
            for j in 0..6 {
                z[i] += self.inv_s[i][j] * (u[j] - self.u0[j]);
            }
        }
        Ok(z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chart() -> ConeChart {
        let g0 = [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 4.0]];
        let a0 = a_at(&gram_entries(&g0));
        // Cholesky of A0 as the frame (the Python test fixture)
        let mut m = [[0.0; 6]; 6];
        for i in 0..6 {
            for j in 0..=i {
                let s: f64 = a0[i][j] - (0..j).map(|k| m[i][k] * m[j][k]).sum::<f64>();
                m[i][j] = if i == j { s.sqrt() } else { s / m[j][j] };
            }
        }
        ConeChart::new(&g0, &m).unwrap()
    }

    #[test]
    fn triangular_solves_match_dense_inverse() {
        let c = chart();
        let z = [0.12, -0.23, 0.05, 0.18, -0.17, 0.09];
        let p = c.point_from_u(&c.coordinates(&z));
        let ju = jacobian_u(&p.l);
        let y = [0.3, -1.1, 0.7, 0.2, 0.5, -0.4];
        let x = solve_ju(&p.r, p.u[3], p.u[4], p.u[5], &y);
        let xt = solve_ju_t(&p.r, p.u[3], p.u[4], p.u[5], &y);
        for i in 0..6 {
            let r: f64 = (0..6).map(|j| ju[i][j] * x[j]).sum();
            let rt: f64 = (0..6).map(|j| ju[j][i] * xt[j]).sum();
            assert!((r - y[i]).abs() < 1e-12 && (rt - y[i]).abs() < 1e-12);
        }
        let b = c.metric(&z);
        let v = [1.0, 2.0, -0.5, 0.3, 0.0, -1.2];
        let bv = c.apply_metric(&p, &v);
        for i in 0..6 {
            let want: f64 = (0..6).map(|j| b[i][j] * v[j]).sum();
            assert!((bv[i] - want).abs() < 1e-10 * (1.0 + want.abs()));
        }
    }

    #[test]
    fn roundtrip_and_measure() {
        let c = chart();
        let z = [0.4, -0.3, 0.2, -0.6, 0.1, 0.5];
        let g = c.gram(&z);
        let back = c.inverse(&g).unwrap();
        for i in 0..6 {
            assert!((back[i] - z[i]).abs() < 1e-10);
        }
        let j = c.jacobian(&z);
        let ld = logdet6(&j) - 0.5 * det3(&g).ln();
        assert!((c.log_measure(&z) - ld).abs() < 1e-10);
    }

    #[test]
    fn refuses_bad_geometry() {
        let c = chart();
        assert!(c.inverse(&[[-2.0, 0.0, 0.0], [0.0, -3.0, 0.0], [0.0, 0.0, -4.0]]).is_err());
        assert!(c.inverse(&[[0.0; 3]; 3]).is_err());
        assert!(c.inverse(&[[1.0, 2.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]).is_err());
    }
}
