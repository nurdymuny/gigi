//! TDD-HAL-IV.9 — Harvest the Part IV canonical SYMPLECTIC_FLOW fixture
//! under `--release`.
//!
//! Closes gate IV.9 per the Part IV plan: the Part IV gold gate (IV.10)
//! writes a byte-identity assertion against a GIGI-INTERNAL reference,
//! not against any external mock. This file's only job is to run the
//! locked GQL block:
//!
//! ```text
//! LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY "S2";
//! GAUGE_FIELD U ON LATTICE buckyball GROUP SU(2) INIT IDENTITY;
//! GIBBS_SAMPLE U BETA 2.5 N_SWEEPS 200 SEED 20260616;
//! E_FIELD E ON GAUGE_FIELD U INIT MAXWELL_BOLTZMANN BETA 2.5 SEED 20260617;
//! SYMPLECTIC_FLOW U FROM (U=U, E=E) BETA 2.5 DT 0.02 N_STEPS 1000
//!     PROJECT_GAUSS TRUE MEASURE_EVERY 20
//!     MEASURE (H_TOTAL, MEAN(PLAQUETTE), GAUSS_RESIDUAL_MAX, Q_SURROGATE)
//!     SEED 20260617;
//! ```
//!
//! through the GQL parser + executor path ONCE, and freeze the final U
//! and E buffers, the 51 measurement chains, the step-index ladder, and
//! the flow diagnostics into
//! `tests/fixtures/halcyon/part_iv/symplectic_flow_canonical.json` (plus
//! a `_provenance.json` side-car).
//!
//! Gate IV.10 later loads this fixture and writes the (a) energy-drift
//! acceptance bound (`max|ΔH/H_0| < 1e-3`) and (b) the byte-identity
//! regression assertion (Halcyon row 1/2 of the A2 matrix) against it.
//!
//! ── Why `#[ignore]` ──
//!
//! 1000 KDK steps × per-step Tikhonov-CG projection × Wilson force is
//! several seconds on the dev box even at the release profile. CI's
//! gate-feature lib test must stay fast, so this harness is marked
//! `#[ignore]` and only runs when invoked explicitly via:
//!
//! ```text
//! cargo test --features halcyon --test harvest_part_iv_canonical \
//!     --release -- --ignored --nocapture
//! ```
//!
//! Re-running the harvest mutates the on-disk fixture in place. The
//! follow-up commit (the second commit landing this harness) pins the
//! `harvest_commit` SHA in the provenance side-car — same pattern as
//! II.2, III.8a, and III.8c.
//!
//! ── Profile pin (III.8c precedent) ──
//!
//! `#[cfg_attr(debug_assertions, ignore)]` on the harvest test would
//! prevent the test from running under `cargo test --features halcyon`
//! (debug profile) entirely. We instead rely on the unconditional
//! `#[ignore]` so the CI default never runs the harvest, and document
//! that the IV.10 byte-identity gates fire only under `--release` (where
//! the fixture was harvested). Re-harvesting under a debug profile
//! would land a different fixture and break IV.10 — DO NOT do that.
//!
//! ── Bit-identity contract ──
//!
//! Cross-binding bit-identity with Halcyon's NumPy PCG64 mock is
//! impossible by design (locked decision 1, inherited). The fixture is
//! the intra-GIGI sentinel: at fixed seeds + β + dt + n_steps, the U/E
//! buffers, every measurement chain entry, and the diagnostics are
//! byte-stable across this binary's runs. The provenance side-car
//! records the RNG, algorithm, seeds, and profile so any future drift
//! is traceable to the offending commit.
//!
//! ── Storage format ──
//!
//! `symplectic_flow_canonical.json`:
//! ```text
//! {
//!   "group": "SU(2)",
//!   "n_edges": 90,
//!   "n_vertices": 60,
//!   "n_faces": 32,
//!   "final_U_bits": [<u64; 360>],
//!   "final_U_decimal": [<f64; 360>],
//!   "final_E_bits": [<u64; 360>],
//!   "final_E_decimal": [<f64; 360>],
//!   "step_indices": [20, 40, …, 1000],   // length 51
//!   "measurement_history": {
//!     "h_total":            { "bits": [<u64; 51>], "decimal": [<f64; 51>] },
//!     "mean_plaquette":     { "bits": [<u64; 51>], "decimal": [<f64; 51>] },
//!     "gauss_residual_max": { "bits": [<u64; 51>], "decimal": [<f64; 51>] },
//!     "q_surrogate":        { "bits": [<u64; 51>], "decimal": [<f64; 51>] }
//!   },
//!   "diagnostics": {
//!     "seed": 20260617,
//!     "beta": 2.5,
//!     "dt": 0.02,
//!     "n_steps_completed": 1000,
//!     "max_energy_drift_rel": <f64>,
//!     "gauss_residual_max":   <f64>,
//!     "cg_iterations_per_step_p99": <f64>   // DIAGNOSTIC ONLY
//!   }
//! }
//! ```
//!
//! Following the II.2 / III.8a envelope convention: every `bits` field
//! is the byte-equality oracle (loaded via `f64::from_bits` in the IV.10
//! gold gate); every `decimal` field is informational only.
//!
//! Group-erasure note: this harvest runs SU(2) only at launch. The
//! `group` tag at the top of the envelope is the slot future
//! U(1)/SU(3)/Z(N) harvests will populate in parallel files
//! (`symplectic_flow_canonical_su3.json`, …). Format stays identical.
//!
//! ── Acceptance bounds (the harvest does NOT commit a fixture that
//! fails these) ──
//!
//! - `max_energy_drift_rel < 1e-3` — Halcyon energy-drift bound.
//! - `gauss_residual_max  < 1e-9`  — per-step projection holds.
//!
//! These mirror the IV.10 gate (a) acceptance bound. If the harvest
//! crosses either, the run aborts via `assert!` BEFORE writing the
//! fixture — better to leave the previous fixture intact than to commit
//! a drifted run.

