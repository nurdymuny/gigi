//! CORRELATOR / EFFECTIVE_MASS parity probe (Q1, docs/HALCYON_QUANTUM_GAP_SPEC.md): read
//! per-configuration time-slice values as JSON and write the engine estimator as JSON.
//! Used by e2e/probes/test_q1_correlator_parity.py, test_q2_exact_2d.py and the Q3 harness.
//! No query route yet.
//!
//! usage: correlator_probe input.json output.json
//! input: {"p": [[p(0), .., p(L-1)], ..], "block": b}   one row per configuration; block
//!        (default 1) is the jackknife block length. The Halcyon-normalisation error is block 1 only.
//!    or: {"ops": [op][cfg][t], "t0": 1, "t1": 2, "block": b, "rule": r}   a GEVP over the operators at
//!        (t0, t1); the projected series goes through EFFECTIVE_MASS. "rule" (default "a4"):
//!        "a4"          Amendment 4, the verb's rule: Amendment-2 GEVP, Q3-C only as a fallback, no plateau
//!                      slice before t1;
//!        "a4_nostart"  A4-1 alone (the A4-start counterfactual);
//!        "q3c"         Amendment 3 (Q3-C always, no start bound);
//!        "a2"          Amendment 2 (no Q3-C, no start bound). Legacy: "truncate": false with no "rule" = "a2".
use gigi::gauge::correlator::{
    effective_mass_blocked, effective_mass_blocked_rule, gevp_effective_mass, gevp_run, jackknife_effective_mass_halcyon,
    GevpRun, NoPlateau, Plateau, PlateauRule,
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
    #[serde(default = "yes")]
    truncate: bool,
    #[serde(default)]
    rule: Option<String>,
}

fn yes() -> bool {
    true
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
    let gj = |run: &GevpRun, rule: &str| {
        let g = &run.gevp;
        json!({"t0": g.t0, "t1": g.t1, "eigenvalues": g.eigenvalues, "vector": g.vector,
               "condition_t0": g.condition_t0, "kept": run.kept_operators, "rule": rule,
               "truncate": run.directions_kept.is_some(), "directions_kept": run.directions_kept,
               "direction_snr_t1": fin(&run.direction_snr_t1)})
    };
    let refuse = |e: String| {
        let out = json!({"gevp": {"refused": e}, "seconds": t0.elapsed().as_secs_f64()});
        std::fs::write(&a[2], serde_json::to_string_pretty(&out).unwrap()).unwrap();
    };
    let (series, gevp_json, r) = match (inp.p, inp.ops) {
        (Some(p), None) => {
            let r = effective_mass_blocked(&p, inp.block).unwrap();
            (p, json!(null), r)
        }
        (None, Some(ops)) => {
            let rule = inp.rule.clone().unwrap_or_else(|| if inp.truncate { "a4".into() } else { "a2".into() });
            match rule.as_str() {
                "a4" | "a4_nostart" => match gevp_effective_mass(&ops, inp.t0, inp.t1, inp.block, rule == "a4") {
                    Ok(gm) => {
                        let mut j = gj(&gm.run, &rule);
                        j["path"] = json!(gm.path.as_str());
                        j["a2_failure"] = json!(gm.a2_failure);
                        (gm.run.series, j, gm.em)
                    }
                    Err(e) => return refuse(e),
                },
                "q3c" | "a2" => match gevp_run(&ops, inp.t0, inp.t1, inp.block, rule == "q3c") {
                    Ok(run) => {
                        let r = effective_mass_blocked_rule(&run.series, inp.block, PlateauRule::AMENDED).unwrap();
                        let j = gj(&run, &rule);
                        (run.series, j, r)
                    }
                    Err(e) => return refuse(e),
                },
                other => panic!("unknown rule {other}"),
            }
        }
        _ => panic!("give exactly one of p or ops"),
    };
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
