//! Four-dimensional hypercubic SU(2) lattice for Q3 (docs/HALCYON_Q3_RUN_SPEC.md): Wilson action,
//! Kennedy–Pendleton heat bath (the engine's `sample_su2_link`) plus overrelaxation, and a plain
//! Metropolis update used only as the Q3-0(b) cross-check.
//!
//! Conventions: links U_μ(x), μ = 0 is time; unit quaternions, scalar first, the product of
//! `kennedy_pendleton::qmul`. The plaquette is U_μν(x) = U_μ(x) U_ν(x+μ) U_μ(x+ν)⁻¹ U_ν(x)⁻¹ and the
//! action S = β Σ_p (1 − ½ tr U_p). For the link U_μ(x) the staple sum V satisfies
//! Σ_{p ∋ U_μ(x)} ½ tr U_p = (U_μ(x) V)₀, which is the form `sample_su2_link` conditions on.
//!
//! This is a new sampler, not `GIBBS_SAMPLE`: it does not carry that verb's sequential-order
//! bit-identity contract with Halcyon's Python. Sweeps are sequential here; a seed in a receipt
//! reproduces an ensemble bit for bit.

use super::kennedy_pendleton::sample_su2_link;
use super::marsaglia_haar::SmallRng;

pub type Q = [f64; 4];

#[inline]
pub fn qmul(a: &Q, b: &Q) -> Q {
    [
        a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
        a[0] * b[1] + b[0] * a[1] + a[2] * b[3] - a[3] * b[2],
        a[0] * b[2] + b[0] * a[2] + a[3] * b[1] - a[1] * b[3],
        a[0] * b[3] + b[0] * a[3] + a[1] * b[2] - a[2] * b[1],
    ]
}

#[inline]
pub fn qinv(a: &Q) -> Q {
    [a[0], -a[1], -a[2], -a[3]]
}

#[inline]
fn qadd(a: &mut Q, b: &Q) {
    for k in 0..4 {
        a[k] += b[k];
    }
}

fn qnormalise(q: Q) -> Q {
    let n = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt().max(1e-300);
    [q[0] / n, q[1] / n, q[2] / n, q[3] / n]
}

/// A periodic L₀ × L₁ × L₂ × L₃ lattice of SU(2) links (direction 0 is time).
#[derive(Clone)]
pub struct Lattice4d {
    pub dims: [usize; 4],
    pub links: Vec<Q>,
}

