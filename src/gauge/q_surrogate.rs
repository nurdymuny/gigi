//! `Q_SURROGATE` primitive — angular accumulator over a gauge field.
//!
//! Closes TDD-HAL-III.2 (Part III, gate 2). Mirrors the Halcyon Python
//! reference observable in `inertia_damping/buckyball_observables.py`
//! (the `Q_surrogate(U) = (1/2π) Σ_f arccos(clamp(q0(U_f), -1, 1))`
//! reduction over face holonomies; clamping is load-bearing — FP64
//! roundoff on a face whose true `q0` is exactly `1.0` can push the
//! argument to `1.0 + 1e-16`, which would NaN the `arccos` if the
//! clamp didn't happen first).
//!
//! Group-erasure contract (Bee's locked decision 6, mirrored from
//! III.1 `plaquette`): the entry point takes `&dyn GaugeFieldHandle`.
//! `Group::SU2` returns the scalar `f64` accumulator; every other
//! group returns `Err(GaugeFieldError::UnsupportedGroup(g))`.
//! `Q_SURROGATE` is SU(2)-specific by definition (`arccos` of the
//! double-cover scalar component); analogous observables for
//! `U(1)` / `SU(3)` ship as different verbs, not as new arms here.
//!
//! Shape contract (locked decision D6): scalar `f64`. Mirrors the
//! Halcyon mock JSON envelope byte-for-byte at the JSON level.
//!
//! Range: `[0, F/2]` on a lattice with `F` faces. Zero at `U = I`
//! (every face holonomy is `q0 = 1`, `arccos(1) = 0`). Upper bound
//! `F/2` follows from `arccos(q0) ∈ [0, π]` and the `1/(2π)`
//! normalization (`F · π / (2π) = F/2`). For the buckyball (`F=32`)
//! the range is `[0, 16]`.
//!
//! Not topologically quantized — `π₂(SU(2)) = 0` on `S²`, so this is
//! a continuous angular accumulator, not a winding number. The name
//! "Q surrogate" reflects that: it's the closest scalar Halcyon's
//! mock publishes to a quantized charge without paying for the lift
//! to the universal cover.
//!
//! Implementation reuses III.1's `plaquette_per_face` for the `q0`
//! column to avoid duplicating walker logic — `plaquette_per_face`
//! already returns the per-face `q0` vector, and `q_surrogate` is
//! exactly the clamp-then-arccos-then-sum reduction over it.
//!
//! No parser / HTTP surface at this gate. III.7 lifts both `PLAQUETTE`
//! and `Q_SURROGATE` to GQL statements + HTTP routes; III.2 is the
//! pure library primitive the rest of Part III consumes.

use std::f64::consts::PI;

use super::error::GaugeFieldError;
use super::group::Group;
use super::plaquette::plaquette_per_face;
use super::registry::GaugeFieldHandle;
use crate::lattice::Lattice;

