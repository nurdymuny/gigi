//! CORRELATOR and EFFECTIVE_MASS as GQL verbs (Q1 registration, docs/HALCYON_QUANTUM_GAP_SPEC.md):
//! the rows equal the kernel on the same series; refusals are by name.
#![cfg(feature = "gauge")]

use gigi::engine::Engine;
use gigi::gauge::correlator::{effective_mass_blocked, gevp, matrix_correlator, project};
use gigi::parser::{self, ExecResult};
use gigi::types::{Record, Value};

fn run(e: &mut Engine, stmt: &str) -> Result<ExecResult, String> {
    parser::execute(e, &parser::parse(stmt)?)
}

struct Lcg(u64);
impl Lcg {
    fn u(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }
    fn gauss(&mut self) -> f64 {
        (-2.0 * self.u().ln()).sqrt() * (std::f64::consts::TAU * self.u()).cos()
    }
}

/// Gaussian series with covariance cosh(m(d − L/2)) (an exact single mass), via Cholesky.
fn cosh_series(m: f64, l: usize, n: usize, rng: &mut Lcg) -> Vec<Vec<f64>> {
    let k = |d: usize| (m * (d as f64 - l as f64 / 2.0)).cosh() / (m * l as f64 / 2.0).cosh();
    let mut ch = vec![vec![0.0; l]; l];
    for i in 0..l {
        for j in 0..=i {
            let d = i.abs_diff(j);
            let mut v = k(d.min(l - d));
            for q in 0..j {
                v -= ch[i][q] * ch[j][q];
            }
            ch[i][j] = if i == j { v.sqrt() } else { v / ch[j][j] };
        }
    }
    (0..n)
        .map(|_| {
            let z: Vec<f64> = (0..l).map(|_| rng.gauss()).collect();
            (0..l).map(|i| 2.0 + (0..=i).map(|q| ch[i][q] * z[q]).sum::<f64>()).collect()
        })
        .collect()
}

fn engine_with(dir: &std::path::Path, a: &[Vec<f64>], b: &[Vec<f64>]) -> Engine {
    let mut e = Engine::open(dir).unwrap();
    run(&mut e, "BUNDLE glue BASE (id NUMERIC) FIBER (cfg NUMERIC, t NUMERIC, p NUMERIC, q NUMERIC);").unwrap();
    let l = a[0].len();
    let recs: Vec<Record> = (0..a.len())
        .flat_map(|c| (0..l).map(move |t| (c, t)))
        .map(|(c, t)| {
            let mut r = Record::new();
            r.insert("id".into(), Value::Float((c * l + t) as f64));
            r.insert("cfg".into(), Value::Float(c as f64));
            r.insert("t".into(), Value::Float(t as f64));
            r.insert("p".into(), Value::Float(a[c][t]));
            r.insert("q".into(), Value::Float(b[c][t]));
            r
        })
        .collect();
    e.batch_insert("glue", &recs).unwrap();
    e
}

fn rows(r: ExecResult) -> Vec<Record> {
    match r {
        ExecResult::Rows(r) => r,
        other => panic!("expected rows, got {other:?}"),
    }
}

fn f(r: &Record, k: &str) -> f64 {
    r.get(k).and_then(|v| v.as_f64()).unwrap_or_else(|| panic!("missing {k}"))
}

