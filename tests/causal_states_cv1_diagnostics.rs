//! Causal States CV1 — scalar diagnostics (TV, Hellinger, KL).
//!
//! Port of the Python `diag_TV`, `diag_Hellinger`, `diag_KL` helpers in
//! `theory/causal_states/validation_tests.py` to Rust.
//!
//! Math target (paper §4 Def 4.1):
//!   - TV(p, q)        = ½ Σ |p_i - q_i|
//!   - Hellinger(p, q) = (1/√2) ‖√p - √q‖₂
//!   - KL(p ‖ q)       = Σ p_i log₂(p_i / q_i)   (or +∞ if q_i = 0 where p_i > 0)
//!
//! Reference numerical point (paper §6.3, validated by Python H5):
//!   At (α, β) = (0.2, 0.3), p = (0.4469, 0.5531), q = (0.5531, 0.4469):
//!     TV  ≈ 0.1062
//!     Hel ≈ 0.0752
//!     KL  ≈ 0.0327 bits

#![cfg(feature = "causal_states")]

use gigi::causal_states::{tv, hellinger, kl, KlValue};

const EPS_TIGHT: f64 = 1e-12;
const EPS_NUMERIC: f64 = 1e-4;

// ─── TV ──────────────────────────────────────────────────────────────────

#[test]
fn cv1_tv_identical_distributions_is_zero() {
    let p = vec![0.4, 0.6];
    assert!((tv(&p, &p) - 0.0).abs() < EPS_TIGHT);
}

#[test]
fn cv1_tv_singular_corners_is_one() {
    // (1, 0) vs (0, 1) — paper Even Process saturating regime.
    let p = vec![1.0, 0.0];
    let q = vec![0.0, 1.0];
    assert!((tv(&p, &q) - 1.0).abs() < EPS_TIGHT);
}

#[test]
fn cv1_tv_reference_numerical_point() {
    // Paper §6.3 reference point, validated by Python H5.
    let p = vec![0.4469, 0.5531];
    let q = vec![0.5531, 0.4469];
    let result = tv(&p, &q);
    assert!(
        (result - 0.1062).abs() < EPS_NUMERIC,
        "TV at HMM reference point: got {result}, expected ≈ 0.1062"
    );
}

#[test]
fn cv1_tv_symmetric() {
    let p = vec![0.7, 0.3];
    let q = vec![0.2, 0.8];
    assert!((tv(&p, &q) - tv(&q, &p)).abs() < EPS_TIGHT);
}

// ─── Hellinger ───────────────────────────────────────────────────────────

#[test]
fn cv1_hellinger_identical_is_zero() {
    let p = vec![0.4, 0.6];
    assert!((hellinger(&p, &p) - 0.0).abs() < EPS_TIGHT);
}

#[test]
fn cv1_hellinger_singular_corners_is_one() {
    let p = vec![1.0, 0.0];
    let q = vec![0.0, 1.0];
    assert!((hellinger(&p, &q) - 1.0).abs() < EPS_TIGHT);
}

#[test]
fn cv1_hellinger_reference_numerical_point() {
    let p = vec![0.4469, 0.5531];
    let q = vec![0.5531, 0.4469];
    let result = hellinger(&p, &q);
    assert!(
        (result - 0.0752).abs() < EPS_NUMERIC,
        "Hellinger at HMM reference point: got {result}, expected ≈ 0.0752"
    );
}

#[test]
fn cv1_hellinger_bounded_above_by_one() {
    let p = vec![0.99, 0.01];
    let q = vec![0.01, 0.99];
    assert!(hellinger(&p, &q) <= 1.0 + EPS_TIGHT);
}

// ─── KL ──────────────────────────────────────────────────────────────────

#[test]
fn cv1_kl_identical_is_zero() {
    let p = vec![0.4, 0.6];
    let r = kl(&p, &p);
    match r {
        KlValue::Finite(v) => assert!(v.abs() < EPS_TIGHT, "KL(p||p) should be 0, got {v}"),
        KlValue::Divergent => panic!("KL(p||p) should be finite zero, got Divergent"),
    }
}

#[test]
fn cv1_kl_singular_corners_diverges() {
    // (1, 0) vs (0, 1) — mutually singular point masses, paper E11.
    let p = vec![1.0, 0.0];
    let q = vec![0.0, 1.0];
    let r = kl(&p, &q);
    assert!(matches!(r, KlValue::Divergent),
            "KL of mutually singular distributions must be Divergent, got {r:?}");
}

#[test]
fn cv1_kl_reference_numerical_point() {
    let p = vec![0.4469, 0.5531];
    let q = vec![0.5531, 0.4469];
    match kl(&p, &q) {
        KlValue::Finite(v) => {
            assert!(
                (v - 0.0327).abs() < EPS_NUMERIC,
                "KL at HMM reference point: got {v}, expected ≈ 0.0327"
            );
        }
        KlValue::Divergent => panic!("KL should be finite at interior-support point"),
    }
}

#[test]
fn cv1_kl_zero_in_p_skipped_safely() {
    // p_i = 0 contributes nothing (0 * log 0 := 0 by convention).
    let p = vec![1.0, 0.0];
    let q = vec![0.5, 0.5];
    match kl(&p, &q) {
        KlValue::Finite(v) => {
            // KL((1,0) || (0.5, 0.5)) = 1 * log2(1/0.5) + 0 * (anything) = 1
            assert!((v - 1.0).abs() < EPS_TIGHT, "KL((1,0)||(.5,.5)) = {v}, expected 1.0");
        }
        KlValue::Divergent => panic!("KL should be finite when p_i=0 happens where q_i>0"),
    }
}

// ─── Domain swap (§8 GP discipline) ──────────────────────────────────────
//
// Same math, three different "field families" — the substrate is
// domain-agnostic. Numerical equality across cases proves it.

#[test]
fn cv1_ds_tv_identical_across_domain_names() {
    // The diagnostic just sees vectors of probabilities — it has no field
    // names. So domain swap is trivially identity by API. Verify by
    // computing the same TV three times against three vector pairs that
    // are bit-identical, simulating three "field name" call sites.
    let pairs = [
        (vec![0.4469, 0.5531], vec![0.5531, 0.4469]),  // vuln-hunt
        (vec![0.4469, 0.5531], vec![0.5531, 0.4469]),  // fraud
        (vec![0.4469, 0.5531], vec![0.5531, 0.4469]),  // education
    ];
    let results: Vec<f64> = pairs.iter().map(|(p, q)| tv(p, q)).collect();
    assert!(results.windows(2).all(|w| (w[0] - w[1]).abs() < EPS_TIGHT));
}
