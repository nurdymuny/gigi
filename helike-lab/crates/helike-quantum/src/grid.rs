//! Separable transforms between Hermite coefficients on an occupation simplex and values on a 6-dimensional
//! Gauss–Hermite tensor grid, processed one (q0, q1) slab and one q2 sub-slab at a time, so no grid-sized array is
//! ever stored and the working set stays cache-sized.
//!
//! A coefficient vector c over states n = (n0..n5) has values f(q) = Σ_n c_n Π_k h_{n_k}(x_{q_k}). The forward
//! transform of a slab returns f on the Q⁴ points (q2..q5) for fixed (q0, q1); the backward transform is its exact
//! adjoint (Σ_q h_n(x_q) g(q)), accumulated into a coefficient vector. With pointwise weights between them this
//! applies ∫ φ_n φ_m F without ever forming the moment matrix.

use crate::basis::State;
use crate::hermite;

pub struct SlabGrid {
    /// one-dimensional coefficient range: n_k < nn
    pub nn: usize,
    /// quadrature order per axis
    pub q: usize,
    pub nodes: Vec<f64>,
    pub weights: Vec<f64>,
    /// e[n * q + j] = h_n(x_j)
    pub e: Vec<f64>,
    n0: Vec<usize>,
    n1: Vec<usize>,
    off4: Vec<usize>,
}

/// Per-slab scratch buffers, reused across slabs on one thread.
pub struct Scratch {
    s4: Vec<f64>,
    t1: Vec<f64>,
    t1o: Vec<f64>,
    a: Vec<f64>,
    b: Vec<f64>,
    v: Vec<f64>,
}

impl SlabGrid {
    pub fn new(states: &[State], nn: usize, q: usize) -> Self {
        let (nodes, weights) = hermite::gauss(q);
        let mut e = vec![0.0; nn * q];
        for (j, &x) in nodes.iter().enumerate() {
            let h = hermite::normalized(x, nn - 1);
            for n in 0..nn {
                e[n * q + j] = h[n];
            }
        }
        let mut n0 = vec![];
        let mut n1 = vec![];
        let mut off4 = vec![];
        for s in states {
            assert!(s.iter().all(|&v| (v as usize) < nn), "state outside the coefficient range");
            n0.push(s[0] as usize);
            n1.push(s[1] as usize);
            off4.push(((s[2] as usize * nn + s[3] as usize) * nn + s[4] as usize) * nn + s[5] as usize);
        }
        SlabGrid { nn, q, nodes, weights, e, n0, n1, off4 }
    }

    pub fn points_per_slab(&self) -> usize {
        self.q.pow(4)
    }

    /// Scratch for batches of `nc` channels (layout: channel innermost).
    pub fn scratch(&self, nc: usize) -> Scratch {
        let (nn, q) = (self.nn, self.q);
        Scratch {
            s4: vec![0.0; nn.pow(4) * nc],
            t1: vec![0.0; q * nn.pow(3) * nc],
            t1o: vec![0.0; q * nn.pow(3) * nc],
            a: vec![0.0; q * nn * nn * nc],
            b: vec![0.0; q * q * nn * nc],
            v: vec![0.0; q * q * q * nc],
        }
    }

    /// out[o][j][i] = Σ_n e[n][j] inp[o][n][i]
    fn axis_fwd(&self, inp: &[f64], outer: usize, inner: usize, out: &mut [f64]) {
        let (nn, q) = (self.nn, self.q);
        if inner == 1 {
            for o in 0..outer {
                let src = &inp[o * nn..(o + 1) * nn];
                let dst = &mut out[o * q..(o + 1) * q];
                dst.fill(0.0);
                for (n, &s) in src.iter().enumerate() {
                    if s != 0.0 {
                        let er = &self.e[n * q..(n + 1) * q];
                        for (d, &c) in dst.iter_mut().zip(er) {
                            *d += s * c;
                        }
                    }
                }
            }
            return;
        }
        out[..outer * q * inner].fill(0.0);
        for o in 0..outer {
            for n in 0..nn {
                let src = &inp[(o * nn + n) * inner..(o * nn + n + 1) * inner];
                for j in 0..q {
                    let c = self.e[n * q + j];
                    let dst = &mut out[(o * q + j) * inner..(o * q + j + 1) * inner];
                    for (d, s) in dst.iter_mut().zip(src) {
                        *d += c * s;
                    }
                }
            }
        }
    }

