//! The Yang-Mills Hessian H and the gauge map G at a left-invariant field, in one Peter-Weyl sector.
//!
//! Setting (as in `hessian_and_gauge` of `scripts/ricci_su3_test2_family.py`): a compact group K with frame algebra
//! basis X_a ([X_a, X_b] = C_ab^c X_c) and left-invariant metric diagonal in it (g_a); a gauge algebra with orthonormal
//! basis T_alpha ([T_b, T_c] = f_bc^a T_a); a background A = sum_i M_i theta^i in the orthonormal coframe; a sector
//! given by a representation rho of the frame algebra. Fields b = sum_i b_i theta^i, b_i in (gauge) (x) V, are indexed
//! (frame i, gauge alpha, V w) with w fastest.
//!
//!   Q(b) = sum_{i<j} |D_ij b|^2 + 2 <F_ij, [b_i, b_j]>,  D_ij b = (e_i + ad M_i) b_j - (e_j + ad M_j) b_i - c_ij^k b_k,
//!   F_ij = [M_i, M_j] - c_ij^k M_k,  G xi = ((e_i + ad M_i) xi)_i,
//!
//! with c the structure constants of the orthonormal frame, e_i acting as rho(X_i)/sqrt(g_i). H is the operator of Q
//! against the L^2 mass; the gauge-orthogonal spectrum is that of H on ker G^*.

use crate::lie::LieAlgebra;
use crate::linalg::{self, CMat};
use crate::rep::Rep;
use serde::{Deserialize, Serialize};

/// A left-invariant metric on K, diagonal in the frame basis.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Geometry {
    pub frame: LieAlgebra,
    pub metric: Vec<f64>,
}

impl Geometry {
    pub fn new(frame: LieAlgebra, metric: Vec<f64>) -> Self {
        assert_eq!(frame.dim, metric.len());
        assert!(metric.iter().all(|&g| g > 0.0), "metric must be positive");
        Self { frame, metric }
    }

    pub fn n(&self) -> usize {
        self.frame.dim
    }

    pub fn sqrt_metric(&self) -> Vec<f64> {
        self.metric.iter().map(|g| g.sqrt()).collect()
    }

    /// Structure constants in the orthonormal frame: c_ij^k = C_ij^k s_k / (s_i s_j), s = sqrt(g).
    pub fn orthonormal_c(&self) -> Vec<f64> {
        let n = self.n();
        let s = self.sqrt_metric();
        let mut c = vec![0.0; n * n * n];
        for i in 0..n {
            for j in 0..n {
                for k in 0..n {
                    c[(i * n + j) * n + k] = self.frame.get(i, j, k) * s[k] / (s[i] * s[j]);
                }
            }
        }
        c
    }

    /// Levi-Civita connection matrices of the orthonormal frame (Koszul): Gamma_i[k][j] = (c_ijk - c_jki + c_kij)/2,
    /// i.e. nabla_{e_i} e_j = sum_k Gamma_i[k][j] e_k. Antisymmetric in (k, j).
    pub fn levi_civita(&self) -> Vec<Vec<f64>> {
        let n = self.n();
        let c = self.orthonormal_c();
        let cc = |i: usize, j: usize, k: usize| c[(i * n + j) * n + k];
        (0..n)
            .map(|i| {
                let mut g = vec![0.0; n * n];
                for k in 0..n {
                    for j in 0..n {
                        g[k * n + j] = 0.5 * (cc(i, j, k) - cc(j, k, i) + cc(k, i, j));
                    }
                }
                g
            })
            .collect()
    }

    /// Ricci tensor in the orthonormal frame, from the Levi-Civita connection (dense, all n).
    pub fn ricci(&self) -> Vec<f64> {
        let n = self.n();
        let c = self.orthonormal_c();
        let gam = self.levi_civita();
        let g = |i: usize, k: usize, j: usize| gam[i][k * n + j];
        let cc = |i: usize, j: usize, k: usize| c[(i * n + j) * n + k];
        // R(e_a, e_b) e_c = nabla_a nabla_b e_c - nabla_b nabla_a e_c - nabla_[a,b] e_c (left-invariant, constant Gamma).
        let mut ric = vec![0.0; n * n];
        for b in 0..n {
            for cidx in 0..n {
                let mut s = 0.0;
                for a in 0..n {
                    // <R(e_a, e_b) e_c, e_a>
                    let mut r = 0.0;
                    for m in 0..n {
                        r += g(a, a, m) * g(b, m, cidx) - g(b, a, m) * g(a, m, cidx);
                        r -= cc(a, b, m) * g(m, a, cidx);
                    }
                    s += r;
                }
                ric[b * n + cidx] = s;
            }
        }
        ric
    }
}

