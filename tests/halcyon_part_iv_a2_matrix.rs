//! TDD-HAL-IV.10 (A2 matrix half) — bit-identity matrix harness for
//! Part IV `SYMPLECTIC_FLOW`. One test per A2 row (see
//! HALCYON_PART_IV_GATES.md when it lands; the matrix is also enumerated
//! in the sprint spec). Mirrors the gold-gate file's profile-pin pattern
//! (byte-equality rows run only under `--release`) and the III.8c
//! precedent.
//!
//! ── A2 matrix (locked decisions IV-F + IV-G + A2 doc) ──
//!
//! - **Row 1**: same seed / β / dt / n_steps, SAME process → STRICT
//!   byte-identical on (final U, final E, h_total, gauss_residual_max).
//!   HARD GATE.
//! - **Row 2**: same seed / β / dt / n_steps, DIFFERENT process (child
//!   `cargo test --features halcyon` invocation), SAME OS, SAME BLAS
//!   → STRICT byte-identical. HARD GATE (best-effort — `#[ignore]`d
//!   when the child harness is not built or the cross-process spawn
//!   isn't desired; documented).
//! - **Row 3**: same seed / β / dt / n_steps, CROSS-OS → 2 ULP
//!   tolerance in trig reductions. SKIP in single-OS CI per locked
//!   decision IV-G. This file documents the contract via an
//!   `#[ignore]` arm; the 2-ULP envelope itself is documented in
//!   HALCYON_PART_IV_GATES.md (not enforced here).
//! - **Row 4**: same seed, DIFFERENT β → NOT bit-identical (assert
//!   `!=`); energy drift + Gauss residual tolerances still hold. NO
//!   byte gate.
//! - **Row 5**: same seed, DIFFERENT dt → NOT bit-identical;
//!   tolerances hold.
//! - **Row 6**: same seed, DIFFERENT n_steps → prefix-equality. First
//!   `min(n1, n2)` steps byte-identical between the two runs. HARD
//!   GATE on the prefix.
//!
//! `cg_iterations_per_step_p99` is a DIAGNOSTIC ONLY signal — never
//! compared in any A2 row.
//!
//! ── Group-erasure note ──
//!
//! Every A2 row runs SU(2)-only. Future U(1)/SU(3) sibling flows ship
//! parallel rows in `halcyon_part_iv_a2_matrix_su3.rs` etc. with the
//! same matrix shape.
//!
//! ── Optionality contract ──
//!
//! Gated on `halcyon` so the no-default-features build stays
//! byte-identical at 852/0.

#![cfg(feature = "halcyon")]

use gigi::parser::{execute, parse, ExecResult};
use gigi::types::Value;

/// Reset every Part-IV-relevant singleton to a clean slate.
fn clear_registries() {
    gigi::gauge::registry::clear();
    gigi::gauge::registry::clear_e_registry();
    gigi::lattice::registry::clear();
    gigi::gauge::clear_symplectic_flow_diagnostics_cache();
}

/// State pulled off one `(LATTICE, GAUGE_FIELD U, E_FIELD E,
/// SYMPLECTIC_FLOW)` run — enough to drive byte-equality + tolerance
/// checks across A2 rows.
#[derive(Debug, Clone)]
struct RunState {
    final_u: Vec<f64>,
    final_e: Vec<f64>,
    h_history: Vec<f64>,
    gauss_history: Vec<f64>,
    max_energy_drift_rel: f64,
    gauss_residual_max: f64,
}

