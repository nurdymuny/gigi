//! Geometric cost-model primitives for the Kähler upgrade
//! (catalog §1.3, §1.4, §1.5; IMPLEMENTATION_PLAN.md L3).
//!
//! Where today's query planner uses scan-cost heuristics, this
//! module gives it bounds with theorems attached:
//!
//! - **Jacobi-field cardinality estimation** — integrate `J'' + K·J = 0`
//!   along a query's trajectory to get a closed-form-bounded
//!   estimate of how many records lie within radius `R`. Bishop and
//!   Günther comparisons translate sectional curvature bounds into
//!   monotone bounds on the volume of trajectory balls.
//!
//! - **Hadamard region check** (L5 consumes — we just provide the
//!   primitive here): Jacobi field non-vanishing on `[0, T]` is
//!   equivalent to no conjugate points, which is equivalent to
//!   `exp_p` being a local diffeomorphism. The estimator surfaces
//!   the first conjugate-point detection so callers can decide
//!   whether to treat the region as Hadamard.
//!
//! Layering (per IMPLEMENTATION_PLAN.md):
//! - **L3.2** (here): `jacobi_estimator` — the math primitive.
//! - **L3.3** (`bundle.rs` extension): cached spectral gap +
//!   incremental update on insert.
//! - **L3.4** (`bin/gigi_stream.rs` extension): Marcella-runtime
//!   surfaces — `GET /v1/bundles/<id>/spectral_gap` endpoint +
//!   `bundle_spectral_gap` field in retrieval responses.
//!
//! Same feature gating as `geometry` and `graph`: only compiled
//! when `--features kahler`.

pub mod jacobi_estimator;

pub use jacobi_estimator::{
    cardinality_bound, jacobi_field, CardinalityBound, JacobiResult,
};
