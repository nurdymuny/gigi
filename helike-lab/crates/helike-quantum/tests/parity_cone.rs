//! Parity against oracle/oracle_cone.json, written by oracle/make_cone_oracle.py from scripts/helike_positive_cone.py.

use helike_quantum::lanczos::Eigs;
use helike_quantum::model::*;
use helike_quantum::operator::{ConeOperator, Mutation, Part};
use helike_quantum::ritz::{self, SolveSettings};
use serde_json::Value;

const S: f64 = 1.15;
const T: f64 = 3.0;
pub const EXTRA: usize = 4;

fn oracle() -> Value {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../oracle/oracle_cone.json");
    serde_json::from_str(&std::fs::read_to_string(p).expect("run oracle/make_cone_oracle.py first")).unwrap()
}

fn vecf(v: &Value) -> Vec<f64> {
    v.as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect()
}

fn m6(v: &Value) -> M6 {
    let f = vecf(v);
    std::array::from_fn(|i| std::array::from_fn(|j| f[6 * i + j]))
}

fn python_background(c: &Value) -> Background {
    let b = &c["background"];
    let g0f = vecf(&b["G0"]);
    let g0: M3 = std::array::from_fn(|i| std::array::from_fn(|j| g0f[3 * i + j]));
    let wf = vecf(&b["w"]);
    let w: [f64; 6] = std::array::from_fn(|i| wf[i]);
    Background::from_parts(Model::berger(S, T, c["g"].as_f64().unwrap()), g0, m6(&b["M"]), w)
}

fn max_diff(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0, f64::max)
}

#[test]
fn background_and_chart() {
    let o = oracle();
    for c in o["controls"].as_array().unwrap() {
        let g = c["g"].as_f64().unwrap();
        let mine = Background::maurer_cartan(S, T, g).unwrap();
        let py = python_background(c);
        let dw = max_diff(&mine.w, &py.w);
        assert!(dw < 1e-12, "frequencies differ by {dw}");
        assert!(mine.grad_norm < 1e-8, "background not stationary: {}", mine.grad_norm);
        let op = ConeOperator::new(py.clone(), 0, EXTRA, Mutation::None).unwrap();
        let shift = c["background"]["energy_shift"].as_f64().unwrap();
        assert!((op.energy_shift - shift).abs() < 1e-11 * shift.abs(), "{} vs {shift}", op.energy_shift);
        let s_py = vecf(&c["background"]["S"]);
        let s_me: Vec<f64> = op.chart.s.iter().flatten().cloned().collect();
        assert!(max_diff(&s_me, &s_py) < 1e-14, "S differs");
        let u0 = vecf(&c["background"]["u0"]);
        assert!(max_diff(&op.chart.u0, &u0) < 1e-13);
        for p in c["chart"].as_array().unwrap() {
            let zf = vecf(&p["z"]);
            let z: [f64; 6] = std::array::from_fn(|i| zf[i]);
            let gram: Vec<f64> = op.chart.gram(&z).iter().flatten().cloned().collect();
            let gp = vecf(&p["gram"]);
            assert!(max_diff(&gram, &gp) < 1e-11 * gp.iter().fold(1.0f64, |a, b| a.max(b.abs())));
            let met: Vec<f64> = op.chart.metric(&z).iter().flatten().cloned().collect();
            let mp = vecf(&p["metric"]);
            assert!(max_diff(&met, &mp) < 1e-10 * mp.iter().fold(1.0f64, |a, b| a.max(b.abs())), "metric differs");
            assert!((op.chart.log_measure(&z) - p["log_measure"].as_f64().unwrap()).abs() < 1e-11);
            let v = py.model.potential(&op.chart.factor(&z));
            let vp = p["v_sourced"].as_f64().unwrap();
            assert!((v - vp).abs() < 1e-10 * vp.abs().max(1.0), "{v} vs {vp}");
        }
    }
}