/// Drive one SYMPLECTIC_FLOW run with the given (β, dt, n_steps,
/// thermalization_seed, mb_seed) parameters and capture the final U/E
/// buffers and the H_TOTAL + GAUSS_RESIDUAL_MAX chains.
///
/// Uses unique lattice + field names per call (via the `tag` argument)
/// so the same in-process registries can hold multiple runs side by
/// side. Each call clears the registries first; if you want multiple
/// runs side by side, hold their results in separate `RunState`s and
/// re-issue the second call after the first returns.
///
/// `measure_every = 20` so a 100-step run yields 5 measurements per
/// chain; cheaper than the 1000-step canonical replay but still
/// captures the trajectory shape for prefix-equality.
fn run_flow(
    engine: &mut gigi::engine::Engine,
    tag: &str,
    beta: f64,
    dt: f64,
    n_steps: usize,
    mb_seed: u64,
) -> RunState {
    clear_registries();
    let lat_name = format!("a2_bb_{tag}");
    let u_name = format!("U_{tag}");
    let e_name = format!("E_{tag}");

    let lat_decl =
        format!("LATTICE {lat_name} FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';");
    let stmt = parse(&lat_decl).expect("parse LATTICE");
    execute(engine, &stmt).expect("exec LATTICE");

    let g_decl = format!(
        "GAUGE_FIELD {u_name} ON LATTICE {lat_name} GROUP SU(2) INIT IDENTITY;"
    );
    let stmt = parse(&g_decl).expect("parse GAUGE_FIELD");
    execute(engine, &stmt).expect("exec GAUGE_FIELD");

    // Republish through register_su2 (D4 fix-up).
    {
        let lat = gigi::lattice::registry::get(&lat_name).expect("declared lattice");
        let su2 = gigi::gauge::SU2GaugeField::new(
            u_name.clone(),
            &lat,
            gigi::gauge::GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init");
        gigi::gauge::registry::register_su2(su2);
    }

    let e_decl = format!(
        "E_FIELD {e_name} ON GAUGE_FIELD {u_name} INIT MAXWELL_BOLTZMANN \
         BETA {beta:.4} SEED {mb_seed};"
    );
    let stmt = parse(&e_decl).expect("parse E_FIELD");
    execute(engine, &stmt).expect("exec E_FIELD");

    let flow = format!(
        "SYMPLECTIC_FLOW {u_name} FROM (U={u_name}, E={e_name}) \
         BETA {beta:.4} DT {dt} N_STEPS {n_steps} \
         PROJECT_GAUSS TRUE MEASURE_EVERY 20 \
         MEASURE (H_TOTAL, GAUSS_RESIDUAL_MAX) SEED {mb_seed};"
    );
    let stmt = parse(&flow).expect("parse SYMPLECTIC_FLOW");
    let rows = match execute(engine, &stmt).expect("exec SYMPLECTIC_FLOW") {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows, got {other:?}"),
    };
    let row = &rows[0];

    let h_label = gigi::gauge::ObservableId::HTotal.label();
    let g_label = gigi::gauge::ObservableId::GaussResidualMax.label();
    let h_history: Vec<f64> = match row.get(h_label) {
        Some(Value::Vector(v)) => v.clone(),
        other => panic!("missing/wrong {h_label}: {other:?}"),
    };
    let gauss_history: Vec<f64> = match row.get(g_label) {
        Some(Value::Vector(v)) => v.clone(),
        other => panic!("missing/wrong {g_label}: {other:?}"),
    };
    let max_energy_drift_rel = match row.get("max_energy_drift_rel") {
        Some(Value::Float(f)) => *f,
        other => panic!("missing/wrong max_energy_drift_rel: {other:?}"),
    };
    let gauss_residual_max = match row.get("gauss_residual_max") {
        Some(Value::Float(f)) => *f,
        other => panic!("missing/wrong gauss_residual_max: {other:?}"),
    };

    let final_u: Vec<f64> = {
        let handle = gigi::gauge::registry::get(&u_name).expect("post-flow U");
        handle.as_dense_buffer().data.clone()
    };
    let final_e: Vec<f64> = {
        let handle = gigi::gauge::registry::get_su2_e_mut(&e_name).expect("post-flow E");
        let guard = handle.lock().expect("e field mutex poisoned");
        guard.buffer.data.clone()
    };

    RunState {
        final_u,
        final_e,
        h_history,
        gauss_history,
        max_energy_drift_rel,
        gauss_residual_max,
    }
}

/// Byte-equality helper: every `(a[i], b[i])` pair has the same
/// `f64::to_bits` pattern. Panics with a descriptive message on the
/// first mismatch.
fn assert_buffers_byte_identical(a: &[f64], b: &[f64], label: &str) {
    assert_eq!(
        a.len(),
        b.len(),
        "A2 {label}: buffer length drift ({} vs {})",
        a.len(),
        b.len()
    );
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(
            x.to_bits(),
            y.to_bits(),
            "A2 {label}: byte-identical drift at [{i}]: \
             a={:#x} b={:#x}",
            x.to_bits(),
            y.to_bits()
        );
    }
}

