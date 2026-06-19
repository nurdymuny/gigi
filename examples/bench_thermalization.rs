//! Local profile of `GIBBS_SAMPLE U BETA 2.5 N_SWEEPS 200 SEED 20260616`
//! on the truncated-icosahedron buckyball (V=60, E=90, F=32, χ=2).
//!
//! Bee's audit harness. Times the production sweep, then runs three
//! micro-benches that isolate the three top phases the sweep visits
//! per edge:
//!
//!   1. staple_sum_at_edge — read N=18000 staples off a frozen field.
//!   2. sample_su2_link    — N=18000 KP draws with a fixed v_eff.
//!   3. mutex acquire/release on the SU(2)-mut Arc — N=200 times
//!      (lock taken once per sweep, not once per edge).
//!
//! Print wall-clock for each + the remainder ("other": qmul / buffer
//! write / measurement / refresh_dyn / RNG cold cache).
//!
//! Run with:
//!   cargo run --release --features halcyon --example bench_thermalization

use std::sync::Arc;
use std::time::Instant;

use gigi::gauge::gibbs_sample::{gibbs_sample, ObservableId};
use gigi::gauge::group_element::GroupElement;
use gigi::gauge::kennedy_pendleton::sample_su2_link;
use gigi::gauge::marsaglia_haar::SmallRng;
use gigi::gauge::registry as gauge_registry;
use gigi::gauge::staple::{build_edge_face_incidence, build_face_edges_cache, staple_sum_at_edge};
use gigi::gauge::su2_gauge_field::{GaugeFieldInit, SU2GaugeField};
use gigi::lattice::registry as lattice_registry;
use gigi::lattice::topology::truncated_icosahedron::buckyball;

const BETA: f64 = 2.5;
const N_SWEEPS: usize = 200;
const SEED: u64 = 20260616;

fn setup_identity(field_name: &str) {
    let bb = buckyball();
    lattice_registry::register(bb.clone());
    let field = SU2GaugeField::new(
        field_name.into(),
        &bb,
        GaugeFieldInit::Identity,
        None,
    )
    .expect("identity init");
    gauge_registry::register_su2(field);
}

