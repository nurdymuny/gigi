//! `PLAQUETTE` primitive — face-holonomy reduction over a gauge field.
//!
//! Closes TDD-HAL-III.1 (Part III, gate 1). The face holonomy already
//! lives in `gauge::holonomy::walk_loop` (group-erased Part-I walker).
//! `plaquette_per_face` lifts that walker over every face of a
//! `Lattice`, reads the SU(2) scalar `q0` off the resulting
//! `GroupElement::SU2 { q0, .. }`, and publishes the per-face vector
//! Halcyon's reference implementation publishes in
//! `inertia_damping/buckyball_observables.py` (the `Re tr(U_f) / 2`
//! plaquette value, which for our `q0 = cos(θ/2)` SU(2) convention is
//! exactly `q0`).
//!
//! Reductions (`plaquette_mean`, `plaquette_sum`) are scalar `f64`
//! returns — locked decision D6/D7 (per-face is `Vec<f64>` of length
//! `F`, mean / sum collapse to a single number, matching the Halcyon
//! mock JSON shapes byte-for-byte).
//!
//! Group-erasure contract: every entry point takes
//! `&dyn GaugeFieldHandle`. The walker is already group-erased; the
//! only group-aware step is the `q0` extraction at the end. We dispatch
//! on `handle.group()` — `Group::SU2` returns the `q0` reduction; every
//! other group returns `Err(GaugeFieldError::UnsupportedGroup(g))`. When
//! a future U(1) / SU(3) / Z_N field ships its own `q0`-equivalent
//! reduction (`Re tr / dim` for SU(N), `cos θ` for U(1), …), the
//! signature on these three functions does not change — only the
//! group-dispatch arms grow.
//!
//! No parser / HTTP surface at this gate. III.7 lifts the library
//! function to a `PLAQUETTE U OVER L` GQL statement + an HTTP route;
//! III.1 is the pure library primitive the rest of Part III consumes.

use super::error::GaugeFieldError;
use super::group::Group;
use super::group_element::GroupElement;
use super::holonomy::{face_edges, walk_loop};
use super::registry::GaugeFieldHandle;
use crate::lattice::Lattice;

/// Reduce every face of `lat` to a single `f64` plaquette value under
/// the gauge field behind `handle`. Returns a `Vec<f64>` of length
/// `lat.n_faces()` in face-index order.
///
/// For `Group::SU2`: the reduction is `q0 = Re tr(U_f) / 2`, read off
/// the `GroupElement::SU2 { q0, .. }` that `walk_loop` returns.
///
/// Other groups: `Err(GaugeFieldError::UnsupportedGroup(g))` — same
/// contract `SU2GaugeField::new` uses for non-SU(2) declarations.
pub fn plaquette_per_face(
    handle: &dyn GaugeFieldHandle,
    lat: &Lattice,
) -> Result<Vec<f64>, GaugeFieldError> {
    match handle.group() {
        Group::SU2 => {
            let n_faces = lat.n_faces();
            let mut out = Vec::with_capacity(n_faces);
            for fidx in 0..n_faces {
                let edges = face_edges(lat, fidx);
                let h = walk_loop(lat, &edges, handle);
                let q0 = match h {
                    GroupElement::SU2 { q0, .. } => q0,
                    // walk_loop on an SU(2)-tagged buffer must return
                    // an SU2 variant; reaching another arm here would
                    // be a programming error in the buffer / walker
                    // contract, not a user-surfaceable failure.
                    _ => unreachable!(
                        "plaquette_per_face: SU(2) handle returned non-SU2 GroupElement"
                    ),
                };
                out.push(q0);
            }
            Ok(out)
        }
        g => Err(GaugeFieldError::UnsupportedGroup(g)),
    }
}

/// Mean plaquette `(1/F) · Σ_f q0_f`. Scalar f64. Locked decision D7.
pub fn plaquette_mean(
    handle: &dyn GaugeFieldHandle,
    lat: &Lattice,
) -> Result<f64, GaugeFieldError> {
    let per_face = plaquette_per_face(handle, lat)?;
    let n = per_face.len();
    if n == 0 {
        return Ok(0.0);
    }
    let sum: f64 = per_face.iter().sum();
    Ok(sum / n as f64)
}

