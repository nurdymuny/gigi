//! Chebyshev-filtered interior eigensolver for the magnetic Laplacian
//! BULK arm (Hallie's 2026-07-17 RH ask, Part 2 — the sparse interior).
//!
//! WHY this exists (and why it is NOT the June-30 σ = 0 spec): the dense
//! `SPECTRAL_GAUGE … MODE MAGNETIC BULK k` path tops out at V = 4096
//! (opt-in 8192) because it materializes the whole V×V complex-Hermitian
//! Laplacian and runs `nalgebra::SymmetricEigen` (O(V³) work, O(V²)
//! memory). To settle whether the 3D magnetic lattice reaches rigid GUE
//! in the V → ∞ limit, Halcyon needs V = 13824 (L = 24) and 32768
//! (L = 32), far past the dense ceiling. Riemann-Hypothesis / number-
//! variance statistics live in the BULK — a contiguous window of
//! consecutive levels at the spectral CENTER — so this is an INTERIOR
//! eigenproblem, genuinely different from the σ = 0 bottom-of-spectrum
//! shift-invert arm (which serves the YM mass gap).
//!
//! THE METHOD (Option B — pure-Rust Chebyshev-filtered subspace, no new
//! dependency):
//!   1. Assemble the magnetic Laplacian as a complex CSR structure
//!      ([`MagneticCsr`]) — never a dense V×V matrix. Assembly convention
//!      matches the dense magnetic path in `src/spectral.rs` EXACTLY
//!      (anchors SP4/SP5): per edge `(i,j,θ)`, `L[i,j] -= e^{+iθ}`,
//!      `L[j,i] -= e^{−iθ}`, `diag[i] += 1`, `diag[j] += 1`.
//!   2. Estimate the spectral band with Gershgorin: every eigenvalue of
//!      the magnetic Laplacian lies in `[0, 2·deg_max]` (PSD, row radius =
//!      diagonal). A guaranteed, zero-matvec enclosure — pad outward so no
//!      eigenvalue maps outside `[−1,1]` after the affine rescale.
//!   3. Build a Jackson-damped Chebyshev BANDPASS filter `p(L)` that
//!      amplifies the target window and damps outside. Jackson damping
//!      suppresses the Gibbs ringing that would otherwise leak
//!      out-of-band levels into the subspace — that leakage is precisely
//!      the completeness/merge failure mode SP2 guards.
//!   4. Block subspace iteration: a random block of dimension
//!      `b = k + oversample`, apply `p(L)` each pass, FULL twice-is-enough
//!      complex reorthogonalization (mirrors the Gram-Schmidt idiom in
//!      `src/sharded/spectral.rs`, adapted to `Complex<f64>` conjugate
//!      inner products), until the in-window Rayleigh-Ritz values
//!      stabilize.
//!   5. Rayleigh-Ritz: project onto the subspace (small dense b×b complex
//!      Hermitian `H = QᴴLQ`), solve with `nalgebra::SymmetricEigen`
//!      (the SAME call the dense path already trusts), keep the Ritz pairs
//!      in the target window.
//!
//! COMPLETENESS (the crux — Hallie's ask #3): a filtered subspace method
//! can silently MISS a bulk level (merge a near-degenerate pair, or drop
//! one at the filter edge). Three honest defenses, none a ground-truth
//! certificate at V = 32768 (none exists there):
//!   1. RESIDUAL GATE — every returned pair has `‖Lv − λv‖/‖v‖ < RES_TOL`,
//!      so each returned value is a TRUE eigenpair (not a filter artifact).
//!   2. COUNT — the converged in-window count must equal the requested k;
//!      fewer signals a possible miss → grow the subspace / raise the
//!      filter degree and retry (bounded); still short → return
//!      `converged < k` with `fully_converged = false` (honest, never a
//!      silent claim of k).
//!   3. OVERSAMPLE — the block is over-dimensioned (the completeness
//!      margin against near-degenerate merges).
//! Plus a cheap NON-authoritative KPM count-below-edge as a contiguity
//! sanity number (O(1/√probes) noise — flags a gross miss, not a proof).
//!
//! Validated EXACTLY vs dense ground truth for V ≤ 2048 in
//! `tests/spectral_interior_basic.rs` (SP1/SP2/SP4): same values, count,
//! multiplicities, no miss, no extra.

#![cfg(feature = "gauge")]

use std::collections::HashMap;

use nalgebra::{Complex, DMatrix, SymmetricEigen};

use crate::spectral::{BulkCenter, BulkSpec, SpectralGaugeError};

/// Complex scalar shorthand.
type C = Complex<f64>;