/// A left-invariant gauge field: M_i (i over the orthonormal coframe) as coefficient vectors in the gauge algebra.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Field {
    pub gauge: LieAlgebra,
    pub m: Vec<Vec<f64>>,
}

impl Field {
    pub fn new(gauge: LieAlgebra, m: Vec<Vec<f64>>) -> Self {
        assert!(m.iter().all(|v| v.len() == gauge.dim));
        Self { gauge, m }
    }

    pub fn r(&self) -> usize {
        self.gauge.dim
    }

    /// F_ij = [M_i, M_j] - c_ij^k M_k for all ordered pairs (row-major n x n of gauge vectors).
    pub fn curvature(&self, geo: &Geometry) -> Vec<Vec<f64>> {
        let n = geo.n();
        let r = self.r();
        let c = geo.orthonormal_c();
        let mut out = vec![vec![0.0; r]; n * n];
        for i in 0..n {
            for j in 0..n {
                let mut f = self.gauge.bracket(&self.m[i], &self.m[j]);
                for k in 0..n {
                    let cijk = c[(i * n + j) * n + k];
                    if cijk != 0.0 {
                        for a in 0..r {
                            f[a] -= cijk * self.m[k][a];
                        }
                    }
                }
                out[i * n + j] = f;
            }
        }
        out
    }

    /// Yang-Mills energy density (1/2) sum_{i<j} |F_ij|^2.
    pub fn energy(&self, geo: &Geometry) -> f64 {
        let n = geo.n();
        let f = self.curvature(geo);
        let mut e = 0.0;
        for i in 0..n {
            for j in i + 1..n {
                e += 0.5 * f[i * n + j].iter().map(|v| v * v).sum::<f64>();
            }
        }
        e
    }
}

/// Deliberate faults for mutation checks: each must make a parity test fail.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mutation {
    None,
    /// Flip the sign of the field-strength coupling 2<F, [b, b]>.
    FlipFlux,
    /// Drop the structure-constant term -c_ij^k b_k from D_ij.
    DropFrameBracket,
    /// Ignore the metric (treat it as flat, g = 1) in e_i and c.
    FlatMetric,
    /// Put the (i, j) flux block untransposed into the (j, i) slot. Since P^T = -P (f antisymmetric) this flips the
    /// sign of the lower block. (Replacing the transpose by -P instead would be a no-op for the same reason, which an
    /// earlier version of this mutation did; the parity test caught that it caught nothing.)
    FluxLowerUntransposed,
}

/// One sector problem: geometry, field and representation.
#[derive(Clone, Debug)]
pub struct Sector<'a> {
    pub geo: &'a Geometry,
    pub field: &'a Field,
    pub rep: &'a Rep,
}

/// H (Hermitian, N x N with N = n r d) and G (N x r d).
pub struct HessianGauge {
    pub h: CMat,
    pub g: CMat,
    pub n: usize,
    pub r: usize,
    pub d: usize,
}

impl<'a> Sector<'a> {
    pub fn new(geo: &'a Geometry, field: &'a Field, rep: &'a Rep) -> Self {
        assert_eq!(field.m.len(), geo.n(), "field needs one gauge vector per frame direction");
        assert_eq!(rep.mats.len(), geo.n(), "representation needs one matrix per frame direction");
        Self { geo, field, rep }
    }

    pub fn dims(&self) -> (usize, usize, usize) {
        (self.geo.n(), self.field.r(), self.rep.dim())
    }

    /// (e_i + ad M_i) on gauge (x) V, for each i.
    fn covariant_parts(&self, s: &[f64]) -> Vec<CMat> {
        let (n, r, d) = self.dims();
        let (ir, id) = (linalg::eye(r), linalg::eye(d));
        (0..n)
            .map(|i| {
                let deriv = linalg::kron(&ir, &linalg::scale(&self.rep.mats[i], linalg::c(1.0 / s[i])));
                let ad = linalg::kron(&linalg::from_real(r, r, &self.field.gauge.ad(&self.field.m[i])), &id);
                deriv + ad
            })
            .collect()
    }

    pub fn hessian_and_gauge(&self) -> HessianGauge {
        self.hessian_and_gauge_mutated(Mutation::None)
    }