/// Total plaquette `Σ_f q0_f`. Scalar f64. Locked decision D7.
pub fn plaquette_sum(
    handle: &dyn GaugeFieldHandle,
    lat: &Lattice,
) -> Result<f64, GaugeFieldError> {
    let per_face = plaquette_per_face(handle, lat)?;
    Ok(per_face.iter().sum())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gauge::registry as gauge_registry;
    use crate::gauge::su2_gauge_field::{GaugeFieldInit, SU2GaugeField};
    use crate::lattice::registry as lattice_registry;
    use crate::lattice::topology::truncated_icosahedron::buckyball;
    use std::sync::Arc;

    /// TDD-HAL-III.1: identity gauge field → every face plaquette is
    /// FP64-exact 1.0; mean and sum are exact (32 and 1.0).
    #[test]
    fn tdd_hal_iii_1_plaquette_identity_is_unity() {
        gauge_registry::clear();
        lattice_registry::clear();
        let bb = buckyball();
        lattice_registry::register(bb.clone());

        let field = SU2GaugeField::new(
            "U_iii_1_id".into(),
            &bb,
            GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init must succeed");
        gauge_registry::register(Arc::new(field));

        let handle = gauge_registry::get("U_iii_1_id").expect("just registered");
        let per = plaquette_per_face(handle.as_ref(), &bb).expect("SU(2) reduction");
        assert_eq!(per.len(), 32, "buckyball has F=32 faces");
        for (i, q) in per.iter().enumerate() {
            // Identity quaternion product is FP64-exact 1.0: every
            // edge contributes (1,0,0,0), so q0_f = 1.0 byte-identical.
            assert_eq!(*q, 1.0, "face {i}: expected q0 = 1.0 exactly, got {q}");
        }

        let mean = plaquette_mean(handle.as_ref(), &bb).expect("mean");
        assert_eq!(mean, 1.0, "mean over identity field = 1.0 exactly");
        let sum = plaquette_sum(handle.as_ref(), &bb).expect("sum");
        assert_eq!(sum, 32.0, "sum over 32 identity faces = 32.0 exactly");
    }

    /// TDD-HAL-III.1: Haar-random gauge field at the canonical seed
    /// → every per-face plaquette is in the SU(2) unit ball
    /// `q0 ∈ [-1, 1]` (the quaternion scalar lives on the unit
    /// 3-sphere, so |q0| ≤ 1 by construction).
    #[test]
    fn tdd_hal_iii_1_plaquette_haar_within_unit_ball() {
        gauge_registry::clear();
        lattice_registry::clear();
        let bb = buckyball();
        lattice_registry::register(bb.clone());

        let field = SU2GaugeField::new(
            "U_iii_1_haar".into(),
            &bb,
            GaugeFieldInit::HaarRandom,
            Some(20260616),
        )
        .expect("haar init must succeed");
        gauge_registry::register(Arc::new(field));

        let handle = gauge_registry::get("U_iii_1_haar").expect("just registered");
        let per = plaquette_per_face(handle.as_ref(), &bb).expect("SU(2) reduction");
        assert_eq!(per.len(), 32, "buckyball has F=32 faces");
        for (i, q) in per.iter().enumerate() {
            assert!(
                *q >= -1.0 && *q <= 1.0,
                "face {i}: q0 = {q} escaped [-1, 1] — SU(2) unit-ball invariant violated"
            );
        }
    }

    /// TDD-HAL-III.1: `plaquette_mean` is the FP64 mean of
    /// `plaquette_per_face`. Equality holds to 1e-15 because the
    /// summation order in `plaquette_mean` is the same `iter().sum()`
    /// the test recomputes here.
    #[test]
    fn tdd_hal_iii_1_plaquette_mean_equals_per_face_mean() {
        gauge_registry::clear();
        lattice_registry::clear();
        let bb = buckyball();
        lattice_registry::register(bb.clone());

        let field = SU2GaugeField::new(
            "U_iii_1_mean".into(),
            &bb,
            GaugeFieldInit::HaarRandom,
            Some(20260616),
        )
        .expect("haar init must succeed");
        gauge_registry::register(Arc::new(field));

        let handle = gauge_registry::get("U_iii_1_mean").expect("just registered");
        let per = plaquette_per_face(handle.as_ref(), &bb).expect("per-face");
        let mean_via_api = plaquette_mean(handle.as_ref(), &bb).expect("mean");
        let mean_recomputed: f64 = per.iter().sum::<f64>() / 32.0;
        assert!(
            (mean_via_api - mean_recomputed).abs() < 1e-15,
            "mean drift: api={mean_via_api}, recomputed={mean_recomputed}"
        );
    }
}