#[test]
fn dense_operators_entrywise() {
    let o = oracle();
    let c = &o["controls"][0];
    let bg = python_background(c);
    let d = &c["dense"];
    for (mq, key, part) in [(2usize, "H2", Part::Full), (2, "T2", Part::Kinetic), (4, "H4", Part::Full)] {
        let op = ConeOperator::new(bg.clone(), mq, EXTRA, Mutation::None).unwrap();
        let mine = op.dense(part);
        let want = vecf(&d[key]);
        let scale = want.iter().fold(1.0f64, |a, b| a.max(b.abs()));
        let diff = max_diff(&mine, &want);
        println!("{key}: N = {}, max |diff| = {diff:.3e} (scale {scale:.3})", op.dim());
        assert!(diff < 1e-11 * scale, "{key} differs by {diff}");
        if mq == 4 {
            let mass = op.dense(Part::Mass);
            let n = op.dim();
            let err = (0..n * n).map(|k| (mass[k] - if k % (n + 1) == 0 { 1.0 } else { 0.0 }).abs()).fold(0.0, f64::max);
            assert!(err < 1e-12, "mass is not the identity: {err}");
        }
    }
}

#[test]
fn reference_amplitudes_match() {
    let o = oracle();
    let c = &o["controls"][0];
    let op = ConeOperator::new(python_background(c), 2, EXTRA, Mutation::None).unwrap();
    let a = &c["amplitudes"];
    let vf = vecf(&a["vectors"]);
    let n = op.dim();
    let vectors: Vec<Vec<f64>> = (0..2).map(|k| (0..n).map(|i| vf[i * 2 + k]).collect()).collect();
    let got = ritz::reference_amplitudes(&op, &vectors, 3).unwrap();
    let want = vecf(&a["want"]);
    assert!(max_diff(&got, &want) < 1e-10, "amplitudes differ by {}", max_diff(&got, &want));
}

fn check_result(r: &ritz::HelikeCompetitor, want: &Value, tag: &str) {
    for ch in ["helike", "competitor"] {
        let mine = if ch == "helike" { &r.helike } else { &r.competitor };
        let w = &want[ch];
        let dg = (mine.gap - w["gap"].as_f64().unwrap()).abs();
        let dw = (mine.weight - w["weight"].as_f64().unwrap()).abs();
        let dl = (mine.log_basis_weight - w["log_basis_weight"].as_f64().unwrap()).abs();
        println!("{tag} {ch}: gap {:.12} (diff {dg:.1e}), weight {:.10} (diff {dw:.1e}), log-basis diff {dl:.1e}", mine.gap, mine.weight);
        assert!(dg < 1e-9 && dw < 1e-9 && dl < 1e-9, "{tag} {ch}: gap diff {dg}, weight diff {dw}, log-basis diff {dl}");
        assert_eq!(mine.level_index as u64, w["level_index"].as_u64().unwrap());
    }
    assert!((r.e0 - want["E0"].as_f64().unwrap()).abs() < 1e-9);
}

#[test]
fn controls_at_mq4_and_mq6() {
    let o = oracle();
    for c in o["controls"].as_array().unwrap() {
        let g = c["g"].as_f64().unwrap();
        let g2 = c["g2"].as_f64().unwrap();
        let bg = Background::maurer_cartan(S, T, g).unwrap();
        for mq in [4usize, 6] {
            let want = &c["results"][mq.to_string()];
            let op = ConeOperator::new(bg.clone(), mq, EXTRA, Mutation::None).unwrap();
            let t0 = std::time::Instant::now();
            let eigs: Eigs = ritz::spectrum(&op, &SolveSettings::default());
            let spec = vecf(&want["spectrum12"]);
            let ds = max_diff(&eigs.values, &spec);
            println!("g2={g2} Mq={mq}: N={} levels diff {ds:.2e}, {} applications, {:.1}s", op.dim(), eigs.applications, t0.elapsed().as_secs_f64());
            assert!(ds < 1e-9, "levels differ by {ds}");
            let r = ritz::helike_and_competitor(&op, &eigs, ritz::helike_frequency(S, T)).unwrap();
            check_result(&r, want, &format!("g2={g2} Mq={mq}"));
        }
    }
}

/// Quadrature order sweep: entrywise H4 error against Python versus the extra nodes per axis (run with --ignored).
#[test]
#[ignore]
fn quadrature_extra_sweep() {
    let o = oracle();
    let c = &o["controls"][0];
    let bg = python_background(c);
    let want = vecf(&c["dense"]["H4"]);
    for extra in [0usize, 1, 2, 3, 4, 6] {
        let op = ConeOperator::new(bg.clone(), 4, extra, Mutation::None).unwrap();
        println!("extra={extra} Q={}: max |H4 diff| = {:.3e}", op.grid.q, max_diff(&op.dense(Part::Full), &want));
    }
}

