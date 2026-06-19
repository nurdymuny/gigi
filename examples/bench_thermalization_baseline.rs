//! Sprint A baseline harness for the face_edges + measurement_history hoist.
//!
//! Measures wall-clock for
//!   GIBBS_SAMPLE U BETA 2.5 N_SWEEPS 200 MEASURE_EVERY 1
//!                MEASURE (MEAN(PLAQUETTE)) SEED 20260616
//! on the truncated-icosahedron buckyball (V=60, E=90, F=32).
//!
//! Protocol:
//!   - 3 warmup iterations (NOT timed) to pay any JIT / cold-cache cost.
//!   - 5 timed iterations.
//!   - Report mean + min wall-clock in ms.
//!   - Report the final MeanPlaquette[199] checksum (must stay byte-identical
//!     post-hoist — TDD-HAL-III.5 + III.8b + IV.10 A2 row 1 are the receipts).
//!
//! Run with:
//!   cargo run --release --features halcyon --example bench_thermalization_baseline

use std::time::Instant;

use gigi::gauge::gibbs_sample::{gibbs_sample, ObservableId};
use gigi::gauge::registry as gauge_registry;
use gigi::gauge::su2_gauge_field::{GaugeFieldInit, SU2GaugeField};
use gigi::lattice::registry as lattice_registry;
use gigi::lattice::topology::truncated_icosahedron::buckyball;

const BETA: f64 = 2.5;
const N_SWEEPS: usize = 200;
const MEASURE_EVERY: usize = 1;
const SEED: u64 = 20260616;
const WARMUPS: usize = 3;
const TIMED: usize = 5;

fn reset_identity(field_name: &str) {
    let bb = buckyball();
    // re-register the lattice (no-op if already there; safe to call)
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

fn one_run(field_name: &str) -> (f64, f64) {
    reset_identity(field_name);
    let t0 = Instant::now();
    let resp = gibbs_sample(
        field_name,
        BETA,
        N_SWEEPS,
        MEASURE_EVERY,
        vec![ObservableId::MeanPlaquette],
        Some(SEED),
    )
    .expect("gibbs_sample");
    let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let chain = resp
        .measurement_history
        .get(&ObservableId::MeanPlaquette)
        .expect("MeanPlaquette chain");
    let final_mean = *chain.last().expect("chain non-empty");
    (elapsed_ms, final_mean)
}

fn main() {
    let bb = buckyball();
    println!("=== Sprint A baseline ===");
    println!(
        "buckyball V={} E={} F={}    β={} n_sweeps={} measure_every={} seed={}",
        bb.n_vertices,
        bb.n_edges(),
        bb.n_faces(),
        BETA,
        N_SWEEPS,
        MEASURE_EVERY,
        SEED
    );
    println!(
        "warmups={} timed={} observables=[MeanPlaquette]",
        WARMUPS, TIMED
    );

    // ── Warmups ────────────────────────────────────────────────────────
    for i in 0..WARMUPS {
        let (ms, final_p) = one_run("U_baseline_warm");
        println!("  warmup[{}] {:>9.3} ms   final<P>={:.12}", i, ms, final_p);
    }

    // ── Timed runs ────────────────────────────────────────────────────
    let mut timings_ms: Vec<f64> = Vec::with_capacity(TIMED);
    let mut final_mp: Vec<f64> = Vec::with_capacity(TIMED);
    for i in 0..TIMED {
        let (ms, final_p) = one_run("U_baseline_timed");
        println!("  timed [{}] {:>9.3} ms   final<P>={:.12}", i, ms, final_p);
        timings_ms.push(ms);
        final_mp.push(final_p);
    }

    let mean = timings_ms.iter().sum::<f64>() / timings_ms.len() as f64;
    let min = timings_ms.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = timings_ms.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    // Sanity: same seed → identical final mean plaquette across timed runs.
    let all_same = final_mp.windows(2).all(|w| w[0] == w[1]);
    let baseline_final = final_mp[0];

    println!("=== summary ===");
    println!(
        "  wall mean = {:>9.3} ms   min = {:>9.3} ms   max = {:>9.3} ms",
        mean, min, max
    );
    println!(
        "  per-sweep mean = {:>7.3} ms",
        mean / N_SWEEPS as f64
    );
    println!(
        "  final MeanPlaquette[{}] = {:.17e}    (same across runs: {})",
        N_SWEEPS - 1,
        baseline_final,
        all_same
    );
    println!(
        "  JSON: {{\"baseline_wall_ms_mean\":{:.6},\"baseline_wall_ms_min\":{:.6},\"baseline_mean_plaquette_at_199\":{:.17e}}}",
        mean, min, baseline_final
    );
}
