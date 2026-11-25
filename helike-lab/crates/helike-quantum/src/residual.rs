//! Rigorous enclosures of full-operator residuals ‖(H − E)ψ‖ / ‖ψ‖ for saved trial states.
//!
//! This is an interval port of `scripts/helike_residual_moments.py` (feat/leakage-gate-amendment, the centred
//! version at d0297072): `Space`, `build_residual` and `norm_squared` (Taylor translation). Every number is an
//! outward-rounded interval (`interval`). There is no quadrature, no truncation and no fitted order.
//!
//! **The object.** The trial state is ψ = f P on the positive cone. f is the Gaussian in the chart coordinate z,
//! divided by the square root of the density. P = Σ C_n h_n(z) is in the orthonormal Hermite basis. Everything is
//! defined exactly by the saved binary64 data: the chart (u0, S), the frequencies w, the coefficients
//! (coeff · mixing, formed here in intervals) and the centres E. Then
//!     (H − E)ψ / f = Σ_a exp(α_a · z) Q_a(z)
//! is a finite exponential-polynomial. The u-cometric is an exact integer Laurent polynomial (checked here against
//! the reference's sympy expansion, term by term). The potential's coefficients are enclosed for the declared
//! constants (s = 1.15, t = 3, g² as a decimal), and also for the binary64 constants the reference used, so the
//! enclosure holds for both readings of the model. ∂/∂u = S⁻ᵀ ∂/∂z uses a verified enclosure of S⁻¹.
//!
//! **The integral.** Completing the square, for β = α_a + α_b and μ = β/(2w):
//!     ⟨e^{α_a z} Q_a, e^{α_b z} Q_b⟩ = e^{Σ β²/(4w)} ⟨Q_a(z + μ), Q_b(z + μ)⟩.
//! It is evaluated in centred form: ‖Σ_a Q_a‖², plus pairwise c + expm1(e)(b + c), with c built only from the
//! translation increments. The translation is the exact finite Taylor sum of the polynomial. Long sums are added in
//! a balanced tree, which keeps the enclosure width near ε·log₂N·Σ|terms| instead of growing by an ulp per term.
//!
//! **What this does not do.** It encloses the uncut residual of the saved state under the registered operator. It
//! does not bound the cutoff leakage (that is `helike_cutoff_bounds.py`, in Arb). It does not evaluate the relative
//! leakage gate or any spectral statement. Those belong to the registered Python runners.

use crate::basis::{states_up_to, State};
use crate::interval::{Cx, Iv, CZERO, ONE, ZERO};
use crate::laurent::{self, Poly};
use crate::model::inv6;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

pub const MEASURE_POWERS: [i64; 6] = [3, 2, 1, 0, 0, 0];
/// The reference's numerical contract on completed-square exponents.
pub const EXPONENT_LIMIT: f64 = 600.0;

/// Deliberate faults; each must move the enclosure off the reference values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mutation {
    None,
    /// omit the density-drift term −½ c m_i ∂_j
    DropMeasureDrift,
    /// omit the translation cross terms c in every pair
    DropCross,
    /// omit the expm1(e) factor (treat every completed-square exponent as 0)
    DropExponential,
    /// Re(A·B) in place of Re(conj(A)·B)
    NoConjugate,
    /// charge operator: omit the density derivative −½ p_i of 1/√ν
    DropDensityDerivative,
    /// charge operator: target −m in place of m
    ReverseCharge,
    /// charge operator: omit −e^{2u2 − u1} from the velocity of c
    DropGeneratorTerm,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Physical,
    Coordinate,
    Derivative,
}

/// Balanced (pairwise) interval sum.
pub fn tree_sum(v: &[Iv]) -> Iv {
    match v.len() {
        0 => ZERO,
        1 => v[0],
        n => tree_sum(&v[..n / 2]) + tree_sum(&v[n / 2..]),
    }
}

/// The Hermite workspace of `Space` in the reference: all six-oscillator states of total degree ≤ `degree`, with
/// columns (states solved together) stored innermost.
pub struct Space {
    pub w: [f64; 6],
    pub degree: usize,
    pub states: Vec<State>,
    pub index: HashMap<State, usize>,
    pub ncol: usize,
    top: Vec<bool>,
    /// (src, dst) with dst = src − e_axis
    down: [Vec<(u32, u32)>; 6],
    /// per axis, indexed by n (1..=degree): √(n/(2w)), √(wn/2), √(2wn)
    coord: [Vec<Iv>; 6],
    half: [Vec<Iv>; 6],
    deriv: [Vec<Iv>; 6],
}

