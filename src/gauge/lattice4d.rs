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

/// Spatial APE smearing, one step, all spatial links at once from the old links:
/// U_i(x) → Proj_SU(2)[U_i(x) + α Σ_{j ≠ i spatial} (U_j(x) U_i(x+ĵ) U_j(x+î)⁻¹
///                                                 + U_j(x−ĵ)⁻¹ U_i(x−ĵ) U_j(x−ĵ+î))].
/// For SU(2) a sum of unit quaternions is a multiple of one, so normalising is the projection.
/// Time links are never smeared. Smeared links transform like links, so every operator built
/// from them stays gauge invariant.
pub fn ape_smear_spatial(lat: &Lattice4d, alpha: f64) -> Lattice4d {
    let mut out = lat.clone();
    for site in 0..lat.n_sites() {
        let x = lat.coords(site);
        for i in 1..4 {
            let mut acc = lat.u(x, i);
            let xi = lat.shift(x, i, true);
            for j in 1..4 {
                if j == i {
                    continue;
                }
                let xj = lat.shift(x, j, true);
                let up = qmul(&qmul(&lat.u(x, j), &lat.u(xj, i)), &qinv(&lat.u(xi, j)));
                let xmj = lat.shift(x, j, false);
                let ximj = lat.shift(xi, j, false);
                let dn = qmul(&qmul(&qinv(&lat.u(xmj, j)), &lat.u(xmj, i)), &lat.u(ximj, j));
                for k in 0..4 {
                    acc[k] += alpha * (up[k] + dn[k]);
                }
            }
            out.links[4 * site + i] = qnormalise(acc);
        }
    }
    out
}

/// Zero-momentum operators on each time slice t (= x₀):
/// * `scalar[t]`  = Σ over spatial sites and the three spatial planes of ½ tr U_ij  (0⁺⁺);
/// * `torelon[t]` = the mean over the three spatial directions i of the sum over transverse
///   positions of ½ tr of the Polyakov line winding direction i (the flux loop).
pub struct SliceOperators {
    pub scalar: Vec<f64>,
    pub torelon: Vec<f64>,
}

pub fn slice_operators(lat: &Lattice4d) -> SliceOperators {
    let nt = lat.dims[0];
    let mut scalar = vec![0.0; nt];
    let mut torelon = vec![0.0; nt];
    for site in 0..lat.n_sites() {
        let x = lat.coords(site);
        let t = x[0];
        for i in 1..4 {
            for j in i + 1..4 {
                scalar[t] += lat.plaquette(x, i, j)[0];
            }
            if x[i] == 0 {
                let mut p: Q = [1.0, 0.0, 0.0, 0.0];
                let mut y = x;
                for _ in 0..lat.dims[i] {
                    p = qmul(&p, &lat.u(y, i));
                    y = lat.shift(y, i, true);
                }
                torelon[t] += p[0] / 3.0;
            }
        }
    }
    SliceOperators { scalar, torelon }
}

/// The operators at each smearing level in `levels` (increasing; level n = n APE steps).
pub fn smeared_operators(lat: &Lattice4d, levels: &[usize], alpha: f64) -> Vec<SliceOperators> {
    let mut out = Vec::with_capacity(levels.len());
    let mut cur = lat.clone();
    let mut done = 0usize;
    for &n in levels {
        assert!(n >= done, "smearing levels must increase");
        for _ in done..n {
            cur = ape_smear_spatial(&cur, alpha);
        }
        done = n;
        out.push(slice_operators(&cur));
    }
    out
}

// ── Checkerboard-parallel updates ──────────────────────────────────────────────────────────
//
// Links U_μ(x) with the same μ and the same site parity (Σ x mod 2) never appear in each other's
// staples (the only μ-links in the staple of U_μ(x) sit at x ± ν, of the other parity), so each
// such class can be updated at once from the old lattice. This needs every extent even, and the
// parallel sweeps refuse otherwise. Every link draws from its own stream, seeded from
// (seed, sweep, μ, site) through splitmix64, so an ensemble is bit-identical for any thread count.
// These are new sweeps with their own reproducibility contract; the sequential sweeps above are
// unchanged.

fn splitmix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E3779B97F4A7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// The random stream of one link update.
pub fn link_rng(seed: u64, sweep: u64, mu: usize, site: usize) -> SmallRng {
    let h = splitmix(splitmix(splitmix(seed) ^ sweep) ^ ((mu as u64) << 48 | site as u64));
    SmallRng::seed_from_u64(h | 1)
}

