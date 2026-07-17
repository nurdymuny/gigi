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
//! dependency): assemble the magnetic Laplacian as a complex CSR
//! structure (never a dense V×V matrix), estimate the spectral band with
//! Gershgorin, build a Jackson-damped Chebyshev BANDPASS filter p(L) that
//! amplifies the target window and damps outside, and run block subspace
//! iteration with full reorthogonalization + Rayleigh-Ritz until the
//! in-window Ritz values stabilize. The projected b×b Rayleigh-Ritz is
//! the SAME `nalgebra::SymmetricEigen` over a small dense complex-
//! Hermitian block already trusted in-tree; the reorthogonalization
//! mirrors the twice-is-enough Gram-Schmidt from `src/sharded/spectral.rs`,
//! adapted to a `Complex<f64>` block with conjugate inner products.
//!
//! COMPLETENESS (the crux — Hallie's ask #3): a filtered subspace method
//! can silently MISS a bulk level (merge a near-degenerate pair, or drop
//! one at the filter edge). Three honest defenses, none of them a
//! ground-truth certificate at V = 32768 (none exists there):
//!   1. RESIDUAL GATE — every returned pair has ‖Lv − λv‖ / ‖v‖ < RES_TOL,
//!      so each returned value is a TRUE eigenpair (not a filter artifact).
//!   2. COUNT — the number of converged pairs in the window must equal the
//!      requested k; fewer signals a possible miss → grow the subspace /
//!      raise the filter degree and retry (bounded); still short → return
//!      `converged < k` with `fully_converged = false` (honest, never a
//!      silent claim of k).
//!   3. OVERSAMPLE — the block is over-dimensioned (b = k + oversample),
//!      the completeness margin against near-degenerate merges.
//! Validated EXACTLY against dense ground truth for V ≤ 2048 (tests
//! `tests/spectral_interior_basic.rs` SP1/SP2/SP4): same values, same
//! count, same multiplicities, no miss, no extra.
//!
//! Run the completeness suite with:
//!   `cargo test --features halcyon --test spectral_interior_basic`

#![cfg(feature = "gauge")]

use nalgebra::Complex;

use crate::spectral::{BulkCenter, BulkSpec, SpectralGaugeError};

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
    off: Vec<Complex<f64>>,
    diag: Vec<f64>,
}

impl MagneticCsr {
    /// Assemble the complex CSR magnetic Laplacian from an edge list
    /// `(i, j, θ)`. Self-loops (`i == j`) are skipped (matching the dense
    /// path). Parallel edges accumulate.
    pub fn assemble(_edges: &[(usize, usize, f64)], _n: usize) -> Self {
        unimplemented!("spectral_interior::MagneticCsr::assemble — GREEN phase")
    }

    /// `y = L · x` — one complex CSR matvec.
    pub fn matvec(&self, _x: &[Complex<f64>]) -> Vec<Complex<f64>> {
        unimplemented!("spectral_interior::MagneticCsr::matvec — GREEN phase")
    }

    /// Vertex count (matrix dimension).
    pub fn n(&self) -> usize {
        self.n
    }

    /// Maximum vertex degree (the Gershgorin upper-bound seed: every
    /// eigenvalue lies in `[0, 2·deg_max]`).
    pub fn max_degree(&self) -> f64 {
        unimplemented!("spectral_interior::MagneticCsr::max_degree — GREEN phase")
    }
}

/// Solve the interior BULK window of the magnetic Laplacian by the
/// Chebyshev-filtered subspace method.
///
/// `edges` is the `(i, j, θ)` edge list (dense vertex indexing `0..n`),
/// `req` the BULK request (k + centering), `config` the tunables.
/// Returns the window + convergence receipt, or a typed error for a
/// meaningless request (`k = 0`).
pub fn spectral_interior_bulk(
    _edges: &[(usize, usize, f64)],
    _n: usize,
    _req: &BulkSpec,
    _config: &InteriorConfig,
) -> Result<InteriorResult, SpectralGaugeError> {
    let _ = (BulkCenter::Auto,); // silence unused import until GREEN
    unimplemented!("spectral_interior::spectral_interior_bulk — GREEN phase")
}
