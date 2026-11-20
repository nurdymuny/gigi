//! Two-dimensional SU(2) lattice gauge theory for Q2 (docs/HALCYON_Q2_RUN_SPEC.md): a
//! Metropolis sampler on an N_s × N_t periodic lattice for any class-function plaquette
//! action, and the two time-slice series EFFECTIVE_MASS is run on (the Polyakov loop, whose
//! correlator is known exactly, and the plaquette row, which has no gap to find).
//!
//! Convention (the run spec's correction 1): β = 4/(g²a²) for both actions.
//! * Wilson:      w(U) = exp(β · ½ tr U).
//! * heat kernel: w(U) = Σ_j (2j+1) exp(−2 j(j+1)/β) χ_j(U), summed to 2j = `HK_TWO_J_MAX`.
//!
//! The sampler is self-contained (its own seeded xoshiro256** generator) so that a seed in a
//! receipt reproduces an ensemble bit for bit.

/// Highest 2j in the heat-kernel character sum (the run spec fixes 59).
pub const HK_TWO_J_MAX: usize = 59;

/// A unit quaternion (a0, a1, a2, a3) = a0 + i a·σ; ½ tr U = a0.
pub type Q = [f64; 4];

fn mul(p: &Q, q: &Q) -> Q {
    [
        p[0] * q[0] - p[1] * q[1] - p[2] * q[2] - p[3] * q[3],
        p[0] * q[1] + p[1] * q[0] + p[2] * q[3] - p[3] * q[2],
        p[0] * q[2] - p[1] * q[3] + p[2] * q[0] + p[3] * q[1],
        p[0] * q[3] + p[1] * q[2] - p[2] * q[1] + p[3] * q[0],
    ]
}

fn inv(q: &Q) -> Q {
    [q[0], -q[1], -q[2], -q[3]]
}

fn normalise(q: Q) -> Q {
    let n = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    [q[0] / n, q[1] / n, q[2] / n, q[3] / n]
}

