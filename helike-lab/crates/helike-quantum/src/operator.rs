//! The positive-cone Ritz operator, applied matrix-free (port of `ConeRitz.__init__`).
//!
//! Trial functions are products of normalized Hermite functions in the chart coordinates z with frequencies w,
//! total quanta ≤ Mq. In that basis
//!     H = ½ Σ_ij D_iᵀ F[B_ij] D_j + F[V − E_shift],
//! where D_i = Σ_j (S⁻¹)_ji ∂_{z_j} − ½ m_i (m = MEASURE_POWERS) maps the trial space into one more occupation shell,
//! B is the chart cometric, V the sourced potential, and F[f] the Gaussian moment matrix of f. The Python code forms
//! F term by term from an exact Laurent expansion (dense, 5,005 × 5,005 per term at Mq = 8). Here F[f] is never
//! formed: f is evaluated pointwise on a Gauss–Hermite grid, one (q0, q1) slab at a time, between a separable
//! forward transform and its adjoint. The functions involved are entire (exponentials of linear forms times
//! polynomials), so the quadrature converges super-exponentially in the order; every run can be repeated at a
//! second order and the receipts record both.

use crate::basis::{self, State};
use crate::chart::{ConeChart, Point, MEASURE_POWERS};
use crate::grid::SlabGrid;
use crate::kron::Kron;
use crate::model::*;
use rayon::prelude::*;
use serde::Serialize;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Part {
    /// T + V − E_shift
    Full,
    /// ½ Σ D_iᵀ F[B_ij] D_j only
    Kinetic,
    /// F[1] on the trial space (must be the identity)
    Mass,
}

/// Deliberate faults for mutation checks; each must break a parity or certificate test.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Mutation {
    None,
    /// drop the −½ m_i measure drift from D_i
    DropMeasureDrift,
    /// flip the sign of the cubic (flux) coupling
    FlipCubic,
    /// replace the chart cometric B by the identity
    FlatMetric,
}

pub struct ConeOperator {
    pub bg: Background,
    pub chart: ConeChart,
    pub mq: usize,
    pub basis: Vec<State>,
    pub ext: Vec<State>,
    pub energy_shift: f64,
    pub grid: SlabGrid,
    pub mutation: Mutation,
    model: Model,
    /// z_k at node j: zq[k][j] = x_j / √w_k
    zq: [Vec<f64>; 6],
    /// Dz_j as (row in ext, col in basis, value)
    dz: [Vec<(usize, usize, f64)>; 6],
    /// exact Kronecker moments (`ConeOperator::exact`); None means the Gauss–Hermite grid operator
    pub kron: Option<Kron>,
}

impl ConeOperator {
    /// `extra` sets the quadrature order Q = (Mq + 2) + extra per axis (the moment integrands are polynomials of
    /// degree ≤ 2(Mq + 1) times entire functions).
    pub fn new(bg: Background, mq: usize, extra: usize, mutation: Mutation) -> Result<Self, String> {
        if bg.w.iter().any(|&w| !(w > 0.0 && w.is_finite())) {
            return Err("background has no stable harmonic normal frame".into());
        }
        let chart = ConeChart::new(&bg.g0, &bg.m)?;
        let basis = basis::states_up_to(mq);
        let ext = basis::states_up_to(mq + 1);
        let index = basis::index_of(&ext);
        let mut dz: [Vec<(usize, usize, f64)>; 6] = Default::default();
        for j in 0..6 {
            for (col, n) in basis.iter().enumerate() {
                for sign in [-1i32, 1] {
                    let o = n[j] as i32 + sign;
                    if o < 0 {
                        continue;
                    }
                    let mut st = *n;
                    st[j] = o as u8;
                    let up = if sign > 0 { 1.0 } else { 0.0 };
                    let v = -(sign as f64) * (bg.w[j] * (n[j] as f64 + up) / 2.0).sqrt();
                    dz[j].push((index[&st], col, v));
                }
            }
        }
        let nn = mq + 2;
        let grid = SlabGrid::new(&ext, nn, nn + extra);
        let zq = std::array::from_fn(|k| grid.nodes.iter().map(|x| x / bg.w[k].sqrt()).collect());
        let mut model = bg.model.clone();
        if mutation == Mutation::FlipCubic {
            model.g = -model.g;
        }
        let energy_shift = model.potential(&chart.factor(&[0.0; 6]));
        Ok(ConeOperator { bg, chart, mq, basis, ext, energy_shift, grid, mutation, model, zq, dz, kron: None })
    }