#[test]
fn verbs_equal_the_kernel_and_refuse_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let mut rng = Lcg(5);
    let (l, n) = (16usize, 1000usize);
    let a = cosh_series(0.5, l, n, &mut rng);
    // a second "operator": the same signal plus independent noise (as a coarser smearing level)
    let b: Vec<Vec<f64>> = a.iter().map(|r| r.iter().map(|x| x + 0.5 * rng.gauss()).collect()).collect();
    let mut e = engine_with(dir.path(), &a, &b);

    // CORRELATOR rows equal the kernel, t by t
    let k = effective_mass_blocked(&a, 10).unwrap();
    let cr = rows(run(&mut e, "CORRELATOR glue ON p BLOCK 10;").unwrap());
    assert_eq!(cr.len(), l);
    for (t, r) in cr.iter().enumerate() {
        assert_eq!(f(r, "t") as usize, t);
        assert!((f(r, "C") - k.correlator[t]).abs() <= 1e-14 * k.correlator[0].abs(), "C at t={t}");
        assert!((f(r, "C_err") - k.correlator_error[t]).abs() <= 1e-12 * k.correlator_error[t].abs().max(1e-300));
    }

    // EFFECTIVE_MASS: the plateau of the kernel
    let p = k.plateau.clone().unwrap();
    let em = rows(run(&mut e, "EFFECTIVE_MASS glue ON p BLOCK 10;").unwrap());
    assert_eq!(em.len(), 1);
    assert!((f(&em[0], "mass") - p.mass).abs() < 1e-14);
    assert!((f(&em[0], "error") - p.error).abs() < 1e-14);
    assert_eq!((f(&em[0], "t_lo") as usize, f(&em[0], "t_hi") as usize), (p.t_lo, p.t_hi));
    assert!((p.mass - 0.5).abs() < 3.0 * p.error, "{p:?}");

    // EFFECTIVE_MASS with GEVP equals the kernel's GEVP projection
    let g = gevp(&matrix_correlator(&[a.clone(), b.clone()]).unwrap(), 1, 2).unwrap();
    let kp = effective_mass_blocked(&project(&[a.clone(), b.clone()], &g.vector), 10).unwrap().plateau.unwrap();
    let eg = rows(run(&mut e, "EFFECTIVE_MASS glue ON p, q BLOCK 10 GEVP 1, 2;").unwrap());
    assert!((f(&eg[0], "mass") - kp.mass).abs() < 1e-12);
    assert!(matches!(eg[0].get("gevp_vector"), Some(Value::Vector(v)) if v.len() == 2));

    // refusals, by name
    let err = run(&mut e, "EFFECTIVE_MASS glue ON p, q;").unwrap_err();
    assert!(err.contains("GEVP"), "{err}");
    let err = run(&mut e, "CORRELATOR glue ON p, q;").unwrap_err();
    assert!(err.contains("exactly one field"), "{err}");
    let err = run(&mut e, "CORRELATOR missing ON p;").unwrap_err();
    assert!(err.contains("not found"), "{err}");
}

#[test]
fn noise_refuses_naming_t_and_incomplete_grids_refuse() {
    let dir = tempfile::tempdir().unwrap();
    let mut rng = Lcg(9);
    let (l, n) = (12usize, 400usize);
    let a: Vec<Vec<f64>> = (0..n).map(|_| (0..l).map(|_| 1.0 + rng.gauss()).collect()).collect();
    let mut e = engine_with(dir.path(), &a, &a);
    let err = run(&mut e, "EFFECTIVE_MASS glue ON p BLOCK 20;").unwrap_err();
    assert!(err.contains("no plateau") && err.contains("at t ="), "{err}");

    // a missing (cfg, t) slot is refused, not filled
    let dir2 = tempfile::tempdir().unwrap();
    let mut e2 = Engine::open(dir2.path()).unwrap();
    run(&mut e2, "BUNDLE g2 BASE (id NUMERIC) FIBER (cfg NUMERIC, t NUMERIC, p NUMERIC);").unwrap();
    for (id, c, t) in [(0, 0, 0), (1, 0, 1), (2, 0, 2), (3, 1, 0), (4, 1, 2)] {
        run(&mut e2, &format!("SECTION g2 (id={id}, cfg={c}, t={t}, p=1.5);")).unwrap();
    }
    let err = run(&mut e2, "CORRELATOR g2 ON p;").unwrap_err();
    assert!(err.contains("exactly once"), "{err}");
}