#![cfg(feature = "halcyon")]

use std::fs;
use std::path::PathBuf;

use gigi::parser::{execute, parse, ExecResult};
use gigi::types::Value;

/// Path to the canonical SYMPLECTIC_FLOW fixture, anchored to the test
/// crate's manifest dir so `cargo test` from any CWD finds it.
fn canonical_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("halcyon")
        .join("part_iv")
        .join("symplectic_flow_canonical.json")
}

/// Path to the provenance side-car (records the harvest contract:
/// RNG, algorithm, seeds, β, dt, n_steps, profile, harvest commit).
/// IV.10 doesn't read this at runtime — it exists so anyone reading
/// the fixture knows the bit-identity contract drop vs the Halcyon
/// NumPy mock and the profile under which it was harvested.
fn canonical_provenance_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("halcyon")
        .join("part_iv")
        .join("symplectic_flow_canonical_provenance.json")
}

/// Serialize a `&[f64]` chain in both bit-pattern (oracle) and decimal
/// (shadow) form — the II.2 / III.8a envelope convention.
fn chain_envelope(chain: &[f64]) -> serde_json::Value {
    let bits: Vec<u64> = chain.iter().map(|x| x.to_bits()).collect();
    serde_json::json!({
        "bits": bits,
        "decimal": chain,
    })
}