    /// The same operator with exact moments: Laurent terms applied as E(α)·q(U) on the occupation simplex
    /// (`kron`). No quadrature error; the grid operator of `new` remains available as an independent cross-check.
    pub fn exact(bg: Background, mq: usize, mutation: Mutation) -> Result<Self, String> {
        let mut op = Self::new(bg, mq, 0, mutation)?;
        op.kron = Some(Kron::new(&op.chart, &op.bg.w, &op.model, mq, mutation == Mutation::FlatMetric)?);
        Ok(op)
    }

    /// "exact" (Kronecker moments) or "quadrature" (Gauss–Hermite grid).
    pub fn moments(&self) -> &'static str {
        if self.kron.is_some() { "exact" } else { "quadrature" }
    }

    pub fn dim(&self) -> usize {
        self.basis.len()
    }

    /// Channels on the extended space for one trial vector: D_0 v .. D_5 v, and E v.
    pub(crate) fn channels(&self, v: &[f64]) -> Vec<Vec<f64>> {
        let ne = self.ext.len();
        let mut dzv = vec![vec![0.0; ne]; 6];
        for j in 0..6 {
            for &(r, c, val) in &self.dz[j] {
                dzv[j][r] += val * v[c];
            }
        }
        let drift = if self.mutation == Mutation::DropMeasureDrift { 0.0 } else { 0.5 };
        let mut out = Vec::with_capacity(7);
        for i in 0..6 {
            let mut u = vec![0.0; ne];
            for j in 0..6 {
                let c = self.chart.inv_s[j][i];
                for (a, b) in u.iter_mut().zip(&dzv[j]) {
                    *a += c * b;
                }
            }
            for k in 0..v.len() {
                u[k] -= drift * MEASURE_POWERS[i] * v[k];
            }
            out.push(u);
        }
        let mut ev = vec![0.0; ne];
        ev[..v.len()].copy_from_slice(v);
        out.push(ev);
        out
    }

    /// Adjoint of `channels`, applied to the 7 accumulated channels of one vector.
    pub(crate) fn gather(&self, r: &[Vec<f64>]) -> Vec<f64> {
        let n = self.dim();
        let drift = if self.mutation == Mutation::DropMeasureDrift { 0.0 } else { 0.5 };
        let mut out = vec![0.0; n];
        for j in 0..6 {
            let ne = self.ext.len();
            let mut tmp = vec![0.0; ne];
            for i in 0..6 {
                let c = self.chart.inv_s[j][i];
                for (a, b) in tmp.iter_mut().zip(&r[i]) {
                    *a += c * b;
                }
            }
            for &(row, col, val) in &self.dz[j] {
                out[col] += val * tmp[row];
            }
        }
        for k in 0..n {
            let mut s = r[6][k];
            for i in 0..6 {
                s -= drift * MEASURE_POWERS[i] * r[i][k];
            }
            out[k] += s;
        }
        out
    }

    /// Geometry and weight at grid point (q0..q5).
    #[inline]
    fn point(&self, q: [usize; 6]) -> (Point, f64, [f64; 6]) {
        let mut z = [0.0; 6];
        let mut w = 1.0;
        for k in 0..6 {
            z[k] = self.zq[k][q[k]];
            w *= self.grid.weights[q[k]];
        }
        let u = self.chart.coordinates(&z);
        (self.chart.point_from_u(&u), w, z)
    }

    /// Apply H (or a part of it) to several trial vectors at once.
    pub fn apply(&self, vs: &[Vec<f64>], part: Part) -> Vec<Vec<f64>> {
        if let Some(k) = &self.kron {
            return vs
                .par_iter()
                .map(|v| {
                    let ch = self.channels(v);
                    match part {
                        Part::Mass => k.mass(&ch[6])[..self.dim()].to_vec(),
                        Part::Kinetic => self.gather(&k.channels_out(&ch, true, false, 0.0)),
                        Part::Full => self.gather(&k.channels_out(&ch, true, true, self.energy_shift)),
                    }
                })
                .collect();
        }
        let nb = vs.len();
        let ne = self.ext.len();
        let chans: Vec<Vec<f64>> = vs.iter().flat_map(|v| self.channels(v)).collect();
        let nc = chans.len();
        let q = self.grid.q;
        let slabs: Vec<(usize, usize)> = (0..q).flat_map(|a| (0..q).map(move |b| (a, b))).collect();
        let refs: Vec<&[f64]> = chans.iter().map(|c| &c[..]).collect();
        let acc = slabs
            .par_iter()
            .fold(
                || (vec![vec![0.0; ne]; nc], self.grid.scratch(nc), ()),
                |(mut acc, mut sc, u), &(q0, q1)| {
                    self.grid.process(&refs, q0, q1, &mut sc, Some(&mut acc), |q2, vals| {
                        for p in 0..vals.len() / nc {
                            let [q3, q4, q5] = self.grid.decode3(p);
                            let (pt, w, _) = self.point([q0, q1, q2, q3, q4, q5]);
                            let pot = match part {
                                Part::Full => self.model.potential(&pt.l) - self.energy_shift,
                                Part::Mass => 1.0,
                                Part::Kinetic => 0.0,
                            };
                            let amat = self.chart.cometric_a(&pt);
                            let row = &mut vals[p * nc..(p + 1) * nc];
                            for bk in 0..nb {
                                let ch = &mut row[7 * bk..7 * bk + 7];
                                if part == Part::Mass {
                                    ch[..6].fill(0.0);
                                } else {
                                    let uv = [ch[0], ch[1], ch[2], ch[3], ch[4], ch[5]];
                                    let bu = if self.mutation == Mutation::FlatMetric { uv } else { self.chart.apply_cometric_u_with(&pt, &amat, &uv) };
                                    for i in 0..6 {
                                        ch[i] = 0.5 * w * bu[i];
                                    }
                                }
                                ch[6] *= w * pot;
                            }
                        }
                    });
                    (acc, sc, u)
                },
            )
            .map(|(acc, _, _)| acc)
            .reduce(
                || vec![vec![0.0; ne]; nc],
                |mut a, b| {
                    for (x, y) in a.iter_mut().zip(&b) {
                        for (p, r) in x.iter_mut().zip(y) {
                            *p += r;
                        }
                    }
                    a
                },
            );
        (0..nb)
            .map(|b| {
                if part == Part::Mass {
                    acc[7 * b + 6][..self.dim()].to_vec()
                } else {
                    self.gather(&acc[7 * b..7 * b + 7])
                }
            })
            .collect()
    }

    /// Dense matrix of a part, column by column (for checks at small Mq). Row-major N×N.
    pub fn dense(&self, part: Part) -> Vec<f64> {
        let n = self.dim();
        let mut out = vec![0.0; n * n];
        let block = 32;
        for start in (0..n).step_by(block) {
            let stop = (start + block).min(n);
            let cols: Vec<Vec<f64>> = (start..stop)
                .map(|c| {
                    let mut e = vec![0.0; n];
                    e[c] = 1.0;
                    e
                })
                .collect();
            let hc = self.apply(&cols, part);
            for (k, col) in hc.iter().enumerate() {
                for r in 0..n {
                    out[r * n + start + k] = col[r];
                }
            }
        }
        out
    }
}