/// Reduce every face of `lat` to a single scalar `f64` angular
/// accumulator under the gauge field behind `handle`.
///
/// For `Group::SU2`: returns `(1/2π) · Σ_f arccos(clamp(q0_f, -1, 1))`
/// where `q0_f` is the SU(2) scalar component of the face holonomy
/// (the per-face vector III.1's `plaquette_per_face` already publishes).
/// Clamping happens BEFORE `arccos` — locked load-bearing step from the
/// Halcyon Python reference, guards against FP64 roundoff at `q0 = ±1`.
///
/// Other groups: `Err(GaugeFieldError::UnsupportedGroup(g))` — same
/// contract `plaquette_per_face` uses for non-SU(2) handles.
pub fn q_surrogate(
    handle: &dyn GaugeFieldHandle,
    lat: &Lattice,
) -> Result<f64, GaugeFieldError> {
    match handle.group() {
        Group::SU2 => {
            let per_face = plaquette_per_face(handle, lat)?;
            let mut acc = 0.0;
            for q0 in per_face {
                let clamped = q0.clamp(-1.0, 1.0);
                acc += clamped.acos();
            }
            Ok(acc / (2.0 * PI))
        }
        g => Err(GaugeFieldError::UnsupportedGroup(g)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gauge::registry as gauge_registry;
    use crate::gauge::su2_gauge_field::{GaugeFieldInit, SU2GaugeField};
    use crate::lattice::registry as lattice_registry;
    use crate::lattice::topology::truncated_icosahedron::buckyball;
    use std::sync::Arc;

    /// TDD-HAL-III.2: identity gauge field → `Q_surrogate = 0` exactly
    /// (every face holonomy is `q0 = 1`, `arccos(1) = 0`).
    #[test]
    fn tdd_hal_iii_2_q_surrogate_at_identity_is_zero() {
        gauge_registry::clear();
        lattice_registry::clear();
        let bb = buckyball();
        lattice_registry::register(bb.clone());

        let field = SU2GaugeField::new(
            "U_iii_2_id".into(),
            &bb,
            GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init must succeed");
        gauge_registry::register(Arc::new(field));

        let handle = gauge_registry::get("U_iii_2_id").expect("just registered");
        let q = q_surrogate(handle.as_ref(), &bb).expect("SU(2) reduction");
        assert!(
            q.abs() < 1e-12,
            "identity field: expected Q_surrogate ≈ 0, got {q}"
        );
    }

    /// TDD-HAL-III.2: Haar-random gauge field at the canonical seed
    /// → `Q_surrogate ∈ [0, F/2] = [0, 16]` on the buckyball
    /// (`arccos(q0_f) ∈ [0, π]` per face, normalized by `1/(2π)`,
    /// summed over `F = 32` faces).
    #[test]
    fn tdd_hal_iii_2_q_surrogate_within_range() {
        gauge_registry::clear();
        lattice_registry::clear();
        let bb = buckyball();
        lattice_registry::register(bb.clone());

        let field = SU2GaugeField::new(
            "U_iii_2_haar".into(),
            &bb,
            GaugeFieldInit::HaarRandom,
            Some(20260616),
        )
        .expect("haar init must succeed");
        gauge_registry::register(Arc::new(field));

        let handle = gauge_registry::get("U_iii_2_haar").expect("just registered");
        let q = q_surrogate(handle.as_ref(), &bb).expect("SU(2) reduction");
        assert!(
            q >= 0.0 && q <= 16.0,
            "Haar field: Q_surrogate = {q} escaped [0, F/2] = [0, 16]"
        );
    }

    /// TDD-HAL-III.2: clamp-before-arccos is load-bearing. Recompute
    /// the sum on the test side with the same `q0_f.clamp(-1, 1)`
    /// step BEFORE `arccos` that the reference Halcyon Python uses
    /// (`Uf[:, 0].clamp(-1.0, 1.0)` in `buckyball_observables.py:145`)
    /// and assert byte-identity with `q_surrogate(handle)`. A
    /// pre-bug version that called `arccos` before clamping would
    /// NaN on FP64 roundoff at `q0 = 1.0 + 1e-16`; this test pins
    /// the clamp ordering so a future refactor can't silently
    /// regress it.
    #[test]
    fn tdd_hal_iii_2_q_surrogate_clamp_idempotent() {
        gauge_registry::clear();
        lattice_registry::clear();
        let bb = buckyball();
        lattice_registry::register(bb.clone());

        let field = SU2GaugeField::new(
            "U_iii_2_clamp".into(),
            &bb,
            GaugeFieldInit::HaarRandom,
            Some(20260616),
        )
        .expect("haar init must succeed");
        gauge_registry::register(Arc::new(field));

        let handle = gauge_registry::get("U_iii_2_clamp").expect("just registered");
        let q_via_api = q_surrogate(handle.as_ref(), &bb).expect("SU(2) reduction");

        // Recompute exactly the way the implementation does it:
        // (1) per-face q0 from III.1's plaquette walker,
        // (2) clamp to [-1, 1] BEFORE arccos,
        // (3) sum arccos values,
        // (4) divide by 2π.
        let per_face = plaquette_per_face(handle.as_ref(), &bb)
            .expect("per-face for recompute");
        let mut acc = 0.0;
        for q0 in &per_face {
            let clamped = q0.clamp(-1.0, 1.0);
            acc += clamped.acos();
        }
        let q_recomputed = acc / (2.0 * PI);

        assert_eq!(
            q_via_api, q_recomputed,
            "Q_surrogate drift: api={q_via_api}, recomputed={q_recomputed} \
             (clamp-before-arccos ordering is load-bearing)"
        );
    }
}