fn main() {
    let bb = buckyball();
    let n_edges = bb.n_edges();
    let n_faces = bb.n_faces();
    println!("─── lattice ────────────────────────────────");
    println!("buckyball V={} E={} F={}", bb.n_vertices, n_edges, n_faces);

    // ── (1) End-to-end GIBBS_SAMPLE wall ───────────────────────────
    setup_identity("U_bench_total");
    // Warm cache once with a no-measurement single sweep so JIT/cold-page
    // costs don't pollute the headline number. Then re-init to identity
    // before the measured run so the starting state is the canonical one.
    {
        let _ = gibbs_sample("U_bench_total", BETA, 1, 0, vec![], Some(SEED))
            .expect("warm sweep");
    }
    setup_identity("U_bench_total");

    let t0 = Instant::now();
    let resp = gibbs_sample(
        "U_bench_total",
        BETA,
        N_SWEEPS,
        1, // measure every sweep — matches the production receipt
        vec![ObservableId::MeanPlaquette],
        Some(SEED),
    )
    .expect("gibbs_sample");
    let total = t0.elapsed();
    let chain = resp
        .measurement_history
        .get(&ObservableId::MeanPlaquette)
        .expect("MeanPlaquette chain");
    let final_mean = *chain.last().expect("chain non-empty");

    println!("─── end-to-end ────────────────────────────");
    println!(
        "GIBBS_SAMPLE β={} n_sweeps={} seed={} → {:.4} s total",
        BETA,
        N_SWEEPS,
        SEED,
        total.as_secs_f64()
    );
    println!("  per-sweep: {:.3} ms", total.as_secs_f64() * 1000.0 / N_SWEEPS as f64);
    println!(
        "  per-edge : {:.2} µs",
        total.as_secs_f64() * 1_000_000.0 / (N_SWEEPS * n_edges) as f64
    );
    println!("  final ⟨P⟩ = {:.6} (chain len = {})", final_mean, chain.len());
    println!("  chain[0]  = {:.6}", chain[0]);

    // ── (2) Sub-bench: staple_sum_at_edge × (N_SWEEPS · n_edges) ───
    //
    // We hold the field at the *post-sweep* state so the staple cost is
    // measured on a realistic thermalized buffer (the production sweep
    // mutates state every edge, so the buffer drifts off identity — this
    // is the steady-state-cost number).
    let field_arc = gauge_registry::get_su2_mut("U_bench_total")
        .expect("registered");
    let lat = lattice_registry::get("buckyball").expect("bb lattice");
    let inc = build_edge_face_incidence(&lat);
    let fec = build_face_edges_cache(&lat);

    let total_edge_visits = N_SWEEPS * n_edges;
    let mut staple_sink_q0 = 0.0_f64;
    let t1 = Instant::now();
    {
        let field = field_arc.lock().expect("field lock");
        for _ in 0..N_SWEEPS {
            for e in 0..n_edges {
                let v = staple_sum_at_edge(&*field, &lat, &inc, &fec, e);
                if let GroupElement::SU2 { q0, .. } = v {
                    // Sum into a sink so the optimizer cannot DCE the call.
                    staple_sink_q0 += q0;
                }
            }
        }
    }
    let staple_total = t1.elapsed();
    println!("─── (a) staple_sum_at_edge × {} ─────────────", total_edge_visits);
    println!("  wall: {:.4} s", staple_total.as_secs_f64());
    println!(
        "  per-call: {:.2} µs   sink={:.6}",
        staple_total.as_secs_f64() * 1_000_000.0 / total_edge_visits as f64,
        staple_sink_q0,
    );

    // ── (3) Sub-bench: sample_su2_link × (N_SWEEPS · n_edges) ─────
    //
    // Fixed v_eff so the KP cost is decoupled from the staple walker.
    // Use v_eff = (2, 0, 0, 0) — the closed-surface IDENTITY staple
    // (k = 2 incident faces, identity content). β=2.5 → ξ = 5, well
    // inside the KP single-iteration-accept regime.
    let v_eff_fixed = [2.0_f64, 0.0, 0.0, 0.0];
    let mut rng_kp = SmallRng::seed_from_u64(SEED);
    let mut kp_sink = 0.0_f64;
    let t2 = Instant::now();
    for _ in 0..total_edge_visits {
        let u = sample_su2_link(v_eff_fixed, BETA, &mut rng_kp);
        kp_sink += u[0];
    }
    let kp_total = t2.elapsed();
    println!("─── (b) sample_su2_link × {} ────────────────", total_edge_visits);
    println!("  wall: {:.4} s", kp_total.as_secs_f64());
    println!(
        "  per-call: {:.2} µs   sink={:.6}",
        kp_total.as_secs_f64() * 1_000_000.0 / total_edge_visits as f64,
        kp_sink,
    );

    // ── (4) Sub-bench: lock/unlock the SU(2)-mut Arc N_SWEEPS times ──
    //
    // Production takes the MutexGuard ONCE for the whole sweep loop and
    // releases at the end. We model that by acquiring + releasing N_SWEEPS
    // times — what a "lock per sweep" pattern would cost. (A "lock per
    // edge" pattern would cost N_SWEEPS · n_edges acquisitions; we time
    // that too for comparison.)
    let mut mutex_sink_per_sweep = 0.0_f64;
    let t3 = Instant::now();
    for _ in 0..N_SWEEPS {
        let g = field_arc.lock().expect("lock");
        mutex_sink_per_sweep += g.buffer.data[0];
    }
    let mutex_per_sweep = t3.elapsed();

    let mut mutex_sink_per_edge = 0.0_f64;
    let t4 = Instant::now();
    for _ in 0..total_edge_visits {
        let g = field_arc.lock().expect("lock");
        mutex_sink_per_edge += g.buffer.data[0];
    }
    let mutex_per_edge = t4.elapsed();

    println!("─── (c) mutex acquire/release ───────────────");
    println!(
        "  N={} (per-sweep pattern): {:.4} ms  ({:.2} ns / lock)",
        N_SWEEPS,
        mutex_per_sweep.as_secs_f64() * 1000.0,
        mutex_per_sweep.as_secs_f64() * 1e9 / N_SWEEPS as f64,
    );
    println!(
        "  N={} (per-edge pattern):  {:.4} ms  ({:.2} ns / lock)   sink={:.6}/{:.6}",
        total_edge_visits,
        mutex_per_edge.as_secs_f64() * 1000.0,
        mutex_per_edge.as_secs_f64() * 1e9 / total_edge_visits as f64,
        mutex_sink_per_sweep,
        mutex_sink_per_edge,
    );

    // ── (5) Sub-bench: just the measurement (MEAN(PLAQUETTE)) × N_SWEEPS ──
    //
    // This is the per-sweep epilogue cost (one plaquette walk over F=32 faces).
    use gigi::gauge::plaquette::plaquette_mean;
    let mut meas_sink = 0.0_f64;
    let t5 = Instant::now();
    {
        let field = field_arc.lock().expect("lock for measurement");
        for _ in 0..N_SWEEPS {
            let m = plaquette_mean(&*field, &lat).expect("plaquette");
            meas_sink += m;
        }
    }
    let measurement_total = t5.elapsed();
    println!("─── (d) plaquette_mean × {} (measurement) ───", N_SWEEPS);
    println!(
        "  wall: {:.4} ms  ({:.2} µs / call)  sink={:.6}",
        measurement_total.as_secs_f64() * 1000.0,
        measurement_total.as_secs_f64() * 1_000_000.0 / N_SWEEPS as f64,
        meas_sink,
    );

    // ── (6) Summary table ─────────────────────────────────────────
    let total_s = total.as_secs_f64();
    let staple_s = staple_total.as_secs_f64();
    let kp_s = kp_total.as_secs_f64();
    let mutex_s = mutex_per_sweep.as_secs_f64();
    let meas_s = measurement_total.as_secs_f64();
    let accounted = staple_s + kp_s + mutex_s + meas_s;
    let other = (total_s - accounted).max(0.0);

    println!("─── breakdown ───────────────────────────────");
    println!("  total           {:>7.4} s   100.0 %", total_s);
    println!("  staple_sum      {:>7.4} s   {:>5.1} %", staple_s, staple_s / total_s * 100.0);
    println!("  sample_su2_link {:>7.4} s   {:>5.1} %", kp_s, kp_s / total_s * 100.0);
    println!("  mutex (per-sweep){:>6.4} s   {:>5.1} %", mutex_s, mutex_s / total_s * 100.0);
    println!("  measurement     {:>7.4} s   {:>5.1} %", meas_s, meas_s / total_s * 100.0);
    println!("  other (residual){:>7.4} s   {:>5.1} %", other, other / total_s * 100.0);

    // Force Arc to live to end so the field stays alive.
    let _ = Arc::strong_count(&field_arc);
}
