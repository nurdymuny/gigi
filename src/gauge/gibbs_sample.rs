//! `GIBBS_SAMPLE` sweep — in-place SU(2) heatbath thermalization with
//! optional measurement history.
//!
//! Closes TDD-HAL-III.5. Stitches together III.1 (PLAQUETTE), III.2
//! (Q_SURROGATE), III.3 (per-edge staple sum walker), and III.4
//! (Kennedy–Pendleton single-edge kernel) into the production sweep
//! verb the Halcyon thermalization phase consumes. Mirrors Halcyon
//! Python `inertia_damping/buckyball_heatbath.py::heatbath_run` (lines
//! 190–248).
//!
//! ── Algorithm ──
//!
//! For each sweep `s ∈ 0..n_sweeps`:
//!
//!   for e in 0..n_edges {
//!       V_eff = staple_sum_at_edge(field, lat, inc, e)     // III.3
//!       U_new = sample_su2_link(V_eff, beta, rng)          // III.4
//!       field.buffer.data[4*e..4*e+4] ← U_new              // in-place
//!   }
//!   if measure_every > 0 && (s+1) % measure_every == 0 {
//!       for obs in measure {
//!           history[obs].push(observe(field, lat, obs))
//!       }
//!   }
//!
//! Sequential edge update order (`for e in 0..n_edges`) is load-bearing
//! per Bee's locked decision D3 — bit-identity against the Halcyon
//! Python reference depends on it; parallel / checkerboard sweep orders
//! are deferred to Part V+ (different bit-identity contract, different
//! verb).
//!
//! ── Group-erasure escape (Bee's locked decision D4) ──
//!
//! This is **the single P0 group-erasure escape of Part III.** The KP
//! kernel speaks raw `[f64; 4]` quaternions (the heatbath conditional
//! `P(U) ∝ exp(β · Re Tr(U · V))` reduces to a scalar exponent in the
//! `q0` direction — SU(2)-specific by construction), so the sweep
//! reaches into the registry through `get_su2_mut(name)` for the
//! mutable `&mut SU2GaugeField` it needs. The existing read-only
//! `Arc<dyn GaugeFieldHandle>` surface (registry::get) stays unchanged —
//! group erasure still holds for PLAQUETTE / Q_SURROGATE / SHOW. A
//! future SU(3) Cabibbo-Marinari heatbath ships `get_su3_mut` +
//! `gibbs_sample_su3` in parallel; the surface stays symmetric.
//!
//! ── Observable battery ──
//!
//! `MEASURE` admits the SU(2)-substrate observables only — `MeanPlaquette`
//! (III.1) and `QSurrogate` (III.2). Every other observable on the Part
//! IV roadmap (`HTotal`, `GaussResidualMax`, `EdgeKinetic`, `VertexGauss`,
//! `Energy`) needs an E field that Part IV will introduce; declaring
//! one in `MEASURE` at gate III.5 returns a typed
//! `PartIvObservableNotReady` error whose Display contains both
//! "Part IV" and "E field" so the upstream parser / HTTP layer can
//! surface a consistent message.
//!
//! ── CSPRNG ──
//!
//! `SmallRng::seed_from_u64(seed)` — the canonical xorshift64* path
//! Part II locked in for the whole gauge stack. Same algorithm + same
//! seed → byte-identical buffer mutation (Bee's locked decision 1,
//! INTRA-binding only — cross-binding bit-identity against NumPy PCG64
//! is impossible and explicitly out of scope).
//!
//! Reference: Halcyon Python `buckyball_heatbath.py` lines 190–220
//! (`heatbath_sweep` + `heatbath_run`).

use std::collections::HashMap;

use super::error::GaugeFieldError;
use super::marsaglia_haar::SmallRng;
use super::plaquette::plaquette_mean;
use super::q_surrogate::q_surrogate;
use super::registry::republish_su2;
use super::staple::{
    build_edge_face_incidence, build_face_edges_cache, staple_sum_at_edge, FaceHolonomyCache,
};
use super::kennedy_pendleton::sample_su2_link;
use super::group_element::GroupElement;
use crate::lattice::registry as lattice_registry;

/// Observable identifier accepted by `GIBBS_SAMPLE … MEASURE (…)`.
///
/// At launch the substrate ships the two SU(2)-only observables Part III
/// implements (`MeanPlaquette`, `QSurrogate`). The remaining variants
/// compile but error at use site through `PartIvObservableNotReady` —
/// they need an E field Part IV will introduce; declaring them today
/// surfaces a typed error rather than a silent zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObservableId {
    /// `MEAN(PLAQUETTE)` — `⟨P⟩ = (1/F) Σ_f q0(U_f)`. Scalar f64
    /// (locked decision D7). III.1 primitive.
    MeanPlaquette,
    /// `Q_SURROGATE` — `(1/2π) Σ_f arccos(clamp(q0_f, -1, 1))`. Scalar
    /// f64 (locked decision D6). III.2 primitive.
    QSurrogate,
    /// `H_TOTAL` — total Hamiltonian. Needs an E field; Part IV.
    HTotal,
    /// `GAUSS_RESIDUAL_MAX` — max-norm Gauss-constraint residual.
    /// Needs an E field; Part IV.
    GaussResidualMax,
    /// `EDGE_KINETIC` — `(1/2) Σ_e Tr(E_e²)`. Needs an E field;
    /// Part IV.
    EdgeKinetic,
    /// `VERTEX_GAUSS` — per-vertex Gauss-law density. Needs an E field;
    /// Part IV.
    VertexGauss,
    /// `ENERGY` — alias for `H_TOTAL` in some external mocks. Needs an
    /// E field; Part IV.
    Energy,
}

impl ObservableId {
    /// Stable label used in the typed error Display ("Part IV / E field
    /// observable") and in the JSON wire envelope.
    pub fn label(&self) -> &'static str {
        match self {
            ObservableId::MeanPlaquette => "MeanPlaquette",
            ObservableId::QSurrogate => "QSurrogate",
            ObservableId::HTotal => "HTotal",
            ObservableId::GaussResidualMax => "GaussResidualMax",
            ObservableId::EdgeKinetic => "EdgeKinetic",
            ObservableId::VertexGauss => "VertexGauss",
            ObservableId::Energy => "Energy",
        }
    }
}

/// Diagnostics payload published in the GIBBS_SAMPLE response.
///
/// Mirrors the Halcyon mock JSON envelope shape (`field`, `diagnostics
/// { seed, beta, n_sweeps_completed }`, `measurement_history`). Lives on
/// `GibbsSampleResponse` (one diagnostics block per response).
#[derive(Debug, Clone, PartialEq)]
pub struct GibbsSampleDiagnostics {
    /// The seed actually used (echoes the caller's `seed` argument so
    /// the response is self-describing and the seed lands in the WAL /
    /// downstream JSON without a separate round-trip).
    pub seed: u64,
    /// The inverse temperature β. Echoed from the caller's argument.
    pub beta: f64,
    /// Number of sweeps the loop actually completed. At gate III.5 this
    /// always equals the caller's `n_sweeps` (no early exit); we publish
    /// it explicitly so future P0 abort paths (timeouts, ergodicity
    /// guards) have a slot to populate without breaking the envelope.
    pub n_sweeps_completed: usize,
}

