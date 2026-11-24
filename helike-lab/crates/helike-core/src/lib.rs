//! helike-core: Yang-Mills Hessians at left-invariant fields on compact groups.
//!
//! General layer:
//! - [`lie`]: Lie algebras by structure constants (su(2) in both conventions, su(N), any matrix basis);
//! - [`rep`]: representations (spin, SU(N) (p,0..0,q), tensor, dual, generated submodules);
//! - [`sector`]: the Hessian H and gauge map G in a Peter-Weyl sector, for any frame algebra, diagonal metric, gauge
//!   algebra, field and representation; curl in three dimensions; curvature, energy, Levi-Civita, Ricci;
//! - [`charge`]: splitting a sector by any circle symmetry (signs fitted, residuals reported), blockwise spectra of H,
//!   H + G G^* and the gauge slice;
//! - [`problem`]: a serialisable problem description for the CLI and receipts.
//!
//! Instance: [`helike`], the circle-symmetric SU(2) family on the Berger spheres (closed forms, ladder, edges).
//!
//! The Python scripts in `scripts/` are the reference. Parity with them is tested in `tests/parity.rs`.

pub mod charge;
pub mod helike;
pub mod lie;
pub mod linalg;
pub mod problem;
pub mod rep;
pub mod roots;
pub mod sector;