/// TDD-HAL-IV.9: harvest the Part IV canonical SYMPLECTIC_FLOW fixture
/// by running the locked GQL block through the parser + executor path
/// ONCE under `--release` and freezing the final U/E buffers, every
/// measurement chain, the step-index ladder, and the diagnostics into
/// `tests/fixtures/halcyon/part_iv/symplectic_flow_canonical.json`.
/// Gate IV.10 reads this fixture to write its energy-drift acceptance
/// + byte-identity regression assertions.
#[test]
#[ignore]
fn tdd_hal_iv_9_harvest_canonical() {
    // Clean every Part-IV-relevant singleton so the harvest is
    // reproducible against any prior test state on this binary.
    gigi::gauge::registry::clear();
    gigi::gauge::registry::clear_e_registry();
    gigi::lattice::registry::clear();
    gigi::gauge::symplectic_flow::clear_diagnostics_cache();

    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = gigi::engine::Engine::open(dir.path()).expect("engine open");

    // ── 1. Declare lattice + GAUGE_FIELD U INIT IDENTITY through the
    //    parser + executor path (same code surface IV.10 will exercise).
    let lat_decl = "LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
    let stmt = parse(lat_decl).expect("parse LATTICE");
    execute(&mut engine, &stmt).expect("exec LATTICE");

    let g_decl = "GAUGE_FIELD U ON LATTICE buckyball \
                  GROUP SU(2) INIT IDENTITY;";
    let stmt = parse(g_decl).expect("parse GAUGE_FIELD");
    execute(&mut engine, &stmt).expect("exec GAUGE_FIELD");

    // The GAUGE_FIELD executor arm registers through `register`
    // (Arc<dyn>). GIBBS_SAMPLE + SYMPLECTIC_FLOW need a SU(2)-mut
    // handle (locked decision D4). Re-publish the same field through
    // `register_su2` so the heatbath sweep can lock the mutable
    // buffer. This mirrors the III.8a + III.8b harvest fix-up.
    {
        let lat = gigi::lattice::registry::get("buckyball")
            .expect("declared lattice");
        let su2 = gigi::gauge::SU2GaugeField::new(
            "U".into(),
            &lat,
            gigi::gauge::GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init");
        gigi::gauge::registry::register_su2(su2);
    }

    // ── 2. Thermalize: 200 Kennedy-Pendleton sweeps at β=2.5.
    //    GIBBS_SAMPLE in-place-mutates the SU(2)-mut handle and
    //    republishes both the mut + dyn read maps, so the post-
    //    thermal U is ready for E_FIELD declaration + SYMPLECTIC_FLOW
    //    consumption with no manual fix-up between verbs.
    let thermalize = "GIBBS_SAMPLE U BETA 2.5 N_SWEEPS 200 SEED 20260616;";
    let stmt = parse(thermalize).expect("parse GIBBS_SAMPLE (thermalize)");
    execute(&mut engine, &stmt).expect("exec GIBBS_SAMPLE (thermalize)");

    // ── 3. Declare E from Maxwell–Boltzmann at β=2.5, seed=20260617.
    //    The MB sampler consumes the seed exactly once at declaration
    //    time, so the SYMPLECTIC_FLOW seed (20260617 again, by design)
    //    is an echo-only slot in the response.
    let e_decl = "E_FIELD E ON GAUGE_FIELD U INIT MAXWELL_BOLTZMANN \
                  BETA 2.5 SEED 20260617;";
    let stmt = parse(e_decl).expect("parse E_FIELD");
    execute(&mut engine, &stmt).expect("exec E_FIELD");

    // ── 4. Run SYMPLECTIC_FLOW with PROJECT_GAUSS TRUE (per-step
    //    projection per locked decision IV-D), DT 0.02, N_STEPS 1000,
    //    MEASURE_EVERY 20 → 51 measurements per observable (steps 20,
    //    40, …, 1000).
    let flow = "SYMPLECTIC_FLOW U FROM (U=U, E=E) BETA 2.5 DT 0.02 \
                N_STEPS 1000 PROJECT_GAUSS TRUE MEASURE_EVERY 20 \
                MEASURE (H_TOTAL, MEAN(PLAQUETTE), GAUSS_RESIDUAL_MAX, \
                Q_SURROGATE) SEED 20260617;";
    let stmt = parse(flow).expect("parse SYMPLECTIC_FLOW");
    let rows = match execute(&mut engine, &stmt).expect("exec SYMPLECTIC_FLOW") {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows envelope, got {other:?}"),
    };
    assert_eq!(rows.len(), 1, "SYMPLECTIC_FLOW returns one row");

    // ── 5. Pull each measurement chain off the Rows envelope by the
    //    parser's `ObservableId::label()` column key.
    let h_total_label = gigi::gauge::ObservableId::HTotal.label();
    let plaquette_label = gigi::gauge::ObservableId::MeanPlaquette.label();
    let gauss_label = gigi::gauge::ObservableId::GaussResidualMax.label();
    let q_label = gigi::gauge::ObservableId::QSurrogate.label();

    let h_total: Vec<f64> = match rows[0].get(h_total_label) {
        Some(Value::Vector(v)) => v.clone(),
        other => panic!("missing/wrong {h_total_label} column: {other:?}"),
    };
    let mean_plaquette: Vec<f64> = match rows[0].get(plaquette_label) {
        Some(Value::Vector(v)) => v.clone(),
        other => panic!("missing/wrong {plaquette_label} column: {other:?}"),
    };
    let gauss_residual_max_chain: Vec<f64> = match rows[0].get(gauss_label) {
        Some(Value::Vector(v)) => v.clone(),
        other => panic!("missing/wrong {gauss_label} column: {other:?}"),
    };
    let q_surrogate: Vec<f64> = match rows[0].get(q_label) {
        Some(Value::Vector(v)) => v.clone(),
        other => panic!("missing/wrong {q_label} column: {other:?}"),
    };

    // Expect 1000 / 20 = 50 measurements... but the SYMPLECTIC_FLOW
    // measurement epilogue triggers when `(s+1) % measure_every == 0`,
    // so step indices are 20, 40, …, 1000 → length 50, NOT 51 (the
    // spec sketch says 51; we honor what the implementation produces
    // and assert against the real length so the fixture and IV.10
    // gate agree). Re-checked III.6 smoke + III.8a harvest: same
    // `s + 1` epilogue convention, so 1000 / 20 = 50 here too.
    assert_eq!(
        h_total.len(),
        50,
        "expected 50 measurements (N_STEPS / MEASURE_EVERY = 1000/20); got {}",
        h_total.len()
    );
    assert_eq!(mean_plaquette.len(), 50);
    assert_eq!(gauss_residual_max_chain.len(), 50);
    assert_eq!(q_surrogate.len(), 50);

    // Step indices: 20, 40, …, 1000 — the cadence implied by
    // `(s+1) % measure_every == 0` with measure_every=20.
    let step_indices: Vec<usize> =
        (1..=50_usize).map(|k| k * 20).collect();
    assert_eq!(*step_indices.first().unwrap(), 20);
    assert_eq!(*step_indices.last().unwrap(), 1000);

    // ── 6. Pull the diagnostics block off the Rows envelope.
    let n_steps_completed = match rows[0].get("n_steps_completed") {
        Some(Value::Integer(n)) => *n as usize,
        other => panic!("missing/wrong n_steps_completed: {other:?}"),
    };
    assert_eq!(n_steps_completed, 1000);

    let max_energy_drift_rel = match rows[0].get("max_energy_drift_rel") {
        Some(Value::Float(f)) => *f,
        other => panic!("missing/wrong max_energy_drift_rel: {other:?}"),
    };
    let gauss_residual_max_final = match rows[0].get("gauss_residual_max") {
        Some(Value::Float(f)) => *f,
        other => panic!("missing/wrong gauss_residual_max: {other:?}"),
    };
    let cg_iterations_per_step_p99 =
        match rows[0].get("cg_iterations_per_step_p99") {
            Some(Value::Float(f)) => *f,
            other => panic!("missing/wrong cg_iterations_per_step_p99: {other:?}"),
        };

    // ── 7. Acceptance bounds (IV.10 mirrors these). If the harvest
    //    drifted past either, abort BEFORE writing the fixture —
    //    better to leave the previous fixture intact than commit a
    //    bad run.
    assert!(
        max_energy_drift_rel < 1e-3,
        "max_energy_drift_rel = {max_energy_drift_rel:e} exceeds 1e-3 \
         Halcyon energy-drift acceptance bound — refusing to commit \
         a drifted fixture"
    );
    assert!(
        gauss_residual_max_final < 1e-9,
        "gauss_residual_max = {gauss_residual_max_final:e} exceeds 1e-9 \
         per-step-projection bound — refusing to commit a drifted fixture"
    );

    // ── 8. Pull the final U and E buffers out of the registries.
    //    SYMPLECTIC_FLOW republishes U through `republish_su2`, so the
    //    dyn read map carries the post-flow snapshot. The E sibling
    //    registry stores the same `Arc<Mutex<…>>` the flow mutated in
    //    place, so the lookup returns the post-flow state directly.
    let final_u_buffer: Vec<f64> = {
        let handle = gigi::gauge::registry::get("U").expect("post-flow U");
        handle.as_dense_buffer().data.clone()
    };
    assert_eq!(
        final_u_buffer.len(),
        90 * 4,
        "buckyball U buffer is (90, 4) row-major"
    );

    let final_e_buffer: Vec<f64> = {
        let handle = gigi::gauge::registry::get_su2_e_mut("E")
            .expect("post-flow E");
        let guard = handle.lock().expect("e field mutex poisoned");
        guard.buffer.data.clone()
    };
    assert_eq!(
        final_e_buffer.len(),
        90 * 4,
        "buckyball E buffer is (90, 4) row-major"
    );

    // Sanity check the q0=0 Lie invariant on every E row (locked
    // decision IV-C). If we ever ship a kernel that drifts q0, this
    // catches it BEFORE the bad fixture lands.
    for edge in 0..90 {
        let q0 = final_e_buffer[edge * 4];
        assert_eq!(
            q0, 0.0,
            "E[{edge}].q0 = {q0} violates the IV-C q0=0 Lie invariant"
        );
    }

    // ── 9. Bits + decimal envelopes for every chain + the two
    //    buffers. The bit-pattern arrays are the IV.10 byte-equality
    //    oracle; the decimal arrays are informational only.
    let final_u_bits: Vec<u64> =
        final_u_buffer.iter().map(|x| x.to_bits()).collect();
    let final_e_bits: Vec<u64> =
        final_e_buffer.iter().map(|x| x.to_bits()).collect();

    let envelope = serde_json::json!({
        "group": "SU(2)",
        "n_edges": 90,
        "n_vertices": 60,
        "n_faces": 32,
        "final_U_bits": final_u_bits,
        "final_U_decimal": final_u_buffer,
        "final_E_bits": final_e_bits,
        "final_E_decimal": final_e_buffer,
        "step_indices": step_indices,
        "measurement_history": {
            "h_total":            chain_envelope(&h_total),
            "mean_plaquette":     chain_envelope(&mean_plaquette),
            "gauss_residual_max": chain_envelope(&gauss_residual_max_chain),
            "q_surrogate":        chain_envelope(&q_surrogate),
        },
        "diagnostics": {
            "seed": 20260617_u64,
            "beta": 2.5_f64,
            "dt":   0.02_f64,
            "n_steps_completed": n_steps_completed,
            "max_energy_drift_rel":         max_energy_drift_rel,
            "gauss_residual_max":           gauss_residual_max_final,
            "cg_iterations_per_step_p99":   cg_iterations_per_step_p99,
        },
    });
    let envelope_json = serde_json::to_string_pretty(&envelope)
        .expect("serialize symplectic_flow_canonical");
    fs::write(canonical_path(), envelope_json)
        .expect("write symplectic_flow_canonical");

    // ── 10. Provenance side-car. `harvest_commit` is filled in by a
    //     follow-up commit once the harvest commit exists (matches
    //     II.2 / III.8a / III.8c pin pattern).
    let provenance = serde_json::json!({
        "source": "gigi::gauge::symplectic_flow (driven through gigi::parser::execute Statement::SymplecticFlow)",
        "beta":   2.5_f64,
        "dt":     0.02_f64,
        "n_steps": 1000_usize,
        "measure_every": 20_usize,
        "measure": [
            "H_TOTAL",
            "MEAN(PLAQUETTE)",
            "GAUSS_RESIDUAL_MAX",
            "Q_SURROGATE",
        ],
        "project_gauss": {
            "enabled":     true,
            "cadence":     "per_leapfrog_step",
            "tikhonov":    1e-14,
            "cg_tol":      1e-10,
            "cg_max_iter": 200,
            "preconditioner": "none",
        },
        "init_U": "IDENTITY",
        "init_E": "MAXWELL_BOLTZMANN",
        "lattice": "TRUNCATED_ICOSAHEDRON (buckyball: V=60, E=90, F=32)",
        "topology": "S2",
        "group":    "SU(2)",
        "seed":                 20260617_u64,
        "thermalization_sweeps": 200_usize,
        "thermalization_seed":  20260616_u64,
        "rng":       "gigi::gauge::marsaglia_haar::SmallRng (xorshift64*)",
        "algorithm": "KDK leapfrog + Tikhonov CG Gauss projection per step",
        "profile":   "release",
        "harvest_commit": "<fill in with the SHA of the TDD-HAL-IV.9 commit once it lands>",
        "purpose": "Part IV gold-gate canonical reference. Cross-binding bit-identity with Halcyon NumPy PCG64 is impossible by design (locked decision 1, inherited); intra-GIGI bit-identity at fixed seeds is the bit-id contract.",
        "note": "Locked decision IV-A pins the production tikhonov default to 1e-14; IV-D pins per-step projection cadence; IV-E pins the CG preconditioner to NONE. The harvest must run under --release; debug-profile re-harvests would land a different fixture and break IV.10.",
    });
    let prov_json = serde_json::to_string_pretty(&provenance)
        .expect("serialize symplectic_flow_canonical_provenance");
    fs::write(canonical_provenance_path(), prov_json)
        .expect("write symplectic_flow_canonical_provenance");

    println!(
        "harvested 50-measurement chains + final (U, E) to {}",
        canonical_path().display()
    );
    println!(
        "  max_energy_drift_rel       = {max_energy_drift_rel:.12e}"
    );
    println!(
        "  gauss_residual_max (final) = {gauss_residual_max_final:.12e}"
    );
    println!(
        "  cg_iterations_per_step_p99 = {cg_iterations_per_step_p99:.6}"
    );
    println!(
        "  provenance at {}",
        canonical_provenance_path().display()
    );
}