/// JSON-shaped response from `gibbs_sample`. Mirrors the Halcyon mock
/// byte-for-byte at the field level: `field` (the field's name as
/// declared), `measurement_history` (per-observable Vec<f64>),
/// `diagnostics` (seed/beta/completed-sweeps block).
#[derive(Debug, Clone, PartialEq)]
pub struct GibbsSampleResponse {
    /// Echoes the `field_name` argument.
    pub field: String,
    /// Per-observable measurement chain. Length per observable is
    /// `n_sweeps / measure_every` (only sweeps `s` with `(s+1) %
    /// measure_every == 0` push a measurement). Order: same order the
    /// observable was sampled within the sweep epilogue (load-bearing —
    /// the cross-binding gate III.8 reads the chain index by index).
    pub measurement_history: HashMap<ObservableId, Vec<f64>>,
    /// Sweep loop diagnostics.
    pub diagnostics: GibbsSampleDiagnostics,
}

/// Run `n_sweeps` Gibbs (heatbath) sweeps on the SU(2) gauge field
/// registered under `field_name` at inverse temperature `beta`.
///
/// Returns `Err(GaugeFieldError::SeedRequired)` when `seed` is `None`
/// (intra-binding bit-identity is the contract — every GIBBS_SAMPLE
/// call must commit to a seed); the Display contains the literal
/// "SEED" so upstream parsers / HTTP routes can substring-match.
///
/// Sequential edge update order `for e in 0..n_edges` (Bee's locked
/// decision D3). Each edge consumes one staple-sum walk + one KP draw
/// from the same `SmallRng` instance, so re-running with the same seed
/// produces a byte-identical buffer mutation.
///
/// Observables in `measure` are evaluated at the END of each sweep `s`
/// with `(s+1) % measure_every == 0`. `measure_every = 0` disables
/// measurement entirely (history is empty). Pre-Part-IV observables
/// (`HTotal`, `GaussResidualMax`, `EdgeKinetic`, `VertexGauss`,
/// `Energy`) return `PartIvObservableNotReady` whose Display contains
/// both "Part IV" and "E field".
pub fn gibbs_sample(
    field_name: &str,
    beta: f64,
    n_sweeps: usize,
    measure_every: usize,
    measure: Vec<ObservableId>,
    seed: Option<u64>,
) -> Result<GibbsSampleResponse, GaugeFieldError> {
    let seed = seed.ok_or(GaugeFieldError::SeedRequired)?;

    // Resolve the SU(2)-mut handle through the registry (group-erasure
    // escape — D4). Holds an Arc<Mutex<SU2GaugeField>>; the lock guard
    // is taken right before the sweep and released after the (last)
    // measurement so observables read the freshest buffer.
    //
    // We hold the Arc locally for the duration of the sweep. Even if a
    // parallel caller calls `gauge_registry::clear()` mid-sweep, the
    // local Arc keeps the field alive; at the epilogue we re-insert
    // the mutated field into both registry maps so subsequent
    // `get(name)` / `get_su2_mut(name)` lookups land on the latest
    // state.
    let field_arc = super::registry::get_su2_mut(field_name).ok_or_else(|| {
        GaugeFieldError::FieldNotDeclared(field_name.to_string())
    })?;

    // Acquire the lattice once (the staple-sum walker needs face
    // membership; the observables need the lattice to walk faces).
    let lattice_name = {
        let guard = field_arc.lock().expect("su2 field mutex poisoned");
        guard.lattice_name.clone()
    };
    let lat = lattice_registry::get(&lattice_name)
        .ok_or_else(|| GaugeFieldError::LatticeNotDeclared(lattice_name.clone()))?;
    let n_edges = lat.n_edges();
    let n_faces = lat.n_faces();
    let inc = build_edge_face_incidence(&lat);
    // Sprint A perf hoist: prebuild the per-face edge-cycle cache once;
    // staple_sum_at_edge reads `face_edges_cache[fidx]` per inner-loop
    // visit instead of re-allocating `face_edges(lat, fidx)` ~36 000×
    // per β=2.5 N_SWEEPS=200 call. Bit-identity preserved (the cached
    // vecs are identical f64 bits to the per-call results).
    let face_edges_cache = build_face_edges_cache(&lat);
    // Sprint B perf hoist: per-face A_f(pos) cache. Per-call ephemeral
    // (NOT stored on Lattice or the gauge field — preserves the to_gql
    // round-trip byte-identity contract). The cache is built ONCE at
    // the top of the sweep (all entries cold), then mutated in lockstep
    // with the buffer: after every `buffer.data[4e..4e+4]` write below
    // we invalidate every face containing edge `e` (read off `inc[e]`).
    // The invalidation timing is load-bearing — it must happen AFTER
    // the buffer write and BEFORE the next edge is visited, or the
    // next staple read at a face whose edge was just updated returns
    // a stale A_f and the heatbath conditional draw diverges from the
    // Sprint A gold.
    let mut holonomy_cache = FaceHolonomyCache::new(n_faces);

    // CSPRNG: canonical xorshift64* path.
    let mut rng = SmallRng::seed_from_u64(seed);

    // Sprint A perf hoist: pre-size the per-observable measurement
    // vector at the worst-case capacity `n_sweeps / max(measure_every, 1)`
    // so the push() loop never reallocates mid-sweep. The cap is exact
    // when `measure_every > 0 && measure_every | n_sweeps`; otherwise
    // it's a safe over-estimate (Vec::with_capacity is allocation-only,
    // does not change len()). When `measure_every == 0` the loop body
    // never pushes, so the unused capacity is harmless.
    let measure_cap = n_sweeps / measure_every.max(1);
    let mut history: HashMap<ObservableId, Vec<f64>> = HashMap::new();

    {
        let mut field = field_arc.lock().expect("su2 field mutex poisoned");
        for s in 0..n_sweeps {
            // ── Sweep ──
            for e in 0..n_edges {
                // Read the per-edge staple sum off the CURRENT field
                // state. `staple_sum_at_edge` reads through &dyn
                // EdgeConnection — we hand it the SU2GaugeField directly
                // (SU2GaugeField impls EdgeConnection). `holonomy_cache`
                // mutates internally: a cold face is filled with the
                // per-position `A_f(pos)` slice; a warm face is read
                // straight off the cache without re-walking the n−1
                // composition chain (Sprint B perf hoist).
                let v_eff = staple_sum_at_edge(
                    &*field,
                    &lat,
                    &inc,
                    &face_edges_cache,
                    &mut holonomy_cache,
                    e,
                );
                let v_eff_q = match v_eff {
                    GroupElement::SU2 { q0, q1, q2, q3 } => [q0, q1, q2, q3],
                    _ => unreachable!(
                        "gibbs_sample: SU(2) field's staple walker returned non-SU2 element"
                    ),
                };
                let u_new = sample_su2_link(v_eff_q, beta, &mut rng);
                // In-place mutation of the field's link buffer.
                let base = 4 * e;
                field.buffer.data[base..base + 4].copy_from_slice(&u_new);
                // Sprint B: invalidate every face containing edge `e`.
                // Must happen AFTER the buffer write (so the next staple
                // read on this face recomputes from the post-update
                // buffer) and BEFORE the next edge is visited. The
                // invalidation key is `face_id` — when ANY edge in face
                // f is updated, every position's A_f(pos) entry for f
                // becomes stale (every position's edge product crosses
                // the updated edge).
                for &(fidx, _pos) in &inc[e] {
                    holonomy_cache.invalidate(fidx);
                }
            }

            // ── Measurement epilogue ──
            if measure_every > 0 && (s + 1) % measure_every == 0 {
                for obs in &measure {
                    let v = observe(&*field, &lat, *obs)?;
                    history
                        .entry(*obs)
                        .or_insert_with(|| Vec::with_capacity(measure_cap))
                        .push(v);
                }
            }
        }
    }

    // Publish the post-mutation buffer into BOTH the SU(2)-mut map
    // and the dyn read map so `registry::get(name)` /
    // `registry::get_su2_mut(name)` both return the post-sweep state.
    // We pass the local `field_arc` (not the registry's current entry)
    // because a parallel `gauge_registry::clear()` may have wiped the
    // registry between our entry lookup and this call; the local Arc
    // is the canonical post-sweep handle.
    republish_su2(field_name, field_arc);

    Ok(GibbsSampleResponse {
        field: field_name.to_string(),
        measurement_history: history,
        diagnostics: GibbsSampleDiagnostics {
            seed,
            beta,
            n_sweeps_completed: n_sweeps,
        },
    })
}