impl Space {
    pub fn new(w: [f64; 6], degree: usize, ncol: usize) -> Result<Space, String> {
        if w.iter().any(|&x| !(x > 0.0 && x.is_finite())) {
            return Err("positive finite frequencies required".into());
        }
        if !(1..=16).contains(&degree) || !(1..=4).contains(&ncol) {
            return Err("degree in 1..16 and one to four columns required".into());
        }
        let states = states_up_to(degree);
        let index: HashMap<State, usize> = states.iter().enumerate().map(|(i, s)| (*s, i)).collect();
        let top = states.iter().map(|s| s.iter().map(|&x| x as usize).sum::<usize>() == degree).collect();
        let mut down: [Vec<(u32, u32)>; 6] = Default::default();
        for (i, s) in states.iter().enumerate() {
            for (axis, d) in down.iter_mut().enumerate() {
                if s[axis] >= 1 {
                    let mut t = *s;
                    t[axis] -= 1;
                    d.push((i as u32, index[&t] as u32));
                }
            }
        }
        let table = |f: &dyn Fn(Iv, Iv) -> Iv| -> [Vec<Iv>; 6] {
            std::array::from_fn(|a| (0..=degree).map(|n| f(Iv::point(w[a]), Iv::int(n as i64))).collect())
        };
        let two = Iv::int(2);
        let coord = table(&|w, n| (n / (two * w)).sqrt());
        let half = table(&|w, n| (w * n / two).sqrt());
        let deriv = table(&|w, n| (two * w * n).sqrt());
        Ok(Space { w, degree, states, index, ncol, top, down, coord, half, deriv })
    }

    pub fn len(&self) -> usize {
        self.states.len() * self.ncol
    }

    pub fn zeros(&self) -> Vec<Cx> {
        vec![CZERO; self.len()]
    }

    fn op(&self, c: &[Cx], axis: usize, kind: Kind) -> Result<Vec<Cx>, String> {
        let k = self.ncol;
        if kind != Kind::Derivative {
            for (s, &t) in self.top.iter().enumerate() {
                if t && c[s * k..(s + 1) * k].iter().any(|v| !v.is_zero()) {
                    return Err("nonzero coefficient would be truncated".into());
                }
            }
        }
        let mut out = self.zeros();
        for &(src, dst) in &self.down[axis] {
            let (src, dst) = (src as usize, dst as usize);
            let n = self.states[src][axis] as usize;
            let (lower, upper) = match kind {
                Kind::Coordinate => (self.coord[axis][n], Some(self.coord[axis][n])),
                Kind::Physical => (self.half[axis][n], Some(-self.half[axis][n])),
                Kind::Derivative => (self.deriv[axis][n], None),
            };
            for j in 0..k {
                let v = c[src * k + j];
                if !v.is_zero() {
                    out[dst * k + j] = out[dst * k + j] + v.scale(lower);
                }
                if let Some(up) = upper {
                    let v = c[dst * k + j];
                    if !v.is_zero() {
                        out[src * k + j] = out[src * k + j] + v.scale(up);
                    }
                }
            }
        }
        Ok(out)
    }

    /// Q(z + μ) − Q(z) by the exact finite Taylor sum, axis by axis (no subtraction of rounded arrays).
    pub fn shift_increment(&self, c: &[Cx], mu: &[Iv; 6]) -> Vec<Cx> {
        let mut increment = self.zeros();
        for axis in 0..6 {
            if mu[axis].is_zero() {
                continue;
            }
            let mut term: Vec<Cx> = c.iter().zip(&increment).map(|(a, b)| *a + *b).collect();
            let mut delta = self.zeros();
            for r in 1..=self.degree {
                term = self.op(&term, axis, Kind::Derivative).expect("derivative never truncates");
                let f = mu[axis] / Iv::int(r as i64);
                let mut any = false;
                for (t, d) in term.iter_mut().zip(delta.iter_mut()) {
                    if !t.is_zero() {
                        *t = t.scale(f);
                        *d = *d + *t;
                        any = true;
                    }
                }
                if !any {
                    break;
                }
            }
            for (i, d) in increment.iter_mut().zip(&delta) {
                *i = *i + *d;
            }
        }
        increment
    }
}

/// One exponential group: exp(α · z) Q(z).
pub struct Group {
    pub key: [i32; 3],
    pub alpha: [Iv; 6],
    pub coeffs: Vec<Cx>,
}

/// The chart, enclosed: (u0, S) exact binary64, S⁻¹ as a verified enclosure.
pub struct ChartIv {
    pub u0: [f64; 6],
    pub s: [[f64; 6]; 6],
    pub inv_s: [[Iv; 6]; 6],
}

impl ChartIv {
    /// S⁻¹ = X + X R + X R² (I − R)⁻¹ with X the floating inverse and R = I − S X (enclosed); the last term is
    /// bounded entrywise by (Σ_k |X_ik|) ρ² / (1 − ρ), ρ = ‖R‖∞ < 1/2.
    pub fn new(u0: [f64; 6], s: [[f64; 6]; 6]) -> Result<ChartIv, String> {
        let x = inv6(&s);
        let mut r = [[ZERO; 6]; 6];
        for i in 0..6 {
            for j in 0..6 {
                let sx = tree_sum(&(0..6).map(|k| Iv::point(s[i][k]) * Iv::point(x[k][j])).collect::<Vec<_>>());
                r[i][j] = (if i == j { ONE } else { ZERO }) - sx;
            }
        }
        let rho = (0..6).map(|i| tree_sum(&(0..6).map(|j| Iv::point(r[i][j].mag())).collect::<Vec<_>>()).hi).fold(0.0, f64::max);
        if !(rho < 0.5) {
            return Err(format!("chart inverse not verified (rho = {rho})"));
        }
        let rho = Iv::point(rho);
        let tail_factor = (rho.sqr() / (ONE - rho)).hi;
        let mut inv_s = [[ZERO; 6]; 6];
        for i in 0..6 {
            let row_abs = tree_sum(&(0..6).map(|k| Iv::point(x[i][k].abs())).collect::<Vec<_>>()).hi;
            let t = (Iv::point(row_abs) * Iv::point(tail_factor)).hi;
            for j in 0..6 {
                let xr = tree_sum(&(0..6).map(|k| Iv::point(x[i][k]) * r[k][j]).collect::<Vec<_>>());
                inv_s[i][j] = Iv::point(x[i][j]) + xr + Iv::new(-t, t);
            }
        }
        Ok(ChartIv { u0, s, inv_s })
    }
}

