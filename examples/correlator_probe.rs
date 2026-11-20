//! CORRELATOR / EFFECTIVE_MASS parity probe (Q1, docs/HALCYON_QUANTUM_GAP_SPEC.md): read
//! per-configuration time-slice values as JSON and write the engine estimator as JSON.
//! Used by e2e/probes/test_q1_correlator_parity.py, test_q2_exact_2d.py and the Q3 harness.
//! No query route yet.
//!
//! usage: correlator_probe input.json output.json
//! input: {"p": [[p(0), .., p(L-1)], ..], "block": b}   one row per configuration; block
//!        (default 1) is the jackknife block length. The Halcyon-normalisation error is block 1 only.
//!    or: {"ops": [op][cfg][t], "t0": 1, "t1": 2, "block": b}   a GEVP over the operators at
//!        (t0, t1); the series projected on the lowest-state vector goes through EFFECTIVE_MASS.
use gigi::gauge::correlator::{
    effective_mass_blocked, gevp, jackknife_effective_mass_halcyon, matrix_correlator, project, NoPlateau, Plateau,
};
use serde::Deserialize;
use serde_json::json;
use std::{fs::File, io::BufReader};

#[derive(Deserialize)]
struct Input {
    #[serde(default)]
    p: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    ops: Option<Vec<Vec<Vec<f64>>>>,
    #[serde(default = "one")]
    t0: usize,
    #[serde(default = "two")]
    t1: usize,
    #[serde(default = "one")]
    block: usize,
}

fn one() -> usize {
    1
}
fn two() -> usize {
    2
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let inp: Input = serde_json::from_reader(BufReader::new(File::open(&a[1]).unwrap())).unwrap();
    let t0 = std::time::Instant::now();
    let fin = |v: &[f64]| v.iter().map(|x| if x.is_finite() { json!(x) } else { json!(null) }).collect::<Vec<_>>();
    let pj = |x: &Result<Plateau, NoPlateau>| match x {
        Ok(p) => json!({"t_lo": p.t_lo, "t_hi": p.t_hi, "mass": p.mass, "error": p.error, "chi2_dof": p.chi2_dof}),
        Err(e) => json!({"refused": e.to_string(), "t": e.t}),
    };
    let (series, gevp_json) = match (inp.p, inp.ops) {
        (Some(p), None) => (p, json!(null)),
        (None, Some(ops)) => {
            let c = matrix_correlator(&ops).unwrap();
            match gevp(&c, inp.t0, inp.t1) {
                Ok(g) => {
                    let gj = json!({"t0": g.t0, "t1": g.t1, "eigenvalues": g.eigenvalues, "vector": g.vector,
                                    "condition_t0": g.condition_t0});
                    (project(&ops, &g.vector), gj)
                }
                Err(e) => {
                    let out = json!({"gevp": {"refused": e}, "seconds": t0.elapsed().as_secs_f64()});
                    std::fs::write(&a[2], serde_json::to_string_pretty(&out).unwrap()).unwrap();
                    return;
                }
            }
        }
        _ => panic!("give exactly one of p or ops"),
    };
    let r = effective_mass_blocked(&series, inp.block).unwrap();
    let halcyon_err =
        if inp.block == 1 && gevp_json.is_null() { jackknife_effective_mass_halcyon(&series).unwrap().1 } else { vec![] };
    let out = json!({
        "block": inp.block, "gevp": gevp_json,
        "C_t": fin(&r.correlator), "C_err_t": fin(&r.correlator_error),
        "m_eff_t": r.m_eff, "m_eff_err_t": fin(&r.m_eff_error),
        "m_eff_err_t_halcyon_normalisation": fin(&halcyon_err),
        "plateau": pj(&r.plateau), "plateau_as_fixed": pj(&r.plateau_as_fixed),
        "seconds": t0.elapsed().as_secs_f64(),
    });
    std::fs::write(&a[2], serde_json::to_string_pretty(&out).unwrap()).unwrap();
}