/// xoshiro256** seeded by splitmix64.
pub struct Rng([u64; 4]);

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut z = seed;
        let mut next = || {
            z = z.wrapping_add(0x9E3779B97F4A7C15);
            let mut x = z;
            x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
            x ^ (x >> 31)
        };
        Rng([next(), next(), next(), next()])
    }
    fn u64(&mut self) -> u64 {
        let s = &mut self.0;
        let r = s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        r
    }
    /// Uniform in [0, 1).
    pub fn uniform(&mut self) -> f64 {
        (self.u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn normal(&mut self) -> f64 {
        let (u, v) = (self.uniform().max(1e-300), self.uniform());
        (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
    }
    fn haar(&mut self) -> Q {
        normalise([self.normal(), self.normal(), self.normal(), self.normal()])
    }
    /// exp(i ε r n·σ) with r uniform in [−1, 1] and n uniform on the sphere: a symmetric proposal.
    fn near_identity(&mut self, eps: f64) -> Q {
        let r = eps * (2.0 * self.uniform() - 1.0);
        let n = normalise([0.0, self.normal(), self.normal(), self.normal()]);
        let (c, s) = (r.cos(), r.sin());
        [c, s * n[1], s * n[2], s * n[3]]
    }
}

/// The plaquette action.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Wilson { beta: f64 },
    HeatKernel { beta: f64 },
}

impl Action {
    /// ln w(U) as a function of c = ½ tr U = cos θ.
    pub fn log_weight(&self, c: f64) -> f64 {
        match *self {
            Action::Wilson { beta } => beta * c,
            Action::HeatKernel { beta } => {
                // sin(nθ)/sin θ = U_{n−1}(cos θ) (Chebyshev, second kind); n = 2j + 1.
                let c = c.clamp(-1.0, 1.0);
                let (mut u_prev, mut u) = (0.0f64, 1.0f64); // U_{-1}, U_0
                let mut w = 0.0;
                for n in 1..=HK_TWO_J_MAX + 1 {
                    let nf = n as f64;
                    w += nf * (-(nf * nf - 1.0) / (2.0 * beta)).exp() * u;
                    let next = 2.0 * c * u - u_prev;
                    u_prev = u;
                    u = next;
                }
                assert!(w > 0.0, "heat-kernel weight not positive at c = {c} (beta too small for the truncation)");
                w.ln()
            }
        }
    }
    pub fn beta(&self) -> f64 {
        match *self {
            Action::Wilson { beta } | Action::HeatKernel { beta } => beta,
        }
    }
}

/// Links U_mu(x, t), mu = 0 spatial, 1 temporal, periodic in both directions.
pub struct Lattice2d {
    pub ns: usize,
    pub nt: usize,
    links: Vec<Q>,
}

impl Lattice2d {
    pub fn hot(ns: usize, nt: usize, rng: &mut Rng) -> Self {
        Lattice2d { ns, nt, links: (0..2 * ns * nt).map(|_| rng.haar()).collect() }
    }
    fn idx(&self, x: usize, t: usize, mu: usize) -> usize {
        2 * ((t % self.nt) * self.ns + (x % self.ns)) + mu
    }
    fn u(&self, x: usize, t: usize, mu: usize) -> Q {
        self.links[self.idx(x, t, mu)]
    }
    /// U_p(x, t) = U_s(x,t) U_t(x+1,t) U_s(x,t+1)⁻¹ U_t(x,t)⁻¹.
    pub fn plaquette(&self, x: usize, t: usize) -> Q {
        let a = mul(&self.u(x, t, 0), &self.u(x + 1, t, 1));
        let b = mul(&a, &inv(&self.u(x, t + 1, 0)));
        mul(&b, &inv(&self.u(x, t, 1)))
    }
    /// The two plaquettes (x, t) containing link (x, t, mu).
    fn plaquettes_of(&self, x: usize, t: usize, mu: usize) -> [(usize, usize); 2] {
        if mu == 0 {
            [(x, t), (x, (t + self.nt - 1) % self.nt)]
        } else {
            [(x, t), ((x + self.ns - 1) % self.ns, t)]
        }
    }
    /// One Metropolis sweep, `hits` proposals per link; returns the number accepted.
    pub fn sweep(&mut self, action: &Action, eps: f64, hits: usize, rng: &mut Rng) -> usize {
        let mut acc = 0;
        for t in 0..self.nt {
            for x in 0..self.ns {
                for mu in 0..2 {
                    let i = self.idx(x, t, mu);
                    let ps = self.plaquettes_of(x, t, mu);
                    for _ in 0..hits {
                        let old = self.links[i];
                        let s_old: f64 = ps.iter().map(|&(px, pt)| action.log_weight(self.plaquette(px, pt)[0])).sum();
                        self.links[i] = normalise(mul(&rng.near_identity(eps), &old));
                        let s_new: f64 = ps.iter().map(|&(px, pt)| action.log_weight(self.plaquette(px, pt)[0])).sum();
                        if (s_new - s_old) >= 0.0 || rng.uniform() < (s_new - s_old).exp() {
                            acc += 1;
                        } else {
                            self.links[i] = old;
                        }
                    }
                }
            }
        }
        acc
    }
    /// p(t) = ½ tr Π_x U_s(x, t), the spatial Polyakov loop at each time slice.
    pub fn polyakov(&self) -> Vec<f64> {
        (0..self.nt)
            .map(|t| {
                let mut p: Q = [1.0, 0.0, 0.0, 0.0];
                for x in 0..self.ns {
                    p = mul(&p, &self.u(x, t, 0));
                }
                p[0]
            })
            .collect()
    }
    /// The mean over x of ½ tr U_p(x, t) at each time slice.
    pub fn plaquette_rows(&self) -> Vec<f64> {
        (0..self.nt)
            .map(|t| (0..self.ns).map(|x| self.plaquette(x, t)[0]).sum::<f64>() / self.ns as f64)
            .collect()
    }
}

/// An ensemble: the two time-slice series per measurement, and the run's bookkeeping.
pub struct Ensemble {
    pub polyakov: Vec<Vec<f64>>,
    pub plaquette_rows: Vec<Vec<f64>>,
    pub acceptance: f64,
    pub eps: f64,
}

/// The proposal width for an action: about half the proposals accepted at the run spec's β.
pub fn default_eps(action: &Action) -> f64 {
    (2.0 / action.beta().sqrt()).min(std::f64::consts::PI)
}

pub fn generate(
    action: Action,
    ns: usize,
    nt: usize,
    n_therm: usize,
    n_meas: usize,
    every: usize,
    seed: u64,
) -> Ensemble {
    let mut rng = Rng::new(seed);
    let mut lat = Lattice2d::hot(ns, nt, &mut rng);
    let (eps, hits) = (default_eps(&action), 4);
    for _ in 0..n_therm {
        lat.sweep(&action, eps, hits, &mut rng);
    }
    let (mut acc, mut tried) = (0usize, 0usize);
    let (mut polyakov, mut plaquette_rows) = (Vec::with_capacity(n_meas), Vec::with_capacity(n_meas));
    for _ in 0..n_meas {
        for _ in 0..every {
            acc += lat.sweep(&action, eps, hits, &mut rng);
            tried += 2 * ns * nt * hits;
        }
        polyakov.push(lat.polyakov());
        plaquette_rows.push(lat.plaquette_rows());
    }
    Ensemble { polyakov, plaquette_rows, acceptance: acc as f64 / tried as f64, eps }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heat_kernel_weight_is_the_character_sum() {
        // at θ = π/2 (c = 0): χ_j = sin((2j+1)π/2) = ±1 for integer 2j+1 odd, 0 for even.
        let b = 4.0;
        let direct: f64 = (0..=HK_TWO_J_MAX)
            .map(|tj| {
                let n = (tj + 1) as f64;
                let th = std::f64::consts::FRAC_PI_2;
                n * (-(n * n - 1.0) / (2.0 * b)).exp() * (n * th).sin() / th.sin()
            })
            .sum();
        assert!((Action::HeatKernel { beta: b }.log_weight(0.0) - direct.ln()).abs() < 1e-12);
        // c = 1: χ_j = 2j+1
        let at1: f64 = (1..=HK_TWO_J_MAX + 1).map(|n| (n * n) as f64 * (-((n * n - 1) as f64) / (2.0 * b)).exp()).sum();
        assert!((Action::HeatKernel { beta: b }.log_weight(1.0) - at1.ln()).abs() < 1e-12);
    }

    #[test]
    fn gauge_invariance_of_the_plaquette_trace() {
        let mut rng = Rng::new(1);
        let mut lat = Lattice2d::hot(3, 4, &mut rng);
        let before: Vec<f64> = (0..4).flat_map(|t| (0..3).map(move |x| (x, t))).map(|(x, t)| lat.plaquette(x, t)[0]).collect();
        let pol = lat.polyakov();
        // gauge transform at every site: U_mu(x) -> g(x) U_mu(x) g(x+mu)^-1
        let g: Vec<Q> = (0..12).map(|_| rng.haar()).collect();
        let gi = |x: usize, t: usize| g[(t % 4) * 3 + (x % 3)];
        let mut new = lat.links.clone();
        for t in 0..4 {
            for x in 0..3 {
                for mu in 0..2 {
                    let (x2, t2) = if mu == 0 { (x + 1, t) } else { (x, t + 1) };
                    new[lat.idx(x, t, mu)] = mul(&mul(&gi(x, t), &lat.u(x, t, mu)), &inv(&gi(x2, t2)));
                }
            }
        }
        lat.links = new;
        let after: Vec<f64> = (0..4).flat_map(|t| (0..3).map(move |x| (x, t))).map(|(x, t)| lat.plaquette(x, t)[0]).collect();
        for (a, b) in before.iter().zip(&after) {
            assert!((a - b).abs() < 1e-12);
        }
        for (a, b) in pol.iter().zip(&lat.polyakov()) {
            assert!((a - b).abs() < 1e-12);
        }
    }

    /// Sampler sanity (run spec): <½ tr U_p> = λ_½ up to O(λ_½^A); here A = 64.
    fn plaquette_mean(action: Action, seed: u64) -> (f64, f64) {
        let e = generate(action, 4, 16, 300, 400, 2, seed);
        let vals: Vec<f64> = e.plaquette_rows.iter().map(|r| r.iter().sum::<f64>() / r.len() as f64).collect();
        let nb = 20;
        let bs = vals.len() / nb;
        let bm: Vec<f64> = (0..nb).map(|b| vals[b * bs..(b + 1) * bs].iter().sum::<f64>() / bs as f64).collect();
        let m = bm.iter().sum::<f64>() / nb as f64;
        let s = (bm.iter().map(|x| (x - m).powi(2)).sum::<f64>() / ((nb - 1) * nb) as f64).sqrt();
        (m, s)
    }

    #[test]
    fn plaquette_mean_is_lambda_half_for_both_actions() {
        let b = 4.0f64;
        let (m, s) = plaquette_mean(Action::HeatKernel { beta: b }, 5);
        let exact = (-1.5 / b).exp();
        assert!((m - exact).abs() < 4.0 * s, "heat kernel {m} ± {s} vs {exact}");
        // Wilson: λ_½ = I_2(4)/I_1(4) = 0.658047267... (scipy)
        let (m, s) = plaquette_mean(Action::Wilson { beta: b }, 6);
        let exact = 0.6580472673593596;
        assert!((m - exact).abs() < 4.0 * s, "Wilson {m} ± {s} vs {exact}");
    }
}