/// The exact decimal a binary64 was written as (its shortest round-trip form), as an interval: "1.15" -> 115/100.
pub fn declared_decimal(x: f64) -> Iv {
    let s = format!("{x}");
    assert!(!s.contains('e'), "declared constant {s} must be a plain decimal");
    let (int, frac) = s.split_once('.').unwrap_or((&s, ""));
    let num: i64 = format!("{int}{frac}").parse().expect("decimal digits");
    let den = 10i64.pow(frac.len() as u32);
    Iv::int(num) / Iv::int(den)
}

/// Shape polynomials of the potential (exact integer coefficients): |row_k|², det L, Σ cof(L)².
fn potential_shapes() -> ([Poly; 3], Poly, Poly) {
    let z = Poly::zero;
    let l: [[Poly; 3]; 3] = [
        [Poly::var(0), z(), z()],
        [Poly::var(3), Poly::var(1), z()],
        [Poly::var(4), Poly::var(5), Poly::var(2)],
    ];
    let rows: [Poly; 3] = std::array::from_fn(|k| (0..3).fold(Poly::zero(), |acc, c| acc.add(&l[k][c].mul(&l[k][c]))));
    let det = Poly::var(0).mul(&Poly::var(1)).mul(&Poly::var(2));
    let cross = |p: &[Poly; 3], q: &[Poly; 3]| -> [Poly; 3] {
        [
            p[1].mul(&q[2]).sub(&p[2].mul(&q[1])),
            p[2].mul(&q[0]).sub(&p[0].mul(&q[2])),
            p[0].mul(&q[1]).sub(&p[1].mul(&q[0])),
        ]
    };
    let cof = [cross(&l[1], &l[2]), cross(&l[2], &l[0]), cross(&l[0], &l[1])];
    let mut cofsq = Poly::zero();
    for row in &cof {
        for e in row {
            cofsq = cofsq.add(&e.mul(e));
        }
    }
    (rows, det, cofsq)
}

/// V = Σ_k (ω_k² − src_k)/2 |row_k|² − g(Σω) det L + (g²/2) Σ cof², for given enclosures of (s, t, g, g²).
fn potential_with(s: Iv, t: Iv, g: Iv, g2: Iv) -> BTreeMap<laurent::Key, Iv> {
    let (rows, det, cofsq) = potential_shapes();
    let one = ONE;
    let two = Iv::int(2);
    let a = (one - s) * (t - two * s);
    let b = (one - s) * ((one - s) / t - s);
    let om2 = [t, one / t, one / t];
    let rt = t.sqrt();
    let omsum = rt + two / rt;
    let lam = [(om2[0] - a) / two, (om2[1] - b) / two, (om2[2] - b) / two];
    let mut v: BTreeMap<laurent::Key, Iv> = BTreeMap::new();
    let mut put = |p: &Poly, f: Iv| {
        for (k, c) in &p.0 {
            assert!(c.fract() == 0.0 && c.abs() < 1e15, "shape coefficient {c} is not a small integer");
            let e = v.entry(*k).or_insert(ZERO);
            *e = *e + f * Iv::point(*c);
        }
    };
    for k in 0..3 {
        put(&rows[k], lam[k]);
    }
    put(&det, -(g * omsum));
    put(&cofsq, g2 / two);
    v.retain(|_, c| !c.is_zero());
    v
}

/// The potential's Laurent coefficients, enclosing (i) the declared decimal constants, (ii) exact arithmetic on the
/// binary64 constants the reference used, and (iii) the reference's own binary64 coefficients.
pub fn potential_enclosure(
    s: f64,
    t: f64,
    g2: f64,
    g_binary64: f64,
    python: &BTreeMap<laurent::Key, f64>,
) -> Result<BTreeMap<laurent::Key, Iv>, String> {
    let g2d = declared_decimal(g2);
    let declared = potential_with(declared_decimal(s), declared_decimal(t), g2d.sqrt(), g2d);
    let g = Iv::point(g_binary64);
    let binary = potential_with(Iv::point(s), Iv::point(t), g, g * g);
    let keys: std::collections::BTreeSet<_> = declared.keys().chain(binary.keys()).chain(python.keys()).collect();
    if keys.len() != declared.len() || keys.len() != python.len() {
        return Err("potential term sets differ between the declared, binary64 and reference expansions".into());
    }
    let mut out = BTreeMap::new();
    for k in keys {
        let (d, b, p) = (declared[k], binary[k], Iv::point(python[k]));
        let h = d.hull(&b).hull(&p);
        if h.width() > 1e-13 * h.mag() {
            return Err(format!("potential coefficient {k:?} disagrees between readings: {d:?} {b:?} {p:?}"));
        }
        out.insert(*k, h);
    }
    Ok(out)
}

