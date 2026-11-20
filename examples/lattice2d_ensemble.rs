//! Q2 ensemble generator (docs/HALCYON_Q2_RUN_SPEC.md): 2D SU(2) Metropolis with the Wilson
//! or heat-kernel action, writing the Polyakov-loop and plaquette-row series as JSON.
//!
//! usage: lattice2d_ensemble ACTION BETA NS NT N_THERM N_MEAS EVERY SEED OUT.json
//!        ACTION = wilson | heat_kernel
use gigi::gauge::lattice2d::{generate, Action};
use serde_json::json;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let beta: f64 = a[2].parse().unwrap();
    let action = match a[1].as_str() {
        "wilson" => Action::Wilson { beta },
        "heat_kernel" => Action::HeatKernel { beta },
        other => panic!("unknown action {other}"),
    };
    let n = |i: usize| a[i].parse::<usize>().unwrap();
    let (ns, nt, n_therm, n_meas, every) = (n(3), n(4), n(5), n(6), n(7));
    let seed: u64 = a[8].parse().unwrap();
    let t0 = std::time::Instant::now();
    let e = generate(action, ns, nt, n_therm, n_meas, every, seed);
    let out = json!({
        "action": a[1], "beta": beta, "ns": ns, "nt": nt, "n_therm": n_therm, "n_meas": n_meas,
        "every": every, "seed": seed, "eps": e.eps, "hits": 4, "acceptance": e.acceptance,
        "seconds": t0.elapsed().as_secs_f64(),
        "polyakov": e.polyakov, "plaquette_rows": e.plaquette_rows,
    });
    std::fs::write(&a[9], serde_json::to_string(&out).unwrap()).unwrap();
}
