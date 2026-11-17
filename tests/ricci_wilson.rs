use gigi::parser::{execute, parse, ExecResult};
use gigi::types::{Record, Value};

fn run(g: [f64;3], normalized: bool) -> Result<Record,String> {
    let stmt = parse(&format!("RICCI_WILSON_AUDIT SU2 METRIC ({},{},{}) {};",g[0],g[1],g[2],if normalized {"NORMALIZED"} else {""}))?;
    let mut engine = gigi::engine::Engine::open_memory().map_err(|e|e.to_string())?;
    match execute(&mut engine,&stmt)? {
        ExecResult::Rows(mut rows) if rows.len()==1 => Ok(rows.remove(0)),
        other => Err(format!("expected an actual audit row: {other:?}")),
    }
}
fn vector(r:&Record,k:&str)->Vec<f64> {
    match r.get(k) {Some(Value::Vector(v))=>v.clone(),other=>panic!("{k}: {other:?}")}
}
fn scalar(r:&Record,k:&str)->f64 {
    match r.get(k) {Some(Value::Float(v))=>*v,other=>panic!("{k}: {other:?}")}
}
fn close(a:f64,b:f64,tol:f64) {assert!((a-b).abs()<=tol,"{a} != {b}");}

#[test]
fn cw00_grammar() {
    assert!(parse("RICCI_WILSON_AUDIT SU2 METRIC (1,2,3) NORMALIZED;").is_ok());
    for q in ["RICCI_WILSON_AUDIT TORUS METRIC (1,2,3);",
        "RICCI_WILSON_AUDIT SU2 METRIC (1,2);",
        "RICCI_WILSON_AUDIT SU2 METRIC (1,2,3) NORMALIZED NORMALIZED;",
        "RICCI_WILSON_AUDIT SU2 METRIC (1,2,3) FIT_CLOCK;"] {
        assert!(parse(q).is_err(),"{q}");
    }
}
#[cfg(not(feature="gauge"))]
#[test]
fn cw00_feature_off() {assert!(run([1.;3],false).unwrap_err().contains("gauge"));}

#[cfg(feature="gauge")]
mod enabled {
    use super::*;
    #[test]
    fn cw01_round_and_noncritical_berger() {
        let round=run([1.;3],false).unwrap();
        assert_eq!(vector(&round,"connection"),vec![1.;3]);
        assert_eq!(vector(&round,"yang_mills_velocity"),vec![0.;3]);
        let r=run([3.,1.,1.],false).unwrap();
        for (k,expected) in [("connection",[-1.,3f64.sqrt(),3f64.sqrt()]),
            ("mapped_ricci_velocity",[48.,-8.*3f64.sqrt(),-8.*3f64.sqrt()]),
            ("yang_mills_velocity",[48.,-8.*3f64.sqrt(),-8.*3f64.sqrt()])] {
            for (a,b) in vector(&r,k).iter().zip(expected) {close(*a,b,1e-12);}
        }
        assert!(scalar(&r,"relative_residual")<1e-12);
    }
    #[test]
    fn cw03_nonround_grid_residual() {
        for a in [0.5,1.,2.,5.] {for b in [0.5,1.,2.,5.] {for c in [0.5,1.,2.,5.] {
            let r=run([a,b,c],false).unwrap();
            let p=vector(&r,"mapped_ricci_velocity");
            let w=vector(&r,"yang_mills_velocity");
            let residual=vector(&r,"residual");
            for i in 0..3 {close(residual[i],p[i]-w[i],1e-15);}
            assert!(scalar(&r,"relative_residual")<2e-12,"{a},{b},{c}: {r:?}");
        }}}
    }
    #[test]
    fn cw04_scale_permutation_and_normalization() {
        let a=run([1.,2.,3.],false).unwrap();
        let b=run([7.,14.,21.],false).unwrap();
        let c=run([3.,1.,2.],false).unwrap();
        let d=run([1.,2.,3.],true).unwrap();
        for i in 0..3 {
            close(vector(&a,"connection")[i],vector(&b,"connection")[i],1e-14);
            close(vector(&a,"yang_mills_velocity")[i]/7.,vector(&b,"yang_mills_velocity")[i],1e-13);
            close(vector(&a,"mapped_ricci_velocity")[i],vector(&c,"mapped_ricci_velocity")[(i+1)%3],1e-13);
            close(vector(&a,"mapped_ricci_velocity")[i],vector(&d,"mapped_ricci_velocity")[i],1e-13);
        }
    }
    #[test]
    fn cw00_refuse_bad_metrics() {
        for g in [[0.,1.,1.],[-1.,1.,1.],[1e-8,1.,1.],[1e-6,1e6,1.]] {
            assert!(run(g,false).unwrap_err().contains("METRIC"));
        }
    }
    #[test]
    fn cw08_scope_is_not_equivalence_verdict() {
        let r=run([3.,1.,1.],false).unwrap();
        assert_eq!(r.get("scope"),Some(&Value::Text("homogeneous_levi_civita_instantaneous".into())));
        assert_eq!(r.get("lattice_correspondence_tested"),Some(&Value::Bool(false)));
        assert_eq!(r.get("fixed_background_equivalence_tested"),Some(&Value::Bool(false)));
        assert_eq!(r.get("time_scale_fitted"),Some(&Value::Bool(false)));
        close(scalar(&r,"time_scale"),1.,0.);
    }
}
