//! Exact moments by Kronecker factorisation, applied on the occupation simplex (no grid, no dense matrix).
//!
//! Every term of the Laurent model (`laurent`) has the form scale · exp(α·z) · q(u3, u4, u5) with q a polynomial of
//! degree ≤ d (= 4 here) in the off-diagonal chart coordinates, each of which is linear in z. On the Hermite basis,
//!     F[exp(α·z) q] = E(α) · q(U),
//! where U_l = u0_l + Σ_k S_lk Z_k uses the exact position operators Z_k (tridiagonal: z φ_n = (√((n+1)/2) φ_{n+1}
//! + √(n/2) φ_{n−1})/√w), and E(α) = ⊗_k E_k(α_k) with the exact one-dimensional moments
//!     E_k[n][m] = ⟨φ_n | exp(α_k z) | φ_m⟩
//! by completed-square Gauss–Hermite quadrature on shifted nodes (`one_dimensional_moments` of the Python code: the
//! integrand becomes a polynomial, so the rule is exact up to rounding). This is an identity, not a truncation:
//! q(U) raises the occupation of a vector on the extended space (total ≤ Mq + 1) by at most d, and E(α) is needed only
//! from total ≤ Mq + 1 + d to total ≤ Mq + 1, where all its entries are exact.
//!
//! E(α) is applied axis by axis. After k axes an intermediate is indexed by (output prefix n_0..n_{k−1} with
//! Σ ≤ N_out) × (input suffix m_k..m_5 with Σ ≤ N_in). The two constraints are independent, so every
//! intermediate is a small dense product set (at most a few hundred thousand entries at Mq = 8).

use crate::basis::{self, State};
use crate::chart::ConeChart;
use crate::hermite;
use crate::laurent;
use crate::model::Model;
use std::collections::{BTreeMap, HashMap};

/// One exponential group: scale · E(α) applied after the polynomial parts.
struct Group {
    scale: f64,
    /// E_k as (N_out + 1) × (N_in + 1), row-major
    e: [Vec<f64>; 6],
    /// kinetic polynomials C_ij of this group: (i, j, [(powers of u3, u4, u5), coefficient])
    kin: Vec<(usize, usize, Vec<([i32; 3], f64)>)>,
    /// potential polynomial of this group
    pot: Vec<([i32; 3], f64)>,
}

/// One axis step of the sum-factorised E application.
struct Stage {
    /// for each s in S_k: (first component m, index of the rest in S_{k+1})
    s_split: Vec<(usize, usize)>,
    /// for each p' in P_{k+1}: (index of the parent prefix in P_k, last component n)
    p_split: Vec<(usize, usize)>,
    n_s_next: usize,
}

pub struct Kron {
    pub n_out: usize,
    pub n_in: usize,
    /// all states with total ≤ N_in (their first entries are exactly the extended states, in the same order)
    n_big: usize,
    n_ext: usize,
    /// Z_k neighbours on the big simplex: up[k][i] = index of state + e_k (or usize::MAX), with √(n+1) factor
    up: [Vec<(usize, f64)>; 6],
    down: [Vec<(usize, f64)>; 6],
    inv_sqrt_w: [f64; 6],
    u0: [f64; 6],
    s: [[f64; 6]; 6],
    stages: Vec<Stage>,
    /// P_6 order -> extended-state index
    p6_to_ext: Vec<usize>,
    groups: Vec<Group>,
    pub n_groups: usize,
}

fn tuples(len: usize, max_sum: usize) -> Vec<State> {
    // all length-`len` tuples (stored left-aligned in a State) with sum ≤ max_sum, lexicographic
    fn rec(pos: usize, len: usize, left: usize, cur: &mut State, out: &mut Vec<State>) {
        if pos == len {
            out.push(*cur);
            return;
        }
        for v in 0..=left {
            cur[pos] = v as u8;
            rec(pos + 1, len, left - v, cur, out);
        }
        cur[pos] = 0;
    }
    let mut out = vec![];
    rec(0, len, max_sum, &mut [0u8; 6], &mut out);
    out
}

fn moments_1d(w: f64, alpha: f64, n_out: usize, n_in: usize) -> Result<Vec<f64>, String> {
    if !(alpha.is_finite() && w > 0.0) || alpha * alpha / (4.0 * w) > 600.0 {
        return Err("exponential moment would exceed the numerical contract".into());
    }
    let q = (n_out + n_in) / 2 + 1; // exact for polynomial degree ≤ 2q − 1 ≥ n_out + n_in
    let (x, wt) = hermite::gauss(q);
    let shift = alpha / (2.0 * w.sqrt());
    let pref = (alpha * alpha / (4.0 * w)).exp();
    let hs: Vec<Vec<f64>> = x.iter().map(|&xi| hermite::normalized(xi + shift, n_in.max(n_out))).collect();
    let mut e = vec![0.0; (n_out + 1) * (n_in + 1)];
    for n in 0..=n_out {
        for m in 0..=n_in {
            e[n * (n_in + 1) + m] = pref * (0..q).map(|j| wt[j] * hs[j][n] * hs[j][m]).sum::<f64>();
        }
    }
    Ok(e)
}