impl Lattice4d {
    pub fn n_sites(&self) -> usize {
        self.dims.iter().product()
    }
    pub fn cold(dims: [usize; 4]) -> Self {
        let n: usize = dims.iter().product();
        Lattice4d { dims, links: vec![[1.0, 0.0, 0.0, 0.0]; 4 * n] }
    }
    pub fn hot(dims: [usize; 4], rng: &mut SmallRng) -> Self {
        let mut l = Self::cold(dims);
        for u in l.links.iter_mut() {
            *u = sample_su2_link([0.0; 4], 1.0, rng); // flat conditional: Haar
        }
        l
    }
    #[inline]
    pub fn site(&self, x: [usize; 4]) -> usize {
        let d = self.dims;
        ((x[0] % d[0] * d[1] + x[1] % d[1]) * d[2] + x[2] % d[2]) * d[3] + x[3] % d[3]
    }
    #[inline]
    pub fn coords(&self, mut s: usize) -> [usize; 4] {
        let d = self.dims;
        let x3 = s % d[3];
        s /= d[3];
        let x2 = s % d[2];
        s /= d[2];
        let x1 = s % d[1];
        [s / d[1], x1, x2, x3]
    }
    #[inline]
    pub fn shift(&self, x: [usize; 4], mu: usize, fwd: bool) -> [usize; 4] {
        let mut y = x;
        y[mu] = if fwd { (x[mu] + 1) % self.dims[mu] } else { (x[mu] + self.dims[mu] - 1) % self.dims[mu] };
        y
    }
    #[inline]
    pub fn u(&self, x: [usize; 4], mu: usize) -> Q {
        self.links[4 * self.site(x) + mu]
    }
    /// U_μν(x).
    pub fn plaquette(&self, x: [usize; 4], mu: usize, nu: usize) -> Q {
        let a = qmul(&self.u(x, mu), &self.u(self.shift(x, mu, true), nu));
        let b = qmul(&a, &qinv(&self.u(self.shift(x, nu, true), mu)));
        qmul(&b, &qinv(&self.u(x, nu)))
    }
    /// The staple sum V of U_μ(x): Σ_{p ∋ U_μ(x)} ½ tr U_p = (U_μ(x) V)₀.
    pub fn staple(&self, x: [usize; 4], mu: usize) -> Q {
        let mut v = [0.0; 4];
        let xm = self.shift(x, mu, true);
        for nu in 0..4 {
            if nu == mu {
                continue;
            }
            // forward: U_ν(x+μ) U_μ(x+ν)⁻¹ U_ν(x)⁻¹
            let xn = self.shift(x, nu, true);
            let f = qmul(&qmul(&self.u(xm, nu), &qinv(&self.u(xn, mu))), &qinv(&self.u(x, nu)));
            qadd(&mut v, &f);
            // backward: U_ν(x+μ−ν)⁻¹ U_μ(x−ν)⁻¹ U_ν(x−ν)
            let xb = self.shift(x, nu, false);
            let xmb = self.shift(xm, nu, false);
            let b = qmul(&qmul(&qinv(&self.u(xmb, nu)), &qinv(&self.u(xb, mu))), &self.u(xb, nu));
            qadd(&mut v, &b);
        }
        v
    }
    /// Mean of ½ tr U_p over all plaquettes.
    pub fn mean_plaquette(&self) -> f64 {
        let mut s = 0.0;
        for site in 0..self.n_sites() {
            let x = self.coords(site);
            for mu in 0..4 {
                for nu in mu + 1..4 {
                    s += self.plaquette(x, mu, nu)[0];
                }
            }
        }
        s / (6 * self.n_sites()) as f64
    }
    /// One heat-bath sweep (Kennedy–Pendleton) over every link, in storage order.
    pub fn heatbath_sweep(&mut self, beta: f64, rng: &mut SmallRng) {
        for site in 0..self.n_sites() {
            let x = self.coords(site);
            for mu in 0..4 {
                let v = self.staple(x, mu);
                self.links[4 * site + mu] = sample_su2_link(v, beta, rng);
            }
        }
    }
    /// One overrelaxation sweep: U → V̂⁻¹ U⁻¹ V̂⁻¹, which keeps (U V)₀ and the Haar measure.
    pub fn overrelax_sweep(&mut self) {
        for site in 0..self.n_sites() {
            let x = self.coords(site);
            for mu in 0..4 {
                let v = self.staple(x, mu);
                let k = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2] + v[3] * v[3]).sqrt();
                if k < 1e-12 {
                    continue;
                }
                let vh = qinv(&[v[0] / k, v[1] / k, v[2] / k, v[3] / k]);
                let u = self.links[4 * site + mu];
                self.links[4 * site + mu] = qnormalise(qmul(&qmul(&vh, &qinv(&u)), &vh));
            }
        }
    }
    /// One Metropolis sweep (Q3-0(b) cross-check only); returns the acceptance rate.
    pub fn metropolis_sweep(&mut self, beta: f64, eps: f64, hits: usize, rng: &mut SmallRng) -> f64 {
        let mut acc = 0usize;
        for site in 0..self.n_sites() {
            let x = self.coords(site);
            for mu in 0..4 {
                let v = self.staple(x, mu);
                for _ in 0..hits {
                    let u = self.links[4 * site + mu];
                    let r = eps * (2.0 * rng.uniform() - 1.0);
                    let n = loop {
                        let n = [0.0, 2.0 * rng.uniform() - 1.0, 2.0 * rng.uniform() - 1.0, 2.0 * rng.uniform() - 1.0];
                        let nn = n[1] * n[1] + n[2] * n[2] + n[3] * n[3];
                        if nn > 1e-6 && nn <= 1.0 {
                            break qnormalise(n);
                        }
                    };
                    let rot = [r.cos(), r.sin() * n[1], r.sin() * n[2], r.sin() * n[3]];
                    let un = qnormalise(qmul(&rot, &u));
                    let ds = beta * (qmul(&un, &v)[0] - qmul(&u, &v)[0]);
                    if ds >= 0.0 || rng.uniform() < ds.exp() {
                        self.links[4 * site + mu] = un;
                        acc += 1;
                    }
                }
            }
        }
        acc as f64 / (4 * self.n_sites() * hits) as f64
    }
    /// Apply a gauge transformation g(x): U_μ(x) → g(x) U_μ(x) g(x+μ)⁻¹ (for tests).
    pub fn gauge_transform(&mut self, g: &[Q]) {
        let old = self.links.clone();
        for site in 0..self.n_sites() {
            let x = self.coords(site);
            for mu in 0..4 {
                let y = self.site(self.shift(x, mu, true));
                self.links[4 * site + mu] = qmul(&qmul(&g[site], &old[4 * site + mu]), &qinv(&g[y]));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bessel_i(n: i32, x: f64) -> f64 {
        // series: sum_k (x/2)^(2k+n) / (k! (k+n)!)
        let mut s = 0.0;
        let mut term = (x / 2.0).powi(n) / (1..=n).map(|k| k as f64).product::<f64>();
        for k in 0..60 {
            s += term;
            term *= (x / 2.0).powi(2) / (((k + 1) * (k + 1 + n)) as f64);
        }
        s
    }

    fn mean_and_err(v: &[f64], blocks: usize) -> (f64, f64) {
        let bs = v.len() / blocks;
        let bm: Vec<f64> = (0..blocks).map(|b| v[b * bs..(b + 1) * bs].iter().sum::<f64>() / bs as f64).collect();
        let m = bm.iter().sum::<f64>() / blocks as f64;
        (m, (bm.iter().map(|x| (x - m).powi(2)).sum::<f64>() / ((blocks - 1) * blocks) as f64).sqrt())
    }

    #[test]
    fn plaquette_equals_link_times_staple() {
        let mut rng = SmallRng::seed_from_u64(3);
        let l = Lattice4d::hot([3, 4, 3, 5], &mut rng);
        for site in [0usize, 7, 59, 179] {
            let x = l.coords(site);
            assert_eq!(l.site(x), site);
            for mu in 0..4 {
                let lhs = qmul(&l.u(x, mu), &l.staple(x, mu))[0];
                let mut rhs = 0.0;
                for nu in 0..4 {
                    if nu != mu {
                        rhs += l.plaquette(x, mu, nu)[0] + l.plaquette(l.shift(x, nu, false), mu, nu)[0];
                    }
                }
                assert!((lhs - rhs).abs() < 1e-12, "site {site} mu {mu}");
            }
        }
    }

    #[test]
    fn q3_0c_plaquette_is_gauge_invariant() {
        let mut rng = SmallRng::seed_from_u64(5);
        let mut l = Lattice4d::hot([3, 3, 4, 3], &mut rng);
        let p = l.mean_plaquette();
        let g: Vec<Q> = (0..l.n_sites()).map(|_| sample_su2_link([0.0; 4], 1.0, &mut rng)).collect();
        l.gauge_transform(&g);
        assert!((l.mean_plaquette() - p).abs() < 1e-12);
    }

    #[test]
    fn q3_0a_strong_coupling() {
        let beta = 0.5;
        let u = bessel_i(2, beta) / bessel_i(1, beta);
        let mut rng = SmallRng::seed_from_u64(11);
        let mut l = Lattice4d::hot([4, 4, 4, 4], &mut rng);
        for _ in 0..50 {
            l.heatbath_sweep(beta, &mut rng);
        }
        let v: Vec<f64> = (0..400).map(|_| { l.heatbath_sweep(beta, &mut rng); l.mean_plaquette() }).collect();
        let (m, s) = mean_and_err(&v, 20);
        assert!((m - u).abs() < 4.0 * s + 5.0 * u.powi(5), "{m} ± {s} vs u = {u}");
    }

    #[test]
    fn q3_0b_heatbath_and_metropolis_agree() {
        let beta = 2.3;
        let mut rng = SmallRng::seed_from_u64(13);
        let mut a = Lattice4d::hot([4, 4, 4, 4], &mut rng);
        for _ in 0..100 {
            a.heatbath_sweep(beta, &mut rng);
            a.overrelax_sweep();
        }
        let va: Vec<f64> = (0..600)
            .map(|_| {
                a.heatbath_sweep(beta, &mut rng);
                a.overrelax_sweep();
                a.mean_plaquette()
            })
            .collect();
        let mut b = Lattice4d::hot([4, 4, 4, 4], &mut rng);
        for _ in 0..300 {
            b.metropolis_sweep(beta, 0.8, 4, &mut rng);
        }
        let vb: Vec<f64> = (0..1500).map(|_| { b.metropolis_sweep(beta, 0.8, 4, &mut rng); b.mean_plaquette() }).collect();
        let (ma, sa) = mean_and_err(&va, 20);
        let (mb, sb) = mean_and_err(&vb, 20);
        assert!((ma - mb).abs() < 4.0 * (sa * sa + sb * sb).sqrt(), "HB {ma} ± {sa}, Metropolis {mb} ± {sb}");
    }
}