/// TDD-HAL-IV.10 (A2 Row 1): same seed/β/dt/n_steps, SAME process →
/// strict byte-identical on (final U, final E, h_total chain,
/// gauss_residual_max chain). The hardest A2 row — failure here means
/// the GIGI side has lost determinism within a single process and the
/// whole IV.10 bit-identity contract collapses.
#[test]
#[cfg_attr(debug_assertions, ignore)]
fn tdd_hal_iv_10_a2_row_1_same_process_strict() {
    let _g = gigi::gauge::registry::test_serial_lock();

    let dir_a = tempfile::tempdir().expect("tempdir A");
    let mut engine_a = gigi::engine::Engine::open(dir_a.path()).expect("engine A");
    let run_a = run_flow(&mut engine_a, "r1a", 2.5, 0.02, 100, 20260617);

    let dir_b = tempfile::tempdir().expect("tempdir B");
    let mut engine_b = gigi::engine::Engine::open(dir_b.path()).expect("engine B");
    let run_b = run_flow(&mut engine_b, "r1b", 2.5, 0.02, 100, 20260617);

    assert_buffers_byte_identical(&run_a.final_u, &run_b.final_u, "row1 final_U");
    assert_buffers_byte_identical(&run_a.final_e, &run_b.final_e, "row1 final_E");
    assert_buffers_byte_identical(&run_a.h_history, &run_b.h_history, "row1 h_total");
    assert_buffers_byte_identical(
        &run_a.gauss_history,
        &run_b.gauss_history,
        "row1 gauss_residual_max chain",
    );
}