impl Kron {
    pub fn new(chart: &ConeChart, w: &[f64; 6], model: &Model, mq: usize, flat_metric: bool) -> Result<Self, String> {
        let metric = laurent::cometric(flat_metric);
        let pot = laurent::potential(model);
        let mut d = pot.abc_degree();
        for row in &metric {
            for p in row {
                d = d.max(p.abc_degree());
            }
        }
        let (n_out, n_in) = (mq + 1, mq + 1 + d);
        let big = basis::states_up_to(n_in);
        let n_ext = basis::dimension(n_out);
        let index = basis::index_of(&big);
        let mut up: [Vec<(usize, f64)>; 6] = Default::default();
        let mut down: [Vec<(usize, f64)>; 6] = Default::default();
        for k in 0..6 {
            for st in &big {
                let n = st[k] as usize;
                let mut a = *st;
                a[k] += 1;
                up[k].push((index.get(&a).copied().unwrap_or(usize::MAX), ((n + 1) as f64 / 2.0).sqrt()));
                let mut b = *st;
                if n > 0 {
                    b[k] -= 1;
                    down[k].push((index[&b], (n as f64 / 2.0).sqrt()));
                } else {
                    down[k].push((usize::MAX, 0.0));
                }
            }
        }
        // stages
        let mut prefixes: Vec<Vec<State>> = (0..=6).map(|k| tuples(k, n_out)).collect();
        let mut suffixes: Vec<Vec<State>> = (0..=6).map(|k| tuples(6 - k, n_in)).collect();
        suffixes[0] = big.clone(); // S_0 in the order of the trial vectors
        prefixes[0] = vec![[0u8; 6]];
        let pidx: Vec<HashMap<State, usize>> = prefixes.iter().map(|v| basis::index_of(v)).collect();
        let sidx: Vec<HashMap<State, usize>> = suffixes.iter().map(|v| basis::index_of(v)).collect();
        let mut stages = vec![];
        for k in 0..6 {
            let s_split = suffixes[k]
                .iter()
                .map(|s| {
                    let mut rest = [0u8; 6];
                    for i in 0..(5 - k) {
                        rest[i] = s[i + 1];
                    }
                    (s[0] as usize, sidx[k + 1][&rest])
                })
                .collect();
            let p_split = prefixes[k + 1]
                .iter()
                .map(|p| {
                    let mut parent = *p;
                    parent[k] = 0;
                    (pidx[k][&parent], p[k] as usize)
                })
                .collect();
            stages.push(Stage { s_split, p_split, n_s_next: suffixes[k + 1].len() });
        }
        let ext_index = basis::index_of(&big[..n_ext]);
        let p6_to_ext = prefixes[6].iter().map(|p| ext_index[p]).collect();
        // groups
        let mut kin_groups: BTreeMap<[i32; 3], Vec<(usize, usize, Vec<([i32; 3], f64)>)>> = BTreeMap::new();
        for i in 0..6 {
            for j in 0..6 {
                for (g, terms) in laurent::by_group(&metric[i][j]) {
                    kin_groups.entry(g).or_default().push((i, j, terms));
                }
            }
        }
        let pot_groups = laurent::by_group(&pot);
        let mut keys: Vec<[i32; 3]> = kin_groups.keys().chain(pot_groups.keys()).copied().collect();
        keys.sort();
        keys.dedup();
        let mut groups = vec![];
        for g in keys {
            let mut alpha = [0.0; 6];
            for ax in 0..6 {
                alpha[ax] = (0..3).map(|i| g[i] as f64 * chart.s[i][ax]).sum();
            }
            let scale = (0..3).map(|i| g[i] as f64 * chart.u0[i]).sum::<f64>().exp();
            let e = [0, 1, 2, 3, 4, 5].map(|ax| moments_1d(w[ax], alpha[ax], n_out, n_in));
            let e: [Vec<f64>; 6] = {
                let mut out: [Vec<f64>; 6] = Default::default();
                for (ax, r) in e.into_iter().enumerate() {
                    out[ax] = r?;
                }
                out
            };
            groups.push(Group {
                scale,
                e,
                kin: kin_groups.remove(&g).unwrap_or_default(),
                pot: pot_groups.get(&g).cloned().unwrap_or_default(),
            });
        }
        let n_groups = groups.len();
        Ok(Kron {
            n_out,
            n_in,
            n_big: big.len(),
            n_ext,
            up,
            down,
            inv_sqrt_w: w.map(|x| 1.0 / x.sqrt()),
            u0: chart.u0,
            s: chart.s,
            stages,
            p6_to_ext,
            groups,
            n_groups,
        })
    }