/// Residual gate: a returned Ritz pair `(λ, v)` is certified a true
/// eigenpair of the magnetic Laplacian when `‖L v − λ v‖ / ‖v‖ < RES_TOL`.
///
/// Pinned at `1e-8`. Dense `nalgebra::SymmetricEigen` residuals run
/// ~1e-12…1e-14, so `1e-8` is a comfortable, honest iterative gate that
/// still certifies each returned pair is a genuine eigenpair while
/// tolerating the finite Chebyshev filter + block-iteration accuracy. The
/// achieved `max_residual` is surfaced on the result so a caller can gate
/// independently (or tighten).
pub const RES_TOL: f64 = 1e-8;

/// Parity tolerance for the CSR matvec vs a dense `L·x` (assembly
/// correctness, anchor SP5). Tighter than [`RES_TOL`] because it is an
/// exact-arithmetic identity up to floating-point summation order.
pub const MATVEC_PARITY_TOL: f64 = 1e-12;

/// Tunables for the interior solver. `Default` gives the pinned
/// production values; tests override `seed` / iteration bounds.
#[derive(Debug, Clone)]
pub struct InteriorConfig {
    /// Residual gate (defaults to [`RES_TOL`]).
    pub res_tol: f64,
    /// Explicit oversample; `None` → auto `max(⌈0.4·k⌉, 60)`.
    pub oversample: Option<usize>,
    /// Max block-subspace iterations per completeness attempt.
    pub max_subspace_iters: usize,
    /// Max completeness retries (grow subspace / raise degree) when the
    /// converged in-window count is short of k.
    pub max_completeness_retries: usize,
    /// Deterministic seed for the initial random complex block.
    pub seed: u64,
    /// KPM moments for the DOS / median / count-below estimates.
    pub kpm_moments: usize,
    /// KPM stochastic probe vectors.
    pub kpm_probes: usize,
    /// Base Chebyshev filter degree floor (raised adaptively).
    pub min_filter_degree: usize,
    /// Hard cap on the Chebyshev filter degree (retries stop growing here).
    pub max_filter_degree: usize,
}

impl Default for InteriorConfig {
    fn default() -> Self {
        InteriorConfig {
            res_tol: RES_TOL,
            oversample: None,
            max_subspace_iters: 40,
            max_completeness_retries: 4,
            seed: 0x9E3779B97F4A7C15,
            kpm_moments: 400,
            kpm_probes: 24,
            min_filter_degree: 24,
            max_filter_degree: 4000,
        }
    }
}

/// Result of the interior BULK solve — the SAME window shape the dense
/// BULK path returns, plus the sparse convergence receipt.
#[derive(Debug, Clone)]
pub struct InteriorResult {
    /// The k-eigenvalue window, ascending. Length == `converged` (the
    /// pairs that passed the residual gate inside the target window).
    pub eigenvalues: Vec<f64>,
    /// The spectral center the window was built around (Auto → the KPM
    /// median-value estimate; AROUND σ → σ; IN [a,b] → the midpoint).
    pub bulk_center: f64,
    /// Number of converged pairs returned in the window (== `requested_k`
    /// when fully converged).
    pub converged: usize,
    /// The k that was requested (after clamping to V).
    pub requested_k: usize,
    /// Max over the returned pairs of `‖L v − λ v‖ / ‖v‖`.
    pub max_residual: f64,
    /// Block-subspace iterations consumed (summed across retries).
    pub iterations: usize,
    /// Completeness retries consumed (subspace grows / degree bumps).
    pub restarts: usize,
    /// `converged == requested_k` — the honest fully-converged flag. When
    /// `false`, the caller has FEWER than k certified pairs (a possible
    /// missed bulk level); never silently claims k.
    pub fully_converged: bool,
    /// Stochastic (KPM/Hutchinson) count of eigenvalues strictly below the
    /// window's lower edge — a cheap, NON-authoritative contiguity sanity
    /// number (O(1/√probes) noise), not a certificate.
    pub count_below_estimate: f64,
}

// ───────────────────────────── Complex CSR ─────────────────────────────

/// Complex CSR magnetic Laplacian. Off-diagonals are stored explicitly
/// (accumulated per edge, so parallel edges add); the diagonal is the
/// real per-vertex degree. Never materializes a dense V×V matrix.
///
/// Assembly convention MUST match the dense magnetic path
/// (`src/spectral.rs`) EXACTLY (anchors SP4/SP5): for each edge
/// `(i, j, θ)` with `i ≠ j`,
/// `L[i,j] -= e^{+iθ}`, `L[j,i] -= e^{−iθ}`, `diag[i] += 1`, `diag[j] += 1`.
pub struct MagneticCsr {
    n: usize,
    row_ptr: Vec<usize>,
    col_idx: Vec<usize>,
    off: Vec<C>,
    diag: Vec<f64>,
    max_deg: f64,
}