/// Replay the exact canonical GQL block IV.9 harvested under
/// (thermalization seed = 20260616, MB seed = 20260617, β = 2.5,
/// dt = 0.02, n_steps = 1000), capturing the final (U, E) buffers. Used
/// by A2 row 2 to compare a live in-process run against the on-disk
/// fixture frozen by a previous process invocation.
fn replay_canonical_block_to_final_buffers(
    engine: &mut gigi::engine::Engine,
) -> (Vec<f64>, Vec<f64>) {
    clear_registries();

    let lat_decl = "LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
    let stmt = parse(lat_decl).expect("parse LATTICE");
    execute(engine, &stmt).expect("exec LATTICE");

    let g_decl =
        "GAUGE_FIELD U ON LATTICE buckyball GROUP SU(2) INIT IDENTITY;";
    let stmt = parse(g_decl).expect("parse GAUGE_FIELD");
    execute(engine, &stmt).expect("exec GAUGE_FIELD");

    {
        let lat = gigi::lattice::registry::get("buckyball").expect("declared lattice");
        let su2 = gigi::gauge::SU2GaugeField::new(
            "U".into(),
            &lat,
            gigi::gauge::GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init");
        gigi::gauge::registry::register_su2(su2);
    }

    let thermalize = "GIBBS_SAMPLE U BETA 2.5 N_SWEEPS 200 SEED 20260616;";
    let stmt = parse(thermalize).expect("parse GIBBS_SAMPLE thermalize");
    execute(engine, &stmt).expect("exec GIBBS_SAMPLE thermalize");

    let e_decl = "E_FIELD E ON GAUGE_FIELD U INIT MAXWELL_BOLTZMANN \
                  BETA 2.5 SEED 20260617;";
    let stmt = parse(e_decl).expect("parse E_FIELD");
    execute(engine, &stmt).expect("exec E_FIELD");

    let flow = "SYMPLECTIC_FLOW U FROM (U=U, E=E) BETA 2.5 DT 0.02 \
                N_STEPS 1000 PROJECT_GAUSS TRUE MEASURE_EVERY 20 \
                MEASURE (H_TOTAL, MEAN(PLAQUETTE), GAUSS_RESIDUAL_MAX, \
                Q_SURROGATE) SEED 20260617;";
    let stmt = parse(flow).expect("parse SYMPLECTIC_FLOW");
    let _ = execute(engine, &stmt).expect("exec SYMPLECTIC_FLOW");

    let final_u: Vec<f64> = {
        let handle = gigi::gauge::registry::get("U").expect("post-flow U");
        handle.as_dense_buffer().data.clone()
    };
    let final_e: Vec<f64> = {
        let handle = gigi::gauge::registry::get_su2_e_mut("E").expect("post-flow E");
        let guard = handle.lock().expect("e field mutex poisoned");
        guard.buffer.data.clone()
    };
    (final_u, final_e)
}

/// TDD-HAL-IV.10 (A2 Row 2): same seed/β/dt/n_steps, DIFFERENT process,
/// same OS, same BLAS → byte-identical. Implementation note: the
/// canonical IV.9 fixture WAS harvested by a separate process
/// invocation under `--release`, so the A2 row 1 same-process strict
/// gate + the IV.10.a gold gate (live-run ≡ fixture) together close the
/// cross-process loop without a runtime child spawn here. We assert
/// that contract by re-loading the fixture inside this test and
/// comparing the live run against the fixture's `final_U_bits` /
/// `final_E_bits` slots — effectively the same byte-equality the gold
/// gate asserts, but scoped to "this process produced a result byte-
/// identical to a state frozen by a DIFFERENT process invocation".
///
/// `#[cfg_attr(windows, ignore)]` gate (per the spec sketch) is OFF —
/// the test reads a fixture rather than spawning a child, so it works
/// on every OS the rest of the suite covers. The runtime child-spawn
/// shape (Command::new the IV.9 harvest binary repurposed) is the
/// stronger form for future cross-binary work; we leave that to a
/// future P1 hardening pass once Halcyon authors the parallel python
/// side per locked decision IV-K.
#[test]
#[cfg_attr(debug_assertions, ignore)]
fn tdd_hal_iv_10_a2_row_2_cross_process_same_os_strict() {
    let _g = gigi::gauge::registry::test_serial_lock();

    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = gigi::engine::Engine::open(dir.path()).expect("engine open");
    // Replay the canonical block — same parameters IV.9 harvested
    // under (thermalization seed = 20260616, MB seed = 20260617).
    let (final_u, final_e) = replay_canonical_block_to_final_buffers(&mut engine);

    // Re-load the fixture frozen by the IV.9 harvest binary — a
    // different process invocation. Byte-identity here is the cross-
    // process same-OS receipt.
    let body = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("halcyon")
            .join("part_iv")
            .join("symplectic_flow_canonical.json"),
    )
    .expect("read IV.9 fixture (run harvest_part_iv_canonical to regenerate)");
    let fixture: serde_json::Value =
        serde_json::from_str(&body).expect("parse symplectic_flow_canonical.json");
    let final_u_fix: Vec<u64> = fixture["final_U_bits"]
        .as_array()
        .expect("final_U_bits array")
        .iter()
        .map(|b| b.as_u64().expect("final_U_bits entry not u64"))
        .collect();
    let final_e_fix: Vec<u64> = fixture["final_E_bits"]
        .as_array()
        .expect("final_E_bits array")
        .iter()
        .map(|b| b.as_u64().expect("final_E_bits entry not u64"))
        .collect();
    assert_eq!(final_u.len(), final_u_fix.len(), "A2 row2 final_U length drift");
    for (i, v) in final_u.iter().enumerate() {
        assert_eq!(
            v.to_bits(),
            final_u_fix[i],
            "A2 row2 final_U[{i}] bit drift vs IV.9 (cross-process): \
             this={:#x} harvest={:#x}",
            v.to_bits(),
            final_u_fix[i]
        );
    }
    assert_eq!(final_e.len(), final_e_fix.len(), "A2 row2 final_E length drift");
    for (i, v) in final_e.iter().enumerate() {
        assert_eq!(
            v.to_bits(),
            final_e_fix[i],
            "A2 row2 final_E[{i}] bit drift vs IV.9 (cross-process): \
             this={:#x} harvest={:#x}",
            v.to_bits(),
            final_e_fix[i]
        );
    }
}

