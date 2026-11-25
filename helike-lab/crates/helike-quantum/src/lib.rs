//! helike-quantum: the sourced homogeneous Yang–Mills condensate on the positive Gram cone.
//!
//! - `model`: the sourced potential on colour singlets, its stationary background and harmonic normal frame;
//! - `chart`: the positive-cone (log-Cholesky) chart, its cometric and measure;
//! - `hermite`, `basis`, `grid`: Hermite functions, occupation simplices, slab-wise separable transforms;
//! - `operator`: the Ritz operator H = T + V applied matrix-free (no moment matrix is ever formed);
//! - `lanczos`: block Lanczos for the lowest levels, with true residuals;
//! - `ritz`: levels, reference weights and the Helike / competitor identification;
//! - `run`: the controls and convergence runners with the frozen decision rules of the Python scripts.
//!
//! Scope: interior trial values on the positive cone. Nothing here certifies the physical (boundary-complete)
//! operator; the quasimode certificates are a separate layer.

pub mod basis;
pub mod chart;
pub mod grid;
pub mod hermite;
pub mod lanczos;
pub mod model;
pub mod operator;
pub mod ritz;
pub mod run;
