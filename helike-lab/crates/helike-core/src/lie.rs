//! Real Lie algebras by structure constants, [X_a, X_b] = sum_c C[a,b,c] X_c.
//!
//! The same type serves as the frame algebra of the base group K (basis X_a, metric diagonal in it) and as the gauge
//! algebra (basis T_alpha, orthonormal for the gauge inner product). Constructors cover the cases in use and a generic
//! route from any basis of anti-Hermitian matrices.

use crate::linalg::{self, CMat, C64};
use faer::Mat;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LieAlgebra {
    pub name: String,
    pub dim: usize,
    /// Structure constants, row-major [a][b][c].
    pub c: Vec<f64>,
}

impl LieAlgebra {
    pub fn new(name: impl Into<String>, dim: usize, c: Vec<f64>) -> Self {
        assert_eq!(c.len(), dim * dim * dim, "structure constants must have dim^3 entries");
        Self { name: name.into(), dim, c }
    }

    #[inline]
    pub fn get(&self, a: usize, b: usize, k: usize) -> f64 {
        self.c[(a * self.dim + b) * self.dim + k]
    }

    /// [u, v] for coefficient vectors.
    pub fn bracket(&self, u: &[f64], v: &[f64]) -> Vec<f64> {
        let n = self.dim;
        let mut out = vec![0.0; n];
        for a in 0..n {
            if u[a] == 0.0 {
                continue;
            }
            for b in 0..n {
                if v[b] == 0.0 {
                    continue;
                }
                for k in 0..n {
                    out[k] += u[a] * v[b] * self.get(a, b, k);
                }
            }
        }
        out
    }

    /// Matrix of ad(v) = [v, .]: ad(v)[b, col] = sum_a v_a C[a, col, b]. (Same as `adm` in the Python builder.)
    pub fn ad(&self, v: &[f64]) -> Vec<f64> {
        let n = self.dim;
        let mut m = vec![0.0; n * n];
        for b in 0..n {
            for col in 0..n {
                let mut s = 0.0;
                for a in 0..n {
                    s += v[a] * self.get(a, col, b);
                }
                m[b * n + col] = s;
            }
        }
        m
    }

    /// Largest violation of antisymmetry C[a,b,.] = -C[b,a,.].
    pub fn antisymmetry_error(&self) -> f64 {
        let n = self.dim;
        let mut e = 0.0f64;
        for a in 0..n {
            for b in 0..n {
                for k in 0..n {
                    e = e.max((self.get(a, b, k) + self.get(b, a, k)).abs());
                }
            }
        }
        e
    }

    /// Largest violation of the Jacobi identity.
    pub fn jacobi_error(&self) -> f64 {
        let n = self.dim;
        let mut e = 0.0f64;
        for a in 0..n {
            for b in 0..n {
                for k in 0..n {
                    for m in 0..n {
                        let mut s = 0.0;
                        for l in 0..n {
                            s += self.get(a, b, l) * self.get(l, k, m)
                                + self.get(b, k, l) * self.get(l, a, m)
                                + self.get(k, a, l) * self.get(l, b, m);
                        }
                        e = e.max(s.abs());
                    }
                }
            }
        }
        e
    }

    /// Largest violation of ad-invariance of the standard inner product (C[a,b,c] totally antisymmetric). Holds for
    /// an orthonormal basis of a compact algebra; the gauge algebra must satisfy it.
    pub fn invariance_error(&self) -> f64 {
        let n = self.dim;
        let mut e = 0.0f64;
        for a in 0..n {
            for b in 0..n {
                for k in 0..n {
                    e = e.max((self.get(a, b, k) + self.get(a, k, b)).abs());
                }
            }
        }
        e
    }

    /// Structure constants from a basis of anti-Hermitian matrices, with the inner product <X, Y> = -kappa tr(XY)
    /// that makes the basis orthonormal (kappa = 1/(-tr X_0^2) is checked against every element). Errors if the
    /// basis is not closed under brackets or not orthonormal.
    pub fn from_matrices(name: impl Into<String>, basis: &[CMat]) -> Result<Self, String> {
        let n = basis.len();
        let tr = |m: &CMat| -> C64 { (0..m.nrows()).map(|i| m[(i, i)]).sum() };
        let k0 = -tr(&(&basis[0] * &basis[0])).re;
        if k0 <= 0.0 {
            return Err("first basis element has non-negative -tr(X^2)".into());
        }
        let kappa = 1.0 / k0;
        let ip = |a: &CMat, b: &CMat| -> f64 { -kappa * tr(&(a * b)).re };
        for a in 0..n {
            for b in 0..n {
                let want = if a == b { 1.0 } else { 0.0 };
                if (ip(&basis[a], &basis[b]) - want).abs() > 1e-12 {
                    return Err(format!("basis not orthonormal at ({a},{b})"));
                }
            }
        }
        let mut c = vec![0.0; n * n * n];
        for a in 0..n {
            for b in 0..n {
                let z = linalg::commutator(&basis[a], &basis[b]);
                let mut recon = linalg::zeros(z.nrows(), z.ncols());
                for k in 0..n {
                    let v = ip(&z, &basis[k]);
                    c[(a * n + b) * n + k] = v;
                    recon = recon + linalg::scale(&basis[k], linalg::c(v));
                }
                if linalg::max_abs_diff(&recon, &z) > 1e-11 {
                    return Err(format!("basis not closed under the bracket at ({a},{b})"));
                }
            }
        }
        Ok(Self::new(name, n, c))
    }