/// TDD-HAL-IV.10 (A2 Row 3): same seed/β/dt/n_steps, CROSS-OS → 2 ULP
/// tolerance in trig reductions. SKIP in single-OS CI per locked
/// decision IV-G — the 2-ULP envelope is documented (not enforced)
/// in HALCYON_PART_IV_GATES.md. This arm exists as a permanent
/// `#[ignore]` marker so anyone scanning the A2 matrix sees the slot
/// is acknowledged and intentionally unverified at runtime.
#[test]
#[ignore = "Cross-OS row — documented 2 ULP envelope, not enforced in single-OS CI \
            (locked decision IV-G; see HALCYON_PART_IV_GATES.md)."]
fn tdd_hal_iv_10_a2_row_3_cross_os_2ulp() {
    // No body — the `#[ignore]` annotation is the contract. If a
    // future Halcyon caller exercises cross-OS bit-identity, they
    // own the comparison harness; this test stays as a permanent
    // marker.
}

/// TDD-HAL-IV.10 (A2 Row 4): same seed, DIFFERENT β (2.5 vs 5.0) →
/// NOT bit-identical; tolerances (energy drift, Gauss residual) still
/// hold. The receipt is "β changes the Hamiltonian (g²=4/β) so the
/// trajectory necessarily differs, AND the physics gates still pass on
/// both runs".
#[test]
fn tdd_hal_iv_10_a2_row_4_different_beta_no_byte_id() {
    let _g = gigi::gauge::registry::test_serial_lock();

    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = gigi::engine::Engine::open(dir.path()).expect("engine open");
    let run_25 = run_flow(&mut engine, "r4a", 2.5, 0.02, 100, 20260617);
    let run_50 = run_flow(&mut engine, "r4b", 5.0, 0.02, 100, 20260617);

    // Buffers must DIFFER somewhere — assert `!=` element-wise.
    assert_ne!(
        run_25.final_u, run_50.final_u,
        "A2 row4: β=2.5 vs β=5.0 must produce DIFFERENT final_U (same seed)"
    );
    assert_ne!(
        run_25.final_e, run_50.final_e,
        "A2 row4: β=2.5 vs β=5.0 must produce DIFFERENT final_E (same seed)"
    );

    // Tolerances still hold on both runs.
    assert!(
        run_25.max_energy_drift_rel < 1e-3,
        "A2 row4 β=2.5 max_energy_drift_rel = {} exceeds 1e-3",
        run_25.max_energy_drift_rel
    );
    assert!(
        run_50.max_energy_drift_rel < 1e-3,
        "A2 row4 β=5.0 max_energy_drift_rel = {} exceeds 1e-3",
        run_50.max_energy_drift_rel
    );
    assert!(
        run_25.gauss_residual_max < 1e-9,
        "A2 row4 β=2.5 gauss_residual_max = {} exceeds 1e-9",
        run_25.gauss_residual_max
    );
    assert!(
        run_50.gauss_residual_max < 1e-9,
        "A2 row4 β=5.0 gauss_residual_max = {} exceeds 1e-9",
        run_50.gauss_residual_max
    );
}