/// Map `f` over `0..n` on `threads` scoped threads, results in order.
pub fn par_map<T: Send, F: Fn(usize) -> T + Sync>(n: usize, threads: usize, f: F) -> Vec<T> {
    let threads = threads.max(1).min(n.max(1));
    let chunk = n.div_ceil(threads);
    let f = &f;
    std::thread::scope(|sc| {
        let hs: Vec<_> = (0..threads)
            .map(|k| sc.spawn(move || (k * chunk..((k + 1) * chunk).min(n)).map(f).collect::<Vec<T>>()))
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    })
}

impl Lattice4d {
    fn require_even(&self) -> Result<(), String> {
        if self.dims.iter().any(|d| d % 2 == 1) {
            return Err(format!("checkerboard sweeps need every extent even (dims {:?})", self.dims));
        }
        Ok(())
    }
    fn parity_sites(&self) -> [Vec<usize>; 2] {
        let mut out = [Vec::new(), Vec::new()];
        for site in 0..self.n_sites() {
            let x = self.coords(site);
            out[(x[0] + x[1] + x[2] + x[3]) % 2].push(site);
        }
        out
    }
    /// One checkerboard heat-bath sweep (sweep index `sweep` keys the random streams).
    pub fn heatbath_sweep_par(&mut self, beta: f64, seed: u64, sweep: u64, threads: usize) -> Result<(), String> {
        self.require_even()?;
        let classes = self.parity_sites();
        for mu in 0..4 {
            for sites in &classes {
                let new = par_map(sites.len(), threads, |i| {
                    let site = sites[i];
                    let v = self.staple(self.coords(site), mu);
                    let mut rng = link_rng(seed, sweep, mu, site);
                    sample_su2_link(v, beta, &mut rng)
                });
                for (i, q) in new.into_iter().enumerate() {
                    self.links[4 * sites[i] + mu] = q;
                }
            }
        }
        Ok(())
    }
    /// One checkerboard overrelaxation sweep.
    pub fn overrelax_sweep_par(&mut self, threads: usize) -> Result<(), String> {
        self.require_even()?;
        let classes = self.parity_sites();
        for mu in 0..4 {
            for sites in &classes {
                let new = par_map(sites.len(), threads, |i| {
                    let site = sites[i];
                    let v = self.staple(self.coords(site), mu);
                    let u = self.links[4 * site + mu];
                    let k = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2] + v[3] * v[3]).sqrt();
                    if k < 1e-12 {
                        return u;
                    }
                    let vh = qinv(&[v[0] / k, v[1] / k, v[2] / k, v[3] / k]);
                    qnormalise(qmul(&qmul(&vh, &qinv(&u)), &vh))
                });
                for (i, q) in new.into_iter().enumerate() {
                    self.links[4 * sites[i] + mu] = q;
                }
            }
        }
        Ok(())
    }
}

/// `ape_smear_spatial` on `threads` threads (identical result: every new link reads old links only).
pub fn ape_smear_spatial_par(lat: &Lattice4d, alpha: f64, threads: usize) -> Lattice4d {
    let new = par_map(lat.n_sites(), threads, |site| {
        let x = lat.coords(site);
        let mut out = [[0.0; 4]; 3];
        for i in 1..4 {
            let mut acc = lat.u(x, i);
            let xi = lat.shift(x, i, true);
            for j in 1..4 {
                if j == i {
                    continue;
                }
                let xj = lat.shift(x, j, true);
                let up = qmul(&qmul(&lat.u(x, j), &lat.u(xj, i)), &qinv(&lat.u(xi, j)));
                let xmj = lat.shift(x, j, false);
                let ximj = lat.shift(xi, j, false);
                let dn = qmul(&qmul(&qinv(&lat.u(xmj, j)), &lat.u(xmj, i)), &lat.u(ximj, j));
                for k in 0..4 {
                    acc[k] += alpha * (up[k] + dn[k]);
                }
            }
            out[i - 1] = qnormalise(acc);
        }
        out
    });
    let mut l = lat.clone();
    for (site, q) in new.into_iter().enumerate() {
        for i in 1..4 {
            l.links[4 * site + i] = q[i - 1];
        }
    }
    l
}