/// The exact integer u-cometric, checked term by term against the reference's (sympy, exact rational) expansion.
pub fn verified_cometric(python: &[(usize, usize, laurent::Key, f64)]) -> Result<[[Poly; 6]; 6], String> {
    let c = laurent::cometric(false);
    let mut seen = 0;
    for &(i, j, k, v) in python {
        let ours = c[i][j].0.get(&k).copied().unwrap_or(0.0);
        if ours != v || v.fract() != 0.0 {
            return Err(format!("cometric term ({i},{j}) {k:?}: {ours} vs reference {v}"));
        }
        seen += 1;
    }
    let total: usize = c.iter().flatten().map(|p| p.0.len()).sum();
    if seen != total {
        return Err(format!("cometric has {total} terms, reference {seen}"));
    }
    Ok(c)
}

/// `build_residual`: the groups of (H − E)ψ / f.
pub fn build_residual(
    chart: &ChartIv,
    basis: &[State],
    w: [f64; 6],
    coeff: &[Cx],
    centers: &[f64],
    metric: &[[Poly; 6]; 6],
    potential: &BTreeMap<laurent::Key, Iv>,
    mutation: Mutation,
) -> Result<(Space, Vec<Group>), String> {
    let ncol = centers.len();
    if coeff.len() != basis.len() * ncol {
        return Err("coefficient shape mismatch".into());
    }
    let (space, p) = embed(w, basis, coeff, ncol, 6)?;
    let first = physical_gradient(&space, chart, &p, true)?;
    let d = |c: &[Cx], i: usize| derivative(&space, chart, c, i, true);
    let mut second: HashMap<(usize, usize), Vec<Cx>> = HashMap::new();
    for i in 0..6 {
        for j in i..6 {
            second.insert((i, j), d(&first[j], i)?);
        }
    }
    let mut groups: BTreeMap<[i32; 3], Vec<Cx>> = BTreeMap::new();
    let mut g0 = p.clone();
    for row in g0.chunks_mut(ncol) {
        for (j, v) in row.iter_mut().enumerate() {
            *v = v.scale(Iv::point(-centers[j]));
        }
    }
    groups.insert([0, 0, 0], g0);
    let mut add = |key: [i32; 6], c: &[Cx], factor: Iv| accumulate(&space, chart, &mut groups, key, c, factor);
    let half = Iv::point(-0.5);
    for i in 0..6 {
        for j in 0..6 {
            for (key, &c) in &metric[i][j].0 {
                let c = Iv::point(c);
                add(*key, &second[&(i.min(j), i.max(j))], half * c)?;
                if mutation != Mutation::DropMeasureDrift {
                    add(*key, &first[j], half * c * Iv::int(MEASURE_POWERS[i]))?;
                }
                if i < 3 {
                    add(*key, &first[j], half * c * Iv::int(key[i] as i64))?;
                } else if key[i] != 0 {
                    let mut lowered = *key;
                    lowered[i] -= 1;
                    add(lowered, &first[j], half * c * Iv::int(key[i] as i64))?;
                }
            }
        }
    }
    for (key, c) in potential {
        add(*key, &p, *c)?;
    }
    Ok((space, finish_groups(chart, groups)))
}

/// The Hermite workspace (saved degree + `headroom`) with the coefficients placed in it.
pub(crate) fn embed(w: [f64; 6], basis: &[State], coeff: &[Cx], ncol: usize, headroom: usize) -> Result<(Space, Vec<Cx>), String> {
    if coeff.len() != basis.len() * ncol {
        return Err("coefficient shape mismatch".into());
    }
    let degree = basis.iter().map(|n| n.iter().map(|&x| x as usize).sum::<usize>()).max().unwrap_or(0) + headroom;
    let space = Space::new(w, degree, ncol)?;
    let mut p = space.zeros();
    for (n, row) in basis.iter().zip(coeff.chunks(ncol)) {
        let i = *space.index.get(n).ok_or("basis state outside the workspace")?;
        p[i * ncol..(i + 1) * ncol].copy_from_slice(row);
    }
    Ok((space, p))
}

/// ∂ψ/∂u_i divided by f, as coefficients: Σ_k (S⁻¹)_{ki} (∂_{z_k} of the Gaussian factor and P) − ½ p_i (the
/// density 1/√ν). `density = false` drops the density derivative (a deliberate fault for the mutation tests).
pub(crate) fn derivative(space: &Space, chart: &ChartIv, c: &[Cx], i: usize, density: bool) -> Result<Vec<Cx>, String> {
    let m = Iv::point(if density { -0.5 * MEASURE_POWERS[i] as f64 } else { 0.0 });
    let mut out: Vec<Cx> = c.iter().map(|v| v.scale(m)).collect();
    for k in 0..6 {
        let f = chart.inv_s[k][i];
        if f.is_zero() {
            continue;
        }
        let ph = space.op(c, k, Kind::Physical)?;
        for (o, v) in out.iter_mut().zip(&ph) {
            if !v.is_zero() {
                *o = *o + v.scale(f);
            }
        }
    }
    Ok(out)
}

pub(crate) fn physical_gradient(space: &Space, chart: &ChartIv, p: &[Cx], density: bool) -> Result<Vec<Vec<Cx>>, String> {
    (0..6).map(|i| derivative(space, chart, p, i, density)).collect()
}

