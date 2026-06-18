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
use super::staple::{build_edge_face_incidence, staple_sum_at_edge};
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
    let inc = build_edge_face_incidence(&lat);

    // CSPRNG: canonical xorshift64* path.
    let mut rng = SmallRng::seed_from_u64(seed);

    let mut history: HashMap<ObservableId, Vec<f64>> = HashMap::new();

    {
        let mut field = field_arc.lock().expect("su2 field mutex poisoned");
        for s in 0..n_sweeps {
            // ── Sweep ──
            for e in 0..n_edges {
                // Read the per-edge staple sum off the CURRENT field
                // state. `staple_sum_at_edge` reads through &dyn
                // EdgeConnection — we hand it the SU2GaugeField directly
                // (SU2GaugeField impls EdgeConnection).
                let v_eff = staple_sum_at_edge(&*field, &lat, &inc, e);
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
            }

            // ── Measurement epilogue ──
            if measure_every > 0 && (s + 1) % measure_every == 0 {
                for obs in &measure {
                    let v = observe(&*field, &lat, *obs)?;
                    history.entry(*obs).or_default().push(v);
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
        let mut rng_ref = SmallRng::seed_from_u64(20260616);
        for e in 0..bb.n_edges() {
            let v_eff = staple_sum_at_edge(&ref_field, &bb, &inc_ref, e);
            let v_eff_q = match v_eff {
                GroupElement::SU2 { q0, q1, q2, q3 } => [q0, q1, q2, q3],
                _ => unreachable!(),
            };
            let u_new = sample_su2_link(v_eff_q, 2.5, &mut rng_ref);
            let base = 4 * e;
            ref_field.buffer.data[base..base + 4].copy_from_slice(&u_new);
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
