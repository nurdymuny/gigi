//! Native Ricci-flow gates. Initial RED is an unknown verb, not a compile error.
use gigi::parser::{execute, parse, ExecResult};
use gigi::types::{Record, Value};

fn run(a: [f64; 3], t: f64, n: usize, options: &str) -> Result<Vec<Record>, String> {
    let q = format!("RICCI_FLOW SU2 METRIC ({},{},{}) TIME {t} STEPS {n} {options};", a[0], a[1], a[2]);
    let stmt = parse(&q)?;
    let mut engine = gigi::engine::Engine::open_memory().map_err(|e| e.to_string())?;
    match execute(&mut engine, &stmt)? {
        ExecResult::Rows(r) => Ok(r),
        other => Err(format!("expected trajectory, got {other:?}")),
    }
}
fn num(r: &Record, k: &str) -> f64 {
    match r.get(k) { Some(Value::Float(x)) => *x, other => panic!("{k}: {other:?}") }
}
fn close(a: f64, b: f64, tol: f64) { assert!((a-b).abs() <= tol, "{a} != {b}; tolerance={tol}"); }

#[test]
fn rf00_native_grammar() {
    assert!(parse("RICCI_FLOW SU2 METRIC (1,2,3) TIME 0.1 STEPS 64 NORMALIZED TOLERANCE 0.00000001;").is_ok());
    for q in [
        "RICCI_FLOW TORUS METRIC (1,1,1) TIME 1 STEPS 64;",
        "RICCI_FLOW SU2 METRIC (1,1) TIME 1 STEPS 64;",
        "RICCI_FLOW SU2 METRIC (1,1,1) TIME 1 STEPS 1.5;",
        "RICCI_FLOW SU2 METRIC (1,1,1) TIME 1 STEPS 64 NORMALIZED NORMALIZED;",
        "RICCI_FLOW SU2 METRIC (1,1,1) TIME 1 STEPS 64 SURGERY;",
    ] { assert!(parse(q).is_err(), "{q}"); }
}

#[cfg(not(feature = "gauge"))]
#[test]
fn rf00_feature_off_refuses() {
    assert!(run([1.;3], 0.1, 16, "").unwrap_err().contains("gauge"));
}

#[cfg(feature = "gauge")]
mod enabled {
    use super::*;
    #[test]
    fn rf01_round_shrink_and_normalized_stationarity() {
        for normalized in [false, true] {
            let rows = run([2.;3], 0.4, 16, if normalized {"NORMALIZED"} else {""}).unwrap();
            assert_eq!(rows.len(), 17);
            for (i,r) in rows.iter().enumerate() {
                let t = 0.4*i as f64/16.;
                let a = if normalized {2.} else {2.-4.*t};
                close(num(r,"t"),t,1e-15);
                for k in ["a","b","c"] { close(num(r,k),a,2e-14); }
                close(num(r,"scalar_curvature"),6./a,1e-12);
                close(num(r,"sectional_ab"),1./a,1e-12);
                close(num(r,"volume_cover"),2.*std::f64::consts::PI.powi(2)*a.powf(1.5),1e-11);
                assert_eq!(r.get("topology_inferred"),Some(&Value::Bool(false)));
                assert_eq!(r.get("surgery_performed"),Some(&Value::Bool(false)));
                assert_eq!(r.get("error_bound_certified"),Some(&Value::Bool(false)));
            }
        }
    }
    #[test]
    fn rf02_berger_and_triaxial_initial_curvature() {
        let r = run([3.,1.,1.],0.001,16,"").unwrap().remove(0);
        close(num(&r,"ricci_a"),18.,1e-13);
        close(num(&r,"ricci_b"),-2.,1e-13);
        close(num(&r,"sectional_ab"),3.,1e-13);
        close(num(&r,"sectional_bc"),-5.,1e-13);
        let r = run([1.,2.,3.],0.001,16,"").unwrap().remove(0);
        close(num(&r,"ricci_a"),0.,1e-13);
        close(num(&r,"ricci_b"),0.,1e-13);
        close(num(&r,"ricci_c"),8.,1e-13);
        close(num(&r,"scalar_curvature"),8./3.,1e-13);
    }
    #[test]
    fn rf03_normalized_volume_and_rounding() {
        let rows = run([0.5,1.,2.],2.,1024,"NORMALIZED").unwrap();
        for r in &rows { close(num(r,"a")*num(r,"b")*num(r,"c"),1.,2e-8); }
        let last = rows.last().unwrap();
        assert!(num(last,"anisotropy")<1.00001);
        close(num(last,"a"),1.,1e-5);
    }
    #[test]
    fn rf03_scaling_and_permutation() {
        let a = run([1.,2.,3.],0.05,32,"").unwrap();
        let b = run([3.,1.,2.],0.05,32,"").unwrap();
        let c = run([10.,20.,30.],0.5,32,"").unwrap();
        for ((x,y),z) in a.iter().zip(&b).zip(&c) {
            close(num(x,"a"),num(y,"b"),1e-13);
            close(num(x,"a")*10.,num(z,"a"),1e-12);
            close(num(x,"scalar_curvature")/10.,num(z,"scalar_curvature"),1e-13);
        }
    }
    #[test]
    fn rf05_negative_scalar_is_not_refused_or_clipped() {
        let r = run([5.,1.,1.],0.01,128,"").unwrap();
        close(num(&r[0],"scalar_curvature"),-2.,1e-13);
        assert!(num(r.last().unwrap(),"b")>1.);
        assert!(num(r.last().unwrap(),"scalar_curvature")>-2.);
    }
    #[test]
    fn rf06_singularity_and_contract_refusals() {
        for t in [0.25,0.3] { assert!(run([1.;3],t,64,"").unwrap_err().contains("positive")); }
        for a in [[0.,1.,1.],[-1.,1.,1.],[1e-8,1.,1.],[1e-6,1e6,1.]] { assert!(run(a,0.1,64,"").is_err()); }
        for n in [0,4097] { assert!(run([1.;3],0.1,n,"").unwrap_err().contains("STEPS")); }
        for t in [0.,-1.,101.] { assert!(run([1.;3],t,64,"").unwrap_err().contains("TIME")); }
        assert!(run([1.;3],0.1,64,"TOLERANCE 0").unwrap_err().contains("TOLERANCE"));
    }
    #[test]
    fn rf06_error_estimate_refuses_underresolved_flow() {
        assert!(run([0.5,1.,2.],0.1,2,"NORMALIZED TOLERANCE 0.000000000001").unwrap_err().contains("estimate"));
    }
}