impl MagneticCsr {
    /// Assemble the complex CSR magnetic Laplacian from an edge list
    /// `(i, j, θ)`. Self-loops (`i == j`) are skipped (matching the dense
    /// path). Parallel edges accumulate.
    pub fn assemble(edges: &[(usize, usize, f64)], n: usize) -> Self {
        // Per-row accumulator: column → summed complex entry. Parallel
        // edges and their conjugate partners fold in here so the CSR is
        // bit-for-bit the dense assembly (up to summation order).
        let mut rows: Vec<HashMap<usize, C>> = vec![HashMap::new(); n];
        let mut diag = vec![0.0f64; n];
        for &(i, j, theta) in edges {
            if i == j {
                continue; // skip self-loops (matches spectral.rs)
            }
            let phase = C::new(theta.cos(), theta.sin());
            *rows[i].entry(j).or_insert(C::new(0.0, 0.0)) -= phase;
            *rows[j].entry(i).or_insert(C::new(0.0, 0.0)) -= phase.conj();
            diag[i] += 1.0;
            diag[j] += 1.0;
        }
        let mut row_ptr = vec![0usize; n + 1];
        let mut col_idx = Vec::new();
        let mut off = Vec::new();
        for (r, row) in rows.iter().enumerate() {
            let mut cols: Vec<usize> = row.keys().copied().collect();
            cols.sort_unstable();
            for c in cols {
                col_idx.push(c);
                off.push(row[&c]);
            }
            row_ptr[r + 1] = col_idx.len();
        }
        let max_deg = diag.iter().copied().fold(0.0, f64::max);
        MagneticCsr { n, row_ptr, col_idx, off, diag, max_deg }
    }

    /// `y = L · x` — one complex CSR matvec.
    pub fn matvec(&self, x: &[C]) -> Vec<C> {
        let mut y = vec![C::new(0.0, 0.0); self.n];
        self.matvec_into(x, &mut y);
        y
    }

    /// `y ← L · x` into a caller-provided buffer (no allocation).
    fn matvec_into(&self, x: &[C], y: &mut [C]) {
        for r in 0..self.n {
            let mut acc = x[r].scale(self.diag[r]);
            for idx in self.row_ptr[r]..self.row_ptr[r + 1] {
                acc += self.off[idx] * x[self.col_idx[idx]];
            }
            y[r] = acc;
        }
    }

    /// Vertex count (matrix dimension).
    pub fn n(&self) -> usize {
        self.n
    }

    /// Maximum vertex degree (the Gershgorin upper-bound seed: every
    /// eigenvalue lies in `[0, 2·deg_max]`).
    pub fn max_degree(&self) -> f64 {
        self.max_deg
    }
}

// ─────────────────────── complex vector primitives ─────────────────────

/// Conjugate inner product ⟨u, v⟩ = Σ conj(u_i)·v_i.
#[inline]
fn cdot(u: &[C], v: &[C]) -> C {
    let mut s = C::new(0.0, 0.0);
    for i in 0..u.len() {
        s += u[i].conj() * v[i];
    }
    s
}

/// `y ← y + a·x` for complex `a`.
#[inline]
fn caxpy(y: &mut [C], a: C, x: &[C]) {
    for i in 0..y.len() {
        y[i] += a * x[i];
    }
}

/// Euclidean norm √(Re⟨v,v⟩).
#[inline]
fn cnorm(v: &[C]) -> f64 {
    let mut s = 0.0;
    for z in v {
        s += z.norm_sqr();
    }
    s.sqrt()
}

// ───────────────────────────── RNG (splitmix) ──────────────────────────