/// Add factor · (Laurent monomial `key` in u) · c to the exponential groups. The polynomial part u_3..u_5 is
/// applied exactly as (u0 + S z) on the Hermite coefficients; the exponential part splits into exp(key · u0) and the
/// group's exp(α · z).
pub(crate) fn accumulate(
    space: &Space,
    chart: &ChartIv,
    groups: &mut BTreeMap<[i32; 3], Vec<Cx>>,
    key: [i32; 6],
    c: &[Cx],
    factor: Iv,
) -> Result<(), String> {
    if factor.is_zero() {
        return Ok(());
    }
    let mut value = c.to_vec();
    for axis in 3..6 {
        if key[axis] < 0 {
            return Err("negative polynomial power in a Laurent key".into());
        }
        for _ in 0..key[axis] {
            let mut out: Vec<Cx> = value.iter().map(|v| v.scale(Iv::point(chart.u0[axis]))).collect();
            for k in 0..6 {
                if chart.s[axis][k] == 0.0 {
                    continue;
                }
                let co = space.op(&value, k, Kind::Coordinate)?;
                let f = Iv::point(chart.s[axis][k]);
                for (o, v) in out.iter_mut().zip(&co) {
                    if !v.is_zero() {
                        *o = *o + v.scale(f);
                    }
                }
            }
            value = out;
        }
    }
    let prefix = [key[0], key[1], key[2]];
    let ex = tree_sum(&(0..3).map(|i| Iv::int(prefix[i] as i64) * Iv::point(chart.u0[i])).collect::<Vec<_>>()).exp();
    let f = factor * ex;
    let slot = groups.entry(prefix).or_insert_with(|| space.zeros());
    for (o, v) in slot.iter_mut().zip(&value) {
        if !v.is_zero() {
            *o = *o + v.scale(f);
        }
    }
    Ok(())
}

pub(crate) fn finish_groups(chart: &ChartIv, groups: BTreeMap<[i32; 3], Vec<Cx>>) -> Vec<Group> {
    groups
        .into_iter()
        .filter(|(_, v)| v.iter().any(|x| !x.is_zero()))
        .map(|(key, coeffs)| {
            let alpha = std::array::from_fn(|k| {
                tree_sum(&(0..3).map(|i| Iv::int(key[i] as i64) * Iv::point(chart.s[i][k])).collect::<Vec<_>>())
            });
            Group { key, alpha, coeffs }
        })
        .collect()
}

/// Per column: Σ over states of f(state, column), tree-summed.
fn column_sums(space: &Space, f: impl Fn(usize) -> Iv) -> Vec<Iv> {
    let k = space.ncol;
    let n = space.states.len();
    (0..k).map(|j| tree_sum(&(0..n).map(|s| f(s * k + j)).collect::<Vec<_>>())).collect()
}

#[derive(Clone, Debug, Serialize)]
pub struct NormSquared {
    /// enclosure of ‖(H − E)ψ_j / f‖² in the Gaussian measure, per column
    pub value: Vec<[f64; 2]>,
    /// Σ |pair contributions| (upper), the conditioning of the centred sum
    pub pair_absolute_sum: Vec<f64>,
    pub pairs: usize,
    pub largest_exponent: f64,
}

/// `norm_squared` (centred, Taylor translation) in interval arithmetic.
pub fn norm_squared(space: &Space, groups: &[Group], mutation: Mutation) -> Result<(Vec<Iv>, NormSquared), String> {
    if groups.is_empty() {
        return Err("nonempty exponential-polynomial required".into());
    }
    let len = space.len();
    let combined: Vec<Cx> = (0..len)
        .map(|i| {
            let re = tree_sum(&groups.iter().map(|g| g.coeffs[i].re).collect::<Vec<_>>());
            let im = tree_sum(&groups.iter().map(|g| g.coeffs[i].im).collect::<Vec<_>>());
            Cx::new(re, im)
        })
        .collect();
    let base = column_sums(space, |i| combined[i].norm_sq());
    let conj = |a: &Cx, b: &Cx| -> Iv {
        if mutation == Mutation::NoConjugate {
            a.re * b.re - a.im * b.im
        } else {
            a.re_conj_mul(b)
        }
    };
    let pairs: Vec<(usize, usize)> = (0..groups.len()).flat_map(|i| (i..groups.len()).map(move |j| (i, j))).collect();
    let results: Vec<Result<(Vec<Iv>, f64), String>> = pairs
        .par_iter()
        .map(|&(i, j)| {
            let (a, b) = (&groups[i], &groups[j]);
            let total: [Iv; 6] = std::array::from_fn(|k| a.alpha[k] + b.alpha[k]);
            let e = tree_sum(&(0..6).map(|k| total[k].sqr() / (Iv::int(4) * Iv::point(space.w[k]))).collect::<Vec<_>>());
            if !(e.hi <= EXPONENT_LIMIT) {
                return Err("completed-square factor exceeds numerical contract".into());
            }
            let mu: [Iv; 6] = std::array::from_fn(|k| total[k] / (Iv::int(2) * Iv::point(space.w[k])));
            let da = space.shift_increment(&a.coeffs, &mu);
            let db = space.shift_increment(&b.coeffs, &mu);
            let baseline = column_sums(space, |s| conj(&a.coeffs[s], &b.coeffs[s]));
            let cross = column_sums(space, |s| {
                conj(&a.coeffs[s], &db[s]) + conj(&da[s], &b.coeffs[s]) + conj(&da[s], &db[s])
            });
            let em = if mutation == Mutation::DropExponential { ZERO } else { e.expm1() };
            let mult = Iv::int(if i == j { 1 } else { 2 });
            let pair: Vec<Iv> = (0..space.ncol)
                .map(|c| {
                    let cr = if mutation == Mutation::DropCross { ZERO } else { cross[c] };
                    (cr + em * (baseline[c] + cr)) * mult
                })
                .collect();
            Ok((pair, e.hi))
        })
        .collect();
    let mut per_col: Vec<Vec<Iv>> = (0..space.ncol).map(|c| vec![base[c]]).collect();
    let mut largest_exponent: f64 = 0.0;
    for r in results {
        let (pair, e) = r?;
        largest_exponent = largest_exponent.max(e);
        for c in 0..space.ncol {
            per_col[c].push(pair[c]);
        }
    }
    let values: Vec<Iv> = per_col.iter().map(|v| tree_sum(v)).collect();
    for v in &values {
        if !v.is_finite() {
            return Err("nonfinite enclosure".into());
        }
    }
    let pair_absolute_sum = per_col.iter().map(|v| tree_sum(&v.iter().map(|x| Iv::point(x.mag())).collect::<Vec<_>>()).hi).collect();
    Ok((
        values.clone(),
        NormSquared {
            value: values.iter().map(|v| [v.lo, v.hi]).collect(),
            pair_absolute_sum,
            pairs: pairs.len(),
            largest_exponent,
        },
    ))
}

