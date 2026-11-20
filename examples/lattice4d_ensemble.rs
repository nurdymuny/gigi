//! Q3 ensemble generator (docs/HALCYON_Q3_RUN_SPEC.md): 4D SU(2) Wilson action, one
//! Kennedy-Pendleton heat-bath sweep plus N_OR overrelaxation sweeps per update, a measurement
//! after every update. Writes the 0++ and flux-loop operator series at each APE smearing level.
//!
//! usage: lattice4d_ensemble BETA LS LT N_THERM N_MEAS N_OR SEED OUT.json [LEVELS] [ALPHA]
//!        LEVELS = comma-separated smearing levels (default 0,4,8,16), ALPHA default 0.5
//!        N_MEAS = 0 runs the timing only: thermalisation sweeps timed, nothing measured.
use gigi::gauge::lattice4d::{smeared_operators, Lattice4d};
use gigi::gauge::marsaglia_haar::SmallRng;
use serde_json::json;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let beta: f64 = a[1].parse().unwrap();
    let n = |i: usize| a[i].parse::<usize>().unwrap();
    let (ls, lt, n_therm, n_meas, n_or) = (n(2), n(3), n(4), n(5), n(6));
    let seed: u64 = a[7].parse().unwrap();
    let levels: Vec<usize> =
        a.get(9).map(|s| s.split(',').map(|x| x.parse().unwrap()).collect()).unwrap_or(vec![0, 4, 8, 16]);
    let alpha: f64 = a.get(10).map(|s| s.parse().unwrap()).unwrap_or(0.5);
    let mut rng = SmallRng::seed_from_u64(seed);
    let mut lat = Lattice4d::hot([lt, ls, ls, ls], &mut rng);
    let update = |lat: &mut Lattice4d, rng: &mut SmallRng| {
        lat.heatbath_sweep(beta, rng);
        for _ in 0..n_or {
            lat.overrelax_sweep();
        }
    };
    let t0 = std::time::Instant::now();
    for _ in 0..n_therm {
        update(&mut lat, &mut rng);
    }
    let therm_s = t0.elapsed().as_secs_f64();
    let t1 = std::time::Instant::now();
    let mut scalar = vec![Vec::with_capacity(n_meas); levels.len()];
    let mut torelon = vec![Vec::with_capacity(n_meas); levels.len()];
    let mut plaq = Vec::with_capacity(n_meas);
    let mut meas_s = 0.0;
    for _ in 0..n_meas {
        update(&mut lat, &mut rng);
        let tm = std::time::Instant::now();
        let ops = smeared_operators(&lat, &levels, alpha);
        meas_s += tm.elapsed().as_secs_f64();
        for (k, o) in ops.into_iter().enumerate() {
            scalar[k].push(o.scalar);
            torelon[k].push(o.torelon);
        }
        plaq.push(lat.mean_plaquette());
    }
    let out = json!({
        "beta": beta, "ls": ls, "lt": lt, "n_therm": n_therm, "n_meas": n_meas, "n_or": n_or,
        "seed": seed, "levels": levels, "alpha": alpha,
        "seconds_thermalisation": therm_s, "seconds_per_update": therm_s / n_therm.max(1) as f64,
        "seconds_measuring": meas_s, "seconds_total": t0.elapsed().as_secs_f64(),
        "seconds_production": t1.elapsed().as_secs_f64(),
        "mean_plaquette": plaq, "scalar": scalar, "torelon": torelon,
    });
    std::fs::write(&a[8], serde_json::to_string(&out).unwrap()).unwrap();
}