/// TDD-HAL-IV.10 (A2 Row 5): same seed, DIFFERENT dt (0.02 vs 0.01) →
/// NOT bit-identical; tolerances hold. Same shape as row 4 with dt
/// substituted for β.
#[test]
fn tdd_hal_iv_10_a2_row_5_different_dt_no_byte_id() {
    let _g = gigi::gauge::registry::test_serial_lock();

    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = gigi::engine::Engine::open(dir.path()).expect("engine open");
    // Use the same INTEGRATED TIME across both runs so each integrates
    // over the same physical span — dt=0.02, n_steps=100  ↔  dt=0.01,
    // n_steps=200. Different step sizes ⇒ different round-off chain
    // ⇒ different bit pattern, but the same Hamiltonian energy-drift
    // bound holds.
    let run_dt02 = run_flow(&mut engine, "r5a", 2.5, 0.02, 100, 20260617);
    let run_dt01 = run_flow(&mut engine, "r5b", 2.5, 0.01, 200, 20260617);

    assert_ne!(
        run_dt02.final_u, run_dt01.final_u,
        "A2 row5: dt=0.02 vs dt=0.01 must produce DIFFERENT final_U"
    );
    assert_ne!(
        run_dt02.final_e, run_dt01.final_e,
        "A2 row5: dt=0.02 vs dt=0.01 must produce DIFFERENT final_E"
    );
    assert!(
        run_dt02.max_energy_drift_rel < 1e-3,
        "A2 row5 dt=0.02 max_energy_drift_rel = {}",
        run_dt02.max_energy_drift_rel
    );
    assert!(
        run_dt01.max_energy_drift_rel < 1e-3,
        "A2 row5 dt=0.01 max_energy_drift_rel = {}",
        run_dt01.max_energy_drift_rel
    );
    assert!(
        run_dt02.gauss_residual_max < 1e-9,
        "A2 row5 dt=0.02 gauss_residual_max = {}",
        run_dt02.gauss_residual_max
    );
    assert!(
        run_dt01.gauss_residual_max < 1e-9,
        "A2 row5 dt=0.01 gauss_residual_max = {}",
        run_dt01.gauss_residual_max
    );
}

/// TDD-HAL-IV.10 (A2 Row 6): same seed, DIFFERENT n_steps → prefix
/// equality. Run with `n_steps=100` and `n_steps=200`; the first 100
/// steps of the 200-step trajectory must be byte-identical to the
/// 100-step trajectory.
///
/// We exploit the `MEASURE_EVERY 20` cadence: the 100-step run yields
/// 5 measurements (steps 20, 40, 60, 80, 100); the 200-step run yields
/// 10 (steps 20, …, 200). The first 5 entries must match byte-for-byte
/// (h_total + gauss_residual_max chains). This is the prefix equality
/// receipt without re-running the integrator step-by-step.
///
/// Note: the FINAL (U, E) buffers necessarily DIFFER between the two
/// runs — the 200-step run has integrated past step 100. The prefix-
/// equality contract lives on the MEASUREMENT CHAINS, not on the final
/// state.
#[test]
#[cfg_attr(debug_assertions, ignore)]
fn tdd_hal_iv_10_a2_row_6_different_n_steps_prefix_equality() {
    let _g = gigi::gauge::registry::test_serial_lock();

    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = gigi::engine::Engine::open(dir.path()).expect("engine open");
    let run_100 = run_flow(&mut engine, "r6a", 2.5, 0.02, 100, 20260617);
    let run_200 = run_flow(&mut engine, "r6b", 2.5, 0.02, 200, 20260617);

    let prefix_len = run_100.h_history.len();
    assert_eq!(prefix_len, 5, "100/20 = 5 measurements on the short run");
    assert_eq!(
        run_200.h_history.len(),
        10,
        "200/20 = 10 measurements on the long run"
    );
    assert!(
        run_200.h_history.len() >= prefix_len,
        "long run must be at least as long as short prefix"
    );

    // Prefix byte-equality on h_total + gauss_residual_max.
    for i in 0..prefix_len {
        assert_eq!(
            run_100.h_history[i].to_bits(),
            run_200.h_history[i].to_bits(),
            "A2 row6 h_total[{i}] prefix drift: short={:#x} long={:#x}",
            run_100.h_history[i].to_bits(),
            run_200.h_history[i].to_bits()
        );
        assert_eq!(
            run_100.gauss_history[i].to_bits(),
            run_200.gauss_history[i].to_bits(),
            "A2 row6 gauss_residual_max[{i}] prefix drift: \
             short={:#x} long={:#x}",
            run_100.gauss_history[i].to_bits(),
            run_200.gauss_history[i].to_bits()
        );
    }
}