    /// U_l v = (u0_l + Σ_k S_lk Z_k) v on the big simplex (l = 3, 4, 5).
    fn apply_u(&self, l: usize, v: &[f64]) -> Vec<f64> {
        let mut out: Vec<f64> = v.iter().map(|x| self.u0[l] * x).collect();
        for k in 0..6 {
            let c = self.s[l][k] * self.inv_sqrt_w[k];
            if c == 0.0 {
                continue;
            }
            for (i, &x) in v.iter().enumerate() {
                if x == 0.0 {
                    continue;
                }
                let (iu, fu) = self.up[k][i];
                // the exactness argument needs every component to stay within total ≤ N_in: refuse, never drop
                assert!(iu != usize::MAX, "occupation left the exact window (total > N_in = {})", self.n_in);
                out[iu] += c * fu * x;
                let (id, fd) = self.down[k][i];
                if id != usize::MAX {
                    out[id] += c * fd * x;
                }
            }
        }
        out
    }

    /// q(U) v for q given by its monomials in (u3, u4, u5).
    fn apply_poly(&self, q: &[([i32; 3], f64)], v: &[f64], acc: &mut [f64]) {
        for (pw, c) in q {
            let mut t = v.to_vec();
            for (l, &p) in pw.iter().enumerate() {
                for _ in 0..p {
                    t = self.apply_u(3 + l, &t);
                }
            }
            for (a, b) in acc.iter_mut().zip(&t) {
                *a += c * b;
            }
        }
    }

    /// E(α) of a group: big simplex -> extended states.
    fn apply_e(&self, g: &Group, x: &[f64]) -> Vec<f64> {
        let ncol = self.n_in + 1;
        let mut t = x.to_vec(); // |P_0| = 1 row
        let mut n_s = x.len();
        for (k, st) in self.stages.iter().enumerate() {
            let e = &g.e[k];
            let np = st.p_split.len();
            let mut next = vec![0.0; np * st.n_s_next];
            for (pp, &(p, n)) in st.p_split.iter().enumerate() {
                let row = &e[n * ncol..(n + 1) * ncol];
                let src = &t[p * n_s..(p + 1) * n_s];
                let dst = &mut next[pp * st.n_s_next..(pp + 1) * st.n_s_next];
                for (s, &(m, rest)) in st.s_split.iter().enumerate() {
                    let v = src[s];
                    if v != 0.0 {
                        dst[rest] += row[m] * v;
                    }
                }
            }
            t = next;
            n_s = st.n_s_next;
        }
        let mut out = vec![0.0; self.n_ext];
        for (p, &ix) in self.p6_to_ext.iter().enumerate() {
            out[ix] = g.scale * t[p];
        }
        out
    }

    fn embed(&self, v: &[f64]) -> Vec<f64> {
        let mut b = vec![0.0; self.n_big];
        b[..v.len()].copy_from_slice(v);
        b
    }

    /// The seven accumulated channels (½ Σ_j F[C_ij] D_j v for i = 0..5, and F[V] v) for `ConeOperator::gather`.
    /// `chans` are the seven channels of `ConeOperator::channels` (D_0 v .. D_5 v, v) on the extended space.
    pub fn channels_out(&self, chans: &[Vec<f64>], kinetic: bool, potential: bool, energy_shift: f64) -> Vec<Vec<f64>> {
        let mut out = vec![vec![0.0; self.n_ext]; 7];
        let big: Vec<Vec<f64>> = chans.iter().map(|c| self.embed(c)).collect();
        for g in &self.groups {
            if kinetic {
                for i in 0..6 {
                    let mut acc = vec![0.0; self.n_big];
                    let mut any = false;
                    for (ii, j, q) in &g.kin {
                        if *ii == i {
                            self.apply_poly(q, &big[*j], &mut acc);
                            any = true;
                        }
                    }
                    if any {
                        for (a, b) in out[i].iter_mut().zip(self.apply_e(g, &acc)) {
                            *a += 0.5 * b;
                        }
                    }
                }
            }
            if potential && !g.pot.is_empty() {
                let mut acc = vec![0.0; self.n_big];
                self.apply_poly(&g.pot, &big[6], &mut acc);
                for (a, b) in out[6].iter_mut().zip(self.apply_e(g, &acc)) {
                    *a += b;
                }
            }
        }
        if potential {
            for (a, b) in out[6].iter_mut().zip(&chans[6]) {
                *a -= energy_shift * b;
            }
        }
        out
    }

    /// F[1] v on the extended space (must reproduce v: the mass check).
    pub fn mass(&self, v_ext: &[f64]) -> Vec<f64> {
        let g = Group {
            scale: 1.0,
            e: std::array::from_fn(|ax| moments_1d(1.0 / (self.inv_sqrt_w[ax] * self.inv_sqrt_w[ax]), 0.0, self.n_out, self.n_in).unwrap()),
            kin: vec![],
            pot: vec![],
        };
        self.apply_e(&g, &self.embed(v_ext))
    }
}

