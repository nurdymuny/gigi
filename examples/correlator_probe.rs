//! CORRELATOR / EFFECTIVE_MASS parity probe (Q1, docs/HALCYON_QUANTUM_GAP_SPEC.md): read
//! per-configuration time-slice values as JSON and write the engine estimator as JSON.
//! Used by e2e/probes/test_q1_correlator_parity.py. No query route yet.
//!
//! usage: correlator_probe input.json output.json
//! input: {"p": [[p(0), .., p(L-1)], ..]}   one row per configuration
use gigi::gauge::correlator::{effective_mass, jackknife_effective_mass_halcyon};
use serde::Deserialize;
use serde_json::json;
use std::{fs::File, io::BufReader};

#[derive(Deserialize)]
struct Input {
    p: Vec<Vec<f64>>,
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let inp: Input = serde_json::from_reader(BufReader::new(File::open(&a[1]).unwrap())).unwrap();
    let t0 = std::time::Instant::now();
    let r = effective_mass(&inp.p).unwrap();
    let (_, halcyon_err) = jackknife_effective_mass_halcyon(&inp.p).unwrap();
    let fin = |v: &[f64]| v.iter().map(|x| if x.is_finite() { json!(x) } else { json!(null) }).collect::<Vec<_>>();
    let plateau = match &r.plateau {
        Ok(p) => json!({"t_lo": p.t_lo, "t_hi": p.t_hi, "mass": p.mass, "error": p.error, "chi2_dof": p.chi2_dof}),
        Err(e) => json!({"refused": e.to_string(), "t": e.t}),
    };
    let out = json!({
        "C_t": fin(&r.correlator), "C_err_t": fin(&r.correlator_error),
        "m_eff_t": r.m_eff, "m_eff_err_t": fin(&r.m_eff_error),
        "m_eff_err_t_halcyon_normalisation": fin(&halcyon_err),
        "plateau": plateau, "seconds": t0.elapsed().as_secs_f64(),
    });
    std::fs::write(&a[2], serde_json::to_string_pretty(&out).unwrap()).unwrap();
}