/// `smeared_operators` with parallel smearing.
pub fn smeared_operators_par(lat: &Lattice4d, levels: &[usize], alpha: f64, threads: usize) -> Vec<SliceOperators> {
    let mut out = Vec::with_capacity(levels.len());
    let mut cur = lat.clone();
    let mut done = 0usize;
    for &n in levels {
        assert!(n >= done, "smearing levels must increase");
        for _ in done..n {
            cur = ape_smear_spatial_par(&cur, alpha, threads);
        }
        done = n;
        out.push(slice_operators(&cur));
    }
    out
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

    #[test]
    fn smeared_operators_are_gauge_invariant() {
        let mut rng = SmallRng::seed_from_u64(17);
        let mut l = Lattice4d::hot([4, 3, 3, 4], &mut rng);
        for _ in 0..5 {
            l.heatbath_sweep(2.3, &mut rng);
        }
        let a = smeared_operators(&l, &[0, 2, 5], 0.5);
        let g: Vec<Q> = (0..l.n_sites()).map(|_| sample_su2_link([0.0; 4], 1.0, &mut rng)).collect();
        l.gauge_transform(&g);
        let b = smeared_operators(&l, &[0, 2, 5], 0.5);
        for (x, y) in a.iter().zip(&b) {
            for t in 0..4 {
                assert!((x.scalar[t] - y.scalar[t]).abs() < 1e-11, "scalar t={t}");
                assert!((x.torelon[t] - y.torelon[t]).abs() < 1e-11, "torelon t={t}");
            }
        }
    }

    #[test]
    fn smearing_raises_the_spatial_plaquette_and_keeps_time_links() {
        let mut rng = SmallRng::seed_from_u64(19);
        let mut l = Lattice4d::hot([3, 4, 4, 4], &mut rng);
        for _ in 0..10 {
            l.heatbath_sweep(2.3, &mut rng);
        }
        let s = ape_smear_spatial(&l, 0.5);
        for site in 0..l.n_sites() {
            assert_eq!(s.links[4 * site], l.links[4 * site]);
        }
        let ops0 = slice_operators(&l);
        let ops1 = slice_operators(&s);
        assert!(ops1.scalar.iter().sum::<f64>() > ops0.scalar.iter().sum::<f64>());
        // the cold lattice is a fixed point of smearing
        let c = Lattice4d::cold([2, 3, 3, 3]);
        let cs = ape_smear_spatial(&c, 0.5);
        assert!(cs.links.iter().all(|q| (q[0] - 1.0).abs() < 1e-15));
    }

    #[test]
    fn checkerboard_is_thread_count_independent_and_refuses_odd_extents() {
        let mut rng = SmallRng::seed_from_u64(23);
        let base = Lattice4d::hot([4, 4, 4, 6], &mut rng);
        let run = |threads: usize| {
            let mut l = base.clone();
            for sw in 0..3 {
                l.heatbath_sweep_par(2.3, 99, sw, threads).unwrap();
                l.overrelax_sweep_par(threads).unwrap();
            }
            l.links
        };
        assert_eq!(run(1), run(7));
        let mut odd = Lattice4d::hot([3, 4, 4, 4], &mut rng);
        assert!(odd.heatbath_sweep_par(2.3, 1, 0, 2).is_err());
    }

    #[test]
    fn parallel_smearing_equals_serial() {
        let mut rng = SmallRng::seed_from_u64(29);
        let l = Lattice4d::hot([4, 4, 4, 4], &mut rng);
        let a = ape_smear_spatial(&l, 0.5);
        let b = ape_smear_spatial_par(&l, 0.5, 5);
        assert_eq!(a.links, b.links);
    }

    #[test]
    fn checkerboard_sampler_passes_q3_0a_and_0b() {
        let mut rng = SmallRng::seed_from_u64(31);
        // (a) strong coupling
        let beta = 0.5;
        let u = bessel_i(2, beta) / bessel_i(1, beta);
        let mut l = Lattice4d::hot([4, 4, 4, 4], &mut rng);
        let mut sw = 0u64;
        for _ in 0..50 {
            l.heatbath_sweep_par(beta, 7, sw, 4).unwrap();
            sw += 1;
        }
        let v: Vec<f64> = (0..400)
            .map(|_| {
                l.heatbath_sweep_par(beta, 7, sw, 4).unwrap();
                sw += 1;
                l.mean_plaquette()
            })
            .collect();
        let (m, s) = mean_and_err(&v, 20);
        assert!((m - u).abs() < 4.0 * s + 5.0 * u.powi(5), "{m} ± {s} vs u = {u}");
        // (b) against Metropolis at beta = 2.3
        let beta = 2.3;
        let mut a = Lattice4d::hot([4, 4, 4, 4], &mut rng);
        for _ in 0..100 {
            a.heatbath_sweep_par(beta, 8, sw, 4).unwrap();
            a.overrelax_sweep_par(4).unwrap();
            sw += 1;
        }
        let va: Vec<f64> = (0..600)
            .map(|_| {
                a.heatbath_sweep_par(beta, 8, sw, 4).unwrap();
                a.overrelax_sweep_par(4).unwrap();
                sw += 1;
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
        assert!((ma - mb).abs() < 4.0 * (sa * sa + sb * sb).sqrt(), "checkerboard {ma} ± {sa}, Metropolis {mb} ± {sb}");
    }
}
