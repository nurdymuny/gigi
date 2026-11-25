//! Exact Laurent expansions of the chart cometric and the sourced potential (port of `laurent_model`).
//!
//! In the Cholesky entries (r0, r1, r2, a, b, c) of L, with r_i = exp(u_i) and (a, b, c) = (u3, u4, u5):
//! - the u-cometric C = J_u⁻¹ A(G) J_u⁻ᵀ is a Laurent polynomial in r and a polynomial in (a, b, c). It is built here
//!   by running the closed-form triangular solves `solve_ju` / `solve_ju_t` on polynomials: their only divisions are
//!   by r_i and 2 r_i², i.e. by monomials, so the result is exact (coefficients are dyadic rationals);
//! - the potential V(L) is a polynomial, built directly from `Model` (so the model's mutations carry over).
//!
//! Each term c · r^k · a^p3 b^p4 c^p5 becomes, with u = u0 + S z,
//!     c · exp(k·u0[0..3]) · exp(α·z) · Π_{l≥3} (u0_l + S_l·z)^{p_l},   α = Σ_{i<3} k_i S_i.
//! Terms are grouped by k (the exponential); the polynomial part is applied exactly with Hermite position operators.

use crate::model::{a_coeffs, Model};
use std::collections::BTreeMap;

/// Exponents of (r0, r1, r2, a, b, c); r exponents may be negative.
pub type Key = [i32; 6];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Poly(pub BTreeMap<Key, f64>);

impl Poly {
    pub fn zero() -> Self {
        Poly(BTreeMap::new())
    }
    pub fn constant(c: f64) -> Self {
        Self::mono(c, [0; 6])
    }
    pub fn mono(c: f64, k: Key) -> Self {
        let mut m = BTreeMap::new();
        if c != 0.0 {
            m.insert(k, c);
        }
        Poly(m)
    }
    /// The variable with index v (0..2: r_v, 3..5: a, b, c).
    pub fn var(v: usize) -> Self {
        let mut k = [0; 6];
        k[v] = 1;
        Self::mono(1.0, k)
    }
    fn push(&mut self, k: Key, c: f64) {
        let e = self.0.entry(k).or_insert(0.0);
        *e += c;
        if *e == 0.0 {
            self.0.remove(&k);
        }
    }
    pub fn add(&self, o: &Poly) -> Poly {
        let mut r = self.clone();
        for (k, c) in &o.0 {
            r.push(*k, *c);
        }
        r
    }
    pub fn sub(&self, o: &Poly) -> Poly {
        self.add(&o.scale(-1.0))
    }
    pub fn scale(&self, s: f64) -> Poly {
        if s == 0.0 {
            return Poly::zero();
        }
        Poly(self.0.iter().map(|(k, c)| (*k, c * s)).collect())
    }
    pub fn mul(&self, o: &Poly) -> Poly {
        let mut r = Poly::zero();
        for (ka, ca) in &self.0 {
            for (kb, cb) in &o.0 {
                let k: Key = std::array::from_fn(|i| ka[i] + kb[i]);
                r.push(k, ca * cb);
            }
        }
        r
    }
    /// Multiply by r_v^e (e may be negative).
    pub fn shift_r(&self, v: usize, e: i32) -> Poly {
        Poly(
            self.0
                .iter()
                .map(|(k, c)| {
                    let mut k = *k;
                    k[v] += e;
                    (k, *c)
                })
                .collect(),
        )
    }
    /// Evaluate at (r0, r1, r2, a, b, c).
    pub fn eval(&self, x: &[f64; 6]) -> f64 {
        self.0
            .iter()
            .map(|(k, c)| c * (0..6).map(|i| x[i].powi(k[i])).product::<f64>())
            .sum()
    }
    /// Maximum total degree in (a, b, c).
    pub fn abc_degree(&self) -> usize {
        self.0.keys().map(|k| (k[3] + k[4] + k[5]) as usize).max().unwrap_or(0)
    }
}

/// Gram entries x = (G00, G11, G22, G01, G02, G12) of L Lᵀ as polynomials.
fn gram_polys() -> [Poly; 6] {
    let (r0, r1, r2) = (Poly::var(0), Poly::var(1), Poly::var(2));
    let (a, b, c) = (Poly::var(3), Poly::var(4), Poly::var(5));
    [
        r0.mul(&r0),
        a.mul(&a).add(&r1.mul(&r1)),
        b.mul(&b).add(&c.mul(&c)).add(&r2.mul(&r2)),
        r0.mul(&a),
        r0.mul(&b),
        a.mul(&b).add(&r1.mul(&c)),
    ]
}