// ---------------------------------------------------------------------------------------------------------------
// Saved states (oracle/export_quasimode_states.py) and the per-control runner.

#[derive(Clone, Debug, Deserialize)]
pub struct PotentialTerm {
    pub key: Vec<i32>,
    pub c: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MetricTerm {
    pub i: usize,
    pub j: usize,
    pub key: Vec<i32>,
    pub c: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReferenceMoments {
    pub value: Vec<f64>,
    pub independent: Vec<f64>,
    pub group_count: usize,
    pub workspace_degree: usize,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SavedControl {
    pub g2: f64,
    #[serde(rename = "Mq")]
    pub mq: usize,
    pub s: f64,
    pub t: f64,
    pub state_file: String,
    pub state_sha256: String,
    pub s_bits: u64,
    pub t_bits: u64,
    pub g2_bits: u64,
    pub g_bits: u64,
    pub u0: Vec<u64>,
    #[serde(rename = "S")]
    pub s_mat: Vec<u64>,
    #[serde(rename = "saved_S_equals_chart_S")]
    pub saved_s_equals_chart_s: bool,
    pub w: Vec<u64>,
    pub basis: Vec<Vec<u8>>,
    pub coeff: Vec<u64>,
    pub coeff_shape: Vec<usize>,
    pub mixing_re: Vec<u64>,
    pub mixing_im: Vec<u64>,
    pub centers: Vec<u64>,
    pub names: Vec<String>,
    pub python_potential: Vec<PotentialTerm>,
    pub python_metric: Vec<MetricTerm>,
    pub reference_centered_moments: ReferenceMoments,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SavedStates {
    pub schema: String,
    pub parent_receipt_sha256: String,
    pub residual_receipt_sha256: String,
    pub controls: Vec<SavedControl>,
}

fn f(bits: &[u64]) -> Vec<f64> {
    bits.iter().map(|&b| f64::from_bits(b)).collect()
}

#[derive(Clone, Debug, Serialize)]
pub struct StateEnclosure {
    pub name: String,
    pub center: f64,
    /// ‖ψ‖² = Σ |C_n|² (Gaussian measure, orthonormal Hermite basis)
    pub norm_sq: [f64; 2],
    /// ‖(H − E)ψ‖² (same measure)
    pub residual_sq: [f64; 2],
    /// r = ‖(H − E)ψ‖ / ‖ψ‖
    pub r_lower: f64,
    pub r_upper: f64,
    /// the reference's centred floating-point value of residual_sq (not an enclosure)
    pub reference_residual_sq: f64,
    pub reference_inside: bool,
    /// distance from the reference value to the enclosure (0 if inside)
    pub reference_outside_by: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ControlEnclosure {
    pub g2: f64,
    #[serde(rename = "Mq")]
    pub mq: usize,
    pub state_file: String,
    pub state_sha256: String,
    pub workspace_degree: usize,
    pub group_count: usize,
    pub moments: NormSquared,
    pub states: Vec<StateEnclosure>,
    pub seconds: f64,
}

/// The saved chart, enclosed.
pub fn saved_chart(c: &SavedControl) -> Result<(ChartIv, [f64; 6]), String> {
    let u0: [f64; 6] = f(&c.u0).try_into().map_err(|_| "u0 must have six entries")?;
    let sv = f(&c.s_mat);
    if sv.len() != 36 {
        return Err("S must have 36 entries".into());
    }
    let s: [[f64; 6]; 6] = std::array::from_fn(|i| std::array::from_fn(|j| sv[i * 6 + j]));
    let w: [f64; 6] = f(&c.w).try_into().map_err(|_| "w must have six entries")?;
    Ok((ChartIv::new(u0, s)?, w))
}

pub fn saved_basis(c: &SavedControl) -> Result<Vec<State>, String> {
    c.basis
        .iter()
        .map(|b| <State>::try_from(b.as_slice()).map_err(|_| "basis rows must have six entries".to_string()))
        .collect()
}

/// The trial coefficients C = coeff · mixing, formed exactly in intervals. With `frozen` (the binary64 product
/// another evaluator stored, as (re, im) arrays), each coefficient is the hull of both, so every bound computed from
/// them holds for the exact product and for the stored one alike.
pub fn trial_coefficients(c: &SavedControl, frozen: Option<(&[f64], &[f64])>) -> Result<Vec<Cx>, String> {
    let ncol = c.centers.len();
    let (rows, cols) = (c.coeff_shape[0], c.coeff_shape[1]);
    if cols != ncol || c.basis.len() != rows || c.coeff.len() != rows * cols || c.mixing_re.len() != ncol * ncol {
        return Err("saved state shape mismatch".into());
    }
    let coeff = f(&c.coeff);
    let (mre, mim) = (f(&c.mixing_re), f(&c.mixing_im));
    let mut cm = vec![CZERO; rows * ncol];
    for n in 0..rows {
        for j in 0..ncol {
            let re = tree_sum(&(0..ncol).map(|k| Iv::point(coeff[n * ncol + k]) * Iv::point(mre[k * ncol + j])).collect::<Vec<_>>());
            let im = tree_sum(&(0..ncol).map(|k| Iv::point(coeff[n * ncol + k]) * Iv::point(mim[k * ncol + j])).collect::<Vec<_>>());
            cm[n * ncol + j] = Cx::new(re, im);
        }
    }
    if let Some((fre, fim)) = frozen {
        if fre.len() != cm.len() || fim.len() != cm.len() {
            return Err("frozen trial coefficients have the wrong shape".into());
        }
        for (i, v) in cm.iter_mut().enumerate() {
            let (a, b) = (Iv::point(fre[i]), Iv::point(fim[i]));
            // the stored product must be a rounding of the exact one, not a different vector
            let tol = 4.0 * f64::EPSILON * v.re.mag().max(v.im.mag()).max(f64::MIN_POSITIVE);
            if (a.mid() - v.re.mid()).abs() > tol + v.re.width() || (b.mid() - v.im.mid()).abs() > tol + v.im.width() {
                return Err(format!("frozen trial coefficient {i} is not a rounding of coeff · mixing"));
            }
            *v = Cx::new(v.re.hull(&a), v.im.hull(&b));
        }
    }
    Ok(cm)
}

/// Enclose the residuals of one saved control.
pub fn enclose_control(c: &SavedControl, mutation: Mutation) -> Result<ControlEnclosure, String> {
    enclose_control_with(c, None, mutation)
}

/// As `enclose_control`, with the coefficients hulled with a frozen binary64 product (see `trial_coefficients`).
pub fn enclose_control_with(
    c: &SavedControl,
    frozen: Option<(&[f64], &[f64])>,
    mutation: Mutation,
) -> Result<ControlEnclosure, String> {
    let t0 = std::time::Instant::now();
    let ncol = c.centers.len();
    let centers = f(&c.centers);
    let (space, groups, _, norms) = residual_parts(c, frozen, 0.0, mutation)?;
    let (values, moments) = norm_squared(&space, &groups, mutation)?;
    let states = (0..ncol)
        .map(|j| {
            let v = values[j];
            let nn = norms[j];
            let refv = c.reference_centered_moments.value[j];
            let lo = Iv::point(v.lo.max(0.0));
            let r_lower = if v.lo > 0.0 { (lo / nn).sqrt().lo } else { 0.0 };
            let r_upper = (Iv::point(v.hi.max(0.0)) / nn).sqrt().hi;
            let outside = if v.contains(refv) { 0.0 } else { (v.lo - refv).max(refv - v.hi) };
            StateEnclosure {
                name: c.names.get(j).cloned().unwrap_or_default(),
                center: centers[j],
                norm_sq: [nn.lo, nn.hi],
                residual_sq: [v.lo, v.hi],
                r_lower,
                r_upper,
                reference_residual_sq: refv,
                reference_inside: v.contains(refv),
                reference_outside_by: outside,
            }
        })
        .collect();
    Ok(ControlEnclosure {
        g2: c.g2,
        mq: c.mq,
        state_file: c.state_file.clone(),
        state_sha256: c.state_sha256.clone(),
        workspace_degree: space.degree,
        group_count: groups.len(),
        moments,
        states,
        seconds: t0.elapsed().as_secs_f64(),
    })
}

/// The exponential-polynomial (H − E')ψ/f of one saved control, with every centre shifted by an exact `shift`
/// (E' = E + shift; 0 for the registered centres), together with the embedded P = ψ/f and the norms ‖ψ‖². Shared by
/// the residual enclosure and the linear moment (`temple`).
pub fn residual_parts(
    c: &SavedControl,
    frozen: Option<(&[f64], &[f64])>,
    shift: f64,
    mutation: Mutation,
) -> Result<(Space, Vec<Group>, Vec<Cx>, Vec<Iv>), String> {
    let ncol = c.centers.len();
    let rows = c.coeff_shape[0];
    let cm = trial_coefficients(c, frozen)?;
    let (chart, w) = saved_chart(c)?;
    let basis = saved_basis(c)?;
    let mut centers = f(&c.centers);
    if shift != 0.0 {
        for e in centers.iter_mut() {
            let shifted = *e + shift;
            if shifted - *e != shift {
                return Err("centre shift is not exact in binary64".into());
            }
            *e = shifted;
        }
    }
    let python_v: BTreeMap<laurent::Key, f64> = c
        .python_potential
        .iter()
        .map(|p| (p.key.as_slice().try_into().expect("six-entry key"), f64::from_bits(p.c)))
        .collect();
    let (sv_, tv_, g2v_) = (f64::from_bits(c.s_bits), f64::from_bits(c.t_bits), f64::from_bits(c.g2_bits));
    if sv_ != c.s || tv_ != c.t || g2v_ != c.g2 || !c.saved_s_equals_chart_s {
        return Err("saved constants or chart disagree with their bit patterns".into());
    }
    let potential = potential_enclosure(sv_, tv_, g2v_, f64::from_bits(c.g_bits), &python_v)?;
    let pm: Vec<(usize, usize, laurent::Key, f64)> =
        c.python_metric.iter().map(|m| (m.i, m.j, m.key.as_slice().try_into().expect("six-entry key"), m.c)).collect();
    let metric = verified_cometric(&pm)?;
    let (space, groups) = build_residual(&chart, &basis, w, &cm, &centers, &metric, &potential, mutation)?;
    let (space_p, p) = embed(w, &basis, &cm, ncol, 6)?;
    if space_p.degree != space.degree {
        return Err("trial embedding and residual workspace differ".into());
    }
    let norms: Vec<Iv> = (0..ncol).map(|j| tree_sum(&(0..rows).map(|n| cm[n * ncol + j].norm_sq()).collect::<Vec<_>>())).collect();
    Ok((space, groups, p, norms))
}

pub fn load_saved_states(path: &std::path::Path) -> Result<SavedStates, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let s: SavedStates = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if s.schema != "helike-lab/quasimode-states/v1" {
        return Err("unexpected saved-state schema".into());
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One exponential group on axis 0 with w = 1 (other axes idle).
    fn one_axis(groups: &[(f64, Vec<(usize, f64)>)], degree: usize) -> (Space, Vec<Group>) {
        let space = Space::new([1.0; 6], degree, 1).unwrap();
        let gs = groups
            .iter()
            .map(|(alpha, coeffs)| {
                let mut c = space.zeros();
                for &(n, v) in coeffs {
                    let i = space.index[&[n as u8, 0, 0, 0, 0, 0]];
                    c[i] = Cx::real(Iv::point(v));
                }
                let mut a = [ZERO; 6];
                a[0] = Iv::point(*alpha);
                Group { key: [0; 3], alpha: a, coeffs: c }
            })
            .collect();
        (space, gs)
    }

    #[test]
    fn planted_cancellation_is_enclosed() {
        // F(z) = 1e4 (exp(1e-6 z) − 1), w = 1: ‖F‖² = 1e8 [e^{1e-12} − 2 e^{1e-12/4} + 1] = 5e-5 + 4.375e-17 + …
        let (space, groups) = one_axis(&[(1e-6, vec![(0, 1e4)]), (0.0, vec![(0, -1e4)])], 2);
        let (v, _) = norm_squared(&space, &groups, Mutation::None).unwrap();
        // mpmath, 40 digits: 0.00005000000000004375000000001614562816…
        assert!(v[0].contains(0.00005000000000004375), "{:?}", v[0]);
        // the old floating-point paths returned 4.999339580535889e-5 (both of them); that must be excluded
        assert!(!v[0].contains(4.999339580535889e-5));
        assert!(v[0].width() < 1e-12, "width {}", v[0].width());
    }

    #[test]
    fn hermite_norm_that_ten_point_quadrature_misses() {
        // h_10 has Gaussian norm 1 (the ten-point Gauss–Hermite rule returns 0 for it)
        let (space, groups) = one_axis(&[(0.0, vec![(10, 1.0)])], 12);
        let (v, _) = norm_squared(&space, &groups, Mutation::None).unwrap();
        assert!(v[0].contains(1.0) && v[0].width() < 1e-14, "{:?}", v[0]);
    }

    #[test]
    fn exponential_norm_by_completed_square() {
        // ‖e^{αz} h_1‖² with w = 1: e^{α²}·(1 + 2·(α/2)²·... ) — check against the closed form
        // ∫ e^{2αz} 2z² dγ, dγ = e^{-z²}/√π: = e^{α²}(1 + 2α²)
        let alpha: f64 = 0.37;
        let (space, groups) = one_axis(&[(alpha, vec![(1, 1.0)])], 4);
        let (v, _) = norm_squared(&space, &groups, Mutation::None).unwrap();
        let want = (alpha * alpha).exp() * (1.0 + 2.0 * alpha * alpha);
        assert!((v[0].mid() - want).abs() < 1e-14 && v[0].width() < 1e-13, "{:?} vs {want}", v[0]);
    }

    #[test]
    fn declared_decimals() {
        assert!(declared_decimal(1.15).contains(1.15));
        let g = declared_decimal(0.004);
        assert!(g.width() < 4e-18 && g.contains(0.004));
        assert_eq!(declared_decimal(3.0), Iv::int(3) / Iv::int(1));
    }
}