struct SplitMix(u64);
impl SplitMix {
    fn new(seed: u64) -> Self {
        SplitMix(seed.wrapping_add(0x9E3779B97F4A7C15))
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// Standard normal via Box-Muller.
    fn normal(&mut self) -> f64 {
        let u1 = self.unit().max(1e-15);
        let u2 = self.unit();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
    /// Complex Gaussian entry.
    fn complex_normal(&mut self) -> C {
        C::new(self.normal(), self.normal())
    }
}

// ─────────────────────── Chebyshev / Jackson filter ────────────────────

/// Affine rescale to the Chebyshev domain: `Ã x = sa·(L x) − sb·x`, so
/// the spectrum `[lo_d, hi_d]` maps into `[−1, 1]`.
struct Rescale {
    sa: f64,
    sb: f64,
}
impl Rescale {
    fn new(lo_d: f64, hi_d: f64) -> Self {
        let sa = 2.0 / (hi_d - lo_d);
        let sb = (hi_d + lo_d) / (hi_d - lo_d);
        Rescale { sa, sb }
    }
    /// λ → t ∈ [−1,1] (clamped to guard the recurrence).
    fn to_cheb(&self, lambda: f64) -> f64 {
        (self.sa * lambda - self.sb).clamp(-1.0, 1.0)
    }
    /// `out ← Ã x = sa·(L x) − sb·x`, using scratch `lx`.
    fn apply(&self, csr: &MagneticCsr, x: &[C], lx: &mut [C], out: &mut [C]) {
        csr.matvec_into(x, lx);
        for i in 0..x.len() {
            out[i] = lx[i].scale(self.sa) - x[i].scale(self.sb);
        }
    }
}

/// Jackson damping kernel `g_l` for a degree-`m` expansion (l = 0..=m).
/// Suppresses Gibbs ringing so out-of-band levels do not leak into the
/// subspace (the completeness/merge guard).
fn jackson_kernel(m: usize) -> Vec<f64> {
    let mp1 = (m + 1) as f64;
    let cot = (std::f64::consts::PI / mp1).cos() / (std::f64::consts::PI / mp1).sin();
    (0..=m)
        .map(|l| {
            let lf = l as f64;
            ((mp1 - lf) * (std::f64::consts::PI * lf / mp1).cos()
                + (std::f64::consts::PI * lf / mp1).sin() * cot)
                / mp1
        })
        .collect()
}

/// Chebyshev coefficients of the ideal bandpass indicator `1_{[ta,tb]}`
/// on `[−1,1]` (l = 0..=m). `ta < tb` are the rescaled window edges.
fn bandpass_coeffs(ta: f64, tb: f64, m: usize) -> Vec<f64> {
    let aca = ta.clamp(-1.0, 1.0).acos();
    let acb = tb.clamp(-1.0, 1.0).acos();
    let pi = std::f64::consts::PI;
    (0..=m)
        .map(|l| {
            if l == 0 {
                (aca - acb) / pi
            } else {
                let lf = l as f64;
                (2.0 / (pi * lf)) * ((lf * aca).sin() - (lf * acb).sin())
            }
        })
        .collect()
}

/// A ready-to-apply Jackson-damped Chebyshev bandpass filter `p(Ã)`.
struct BandpassFilter {
    coeff: Vec<f64>, // c_l · g_l, precombined
    degree: usize,
}
impl BandpassFilter {
    fn new(ta: f64, tb: f64, degree: usize) -> Self {
        let g = jackson_kernel(degree);
        let c = bandpass_coeffs(ta, tb, degree);
        let coeff: Vec<f64> = (0..=degree).map(|l| c[l] * g[l]).collect();
        BandpassFilter { coeff, degree }
    }

    /// `out ← p(Ã)·x` via the three-term Chebyshev recurrence. Uses only
    /// the complex CSR matvec (through `rescale`) + axpy — no dense L.
    fn apply(&self, csr: &MagneticCsr, rescale: &Rescale, x: &[C], scratch: &mut FilterScratch) {
        let n = x.len();
        // t0 = x  (T_0). out = coeff[0]·t0.
        scratch.t_prev.copy_from_slice(x);
        for i in 0..n {
            scratch.out[i] = scratch.t_prev[i].scale(self.coeff[0]);
        }
        if self.degree >= 1 {
            // t1 = Ã x  (T_1). out += coeff[1]·t1.
            rescale.apply(csr, x, &mut scratch.lx, &mut scratch.t_cur);
            let c1 = self.coeff[1];
            for i in 0..n {
                scratch.out[i] += scratch.t_cur[i].scale(c1);
            }
        }
        for l in 2..=self.degree {
            // t_next = 2 Ã t_cur − t_prev
            rescale.apply(csr, &scratch.t_cur, &mut scratch.lx, &mut scratch.t_next);
            for i in 0..n {
                scratch.t_next[i] = scratch.t_next[i].scale(2.0) - scratch.t_prev[i];
            }
            let cl = self.coeff[l];
            for i in 0..n {
                scratch.out[i] += scratch.t_next[i].scale(cl);
            }
            std::mem::swap(&mut scratch.t_prev, &mut scratch.t_cur);
            std::mem::swap(&mut scratch.t_cur, &mut scratch.t_next);
        }
    }
}

/// Reusable scratch buffers for one filter application (length n each).
struct FilterScratch {
    t_prev: Vec<C>,
    t_cur: Vec<C>,
    t_next: Vec<C>,
    lx: Vec<C>,
    out: Vec<C>,
}
impl FilterScratch {
    fn new(n: usize) -> Self {
        let z = || vec![C::new(0.0, 0.0); n];
        FilterScratch { t_prev: z(), t_cur: z(), t_next: z(), lx: z(), out: z() }
    }
}

// ──────────────────────────── KPM DOS estimate ─────────────────────────

/// Kernel-polynomial-method DOS moments + integrated count. Reuses the
/// SAME complex CSR matvec / Chebyshev recurrence as the filter. Gives the
/// median-value estimate (Auto center), the window half-width for a target
/// level count, and the NON-authoritative count-below-edge sanity number.
struct KpmDos {
    mu: Vec<f64>, // μ_l = (1/S) Σ Re zᴴ T_l(Ã) z, μ_0 ≈ n
    g: Vec<f64>,  // Jackson kernel
    n: usize,
}
impl KpmDos {
    fn new(csr: &MagneticCsr, rescale: &Rescale, moments: usize, probes: usize, seed: u64) -> Self {
        let n = csr.n();
        let m = moments.max(2).min(4 * n + 8);
        let mut mu = vec![0.0f64; m + 1];
        let mut rng = SplitMix::new(seed ^ 0xA5A5_1234_5678_9ABC);
        let mut t_prev = vec![C::new(0.0, 0.0); n];
        let mut t_cur = vec![C::new(0.0, 0.0); n];
        let mut t_next = vec![C::new(0.0, 0.0); n];
        let mut lx = vec![C::new(0.0, 0.0); n];
        for _s in 0..probes.max(1) {
            // Rademacher-complex probe: entries (±1 ± i)/√2, E[z zᴴ] = I.
            let z: Vec<C> = (0..n)
                .map(|_| {
                    let re = if rng.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
                    let im = if rng.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
                    C::new(re, im).scale(std::f64::consts::FRAC_1_SQRT_2)
                })
                .collect();
            t_prev.copy_from_slice(&z);
            rescale.apply(csr, &z, &mut lx, &mut t_cur);
            mu[0] += cdot(&z, &t_prev).re;
            mu[1] += cdot(&z, &t_cur).re;
            for l in 2..=m {
                rescale.apply(csr, &t_cur, &mut lx, &mut t_next);
                for i in 0..n {
                    t_next[i] = t_next[i].scale(2.0) - t_prev[i];
                }
                mu[l] += cdot(&z, &t_next).re;
                std::mem::swap(&mut t_prev, &mut t_cur);
                std::mem::swap(&mut t_cur, &mut t_next);
            }
        }
        let inv = 1.0 / probes.max(1) as f64;
        for v in mu.iter_mut() {
            *v *= inv;
        }
        KpmDos { mu, g: jackson_kernel(m), n }
    }

    /// Estimated count of eigenvalues ≤ the value with rescaled coordinate
    /// `t`. Closed form of ∫_{−1}^{t} ρ(t')dt' (KPM integrated DOS):
    /// `N(t) = n(1 − θ/π) − (2/π) Σ_{l≥1} g_l μ_l sin(lθ)/l`, θ = acos(t).
    fn count_below_t(&self, t: f64) -> f64 {
        let t = t.clamp(-1.0, 1.0);
        let theta = t.acos();
        let pi = std::f64::consts::PI;
        let mut s = self.mu[0] * (1.0 - theta / pi);
        for l in 1..self.mu.len() {
            let lf = l as f64;
            s -= (2.0 / pi) * self.g[l] * self.mu[l] * (lf * theta).sin() / lf;
        }
        s.clamp(0.0, self.n as f64)
    }

    fn count_below_lambda(&self, rescale: &Rescale, lambda: f64) -> f64 {
        self.count_below_t(rescale.to_cheb(lambda))
    }

    /// Number of eigenvalues in `[a,b]` (estimate).
    fn count_in(&self, rescale: &Rescale, a: f64, b: f64) -> f64 {
        (self.count_below_lambda(rescale, b) - self.count_below_lambda(rescale, a)).max(0.0)
    }

    /// The value whose estimated rank is `⌊n/2⌋` (the positional median),
    /// by bisection on the monotone integrated count. Consistent with the
    /// dense pinned Auto center (positional median).
    fn median_lambda(&self, rescale: &Rescale, lo_d: f64, hi_d: f64) -> f64 {
        let target = self.n as f64 / 2.0;
        let mut lo = lo_d;
        let mut hi = hi_d;
        for _ in 0..80 {
            let mid = 0.5 * (lo + hi);
            if self.count_below_lambda(rescale, mid) < target {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        0.5 * (lo + hi)
    }

    /// Half-width `h` about `center` so that `[center−h, center+h]` holds
    /// ~`levels` eigenvalues (bisection on the monotone in-band count).
    fn half_width_for(
        &self,
        rescale: &Rescale,
        center: f64,
        levels: f64,
        span: f64,
    ) -> f64 {
        let mut lo = 0.0;
        let mut hi = span;
        for _ in 0..60 {
            let mid = 0.5 * (lo + hi);
            if self.count_in(rescale, center - mid, center + mid) < levels {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        0.5 * (lo + hi)
    }
}

// ────────────────────────── block subspace core ────────────────────────

/// One Rayleigh-Ritz pair after a subspace sweep — the value and its
/// residual-gate number (the eigenvectors are not surfaced).
struct RitzPair {
    value: f64,
    residual: f64,
}

/// Result of a subspace-iteration run at a fixed (filter, b_dim).
struct SweepOut {
    pairs: Vec<RitzPair>,
    iterations: usize,
}

/// Run block subspace iteration with the Chebyshev bandpass filter,
/// twice-is-enough complex reorthogonalization, and Rayleigh-Ritz, until
/// the in-window Ritz values stabilize or `max_iters` is hit.
#[allow(clippy::too_many_arguments)]
fn subspace_iterate(
    csr: &MagneticCsr,
    rescale: &Rescale,
    filter: &BandpassFilter,
    b_dim: usize,
    window_lo: f64,
    window_hi: f64,
    res_tol: f64,
    max_iters: usize,
    seed: u64,
) -> SweepOut {
    let n = csr.n();
    let b_dim = b_dim.min(n).max(1);

    // Initial random complex block (columns).
    let mut rng = SplitMix::new(seed ^ 0xC0FF_EE00_1234_5678);
    let mut block: Vec<Vec<C>> = (0..b_dim)
        .map(|_| (0..n).map(|_| rng.complex_normal()).collect())
        .collect();

    let mut scratch = FilterScratch::new(n);
    let mut lb: Vec<Vec<C>> = vec![vec![C::new(0.0, 0.0); n]; b_dim];
    let mut prev_window: Option<Vec<f64>> = None;
    let mut iterations = 0;
    let mut last_pairs: Vec<RitzPair> = Vec::new();

    for _it in 0..max_iters.max(1) {
        iterations += 1;

        // 1. Apply the bandpass filter to every column.
        for col in block.iter_mut() {
            filter.apply(csr, rescale, col, &mut scratch);
            col.copy_from_slice(&scratch.out);
        }

        // 2. FULL twice-is-enough complex reorthonormalization.
        reorthonormalize(&mut block, &mut rng);

        // 3. LB = L · block.
        for q in 0..b_dim {
            csr.matvec_into(&block[q], &mut lb[q]);
        }

        // 4. Rayleigh-Ritz projection H = Bᴴ L B (b×b complex Hermitian).
        let mut h = DMatrix::<C>::zeros(b_dim, b_dim);
        for p in 0..b_dim {
            for q in 0..b_dim {
                h[(p, q)] = cdot(&block[p], &lb[q]);
            }
        }
        // Symmetrize to kill FP asymmetry (mirrors the dense conjugate
        // assembly): H ← (H + Hᴴ)/2.
        for p in 0..b_dim {
            for q in (p + 1)..b_dim {
                let avg = (h[(p, q)] + h[(q, p)].conj()).scale(0.5);
                h[(p, q)] = avg;
                h[(q, p)] = avg.conj();
            }
            h[(p, p)] = C::new(h[(p, p)].re, 0.0);
        }
        let eig = SymmetricEigen::new(h);
        let theta: Vec<f64> = eig.eigenvalues.iter().copied().collect();
        let z = eig.eigenvectors; // b×b complex, columns = Ritz coords

        // 5. Ritz vectors Y = B·Z and LY = LB·Z; residual per pair.
        let mut pairs: Vec<RitzPair> = Vec::with_capacity(b_dim);
        let mut vectors: Vec<Vec<C>> = Vec::with_capacity(b_dim);
        for q in 0..b_dim {
            let mut y = vec![C::new(0.0, 0.0); n];
            let mut ly = vec![C::new(0.0, 0.0); n];
            for p in 0..b_dim {
                let zc = z[(p, q)];
                caxpy(&mut y, zc, &block[p]);
                caxpy(&mut ly, zc, &lb[p]);
            }
            // residual = ‖L y − θ y‖ / ‖y‖.
            let yn = cnorm(&y).max(1e-300);
            let mut r = 0.0;
            for i in 0..n {
                r += (ly[i] - y[i].scale(theta[q])).norm_sqr();
            }
            let residual = r.sqrt() / yn;
            pairs.push(RitzPair { value: theta[q], residual });
            vectors.push(y);
        }

        // 6. Rayleigh-Ritz restart: rotate the block onto the Ritz vectors
        //    (best available starting subspace for the next filter pass).
        for (q, v) in vectors.iter().enumerate() {
            block[q].copy_from_slice(v);
        }

        // 7. Convergence: the k-nearest-center CONVERGED values are stable.
        let mut in_window: Vec<f64> = pairs
            .iter()
            .filter(|p| p.residual < res_tol && p.value >= window_lo && p.value <= window_hi)
            .map(|p| p.value)
            .collect();
        in_window.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let stable = match &prev_window {
            Some(prev) if prev.len() == in_window.len() && !in_window.is_empty() => in_window
                .iter()
                .zip(prev.iter())
                .all(|(a, b)| (a - b).abs() < res_tol),
            _ => false,
        };
        last_pairs = pairs;
        if stable {
            break;
        }
        prev_window = Some(in_window);
    }

    SweepOut { pairs: last_pairs, iterations }
}

/// FULL twice-is-enough complex Gram-Schmidt: orthonormalize each column
/// against all previously normalized columns twice (mirrors the sharded
/// Lanczos reorthogonalization idiom, adapted to conjugate inner
/// products). A column that collapses (linearly dependent) is re-seeded.
fn reorthonormalize(block: &mut [Vec<C>], rng: &mut SplitMix) {
    let b = block.len();
    let n = if b > 0 { block[0].len() } else { 0 };
    for c in 0..b {
        let mut attempts = 0;
        loop {
            // Two passes of projection against the already-orthonormal 0..c.
            for _pass in 0..2 {
                for p in 0..c {
                    let proj = cdot_slices(block, p, c);
                    let (src, dst) = split_two(block, p, c);
                    caxpy(dst, -proj, src);
                }
            }
            let nrm = cnorm(&block[c]);
            if nrm > 1e-11 {
                let inv = 1.0 / nrm;
                for z in block[c].iter_mut() {
                    *z = z.scale(inv);
                }
                break;
            }
            // Collapsed → re-seed and retry (bounded).
            attempts += 1;
            if attempts > 3 {
                // Give up gracefully: leave a canonical unit vector.
                for (i, z) in block[c].iter_mut().enumerate() {
                    *z = if i == c % n.max(1) { C::new(1.0, 0.0) } else { C::new(0.0, 0.0) };
                }
                continue;
            }
            for z in block[c].iter_mut() {
                *z = rng.complex_normal();
            }
        }
    }
}

/// ⟨block[p], block[c]⟩ without borrowing conflicts.
#[inline]
fn cdot_slices(block: &[Vec<C>], p: usize, c: usize) -> C {
    let a = &block[p];
    let b = &block[c];
    let mut s = C::new(0.0, 0.0);
    for i in 0..a.len() {
        s += a[i].conj() * b[i];
    }
    s
}

/// Disjoint mutable/immutable split of two distinct columns `p` (read) and
/// `c` (write).
#[inline]
fn split_two(block: &mut [Vec<C>], p: usize, c: usize) -> (&[C], &mut [C]) {
    debug_assert_ne!(p, c);
    if p < c {
        let (left, right) = block.split_at_mut(c);
        (&left[p], &mut right[0])
    } else {
        let (left, right) = block.split_at_mut(p);
        (&right[0], &mut left[c])
    }
}

// ─────────────────────────────── driver ────────────────────────────────

/// Solve the interior BULK window of the magnetic Laplacian by the
/// Chebyshev-filtered subspace method.
///
/// `edges` is the `(i, j, θ)` edge list (dense vertex indexing `0..n`),
/// `req` the BULK request (k + centering), `config` the tunables.
/// Returns the window + convergence receipt, or a typed error for a
/// meaningless request (`k = 0`).
pub fn spectral_interior_bulk(
    edges: &[(usize, usize, f64)],
    n: usize,
    req: &BulkSpec,
    config: &InteriorConfig,
) -> Result<InteriorResult, SpectralGaugeError> {
    if req.k == 0 {
        return Err(SpectralGaugeError::InvalidLimit { got: 0 });
    }
    if let BulkCenter::Interval { lo, hi } = req.center {
        if lo > hi {
            return Err(SpectralGaugeError::InvalidInterval { lo, hi });
        }
    }
    let k = req.k.min(n.max(1));

    let csr = MagneticCsr::assemble(edges, n);
    let deg_max = csr.max_degree();

    // Gershgorin enclosure: λ ∈ [0, 2·deg_max]; PSD ⇒ λ_min ≥ 0. Pad
    // outward so no eigenvalue maps outside [−1,1] after the rescale.
    let lambda_min = 0.0;
    let lambda_max = (2.0 * deg_max).max(1e-9);
    let span = lambda_max - lambda_min;
    let pad = span * 1e-3 + 1e-9;
    let lo_d = lambda_min - pad;
    let hi_d = lambda_max + pad;
    let rescale = Rescale::new(lo_d, hi_d);

    let kpm = KpmDos::new(
        &csr,
        &rescale,
        config.kpm_moments,
        config.kpm_probes,
        config.seed,
    );

    // Resolve the center value.
    let center = match req.center {
        BulkCenter::Auto => kpm.median_lambda(&rescale, lo_d, hi_d),
        BulkCenter::Around(sigma) => sigma,
        BulkCenter::Interval { lo, hi } => 0.5 * (lo + hi),
    };

    let base_oversample = config
        .oversample
        .unwrap_or_else(|| ((0.4 * k as f64).ceil() as usize).max(60));

    // Adaptive retry state.
    let mut oversample = base_oversample;
    let mut degree_mult = 1.0f64;
    let mut width_mult = 1.0f64;

    let mut total_iters = 0usize;
    let mut best: Option<InteriorResult> = None;

    for attempt in 0..=config.max_completeness_retries {
        // Passband + selection interval per center type.
        let (pass_lo, pass_hi, target_levels) = match req.center {
            BulkCenter::Interval { lo, hi } => {
                let band = kpm.count_in(&rescale, lo, hi).max(1.0);
                let margin = kpm
                    .half_width_for(&rescale, 0.5 * (lo + hi), band + oversample as f64, span)
                    .max(hi - lo);
                let mid = 0.5 * (lo + hi);
                (mid - margin, mid + margin, band)
            }
            _ => {
                // Around/Auto: passband holds ~k + oversample/2 levels.
                let want = (k as f64 + 0.5 * oversample as f64) * width_mult;
                let h = kpm
                    .half_width_for(&rescale, center, want.min(n as f64), span)
                    .max(span * 1e-6);
                (center - h, center + h, k as f64)
            }
        };

        // Block dimension: comfortably above the passband level count.
        let pass_levels = kpm.count_in(&rescale, pass_lo, pass_hi);
        let b_dim = ((pass_levels.ceil() as usize) + oversample)
            .max(k + oversample)
            .min(n.max(1));

        // Filter degree from the passband width relation
        // m ≈ α·(λmax−λmin)/Δλ, ×1.4 Jackson widening, degree_mult on retry.
        let delta = (pass_hi - pass_lo).max(span * 1e-9);
        let est = (10.0 * span / delta) * degree_mult;
        let degree = (est.ceil() as usize)
            .max(config.min_filter_degree)
            .min(config.max_filter_degree)
            .min(4 * n + 8);

        let ta = rescale.to_cheb(pass_lo);
        let tb = rescale.to_cheb(pass_hi);
        let filter = BandpassFilter::new(ta, tb, degree);

        let sweep = subspace_iterate(
            &csr,
            &rescale,
            &filter,
            b_dim,
            pass_lo,
            pass_hi,
            config.res_tol,
            config.max_subspace_iters,
            config.seed.wrapping_add(attempt as u64 * 0x1000),
        );
        total_iters += sweep.iterations;

        // Gather converged pairs (residual-gated) and select the window.
        let (window, converged, max_res) =
            select_window(&sweep, &req.center, center, k, config.res_tol);

        let count_below = kpm.count_below_lambda(
            &rescale,
            window.first().copied().unwrap_or(center),
        );

        let fully = match req.center {
            BulkCenter::Interval { .. } => converged >= target_levels.round() as usize && converged == window.len(),
            _ => converged == k,
        };

        let result = InteriorResult {
            eigenvalues: window,
            bulk_center: center,
            converged,
            requested_k: k,
            max_residual: max_res,
            iterations: total_iters,
            restarts: attempt,
            fully_converged: fully,
            count_below_estimate: count_below,
        };

        // Keep the best (most converged) attempt.
        let improved = match &best {
            None => true,
            Some(b) => result.converged > b.converged,
        };
        if improved {
            best = Some(result.clone());
        }
        if fully {
            return Ok(result);
        }

        // Retry: grow the block, widen the passband, sharpen the filter.
        oversample = (oversample * 3 / 2).max(oversample + 16);
        degree_mult *= 1.4;
        width_mult *= 1.3;
    }

    // Exhausted retries — return the best honest partial (converged < k,
    // fully_converged = false). Never silently claims k.
    Ok(best.expect("at least one attempt runs"))
}

/// Select the returned window from a completed sweep, residual-gated.
///
/// - Around/Auto: the `k` CONVERGED pairs nearest `center` by value.
/// - Interval: all CONVERGED pairs in `[sel_lo, sel_hi]`, clamped to the
///   `k` nearest the midpoint if over-full.
fn select_window(
    sweep: &SweepOut,
    center_kind: &BulkCenter,
    center: f64,
    k: usize,
    res_tol: f64,
) -> (Vec<f64>, usize, f64) {
    // Converged (residual-gated) pairs.
    let mut conv: Vec<f64> = sweep
        .pairs
        .iter()
        .filter(|p| p.residual < res_tol)
        .map(|p| p.value)
        .collect();
    conv.sort_by(|a, b| a.partial_cmp(b).unwrap());

    match center_kind {
        BulkCenter::Interval { lo, hi } => {
            let mut band: Vec<f64> =
                conv.iter().copied().filter(|&x| x >= *lo && x <= *hi).collect();
            if band.len() > k {
                let mid = 0.5 * (lo + hi);
                band.sort_by(|a, b| {
                    (a - mid).abs().partial_cmp(&(b - mid).abs()).unwrap()
                });
                band.truncate(k);
                band.sort_by(|a, b| a.partial_cmp(b).unwrap());
            }
            let converged = band.len();
            let max_res = window_max_residual(sweep, &band);
            (band, converged, max_res)
        }
        _ => {
            // k nearest center by value, among converged.
            let mut by_dist = conv.clone();
            by_dist.sort_by(|a, b| {
                (a - center).abs().partial_cmp(&(b - center).abs()).unwrap()
            });
            by_dist.truncate(k);
            by_dist.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let converged = by_dist.len();
            let max_res = window_max_residual(sweep, &by_dist);
            (by_dist, converged, max_res)
        }
    }
}

/// Max residual over the returned window values (match each returned value
/// back to its Ritz pair). Returns 0 for an empty window.
fn window_max_residual(sweep: &SweepOut, window: &[f64]) -> f64 {
    let mut m = 0.0f64;
    for &val in window {
        // Nearest Ritz pair by value.
        let mut best = f64::INFINITY;
        let mut best_res = 0.0;
        for p in &sweep.pairs {
            let d = (p.value - val).abs();
            if d < best {
                best = d;
                best_res = p.residual;
            }
        }
        m = m.max(best_res);
    }
    m
}
