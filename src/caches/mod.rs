//! Cache infrastructure for the GIGI engine.
//!
//! Houses the generic [`single_flight::SingleFlightCache`] consolidated
//! from three independent reimplementations of the same correctness-
//! critical pattern. See `single_flight.rs` for the audit history
//! (workflow w2n0fgqkk, 2026-06-20) and the load-bearing semantics
//! the generic preserves.

pub mod single_flight;