/// `chart::solve_ju` on polynomials: J_u x = y.
fn solve_ju(y: &[Poly; 6]) -> [Poly; 6] {
    let (a, b, c) = (Poly::var(3), Poly::var(4), Poly::var(5));
    let mut x: [Poly; 6] = Default::default();
    x[0] = y[0].scale(0.5).shift_r(0, -2);
    x[3] = y[3].sub(&x[0].mul(&a).shift_r(0, 1)).shift_r(0, -1);
    x[4] = y[4].sub(&x[0].mul(&b).shift_r(0, 1)).shift_r(0, -1);
    x[1] = y[1].sub(&a.mul(&x[3]).scale(2.0)).scale(0.5).shift_r(1, -2);
    x[5] = y[5].sub(&x[1].mul(&c).shift_r(1, 1)).sub(&b.mul(&x[3])).sub(&a.mul(&x[4])).shift_r(1, -1);
    x[2] = y[2].sub(&b.mul(&x[4]).scale(2.0)).sub(&c.mul(&x[5]).scale(2.0)).scale(0.5).shift_r(2, -2);
    x
}

/// `chart::solve_ju_t` on polynomials: J_uᵀ x = y.
fn solve_ju_t(y: &[Poly; 6]) -> [Poly; 6] {
    let (a, b, c) = (Poly::var(3), Poly::var(4), Poly::var(5));
    let mut x: [Poly; 6] = Default::default();
    x[2] = y[2].scale(0.5).shift_r(2, -2);
    x[5] = y[5].sub(&c.mul(&x[2]).scale(2.0)).shift_r(1, -1);
    x[1] = y[1].sub(&c.mul(&x[5]).shift_r(1, 1)).scale(0.5).shift_r(1, -2);
    x[4] = y[4].sub(&b.mul(&x[2]).scale(2.0)).sub(&a.mul(&x[5])).shift_r(0, -1);
    x[3] = y[3].sub(&a.mul(&x[1]).scale(2.0)).sub(&b.mul(&x[5])).shift_r(0, -1);
    x[0] = y[0].sub(&a.mul(&x[3]).shift_r(0, 1)).sub(&b.mul(&x[4]).shift_r(0, 1)).scale(0.5).shift_r(0, -2);
    x
}

/// The u-cometric C_ij = (J_u⁻¹ A(G) J_u⁻ᵀ)_ij as exact Laurent polynomials. `flat` replaces it by the identity
/// (the FlatMetric mutation).
pub fn cometric(flat: bool) -> [[Poly; 6]; 6] {
    let mut out: [[Poly; 6]; 6] = Default::default();
    if flat {
        for i in 0..6 {
            out[i][i] = Poly::constant(1.0);
        }
        return out;
    }
    let x = gram_polys();
    let ac = a_coeffs();
    for j in 0..6 {
        let mut e: [Poly; 6] = Default::default();
        e[j] = Poly::constant(1.0);
        let t = solve_ju_t(&e);
        let mut ay: [Poly; 6] = Default::default();
        for i in 0..6 {
            for jj in 0..6 {
                for m in 0..6 {
                    if ac[i][jj][m] != 0.0 {
                        ay[i] = ay[i].add(&x[m].mul(&t[jj]).scale(ac[i][jj][m]));
                    }
                }
            }
        }
        let col = solve_ju(&ay);
        for i in 0..6 {
            out[i][j] = col[i].clone();
        }
    }
    out
}

/// The sourced potential V(L) of `Model::potential`, as a polynomial in the Cholesky entries.
pub fn potential(model: &Model) -> Poly {
    let z = Poly::zero;
    let l: [[Poly; 3]; 3] = [
        [Poly::var(0), z(), z()],
        [Poly::var(3), Poly::var(1), z()],
        [Poly::var(4), Poly::var(5), Poly::var(2)],
    ];
    let cross = |p: &[Poly; 3], q: &[Poly; 3]| -> [Poly; 3] {
        [
            p[1].mul(&q[2]).sub(&p[2].mul(&q[1])),
            p[2].mul(&q[0]).sub(&p[0].mul(&q[2])),
            p[0].mul(&q[1]).sub(&p[1].mul(&q[0])),
        ]
    };
    let cof = [cross(&l[1], &l[2]), cross(&l[2], &l[0]), cross(&l[0], &l[1])];
    let mut v = Poly::zero();
    for k in 0..3 {
        for c in 0..3 {
            let f = l[k][c].scale(model.omega[k]).sub(&cof[k][c].scale(model.g));
            v = v.add(&f.mul(&f).scale(0.5)).sub(&l[k][c].mul(&l[k][c]).scale(0.5 * model.sources[k]));
        }
    }
    v
}

