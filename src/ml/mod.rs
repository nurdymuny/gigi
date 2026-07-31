//! GIGI's geometric ML suite — the compute core behind the
//! `/v1/bundles/{name}/{scan, scan/fit, cluster, infer, reduce, prescribe,
//! solve, circulation, factorize, changepoints}` REST endpoints.
//!
//! Extracted mechanically from `src/bin/gigi_stream.rs` (stream-extraction
//! phase 1, see EXTRACTION_MAP.md). The axum handlers and the `/v1/ml`
//! discovery catalog remain in the binary; each handler is a thin wrapper
//! that binds the bundle + JSON wire format around these functions.

pub mod changepoints;
pub mod circulation;
pub mod cluster;
pub mod factorize;
pub mod infer;
pub mod prescribe;
pub mod reduce;
pub mod scan;
pub mod solve;

#[cfg(test)]
pub mod test_support;

pub use changepoints::*;
pub use circulation::*;
pub use cluster::*;
pub use factorize::*;
pub use infer::*;
pub use prescribe::*;
pub use reduce::*;
pub use scan::*;
pub use solve::*;