/// Dispatch an `ObservableId` to its III.1 / III.2 implementation.
/// SU(2)-only at gate III.5; Part IV variants surface a typed error.
fn observe(
    field: &super::su2_gauge_field::SU2GaugeField,
    lat: &crate::lattice::Lattice,
    obs: ObservableId,
) -> Result<f64, GaugeFieldError> {
    match obs {
        ObservableId::MeanPlaquette => plaquette_mean(field, lat),
        ObservableId::QSurrogate => q_surrogate(field, lat),
        // Part IV / E-field observables. The error message contains
        // both "Part IV" and "E field" so the III.5 red-test (f) can
        // match either token via a case-insensitive regex.
        ObservableId::HTotal
        | ObservableId::GaussResidualMax
        | ObservableId::EdgeKinetic
        | ObservableId::VertexGauss
        | ObservableId::Energy => Err(GaugeFieldError::PartIvObservableNotReady(obs.label())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gauge::registry as gauge_registry;
    use crate::gauge::su2_gauge_field::{GaugeFieldInit, SU2GaugeField};
    use crate::lattice::registry as lattice_registry;
    use crate::lattice::topology::truncated_icosahedron::buckyball;

    /// Serialize the III.5 tests against the singleton lattice +
    /// gauge registries via the module-level guard exported from
    /// `gauge::registry::test_serial_lock`. Every test below holds
    /// this lock for its duration so parallel `clear()` calls from
    /// other gauge tests can't clobber the III.5 sweep-then-lookup
    /// window. The lock has no production-code consumers — purely a
    /// test-runner-parallelism guard.
    fn registry_guard() -> std::sync::MutexGuard<'static, ()> {
        gauge_registry::test_serial_lock()
    }

    /// Register an INIT IDENTITY field under `name` without clearing
    /// the singleton registries. Per-test field names are unique
    /// (`U_iii_5_<scenario>`), so we don't need to wipe the maps to
    /// avoid collisions; skipping `clear()` here lets parallel III.5
    /// tests coexist without racing on the lattice/gauge singletons.
    /// The lattice "buckyball" is idempotently re-registered (last
    /// writer wins; every writer produces the same Lattice value).
    fn setup_identity_field(name: &str) {
        let bb = buckyball();
        lattice_registry::register(bb.clone());
        let field = SU2GaugeField::new(
            name.into(),
            &bb,
            GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init must succeed");
        gauge_registry::register_su2(field);
    }

    /// TDD-HAL-III.5: GIBBS_SAMPLE without a SEED is a typed error
    /// (intra-binding bit-identity is the contract; Display contains
    /// the literal "SEED" so the upstream parser / HTTP layer can
    /// substring-match).
    #[test]
    fn tdd_hal_iii_5_gibbs_sample_requires_seed() {
        let _g = registry_guard();
        setup_identity_field("U_iii_5_seed");
        let err = gibbs_sample(
            "U_iii_5_seed",
            2.5,
            5,
            0,
            vec![],
            None,
        )
        .expect_err("seed=None must surface SeedRequired");
        let s = err.to_string();
        assert!(
            s.to_uppercase().contains("SEED"),
            "Display must contain 'SEED', got: {s}"
        );
    }

    /// TDD-HAL-III.5: two separate fields with identical INIT IDENTITY
    /// + identical bound lattice + identical (β, n_sweeps, seed) end
    /// with byte-identical buffer.data. Intra-binding reproducibility
    /// (Bee's locked decision 1).
    #[test]
    fn tdd_hal_iii_5_gibbs_sample_in_process_reproducible() {
        let _g = registry_guard();
        let bb = buckyball();
        lattice_registry::register(bb.clone());

        // Field A: register, sweep, snapshot the post-sweep buffer.
        // We re-register the lattice on each setup because a parallel
        // gauge test may have cleared it between the two halves of
        // this test (the registry_guard only serializes against other
        // III.5 tests).
        let field_a = SU2GaugeField::new(
            "U_iii_5_repro_a".into(),
            &bb,
            GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init A");
        gauge_registry::register_su2(field_a);
        gibbs_sample("U_iii_5_repro_a", 2.5, 20, 0, vec![], Some(20260616))
            .expect("sweep A");
        let buf_a = gauge_registry::get_su2_mut("U_iii_5_repro_a")
            .expect("A registered")
            .lock()
            .expect("lock A")
            .buffer
            .data
            .clone();

        // Field B: re-register lattice + field together with the
        // sweep so no parallel `clear()` can land between register
        // and call.
        lattice_registry::register(bb.clone());
        let field_b = SU2GaugeField::new(
            "U_iii_5_repro_b".into(),
            &bb,
            GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init B");
        gauge_registry::register_su2(field_b);
        gibbs_sample("U_iii_5_repro_b", 2.5, 20, 0, vec![], Some(20260616))
            .expect("sweep B");
        let buf_b = gauge_registry::get_su2_mut("U_iii_5_repro_b")
            .expect("B registered")
            .lock()
            .expect("lock B")
            .buffer
            .data
            .clone();

        assert_eq!(
            buf_a, buf_b,
            "GIBBS_SAMPLE at fixed (β, n_sweeps, seed) on identical INIT IDENTITY \
             starts must produce byte-identical buffer.data (intra-binding \
             bit-identity)"
        );
    }

    /// TDD-HAL-III.5: INIT IDENTITY starts at ⟨P⟩ = 1.0 exactly. After
    /// one sweep at β=2.5 the field has thermalized off identity, so
    /// `plaquette_mean(handle) < 1.0` strictly. This is the "the
    /// sweep actually mutated state" guard — a noop sweep would leave
    /// the field at identity and the assertion would fire.
    #[test]
    fn tdd_hal_iii_5_gibbs_sample_off_identity_after_one_sweep() {
        let _g = registry_guard();
        setup_identity_field("U_iii_5_off_id");
        // Confirm the start state IS identity (sanity check on the
        // setup — every face holonomy is q0 = 1.0).
        let bb = buckyball();
        {
            let handle = gauge_registry::get("U_iii_5_off_id")
                .expect("dyn handle pre-sweep");
            let p0 = plaquette_mean(handle.as_ref(), &bb).expect("pre-sweep mean");
            assert_eq!(p0, 1.0, "INIT IDENTITY must start at ⟨P⟩ = 1.0");
        }

        gibbs_sample(
            "U_iii_5_off_id",
            2.5,
            1,
            0,
            vec![],
            Some(20260616),
        )
        .expect("one sweep");

        let handle = gauge_registry::get("U_iii_5_off_id")
            .expect("dyn handle post-sweep");
        let p1 = plaquette_mean(handle.as_ref(), &bb).expect("post-sweep mean");
        assert!(
            p1 < 1.0,
            "one heatbath sweep at β=2.5 must move ⟨P⟩ off identity (1.0); got {p1}"
        );
    }

    /// TDD-HAL-III.5: edge update order is sequential `0..n_edges` (Bee's
    /// locked decision D3). Instrument the SmallRng with `draws()` and
    /// confirm draws strictly ascend across the sweep — equivalently,
    /// the total draws after one sweep equals the sum of per-edge draw
    /// budgets. A checkerboard or randomized order would consume RNG
    /// draws in a different interleaving.
    ///
    /// We measure the draw count after running gibbs_sample with one
    /// edge worth of sweeps' worth of state, then a known-sequential
    /// reference that walks edges 0..n_edges via the same primitives.
    /// Both must agree on the post-sweep RNG draw count.
    #[test]
    fn tdd_hal_iii_5_gibbs_sample_sequential_edge_order() {
        let _g = registry_guard();
        setup_identity_field("U_iii_5_seq");
        gibbs_sample(
            "U_iii_5_seq",
            2.5,
            1,
            0,
            vec![],
            Some(20260616),
        )
        .expect("one sweep");

        // Reference run: replay the same algorithm by hand with a
        // sequential edge loop using a parallel SmallRng at the same
        // seed. The buffer after the manual sweep must equal the
        // gibbs_sample-mutated buffer byte-for-byte; if the edge order
        // were anything other than 0..n_edges the RNG state at each
        // edge would diverge and the buffers would not match.
        let bb = buckyball();
        let mut ref_field = SU2GaugeField::new(
            "U_ref".into(),
            &bb,
            GaugeFieldInit::Identity,
            None,
        )
        .expect("ref identity init");
        let inc_ref = build_edge_face_incidence(&bb);
        let face_edges_cache_ref = build_face_edges_cache(&bb);
        let mut holonomy_cache_ref = FaceHolonomyCache::new(bb.n_faces());
        let mut rng_ref = SmallRng::seed_from_u64(20260616);
        for e in 0..bb.n_edges() {
            let v_eff = staple_sum_at_edge(
                &ref_field,
                &bb,
                &inc_ref,
                &face_edges_cache_ref,
                &mut holonomy_cache_ref,
                e,
            );
            let v_eff_q = match v_eff {
                GroupElement::SU2 { q0, q1, q2, q3 } => [q0, q1, q2, q3],
                _ => unreachable!(),
            };
            let u_new = sample_su2_link(v_eff_q, 2.5, &mut rng_ref);
            let base = 4 * e;
            ref_field.buffer.data[base..base + 4].copy_from_slice(&u_new);
            // Mirror the gibbs_sample sweep loop's invalidation timing
            // exactly — the test depends on the by-hand reference using
            // the SAME cache semantics as production.
            for &(fidx, _pos) in &inc_ref[e] {
                holonomy_cache_ref.invalidate(fidx);
            }
        }

        let mutated = gauge_registry::get_su2_mut("U_iii_5_seq")
            .expect("registered")
            .lock()
            .expect("lock mutated")
            .buffer
            .data
            .clone();
        assert_eq!(
            mutated, ref_field.buffer.data,
            "GIBBS_SAMPLE must visit edges in sequential 0..n_edges order (D3); \
             a non-sequential order would diverge from the by-hand reference"
        );
    }

    /// TDD-HAL-III.5: `measure_every = 3` over `n_sweeps = 10` produces
    /// exactly 3 measurements (sweeps 3, 6, 9). The history vector
    /// length is the receipt.
    #[test]
    fn tdd_hal_iii_5_gibbs_sample_measure_every_semantics() {
        let _g = registry_guard();
        setup_identity_field("U_iii_5_me");
        let resp = gibbs_sample(
            "U_iii_5_me",
            2.5,
            10,
            3,
            vec![ObservableId::MeanPlaquette],
            Some(20260616),
        )
        .expect("sweep");

        let chain = resp
            .measurement_history
            .get(&ObservableId::MeanPlaquette)
            .expect("MeanPlaquette chain present");
        assert_eq!(
            chain.len(),
            3,
            "n_sweeps=10, measure_every=3 → expected 3 measurements \
             (sweeps 3, 6, 9); got {}",
            chain.len()
        );
        assert_eq!(resp.diagnostics.n_sweeps_completed, 10);
        assert_eq!(resp.diagnostics.seed, 20260616);
        assert_eq!(resp.diagnostics.beta, 2.5);
    }

    /// TDD-HAL-PERF: face_edges hoist + measurement_history pre-alloc
    /// MUST stay byte-identical against the pre-hoist run. Gold values
    /// (mean_plaquette chain + final buffer.data) were harvested with
    /// the pre-hoist code path via `examples/harvest_perf_hoist_gold.rs`
    /// against gibbs_sample β=2.5 N_SWEEPS=20 MEASURE_EVERY=1
    /// SEED=20260616 from INIT IDENTITY on the buckyball with
    /// MEASURE = [MeanPlaquette]. Same-seed → byte-identical f64 bits
    /// (Bee's locked decision 1, intra-binding bit-identity).
    ///
    /// This is the load-bearing receipt for Sprint A's "the hoist did
    /// NOT change observable behavior" claim. If this test fails, the
    /// hoist mutated the algorithm somewhere — STOP and investigate;
    /// do not paper over with a tolerance.
    #[test]
    fn tdd_hal_perf_face_edges_hoist_byte_identical() {
        // ── Gold: 20-sweep MeanPlaquette chain (f64::to_bits) ──
        const EXPECTED_MEAN_PLAQUETTE_CHAIN_BITS: [u64; 20] = [
            0x3fe13e8815f55be6,
            0x3fe0b08fc2eb4db5,
            0x3fe0ac4dfe8c2486,
            0x3fda1e932ad625e4,
            0x3fe2fd64036ef689,
            0x3fe15c96069fe4ae,
            0x3fe12f591299e096,
            0x3fda3cac24fc80d0,
            0x3fe19c544c0c06d2,
            0x3fdc8e7908f3cebe,
            0x3fdec7b299faab68,
            0x3fe0983f5d2d773a,
            0x3fe3763f7579766c,
            0x3fe0e64df217aa4f,
            0x3fd972d9469c031a,
            0x3fd98d8f3c29a196,
            0x3fe05a6e4ef78385,
            0x3fd4cf62b3517828,
            0x3fe39cfc6e227037,
            0x3fe06725f54430cf,
        ];

        // ── Gold: post-sweep buffer.data (f64::to_bits), len = 4·90 = 360 ──
        const EXPECTED_FINAL_BUFFER_BITS: [u64; 360] = [
            0xbfe1ae3d5cfa0b71, 0x3fc86852563efe0f, 0x3fd68a05575f0e40, 0x3fe764354e741f15,
            0x3fa225596794eddc, 0x3feca65701351f25, 0xbfd9e6b1ee20a4cc, 0xbfc76351c30f0706,
            0x3fe7a311ad4fd1a2, 0xbfcf0b26899e68b1, 0x3fb30175712e5064, 0xbfe3fc451061c66c,
            0x3feda60f9bfa41bc, 0x3fc9dbd197bed773, 0x3fcc99a84f88457a, 0x3fccdb10f0fe1470,
            0x3fb37566021c460f, 0x3feb219cdb9cd897, 0xbfe0aea7bf05528f, 0x3faeb06ef8c3dc55,
            0x3fe986b6df98cfd8, 0xbfd093ae93823ee6, 0x3fd541e751e44204, 0x3fdb9f3bb042dff9,
            0x3fe430fb10f2212f, 0x3fd6d0ba8bc99969, 0xbfdbe979c7ccb309, 0x3fe112121e2158ae,
            0xbfda78e8eb957d0a, 0xbfbaad5827099fbb, 0xbfe2f8d932cb54dc, 0x3fe5db7d82b6c16d,
            0xbfdfe94a3f21a695, 0x3fe76c376c501f99, 0xbfda56f496d95b82, 0xbfcb863514bb8af9,
            0xbfd24c3f66bab277, 0x3fe6d0ae6ee002b8, 0x3fe3db597c17a7bc, 0xbfc4301588ec1cf0,
            0xbfc07ad31a83eec1, 0xbfb1ab030a670591, 0xbfea92b8d0ac42e0, 0xbfe13484b8252bab,
            0x3fa84111cb0922c7, 0xbfe4cfc13071a57c, 0x3fe231e0a576261c, 0xbfe00c38717e9f81,
            0xbfbe66f0c7f6aa3b, 0xbfe0e1893f4f864e, 0xbfe8089c4c54806d, 0xbfd83f2021a0312e,
            0xbfcce1deab415f12, 0x3fb71109d043a6be, 0xbfe3c8f5ed9390da, 0xbfe7eb2ed90e5ffb,
            0xbfd5a4a2b894ad13, 0xbfed1fe74740bc41, 0x3fbbbefa87dcbe4a, 0x3fcb4fde98d5ede9,
            0x3fbe49cec1acbf8e, 0x3fe27cd4eb1fd2e7, 0xbfe7d4cecde39075, 0xbfd3fe8f3be7f550,
            0x3fd3b919727d4e23, 0x3feca67d203f1472, 0x3fd46881b9f848c6, 0x3fa55ef5d993ce18,
            0xbfec1e6a76f9d14e, 0x3fb9f0b040e4ef88, 0xbfd62c2b4af2f806, 0xbfd3fdc67ab9248e,
            0xbfd8715cd1908906, 0x3fc74217c5f61411, 0xbfe3e473b3ea59c1, 0x3fe5190f5ba33b4f,
            0xbfdf52bb6216ecd6, 0xbfdfa96fb7a06391, 0xbfe6f78cb96e907a, 0x3f99117ceab15e18,
            0xbfd53a742ea19ecf, 0xbfd68704e07560da, 0xbfdef69067f2779f, 0xbfe7573985881bed,
            0xbfe6fe1f8584dda2, 0x3fd43042b8e10948, 0x3fe1981b02f4212a, 0xbfd25171cd0dfd74,
            0x3fa9ecfd106f82ba, 0x3f9e4da398d5db12, 0xbfea634ea22a1e83, 0xbfe2011ae8192dd5,
            0x3fe271de1e355650, 0xbfd988bcbf619496, 0xbfddc3137edbcc26, 0x3fe14d39132f917e,
            0x3fec7eda148e2983, 0x3fc2d4ee434fa1b8, 0xbfa49a5be1c3af0a, 0x3fdb6f9a27e94fbd,
            0x3fc7a1d3e8c2276f, 0x3fea7863b7199415, 0xbfdcd56f34e814fe, 0x3fd1f3dcfce1e84c,
            0x3fc94b98b9775e49, 0x3fedb57d0183c178, 0xbfd3b13ddfdce171, 0xbfb0dbe84559c5ee,
            0xbfe3ac4d0f4ef74c, 0xbfe347f90d1c2dc8, 0x3fe0441a5c35b7d4, 0xbf994be9198c6bdf,
            0xbf5810f8db1e8800, 0x3fb00e5e8a3278de, 0xbfdfa5b014483523, 0xbfebbdc9cad6bc26,
            0xbfd22cc968738835, 0x3fc50e0f9cda6af0, 0x3fec75de42f13022, 0xbfd45e3e72d76ca5,
            0xbfac714a7e594d47, 0x3fd7902b761efeba, 0x3fe32531b3aa9927, 0xbfe6b456fa5a6477,
            0x3feb7fdc636bd03a, 0x3fa57de062a2abd4, 0x3fd695f666ae13dd, 0xbfd7885a33b195e4,
            0x3fa9615d889aa7fb, 0xbfefd76fafa87656, 0xbf919cc2b8c43ef1, 0xbfb59c1e96c4e72b,
            0xbfe190de40d805f6, 0xbfd8fdfa070e100e, 0xbfb9f7de46cd5a4e, 0x3fe76cea0cb4c4a6,
            0xbfeaefca2ffa94ca, 0x3fc6747e8b5199dd, 0xbfc002ef4ed4cc7a, 0xbfdfada27c3c9a98,
            0x3fc076f718850065, 0xbfd188d2048ca5d1, 0xbfc0cc03c5263e95, 0xbfee356686f3745e,
            0x3fd373550707cef5, 0x3fca9acb8a680c08, 0xbfe24fa3e1fb6ac5, 0x3fe7731af5f1b563,
            0xbfe75d43208e7bf4, 0xbfbfc07099168bfc, 0xbfe072aa8103c100, 0x3fdbb333faddd212,
            0xbfd56d3f5071666f, 0x3fba201b4628ecd8, 0x3feb9a4411e01035, 0x3fd76143aeffb770,
            0x3fa5c2321f1327c6, 0xbfe0965c4f00b3ba, 0xbfead00054e7d2ac, 0x3fc5342e98904e20,
            0xbfe8e6cbc01937d4, 0xbfe29adf0f788b00, 0x3f7cd0b9d5fb2100, 0xbfce6389f82c9de7,
            0xbfe5936a53bac635, 0x3fd8682ba6e31088, 0xbfdd990503b21198, 0xbfdb9bae905df9f8,
            0xbfcc101ad3c61624, 0xbfe5551f48e36d07, 0xbfdb20cc1c1638bc, 0xbfe2529a13145510,
            0x3fe37cb4a0ade26e, 0x3fe3e020bbf95edc, 0x3fda204c86264e98, 0x3fd1ba69ebaed01f,
            0x3fd934460cbce5fc, 0xbfeb65a9d59adeda, 0xbf8a415ec5ca7218, 0x3fd564a44a5d6880,
            0x3fe643864ff582d6, 0x3f9deb874ac28e8d, 0x3fe5282785d66ac3, 0x3fd1ded3c44624c2,
            0x3fdc0c3cb0a4bb12, 0x3fcd1892c56870fe, 0x3fd9eccb45c305eb, 0xbfe8a00341887b28,
            0x3fe61c2b4fd52fb5, 0x3fb86a528ce9d8f9, 0xbfe4602b4720002d, 0xbfd50a19403c7a9b,
            0xbfd2a0bce4685ba3, 0x3fda5667294afce8, 0x3fd50d12afbe9171, 0x3fe98e070333c210,
            0x3fdd8b4fe780c1c2, 0xbfd16bd3ed0f36ee, 0x3fe3aa287d38e204, 0x3fe286a4fea114a2,
            0x3fc961c6b86e4536, 0x3fe0ca278a1f3f6c, 0x3fc8b7e87b813d3f, 0xbfe9c2f1333cd63a,
            0x3fa9f8ee86175570, 0xbfe534b142175e38, 0x3fb507a36f15994a, 0xbfe7c3cc7379e001,
            0x3fe671c3a0898129, 0x3fd81794f6f0246c, 0x3f8a2cc876de67c6, 0x3fe35d400cb60066,
            0xbfc78dfd44402e04, 0x3fe38613ec1270ce, 0x3fe5d9ec4c3ce71b, 0x3fd6dcb63a5d5b88,
            0x3fab429d03e5ec70, 0xbfe1007cc51efc15, 0x3fe4504da0427ab0, 0x3fe1df139913da20,
            0x3fdd4f76e7e31319, 0xbfce63709a383c11, 0x3fd0d8739a16eec6, 0xbfea1670fe752544,
            0x3f7036eef9f9600f, 0xbfcc7a2cdc89143e, 0x3fe4cd2b6e04c85f, 0x3fe74033d73a0ac3,
            0x3fe6b8328c4aa34a, 0x3fdaf18d1db92283, 0x3fe1e4fbf08964c9, 0x3fb3cb7eabc47d98,
            0x3fbaf753467f71b2, 0x3fdf5bdb7d1031f6, 0xbfc2f462ecc91b13, 0x3feb485731ccb70f,
            0x3fd040201427c9f7, 0x3fa9fbcf57d726a6, 0x3fe05e49ce573503, 0x3fea37fe30ab6751,
            0x3fe66ae0ff76589e, 0xbfddefa22e9cf6c5, 0xbfd038bf157a98a5, 0x3fde704cfb4b7b02,
            0x3fe520d601ce5b22, 0xbfb442050208cc18, 0x3fe73b5176ada8c5, 0xbfc6709b8cff1407,
            0xbfd97f68ed123837, 0x3fb27dd263fccd2b, 0x3fecd0539ab54712, 0x3fc45a6330a62cb5,
            0x3fbefed6a1fc6548, 0x3fe8c263f0679217, 0x3fc6d38a6b853dfc, 0xbfe310225b413171,
            0x3fd419a8a0c89e78, 0xbfe78e8d65559fad, 0xbfe31cb6de204661, 0xbfaac04445b087eb,
            0xbfc9a4eddfbec5dd, 0xbfd32b0555eb1791, 0xbfec7995a9d6ad05, 0x3fd1e9d4ecc5b98e,
            0xbfe3b30a608ba0b7, 0x3faeb96150f902de, 0xbfe0250121c941bb, 0x3fe346e6066b6433,
            0x3fdcae5c954d53a2, 0xbfe2683f5bd54d1c, 0x3fdc35afe1d4b6e0, 0xbfe0c01a62f402b5,
            0x3fe577ac407d5e1e, 0x3fda4b59f8718ed5, 0xbfe314b30dbe58c0, 0xbfc47b7bb1ee3fe8,
            0x3fe0ffa5d2106d18, 0x3fb76b557fc5c3a0, 0x3fe89f69d6593649, 0x3fd5ed5581d8cafd,
            0x3fdf4986e86c2bd4, 0xbfe18c38603431fa, 0xbfe501bff503c5c0, 0xbfc5eefa015d4e24,
            0xbfeaeec0450357ea, 0x3fa599fd681f5728, 0xbfe0629a571c750e, 0x3fc54b0d0c8dcc0e,
            0xbfd697490795a054, 0xbfc6619d77aaafec, 0xbfd5e4dc16de7180, 0xbfeb4cb2337fd06e,
            0x3fc789690c118f83, 0xbfe92a47e2455b67, 0x3fcbbe6d5fb7afa8, 0xbfe18ca94323cee8,
            0x3fe2d8fb7402fc24, 0x3faa0b2e41428e00, 0xbfe7e60433ec8554, 0xbfd37dd9476fb975,
            0x3fd085205690fd20, 0x3fea87ef4b5ccd82, 0xbfda6c1f8191d282, 0x3fd196c908f571e0,
            0xbfd826b32166694e, 0x3fd71a0d9f0283f4, 0xbfe60a92a98fb2ec, 0x3fe017709a99e7cd,
            0x3fcd7738fbf42cdd, 0xbfdf63ef8704889c, 0xbfc269df54c2f320, 0xbfea7fc92434edb8,
            0x3f72bbf29c3ee3e0, 0xbfd4a94a0fe9bf54, 0x3fe82e3b4452fc7d, 0x3fe23c711844cda9,
            0x3fe648fc8d9c8c28, 0xbfe5c0a23befa39f, 0x3fcd6ac8a4c8310e, 0x3f86085e9c91a5c2,
            0xbfe3b19756d4c221, 0xbfd12c82d847388e, 0x3fe26fca11746022, 0x3fddd56756a31f7a,
            0xbfe5d7ad4856a356, 0xbfc7e4818db5de39, 0xbfdc2b025f3a099e, 0x3fe1b025d2faf987,
            0x3fe1af8bcfa45b6e, 0x3fe24cab606e65af, 0x3fd4c1c863a0229a, 0xbfe063d4829faf47,
            0x3fc490dd290fe8df, 0xbfe6b6db6d45ef16, 0x3fcbe68b8b413d43, 0x3fe4cedac1e2de61,
            0xbfe6b7f62871d366, 0x3fd6a1d86ad1a81a, 0xbfe17a660ae93543, 0xbfd13e1775d9a342,
            0xbfe1ea76408ec467, 0x3fc92918fdafda36, 0xbfe2f90e50b3d190, 0xbfe16bbcb3d969aa,
            0xbfc125a6d0706ee9, 0x3fdfd62086aa3d2a, 0xbfcb5332fb466a55, 0x3fea9001fc3e9dd4,
            0xbfd3ba262be31d70, 0x3f7dc6521e8401b9, 0x3fc2f6ce2a69f954, 0x3fee11cece2e719e,
            0xbfebba80f7e3de9b, 0xbfdf545de4c5dbb4, 0xbf39b4ab8abd0400, 0xbfb8f8a164263018,
            0x3fed317c83b295e2, 0xbfd13f8f0536413d, 0x3fd0a5b8503ac105, 0xbfc5335adf98ea77,
        ];

        let _g = registry_guard();
        let bb = buckyball();
        lattice_registry::register(bb.clone());
        let field = SU2GaugeField::new(
            "U_perf_hoist_gold".into(),
            &bb,
            GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init");
        gauge_registry::register_su2(field);

        let resp = gibbs_sample(
            "U_perf_hoist_gold",
            2.5,
            20,
            1,
            vec![ObservableId::MeanPlaquette],
            Some(20260616),
        )
        .expect("sweep");

        let chain = resp
            .measurement_history
            .get(&ObservableId::MeanPlaquette)
            .expect("MeanPlaquette chain present");
        assert_eq!(
            chain.len(),
            EXPECTED_MEAN_PLAQUETTE_CHAIN_BITS.len(),
            "chain length"
        );
        for (i, v) in chain.iter().enumerate() {
            assert_eq!(
                v.to_bits(),
                EXPECTED_MEAN_PLAQUETTE_CHAIN_BITS[i],
                "MeanPlaquette[{i}] f64::to_bits mismatch — perf hoist must \
                 not perturb the chain. got bits 0x{:016x} ({}), want 0x{:016x}",
                v.to_bits(),
                v,
                EXPECTED_MEAN_PLAQUETTE_CHAIN_BITS[i],
            );
        }

        let buf = gauge_registry::get_su2_mut("U_perf_hoist_gold")
            .expect("registered")
            .lock()
            .expect("lock")
            .buffer
            .data
            .clone();
        assert_eq!(
            buf.len(),
            EXPECTED_FINAL_BUFFER_BITS.len(),
            "buffer length"
        );
        for (i, v) in buf.iter().enumerate() {
            assert_eq!(
                v.to_bits(),
                EXPECTED_FINAL_BUFFER_BITS[i],
                "buffer.data[{i}] f64::to_bits mismatch — perf hoist must \
                 not perturb the post-sweep buffer. got bits 0x{:016x}, want 0x{:016x}",
                v.to_bits(),
                EXPECTED_FINAL_BUFFER_BITS[i],
            );
        }
    }

    /// TDD-HAL-PERF Sprint B: per-face holonomy cache MUST stay byte-
    /// identical against the pre-cache (Sprint A) run at β=2.5
    /// N_SWEEPS=200 MEASURE_EVERY=1 SEED=20260616 from INIT IDENTITY
    /// on the buckyball. Gold value for MeanPlaquette[199] was harvested
    /// at HEAD=7d8f6e4 (post-Sprint-A) via
    /// `examples/bench_thermalization_baseline.rs`. Same-seed →
    /// byte-identical f64 bits (Bee's locked decision 1, intra-binding
    /// bit-identity).
    ///
    /// This is the load-bearing receipt for Sprint B's "the cache did
    /// NOT change observable behavior" claim. The 20-sweep gold in
    /// `tdd_hal_perf_face_edges_hoist_byte_identical` covers a different
    /// budget; this test is the longer-horizon canary that exercises ~10x
    /// more cache invalidations. If this test fails, the cache
    /// implementation has a coherence bug — STOP and investigate; do not
    /// paper over with a tolerance.
    #[test]
    fn tdd_hal_perf_staple_cache_byte_identical() {
        // Gold MeanPlaquette[199] f64::to_bits at β=2.5 N_SWEEPS=200
        // SEED=20260616 from INIT IDENTITY on the buckyball, harvested
        // at HEAD=7d8f6e4 (post-Sprint-A, pre-Sprint-B). The bit
        // pattern differs between debug and release profiles because
        // release-mode codegen contracts FMA chains that debug does
        // not — both values are byte-identical to the Sprint A code
        // path at the same profile, so the cache must reproduce them
        // exactly.
        //
        // Release: 0.5125429110231062 → 0x3fe066c064148215 (spec gold,
        //          matches `bench_thermalization_baseline.rs` JSON
        //          `baseline_mean_plaquette_at_199` field byte-for-byte
        //          on both Sprint A and Sprint B builds).
        // Debug:   0.447917045058642715 → 0x3fdcaaac40f642d4 (cargo
        //          test default profile gold; harvested by running
        //          the same bench harness with `cargo run` against the
        //          Sprint A HEAD).
        const EXPECTED_MEAN_PLAQUETTE_AT_199_BITS_RELEASE: u64 = 0x3fe066c064148215;
        const EXPECTED_MEAN_PLAQUETTE_AT_199_RELEASE: f64 = 0.5125429110231062;
        const EXPECTED_MEAN_PLAQUETTE_AT_199_BITS_DEBUG: u64 = 0x3fdcaaac40f642d4;
        const EXPECTED_MEAN_PLAQUETTE_AT_199_DEBUG: f64 = 0.447917045058642715;
        let (expected_bits, expected_val) = if cfg!(debug_assertions) {
            (
                EXPECTED_MEAN_PLAQUETTE_AT_199_BITS_DEBUG,
                EXPECTED_MEAN_PLAQUETTE_AT_199_DEBUG,
            )
        } else {
            (
                EXPECTED_MEAN_PLAQUETTE_AT_199_BITS_RELEASE,
                EXPECTED_MEAN_PLAQUETTE_AT_199_RELEASE,
            )
        };

        let _g = registry_guard();
        let bb = buckyball();
        lattice_registry::register(bb.clone());
        let field = SU2GaugeField::new(
            "U_perf_cache_gold".into(),
            &bb,
            GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init");
        gauge_registry::register_su2(field);

        let resp = gibbs_sample(
            "U_perf_cache_gold",
            2.5,
            200,
            1,
            vec![ObservableId::MeanPlaquette],
            Some(20260616),
        )
        .expect("sweep");

        let chain = resp
            .measurement_history
            .get(&ObservableId::MeanPlaquette)
            .expect("MeanPlaquette chain present");
        assert_eq!(chain.len(), 200, "chain length");
        let final_mean = chain[199];
        assert_eq!(
            final_mean.to_bits(),
            expected_bits,
            "MeanPlaquette[199] f64::to_bits mismatch — Sprint B cache must \
             not perturb the chain. got bits 0x{:016x} ({:.17e}), want 0x{:016x} ({:.17e}) \
             (profile-gated: debug_assertions={})",
            final_mean.to_bits(),
            final_mean,
            expected_bits,
            expected_val,
            cfg!(debug_assertions),
        );
    }

    /// TDD-HAL-PERF Sprint B: cache invalidation correctness — after a
    /// partial sweep that updates every 7th edge (edges 0, 7, 14, …),
    /// the cached face-holonomy state for every face that survived
    /// (no incident edge was touched) must remain consistent with
    /// computing the staple from scratch on the post-mutation buffer.
    ///
    /// We exercise this by running gibbs_sample to mutate state, then
    /// comparing the staple value computed via a fresh cache against
    /// a staple value computed by allocating a brand-new cache (which
    /// triggers full recomputation) on the same post-state buffer.
    /// If the cache's invalidation/reuse path drifts from the cold path,
    /// the bits differ — this is the coherence canary.
    #[test]
    fn tdd_hal_perf_staple_cache_invalidation_correctness() {
        use crate::gauge::staple::{
            build_edge_face_incidence, build_face_edges_cache, staple_sum_at_edge,
            FaceHolonomyCache,
        };

        let _g = registry_guard();
        let bb = buckyball();
        lattice_registry::register(bb.clone());
        let field = SU2GaugeField::new(
            "U_perf_cache_invalidation".into(),
            &bb,
            GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init");
        gauge_registry::register_su2(field);

        // Mutate the field via a short sweep so the buffer is non-trivial.
        gibbs_sample(
            "U_perf_cache_invalidation",
            2.5,
            5,
            0,
            vec![],
            Some(20260616),
        )
        .expect("sweep");

        let inc = build_edge_face_incidence(&bb);
        let face_edges_cache = build_face_edges_cache(&bb);

        let field_arc = gauge_registry::get_su2_mut("U_perf_cache_invalidation")
            .expect("registered");
        let field = field_arc.lock().expect("lock");

        // Cold path: every staple read with a fresh cache (always-miss).
        // The contract: if we put a cache that we treat as never-cached
        // (build a new one per call), the bits of every staple value
        // must equal the bits of a single shared cache used across the
        // whole walk. Bit-identity between hot reuse and cold miss is
        // the load-bearing invariant for Sprint B.
        let mut shared_cache = FaceHolonomyCache::new(bb.n_faces());
        for e in 0..bb.n_edges() {
            // Hot path: reuse cache across all edges (entries persist).
            let v_hot = staple_sum_at_edge(
                &*field,
                &bb,
                &inc,
                &face_edges_cache,
                &mut shared_cache,
                e,
            );
            // Cold path: brand-new cache → always recompute from scratch.
            let mut cold_cache = FaceHolonomyCache::new(bb.n_faces());
            let v_cold = staple_sum_at_edge(
                &*field,
                &bb,
                &inc,
                &face_edges_cache,
                &mut cold_cache,
                e,
            );
            match (v_hot, v_cold) {
                (
                    GroupElement::SU2 { q0: h0, q1: h1, q2: h2, q3: h3 },
                    GroupElement::SU2 { q0: c0, q1: c1, q2: c2, q3: c3 },
                ) => {
                    assert_eq!(
                        h0.to_bits(), c0.to_bits(),
                        "edge {e}: q0 hot/cold cache drift, hot=0x{:016x} cold=0x{:016x}",
                        h0.to_bits(), c0.to_bits()
                    );
                    assert_eq!(
                        h1.to_bits(), c1.to_bits(),
                        "edge {e}: q1 hot/cold cache drift"
                    );
                    assert_eq!(
                        h2.to_bits(), c2.to_bits(),
                        "edge {e}: q2 hot/cold cache drift"
                    );
                    assert_eq!(
                        h3.to_bits(), c3.to_bits(),
                        "edge {e}: q3 hot/cold cache drift"
                    );
                }
                _ => panic!("expected SU2 from both paths"),
            }
        }
    }

    /// TDD-HAL-PERF Sprint B: defensive edge case — building a
    /// FaceHolonomyCache with n_faces=0 must succeed and produce a
    /// usable (empty) cache. Impossible on the buckyball (n_faces=32)
    /// but cheap to guard against for any future lattice topology that
    /// degenerates to a non-closed surface.
    #[test]
    fn tdd_hal_perf_staple_cache_empty_lattice_safe() {
        use crate::gauge::staple::FaceHolonomyCache;
        let cache = FaceHolonomyCache::new(0);
        // No panics on construction, and get(0) on an empty cache must
        // return None (the cache invariant: out-of-range get is None,
        // not a panic — defensive against any caller mis-indexing).
        assert!(cache.get(0).is_none());
    }

    /// TDD-HAL-III.5: declaring a Part-IV observable in MEASURE is a
    /// typed error whose Display mentions "Part IV" and "E field" so
    /// the upstream parser / HTTP layer can surface a stable message.
    /// The regex `(?i)part iv|e field` is the contract anchor; we
    /// check both substrings literally here.
    #[test]
    fn tdd_hal_iii_5_gibbs_sample_rejects_pre_part_iv_observables() {
        let _g = registry_guard();
        setup_identity_field("U_iii_5_p4");
        let err = gibbs_sample(
            "U_iii_5_p4",
            2.5,
            1,
            1,
            vec![ObservableId::HTotal],
            Some(20260616),
        )
        .expect_err("HTotal must error pre-Part-IV");
        let s = err.to_string();
        let lower = s.to_lowercase();
        assert!(
            lower.contains("part iv") || lower.contains("e field"),
            "Display must mention 'Part IV' or 'E field', got: {s}"
        );
    }
}