/// Split a polynomial by its r-exponent (the exponential group); each group keeps its (a, b, c) polynomial.
pub fn by_group(p: &Poly) -> BTreeMap<[i32; 3], Vec<([i32; 3], f64)>> {
    let mut g: BTreeMap<[i32; 3], Vec<([i32; 3], f64)>> = BTreeMap::new();
    for (k, c) in &p.0 {
        g.entry([k[0], k[1], k[2]]).or_default().push(([k[3], k[4], k[5]], *c));
    }
    g
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::{solve_ju as sj, solve_ju_t as sjt};
    use crate::model::{a_at, M3};

    fn rand_point(seed: u64) -> [f64; 6] {
        let mut s = seed;
        let mut next = || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((s >> 11) as f64) / ((1u64 << 53) as f64)
        };
        [0.4 + next(), 0.4 + next(), 0.4 + next(), next() - 0.5, next() - 0.5, next() - 0.5]
    }

    #[test]
    fn cometric_matches_the_triangular_solves() {
        let c = cometric(false);
        let ac = a_coeffs();
        for seed in 1..20u64 {
            let p = rand_point(seed);
            let (r, a, b, cc) = ([p[0], p[1], p[2]], p[3], p[4], p[5]);
            let x = [r[0] * r[0], a * a + r[1] * r[1], b * b + cc * cc + r[2] * r[2], r[0] * a, r[0] * b, a * b + r[1] * cc];
            let am = a_at(&x);
            for j in 0..6 {
                let mut e = [0.0; 6];
                e[j] = 1.0;
                let t = sjt(&r, a, b, cc, &e);
                let mut ay = [0.0; 6];
                for i in 0..6 {
                    ay[i] = (0..6).map(|k| am[i][k] * t[k]).sum();
                }
                let col = sj(&r, a, b, cc, &ay);
                for i in 0..6 {
                    let d = (c[i][j].eval(&p) - col[i]).abs();
                    assert!(d < 1e-11 * (1.0 + col[i].abs()), "C[{i}][{j}] off by {d}");
                }
            }
            let _ = ac;
        }
        for i in 0..6 {
            for j in 0..6 {
                assert_eq!(c[i][j], c[j][i], "cometric is not exactly symmetric at ({i},{j})");
                for k in c[i][j].0.keys() {
                    assert!(k[3] >= 0 && k[4] >= 0 && k[5] >= 0, "negative power of an offdiagonal entry");
                }
            }
        }
    }

    #[test]
    fn potential_matches_the_model() {
        let m = Model::berger(1.15, 3.0, 0.07);
        let v = potential(&m);
        for seed in 1..20u64 {
            let p = rand_point(seed);
            let l: M3 = [[p[0], 0.0, 0.0], [p[3], p[1], 0.0], [p[4], p[5], p[2]]];
            let want = m.potential(&l);
            assert!((v.eval(&p) - want).abs() < 1e-12 * (1.0 + want.abs()));
        }
    }

    #[test]
    fn group_counts_match_the_python_expansion() {
        // laurent_model at (1.15, 3, sqrt 0.004): 29 metric terms in the upper triangle in 16 exponential groups,
        // 15 potential terms in 9 groups, (a, b, c) degree at most 4 throughout.
        let c = cometric(false);
        let mut terms = 0;
        let mut groups = std::collections::BTreeSet::new();
        let mut deg = 0;
        for i in 0..6 {
            for j in i..6 {
                terms += c[i][j].0.len();
                for k in c[i][j].0.keys() {
                    groups.insert([k[0], k[1], k[2]]);
                }
                deg = deg.max(c[i][j].abc_degree());
            }
        }
        assert_eq!((terms, groups.len(), deg), (29, 16, 4));
        let v = potential(&Model::berger(1.15, 3.0, 0.004f64.sqrt()));
        assert_eq!((v.0.len(), by_group(&v).len(), v.abc_degree()), (15, 9, 4));
    }
}
