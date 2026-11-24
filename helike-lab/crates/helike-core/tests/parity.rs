//! Parity with the Python reference (oracle/oracle_m0.json, written by oracle/make_oracle.py), and mutation checks:
//! every deliberate fault in the builder must break parity.

use helike_core::helike::{self, Member, SectorData, Spectrum};
use helike_core::lie::LieAlgebra;
use helike_core::linalg::{self, CMat, C64};
use helike_core::rep::Rep;
use helike_core::sector::{Field, Geometry, Mutation, Sector};
use serde_json::Value;
use std::sync::OnceLock;

fn oracle() -> &'static Value {
    static O: OnceLock<Value> = OnceLock::new();
    O.get_or_init(|| {
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../oracle/oracle_m0.json");
        serde_json::from_str(&std::fs::read_to_string(p).expect("run oracle/make_oracle.py first")).unwrap()
    })
}

fn mat(v: &Value) -> CMat {
    let (r, c) = (v["rows"].as_u64().unwrap() as usize, v["cols"].as_u64().unwrap() as usize);
    let re: Vec<f64> = v["re"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
    let im: Vec<f64> = v["im"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
    faer::Mat::from_fn(r, c, |i, j| C64::new(re[i * c + j], im[i * c + j]))
}

fn vecf(v: &Value) -> Vec<f64> {
    v.as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect()
}

fn f(v: &Value, k: &str) -> f64 {
    v[k].as_f64().unwrap()
}

fn max_diff(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(a.len(), b.len(), "spectrum lengths differ");
    a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0, f64::max)
}

fn su2_hg(p: &Value, mutation: Mutation) -> (CMat, CMat, Option<CMat>) {
    let m = Member::new(f(p, "x"), f(p, "y"), f(p, "t"));
    let (geo, field, rep) = (m.geometry(), m.field(), Rep::spin(p["j"].as_u64().unwrap() as usize));
    let sec = Sector::new(&geo, &field, &rep);
    let hg = sec.hessian_and_gauge_mutated(mutation);
    (hg.h, hg.g, sec.curl())
}

#[test]
fn su2_family_h_g_curl_entrywise() {
    let mut worst = (0.0f64, 0.0f64, 0.0f64);
    for p in oracle()["su2_points"].as_array().unwrap() {
        let (h, g, curl) = su2_hg(p, Mutation::None);
        worst.0 = worst.0.max(linalg::max_abs_diff(&h, &mat(&p["H"])));
        worst.1 = worst.1.max(linalg::max_abs_diff(&g, &mat(&p["G"])));
        worst.2 = worst.2.max(linalg::max_abs_diff(&curl.unwrap(), &mat(&p["curl"])));
    }
    println!("su2 entrywise: H {:.2e}, G {:.2e}, curl {:.2e}", worst.0, worst.1, worst.2);
    assert!(worst.0 <= 1e-12 && worst.1 <= 1e-12 && worst.2 <= 1e-12, "{worst:?}");
}

#[test]
fn su2_spectra_and_slice() {
    let mut worst = (0.0f64, 0.0f64);
    for p in oracle()["su2_points"].as_array().unwrap() {
        let m = Member::new(f(p, "x"), f(p, "y"), f(p, "t"));
        let j = p["j"].as_u64().unwrap() as usize;
        let sd = SectorData::new(&m, j);
        worst.0 = worst.0.max(max_diff(&sd.hg.spectrum(), &vecf(&p["spectrum"])));
        worst.1 = worst.1.max(max_diff(&sd.hg.slice_spectrum(), &vecf(&p["slice"])));
        // blockwise slice spectrum equals the dense one
        let blockwise = helike_core::charge::flatten(&sd.split.slice_block_spectra(&sd.hg));
        worst.1 = worst.1.max(max_diff(&blockwise, &vecf(&p["slice"])));
    }
    println!("su2 spectra {:.2e}, slice {:.2e}", worst.0, worst.1);
    assert!(worst.0 <= 1e-10 && worst.1 <= 1e-10, "{worst:?}");
}

#[test]
fn su2_flux_matrix_and_ladder_closed_forms() {
    for p in oracle()["su2_points"].as_array().unwrap() {
        let m = Member::new(f(p, "x"), f(p, "y"), f(p, "t"));
        let j = p["j"].as_u64().unwrap() as usize;
        let (geo, field, rep) = (m.geometry(), m.field(), Rep::spin(j));
        let sec = Sector::new(&geo, &field, &rep);
        let hg = sec.hessian_and_gauge();
        let curl = sec.curl().unwrap();
        // H = curl^2 + Phi
        let phi_py = mat(&p["Phi"]);
        let resid = linalg::max_abs_diff(&hg.h, &(&curl * &curl + &phi_py));
        assert!(resid < 1e-11, "H = curl^2 + Phi: {resid:e}");
        // Phi = phi (x) I with the closed-form spectrum
        let mut want: Vec<f64> = m.flux_spectrum().iter().flat_map(|v| std::iter::repeat(*v).take(j + 1)).collect();
        want.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!(max_diff(&linalg::eigvalsh(&phi_py), &want) < 1e-11);
        // the twist is exact: its closed form is an eigenvalue with multiplicity >= 2
        let lam = m.twist(j);
        let n_at = hg.spectrum().iter().filter(|v| (*v - lam).abs() < 1e-9 * lam.abs().max(1.0)).count();
        assert!(n_at >= 2, "twist multiplicity {n_at}");
    }
}

#[test]
fn su3_family_2b_entrywise_and_spectra() {
    let alg = LieAlgebra::su_n(3);
    let mut worst = (0.0f64, 0.0f64, 0.0f64);
    for p in oracle()["su3_points"].as_array().unwrap() {
        let t = f(p, "t");
        let mut g = vec![1.0; 8];
        g[7] = t;
        let geo = Geometry::new(alg.clone(), g.clone());
        let m: Vec<Vec<f64>> = (0..8).map(|a| (0..8).map(|b| if a == b { 0.5 / g[a].sqrt() } else { 0.0 }).collect()).collect();
        let field = Field::new(alg.clone(), m);
        for (key, rec) in p["sectors"].as_object().unwrap() {
            let (pp, qq): (usize, usize) = {
                let v: Vec<usize> = key.split(',').map(|s| s.parse().unwrap()).collect();
                (v[0], v[1])
            };
            let rep = Rep::su_n_pq(3, pp, qq);
            let hg = Sector::new(&geo, &field, &rep).hessian_and_gauge();
            if rec.get("H").is_some() {
                // The irrep basis is fixed by the same construction (highest weight, simple lowerings, Gram-Schmidt),
                // so H and G agree entrywise, not only up to a unitary.
                worst.0 = worst.0.max(linalg::max_abs_diff(&hg.h, &mat(&rec["H"])));
                worst.0 = worst.0.max(linalg::max_abs_diff(&hg.g, &mat(&rec["G"])));
            }
            worst.1 = worst.1.max(max_diff(&hg.spectrum(), &vecf(&rec["spectrum"])));
            worst.2 = worst.2.max(max_diff(&hg.slice_spectrum(), &vecf(&rec["slice"])));
        }
    }
    println!("su3: entrywise {:.2e}, spectrum {:.2e}, slice {:.2e}", worst.0, worst.1, worst.2);
    assert!(worst.0 <= 1e-12 && worst.1 <= 1e-10 && worst.2 <= 1e-10, "{worst:?}");
}

#[test]
fn phi_a_and_phi_s_match_python() {
    for p in oracle()["phi_points"].as_array().unwrap() {
        let m = Member::new(f(p, "x"), f(p, "y"), f(p, "t"));
        let jmax = p["jmax"].as_u64().unwrap() as usize;
        let (va, ja, qa) = helike::phi_a(&m, jmax);
        let (vs, js) = helike::phi_s(&m, jmax);
        let arg_a = p["arg_A"].as_array().unwrap();
        println!("({:.3},{:.3},{:.3}) phi_A {va:.12} vs {:.12} at ({ja},{qa}); phi_S {vs:.12} vs {:.12} at {js}",
                 m.x, m.y, m.t, f(p, "phi_A"), f(p, "phi_S"));
        assert!((va - f(p, "phi_A")).abs() < 1e-10, "phi_A");
        assert!((vs - f(p, "phi_S")).abs() < 1e-10, "phi_S");
        assert_eq!(ja, arg_a[0].as_u64().unwrap() as usize, "phi_A sector");
        // the minimising charge is decided up to the mirror q <-> -q, which ties exactly
        assert!((qa.abs() - arg_a[1].as_f64().unwrap().abs()).abs() < 1e-9, "phi_A charge up to mirror");
        assert_eq!(js, p["arg_S"].as_u64().unwrap() as usize, "phi_S sector");
    }
}

#[test]
fn table_edges_match_python() {
    for e in oracle()["edges"].as_array().unwrap() {
        let (t, y) = (f(e, "t"), f(e, "y"));
        let jmax = e["jmax"].as_u64().unwrap() as usize;
        let n = e["n"].as_u64().unwrap() as usize;
        let xa = helike::edge(Spectrum::A, y, t, jmax, f(e, "lo"), f(e, "hi"), n).unwrap();
        let xs = helike::edge(Spectrum::S, y, t, jmax, f(e, "lo"), f(e, "hi"), n).unwrap();
        println!("t={t} y={y:.4}: x*_A {xa:.12} vs {:.12}; x*_S {xs:.12} vs {:.12}", f(e, "x_star_A"), f(e, "x_star_S"));
        assert!((xa - f(e, "x_star_A")).abs() < 1e-10);
        assert!((xs - f(e, "x_star_S")).abs() < 1e-10);
    }
}

#[test]
fn berger_crossings() {
    // Along the Berger path the A-edge is crossed at t_A and the S-edge at t* (Floor paper, By Enough).
    let ta = helike_core::roots::brent(|t| helike::phi(&Member::berger(t), Spectrum::A, 12), 2.70, 2.76, 1e-13, 200).unwrap();
    let ts = helike_core::roots::brent(|t| helike::phi(&Member::berger(t), Spectrum::S, 12), 2.68, 2.72, 1e-13, 200).unwrap();
    println!("t_A {ta:.16}, t* {ts:.16}");
    assert!((ta - 2.731_122_278_238_113_6).abs() < 1e-10, "t_A");
    assert!((ts - 2.698_026_940_520_026_3).abs() < 1e-9, "t*");
}

#[test]
fn mutations_break_parity() {
    let pts = oracle()["su2_points"].as_array().unwrap();
    for mutation in [Mutation::FlipFlux, Mutation::DropFrameBracket, Mutation::FlatMetric, Mutation::FluxLowerUntransposed] {
        let worst = pts
            .iter()
            .map(|p| {
                let (h, g, _) = su2_hg(p, mutation);
                linalg::max_abs_diff(&h, &mat(&p["H"])).max(linalg::max_abs_diff(&g, &mat(&p["G"])))
            })
            .fold(0.0f64, f64::max);
        println!("{mutation:?}: worst deviation {worst:.3e}");
        assert!(worst > 1e-3, "mutation {mutation:?} was not caught");
    }
}

#[test]
fn charge_split_is_exact_on_the_family() {
    for p in oracle()["su2_points"].as_array().unwrap() {
        let m = Member::new(f(p, "x"), f(p, "y"), f(p, "t"));
        let j = p["j"].as_u64().unwrap() as usize;
        let sd = SectorData::new(&m, j);
        assert!(sd.split.commutator_residual < 1e-11 && sd.split.intertwiner_residual < 1e-11);
        // largest charge is l + 2, a 1 x 1 block carrying the twist
        let qmax = sd.split.charges().into_iter().fold(f64::MIN, f64::max);
        assert!((qmax - (j as f64 / 2.0 + 2.0)).abs() < 1e-9);
        let blocks = sd.split.block_spectra(&sd.hg.h_plus_ggstar());
        let top = blocks.iter().next_back().unwrap().1;
        assert_eq!(top.len(), 1);
        assert!((top[0] - m.twist(j)).abs() < 1e-10);
        // every block has at most nine states
        assert!(sd.split.field_bases.values().all(|b| b.ncols() <= 9));
    }
}