    /// out[o][n][i] = Σ_j e[n][j] inp[o][j][i]
    fn axis_bwd(&self, inp: &[f64], outer: usize, inner: usize, out: &mut [f64]) {
        let (nn, q) = (self.nn, self.q);
        if inner == 1 {
            for o in 0..outer {
                let src = &inp[o * q..(o + 1) * q];
                for n in 0..nn {
                    let er = &self.e[n * q..(n + 1) * q];
                    out[o * nn + n] = er.iter().zip(src).map(|(a, b)| a * b).sum();
                }
            }
            return;
        }
        out[..outer * nn * inner].fill(0.0);
        for o in 0..outer {
            for j in 0..q {
                let src = &inp[(o * q + j) * inner..(o * q + j + 1) * inner];
                for n in 0..nn {
                    let c = self.e[n * q + j];
                    let dst = &mut out[(o * nn + n) * inner..(o * nn + n + 1) * inner];
                    for (d, s) in dst.iter_mut().zip(src) {
                        *d += c * s;
                    }
                }
            }
        }
    }

    /// Process the slab (q0, q1) for a batch of coefficient vectors, one q2 sub-slab at a time so the working set
    /// stays cache-sized. For each q2, `f(q2, vals)` receives the values on the Q³ points (q3, q4, q5), layout
    /// `vals[((q3·Q + q4)·Q + q5) · nc + c]`, and may overwrite them in place. If `acc` is given, the adjoint
    /// transform of the overwritten values is accumulated into it: acc[c][k] += Σ_points h_{n(k)}(x_q) vals.
    pub fn process<F>(&self, coeffs: &[&[f64]], q0: usize, q1: usize, sc: &mut Scratch, mut acc: Option<&mut [Vec<f64>]>, mut f: F)
    where
        F: FnMut(usize, &mut [f64]),
    {
        let (nn, q) = (self.nn, self.q);
        let nc = coeffs.len();
        let Scratch { s4, t1, t1o, a, b, v } = sc;
        let n3 = nn * nn * nn * nc;
        let s4 = &mut s4[..nn * n3];
        s4.fill(0.0);
        for k in 0..self.off4.len() {
            let fct = self.e[self.n0[k] * q + q0] * self.e[self.n1[k] * q + q1];
            let base = self.off4[k] * nc;
            for (c, co) in coeffs.iter().enumerate() {
                s4[base + c] += fct * co[k];
            }
        }
        self.axis_fwd(s4, 1, n3, t1);
        for q2 in 0..q {
            let src = &t1[q2 * n3..(q2 + 1) * n3];
            self.axis_fwd(src, 1, nn * nn * nc, a);
            self.axis_fwd(a, q, nn * nc, b);
            self.axis_fwd(b, q * q, nc, v);
            f(q2, &mut v[..q * q * q * nc]);
            if acc.is_some() {
                self.axis_bwd(v, q * q, nc, b);
                self.axis_bwd(b, q, nn * nc, a);
                self.axis_bwd(a, 1, nn * nn * nc, &mut t1o[q2 * n3..(q2 + 1) * n3]);
            }
        }
        if let Some(acc) = acc.as_deref_mut() {
            self.axis_bwd(t1o, 1, n3, s4);
            for k in 0..self.off4.len() {
                let fct = self.e[self.n0[k] * q + q0] * self.e[self.n1[k] * q + q1];
                let base = self.off4[k] * nc;
                for (c, ac) in acc.iter_mut().enumerate() {
                    ac[k] += fct * s4[base + c];
                }
            }
        }
    }

    /// Decode a sub-slab point index into (q3, q4, q5).
    #[inline]
    pub fn decode3(&self, p: usize) -> [usize; 3] {
        let q = self.q;
        [p / (q * q), (p / q) % q, p % q]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basis::states_up_to;

    #[test]
    fn forward_backward_reproduce_identity_moments() {
        // Σ_q w(q) f_a(q) f_b(q) over the grid = δ_ab for a quadrature that integrates the products exactly
        let states = states_up_to(3);
        let g = SlabGrid::new(&states, 4, 6);
        let mut sc = g.scratch(1);
        let n = states.len();
        let mut gram = vec![0.0; n * n];
        for a in 0..n {
            let mut ca = vec![0.0; n];
            ca[a] = 1.0;
            let mut acc = vec![vec![0.0; n]];
            for q0 in 0..g.q {
                for q1 in 0..g.q {
                    let wts = g.weights.clone();
                    g.process(&[&ca[..]], q0, q1, &mut sc, Some(&mut acc), |q2, v| {
                        for p in 0..v.len() {
                            let [q3, q4, q5] = [p / 36, (p / 6) % 6, p % 6];
                            v[p] *= wts[q0] * wts[q1] * wts[q2] * wts[q3] * wts[q4] * wts[q5];
                        }
                    });
                }
            }
            for b in 0..n {
                gram[a * n + b] = acc[0][b];
            }
        }
        for a in 0..n {
            for b in 0..n {
                let want = if a == b { 1.0 } else { 0.0 };
                assert!((gram[a * n + b] - want).abs() < 1e-13, "{a},{b}: {}", gram[a * n + b]);
            }
        }
    }
}