    #[doc(hidden)]
    pub fn hessian_and_gauge_mutated(&self, mutation: Mutation) -> HessianGauge {
        let (n, r, d) = self.dims();
        let geo_flat;
        let geo = if mutation == Mutation::FlatMetric {
            geo_flat = Geometry::new(self.geo.frame.clone(), vec![1.0; n]);
            &geo_flat
        } else {
            self.geo
        };
        let s = geo.sqrt_metric();
        let c = geo.orthonormal_c();
        let rd = r * d;
        let nn = n * rd;
        let parts = Sector { geo, field: self.field, rep: self.rep }.covariant_parts(&s);
        let mut h = linalg::zeros(nn, nn);
        let eye_rd = linalg::eye(rd);
        for i in 0..n {
            for j in i + 1..n {
                // D_ij : N -> rd
                let mut dmat = linalg::zeros(rd, nn);
                add_block(&mut dmat, 0, j * rd, &parts[i], 1.0);
                add_block(&mut dmat, 0, i * rd, &parts[j], -1.0);
                if mutation != Mutation::DropFrameBracket {
                    for k in 0..n {
                        let cijk = c[(i * n + j) * n + k];
                        if cijk != 0.0 {
                            add_block(&mut dmat, 0, k * rd, &eye_rd, -cijk);
                        }
                    }
                }
                h = h + linalg::adjoint(&dmat) * &dmat;
            }
        }
        // flux coupling: x^T P y = <F_ij, [x, y]>, P[b, c] = sum_a F_a f[b, c, a]
        let id = linalg::eye(d);
        let fsign = if mutation == Mutation::FlipFlux { -1.0 } else { 1.0 };
        let f = Field::new(self.field.gauge.clone(), self.field.m.clone()).curvature(geo);
        for i in 0..n {
            for j in i + 1..n {
                let fij = &f[i * n + j];
                if fij.iter().all(|v| *v == 0.0) {
                    continue;
                }
                let mut p = vec![0.0; r * r];
                for b in 0..r {
                    for cc in 0..r {
                        let mut v = 0.0;
                        for a in 0..r {
                            v += fij[a] * self.field.gauge.get(b, cc, a);
                        }
                        p[b * r + cc] = fsign * v;
                    }
                }
                let blk = linalg::kron(&linalg::from_real(r, r, &p), &id);
                let blk_t = if mutation == Mutation::FluxLowerUntransposed { blk.clone() } else { linalg::transpose(&blk) };
                add_block(&mut h, i * rd, j * rd, &blk, 1.0);
                add_block(&mut h, j * rd, i * rd, &blk_t, 1.0);
            }
        }
        let mut g = linalg::zeros(nn, rd);
        for i in 0..n {
            add_block(&mut g, i * rd, 0, &parts[i], 1.0);
        }
        HessianGauge { h: linalg::hermitize(&h), g, n, r, d }
    }

    /// curl_A in three dimensions: (curl b)_k = D_ij b for (i, j, k) cyclic (the Hessian of Chern-Simons at A).
    /// Defined for n = 3 only.
    pub fn curl(&self) -> Option<CMat> {
        let (n, r, d) = self.dims();
        if n != 3 {
            return None;
        }
        let s = self.geo.sqrt_metric();
        let c = self.geo.orthonormal_c();
        let rd = r * d;
        let parts = self.covariant_parts(&s);
        let eye_rd = linalg::eye(rd);
        let mut out = linalg::zeros(3 * rd, 3 * rd);
        for ((i, j), k) in [((1, 2), 0), ((2, 0), 1), ((0, 1), 2)] {
            add_block(&mut out, k * rd, j * rd, &parts[i], 1.0);
            add_block(&mut out, k * rd, i * rd, &parts[j], -1.0);
            for m in 0..3 {
                let cijm = c[(i * 3 + j) * 3 + m];
                if cijm != 0.0 {
                    add_block(&mut out, k * rd, m * rd, &eye_rd, -cijm);
                }
            }
        }
        Some(out)
    }
}

impl HessianGauge {
    pub fn dim(&self) -> usize {
        self.h.nrows()
    }

    /// Full spectrum of H.
    pub fn spectrum(&self) -> Vec<f64> {
        linalg::eigvalsh(&self.h)
    }

    /// H + G G^*.
    pub fn h_plus_ggstar(&self) -> CMat {
        linalg::hermitize(&(&self.h + &self.g * linalg::adjoint(&self.g)))
    }

    /// Spectrum of H restricted to the gauge slice ker G^* (rank of G by sv > 1e-10 max sv).
    pub fn slice_spectrum(&self) -> Vec<f64> {
        let (k, _) = linalg::cokernel(&self.g, 1e-10);
        linalg::compressed_eigvalsh(&self.h, &k)
    }
}

/// target[r0.., c0..] += s * block
pub(crate) fn add_block(target: &mut CMat, r0: usize, c0: usize, block: &CMat, s: f64) {
    for j in 0..block.ncols() {
        for i in 0..block.nrows() {
            target[(r0 + i, c0 + j)] += block[(i, j)] * s;
        }
    }
}