/// Each deliberate fault must break entrywise parity of H at Mq = 4 (the unmutated operator passes at 1e-11).
#[test]
fn mutations_are_caught() {
    let o = oracle();
    let c = &o["controls"][0];
    let bg = python_background(c);
    let want = vecf(&c["dense"]["H4"]);
    for m in [Mutation::DropMeasureDrift, Mutation::FlipCubic, Mutation::FlatMetric] {
        let op = ConeOperator::new(bg.clone(), 4, EXTRA, m).unwrap();
        let d = max_diff(&op.dense(Part::Full), &want);
        println!("{m:?}: max |H4 diff| = {d:.3e}");
        assert!(d > 1e-3, "{m:?} was not caught (diff {d})");
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Exact Kronecker moments (`ConeOperator::exact`): the same oracle, no quadrature.

#[test]
fn exact_dense_operators_entrywise() {
    let o = oracle();
    let c = &o["controls"][0];
    let bg = python_background(c);
    let d = &c["dense"];
    for (mq, key, part) in [(2usize, "H2", Part::Full), (2, "T2", Part::Kinetic), (4, "H4", Part::Full)] {
        let op = ConeOperator::exact(bg.clone(), mq, Mutation::None).unwrap();
        assert_eq!(op.moments(), "exact");
        let mine = op.dense(part);
        let want = vecf(&d[key]);
        let scale = want.iter().fold(1.0f64, |a, b| a.max(b.abs()));
        let diff = max_diff(&mine, &want);
        println!("exact {key}: N = {}, max |diff| = {diff:.3e} (scale {scale:.3})", op.dim());
        assert!(diff < 1e-12 * scale, "exact {key} differs by {diff}");
        // the grid operator is an independent implementation of the same matrix
        let grid = ConeOperator::new(bg.clone(), mq, EXTRA, Mutation::None).unwrap().dense(part);
        let dg = max_diff(&mine, &grid);
        println!("exact vs grid {key}: {dg:.3e}");
        assert!(dg < 1e-11 * scale, "exact and grid {key} differ by {dg}");
        if mq == 4 {
            let mass = op.dense(Part::Mass);
            let n = op.dim();
            let err = (0..n * n).map(|k| (mass[k] - if k % (n + 1) == 0 { 1.0 } else { 0.0 }).abs()).fold(0.0, f64::max);
            assert!(err < 1e-13, "exact mass is not the identity: {err}");
        }
    }
}

#[test]
fn exact_controls_at_mq4_and_mq6() {
    let o = oracle();
    for c in o["controls"].as_array().unwrap() {
        let g = c["g"].as_f64().unwrap();
        let g2 = c["g2"].as_f64().unwrap();
        let bg = Background::maurer_cartan(S, T, g).unwrap();
        for mq in [4usize, 6] {
            let want = &c["results"][mq.to_string()];
            let op = ConeOperator::exact(bg.clone(), mq, Mutation::None).unwrap();
            let t0 = std::time::Instant::now();
            let eigs: Eigs = ritz::spectrum(&op, &SolveSettings::default());
            let ds = max_diff(&eigs.values, &vecf(&want["spectrum12"]));
            println!("exact g2={g2} Mq={mq}: N={} levels diff {ds:.2e}, {} applications, {:.1}s", op.dim(), eigs.applications, t0.elapsed().as_secs_f64());
            assert!(ds < 1e-9, "exact levels differ by {ds}");
            let r = ritz::helike_and_competitor(&op, &eigs, ritz::helike_frequency(S, T)).unwrap();
            check_result(&r, want, &format!("exact g2={g2} Mq={mq}"));
        }
    }
}

#[test]
fn exact_mutations_are_caught() {
    let o = oracle();
    let c = &o["controls"][0];
    let bg = python_background(c);
    let want = vecf(&c["dense"]["H4"]);
    for m in [Mutation::DropMeasureDrift, Mutation::FlipCubic, Mutation::FlatMetric] {
        let d = max_diff(&ConeOperator::exact(bg.clone(), 4, m).unwrap().dense(Part::Full), &want);
        println!("exact {m:?}: max |H4 diff| = {d:.3e}");
        assert!(d > 1e-3, "exact {m:?} was not caught (diff {d})");
    }
}