    /// su(2) as the frame algebra of S^3 = SU(2): [e_a, e_b] = 2 eps_abc e_c (the Floor and By Enough convention).
    pub fn su2_frame() -> Self {
        let mut l = Self::so3_cross();
        for v in l.c.iter_mut() {
            *v *= 2.0;
        }
        l.name = "su2_frame".into();
        l
    }

    /// su(2) = so(3) with the cross product, [tau_a, tau_b] = eps_abc tau_c (the gauge algebra of the family).
    pub fn so3_cross() -> Self {
        let mut c = vec![0.0; 27];
        for (i, j, k) in [(0, 1, 2), (1, 2, 0), (2, 0, 1)] {
            c[(i * 3 + j) * 3 + k] = 1.0;
            c[(j * 3 + i) * 3 + k] = -1.0;
        }
        Self::new("su2_cross", 3, c)
    }

    /// su(N) with X = -(i/2) lambda for the generalised Gell-Mann matrices (orthonormal for -2 tr XY). For N = 3 the
    /// basis order is the Gell-Mann order lambda_1..lambda_8, as in `ricci_su3_test2_family.py`.
    pub fn su_n(n: usize) -> Self {
        let basis = gell_mann_basis(n);
        Self::from_matrices(format!("su{n}"), &basis).expect("su(N) basis is orthonormal and closed")
    }

    /// The anti-Hermitian matrices X_a = -(i/2) lambda_a used for su(N).
    pub fn su_n_matrices(n: usize) -> Vec<CMat> {
        gell_mann_basis(n)
    }
}

/// X_a = -(i/2) lambda_a, generalised Gell-Mann matrices of su(n). For n = 3 the order matches lambda_1..lambda_8.
fn gell_mann_basis(n: usize) -> Vec<CMat> {
    let mut lam: Vec<CMat> = Vec::new();
    let unit = |r: usize, s: usize| -> CMat { Mat::from_fn(n, n, |i, j| if i == r && j == s { linalg::ONE } else { linalg::ZERO }) };
    if n == 3 {
        // Standard Gell-Mann order.
        let sym = |a: usize, b: usize| unit(a, b) + unit(b, a);
        let asym = |a: usize, b: usize| linalg::scale(&unit(b, a), linalg::I) - linalg::scale(&unit(a, b), linalg::I);
        lam.push(sym(0, 1));
        lam.push(asym(0, 1));
        lam.push(unit(0, 0) - unit(1, 1));
        lam.push(sym(0, 2));
        lam.push(asym(0, 2));
        lam.push(sym(1, 2));
        lam.push(asym(1, 2));
        let d = Mat::from_fn(3, 3, |i, j| if i == j { linalg::c(if i < 2 { 1.0 } else { -2.0 } / 3f64.sqrt()) } else { linalg::ZERO });
        lam.push(d);
    } else {
        for a in 0..n {
            for b in a + 1..n {
                lam.push(unit(a, b) + unit(b, a));
                lam.push(linalg::scale(&unit(b, a), linalg::I) - linalg::scale(&unit(a, b), linalg::I));
            }
        }
        for k in 1..n {
            let norm = (2.0 / (k * (k + 1)) as f64).sqrt();
            lam.push(Mat::from_fn(n, n, |i, j| {
                if i != j {
                    linalg::ZERO
                } else if i < k {
                    linalg::c(norm)
                } else if i == k {
                    linalg::c(-(k as f64) * norm)
                } else {
                    linalg::ZERO
                }
            }));
        }
    }
    lam.iter().map(|l| linalg::scale(l, C64::new(0.0, -0.5))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn su2_conventions() {
        let f = LieAlgebra::su2_frame();
        assert_eq!(f.get(0, 1, 2), 2.0);
        assert_eq!(f.get(1, 0, 2), -2.0);
        let g = LieAlgebra::so3_cross();
        assert_eq!(g.bracket(&[1., 0., 0.], &[0., 1., 0.]), vec![0., 0., 1.]);
        assert!(g.jacobi_error() < 1e-15 && g.invariance_error() < 1e-15);
    }

    #[test]
    fn su_n_is_a_compact_lie_algebra() {
        for n in 2..=4 {
            let l = LieAlgebra::su_n(n);
            assert_eq!(l.dim, n * n - 1);
            assert!(l.antisymmetry_error() < 1e-12, "su({n}) antisymmetry");
            assert!(l.jacobi_error() < 1e-12, "su({n}) jacobi");
            assert!(l.invariance_error() < 1e-12, "su({n}) invariance");
        }
        // su(3): f_123 = 1 in the Gell-Mann basis.
        assert!((LieAlgebra::su_n(3).get(0, 1, 2) - 1.0).abs() < 1e-14);
    }
}
