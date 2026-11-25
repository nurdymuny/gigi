//! helike-quantum: the sourced homogeneous Yang–Mills condensate on the positive Gram cone.
//!
//! - `model`: the sourced potential on colour singlets, its stationary background and harmonic normal frame;
//! - `chart`: the positive-cone (log-Cholesky) chart, its cometric and measure;
//! - `hermite`, `basis`, `grid`: Hermite functions, occupation simplices, slab-wise separable transforms;
//! - `laurent`, `kron`: exact Laurent expansions and exact Kronecker moments on the occupation simplex;
//! - `operator`: the Ritz operator H = T + V applied matrix-free (no moment matrix is ever formed);
//! - `lanczos`: block Lanczos for the lowest levels, with true residuals;
//! - `ritz`: levels, reference weights and the Helike / competitor identification;
//! - `interval`, `residual`: outward-rounded interval arithmetic, and rigorous enclosures of the full-operator
//!   residuals ‖(H − E)ψ‖ of saved trial states (exact centred moment sum);
//! - `projection`: the exact SO(2) charge projector by its norm bound: enclosed charge defects ‖(L − m)ψ‖ and the
//!   projected-sector residual bound (r + ℓ)/q;
//! - `run`: the controls and convergence runners with the frozen decision rules of the Python scripts.
//!
//! Scope: interior trial values on the positive cone. Nothing here certifies the physical (boundary-complete)
//! operator; the quasimode certificates are a separate layer.

pub mod basis;
pub mod export;
pub mod chart;
pub mod grid;
pub mod hermite;
pub mod interval;
pub mod kron;
pub mod laurent;
pub mod lanczos;
pub mod model;
pub mod operator;
pub mod projection;
pub mod residual;
pub mod ritz;
pub mod run;
pub mod temple;
