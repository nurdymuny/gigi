//! Causal States CV2 — update operator framework.
//!
//! Port of the Python `even_U0` / `even_U1` / `hmm_update` (paper §5.1–5.2,
//! §6.1–6.2) into the Rust `causal_states` module. The framework is the
//! `UpdateOperator` trait + two concrete operators:
//!   - `EvenU0`, `EvenU1` — Even Process (sofic regime)
//!   - `HmmUpdate { alpha, beta, symbol }` — noisy 2-state HMM (smooth regime)
//!
//! CV3 then takes a pair of these and computes Ω = U_a∘U_b − U_b∘U_a in TV
//! / Hellinger / KL via the CV1 diagnostics.
//!
//! Math claims this test file pins:
//!
//! Even Process (paper §5):
//!   - E3: U_0(p) = (1, 0) for every p with p_0 > 0.
//!   - E5: U_0(0, 1) is inadmissible (returns Err).
//!   - E4: U_1 closed form on the 4 orbit points.
//!   - E6: orbit under {U_0, U_1} from μ = (2/3, 1/3) is exactly 4 points.
//!   - E9: TV of Ω at interior beliefs saturates at 1.
//!
//! Noisy 2-state HMM (paper §6):
//!   - H1: a = 1 − α − β + 2αβ matches U_0(μ)_0.
//!   - H5: reference numerical point at (α, β) = (0.2, 0.3), μ = (.5, .5):
//!         r = U_{01}(μ)_0 ≈ 0.4469.
//!   - H6: closed-form Eq 6.4 matches direct U_{01} − U_{10} TV in
//!         smooth regime.
//!
//! Substrate discipline:
//!   - Trait `UpdateOperator: apply(&self, &[f64]) -> Result<Vec<f64>, _>`.
//!     So CV3's commutator can take `&[Box<dyn UpdateOperator>]` and the
//!     same machinery serves Even, HMM, and anything Marcella plugs in.
//!   - Errors are typed (Inadmissible / ZeroNorm / UnknownSymbol).

#![cfg(feature = "causal_states")]

use gigi::causal_states::{
    even_update_word, hmm_closed_form_tv, hmm_update_word, tv, EvenU0, EvenU1, HmmUpdate,
    UpdateError, UpdateOperator,
};

const EPS_TIGHT: f64 = 1e-12;
const EPS_NUMERIC: f64 = 1e-4;

// ─── Even Process U_0 (paper Eq 5.3) ─────────────────────────────────────

#[test]
fn cv2_even_u0_collapses_interior_to_corner() {
    // E3: every belief with p_0 > 0 → (1, 0).
    let interiors = [
        vec![2.0 / 3.0, 1.0 / 3.0],
        vec![0.5, 0.5],
        vec![0.9, 0.1],
        vec![0.01, 0.99],
        vec![1.0, 0.0], // also collapses to itself
    ];
    for p in &interiors {
        let out = EvenU0.apply(p).expect("admissible");
        assert!(
            (out[0] - 1.0).abs() < EPS_TIGHT && (out[1] - 0.0).abs() < EPS_TIGHT,
            "U_0({p:?}) = {out:?}, expected (1, 0)"
        );
    }
}

#[test]
fn cv2_even_u0_inadmissible_at_corner() {
    // E5: U_0(0, 1) raises.
    let p = vec![0.0, 1.0];
    let r = EvenU0.apply(&p);
    assert!(
        matches!(r, Err(UpdateError::Inadmissible(_))),
        "U_0(0, 1) should be inadmissible, got {r:?}"
    );
}

// ─── Even Process U_1 (paper Eq 5.4) ─────────────────────────────────────

#[test]
fn cv2_even_u1_closed_form_on_orbit() {
    // E4: U_1 on the four reachable orbit points (paper §5.3).
    let cases: &[(Vec<f64>, Vec<f64>)] = &[
        (vec![2.0 / 3.0, 1.0 / 3.0], vec![0.5, 0.5]),
        (vec![0.5, 0.5], vec![2.0 / 3.0, 1.0 / 3.0]),
        (vec![1.0, 0.0], vec![0.0, 1.0]),
        (vec![0.0, 1.0], vec![1.0, 0.0]),
    ];
    for (inp, expected) in cases {
        let out = EvenU1.apply(inp).expect("U_1 admissible everywhere on Δ");
        for (a, b) in out.iter().zip(expected.iter()) {
            assert!(
                (a - b).abs() < EPS_TIGHT,
                "U_1({inp:?}) = {out:?}, expected {expected:?}"
            );
        }
    }
}

// ─── Iterated update along an observation word ───────────────────────────

#[test]
fn cv2_even_word_01_at_mu() {
    // U_{01}(μ) = U_1(U_0(μ)) = U_1(1, 0) = (0, 1).
    let mu = vec![2.0 / 3.0, 1.0 / 3.0];
    let out = even_update_word(&mu, "01").expect("admissible");
    assert!((out[0] - 0.0).abs() < EPS_TIGHT && (out[1] - 1.0).abs() < EPS_TIGHT,
            "U_{{01}}(μ) = {out:?}, expected (0, 1)");
}

#[test]
fn cv2_even_word_10_at_mu() {
    // U_{10}(μ) = U_0(U_1(μ)) = U_0(0.5, 0.5) = (1, 0).
    let mu = vec![2.0 / 3.0, 1.0 / 3.0];
    let out = even_update_word(&mu, "10").expect("admissible");
    assert!((out[0] - 1.0).abs() < EPS_TIGHT && (out[1] - 0.0).abs() < EPS_TIGHT,
            "U_{{10}}(μ) = {out:?}, expected (1, 0)");
}

#[test]
fn cv2_even_commutator_tv_saturates_at_mu() {
    // E9: TV(U_{01}(μ), U_{10}(μ)) = 1 on the Even Process — sofic saturation.
    let mu = vec![2.0 / 3.0, 1.0 / 3.0];
    let u01 = even_update_word(&mu, "01").unwrap();
    let u10 = even_update_word(&mu, "10").unwrap();
    let d = tv(&u01, &u10);
    assert!((d - 1.0).abs() < EPS_TIGHT, "TV at μ = {d}, expected 1");
}

#[test]
fn cv2_even_unknown_symbol_errors() {
    let mu = vec![0.5, 0.5];
    let r = even_update_word(&mu, "02"); // '2' isn't a valid symbol
    assert!(
        matches!(r, Err(UpdateError::UnknownSymbol('2'))),
        "expected UnknownSymbol('2'), got {r:?}"
    );
}

// ─── Orbit-closure check — E6 ────────────────────────────────────────────

#[test]
fn cv2_even_orbit_closure_is_four_points() {
    // E6: orbit under admissible {U_0, U_1} from μ = exactly 4 points.
    use std::collections::HashSet;
    fn round_pt(p: &[f64]) -> (u64, u64) {
        // Round to 8 decimals for set membership.
        ((p[0] * 1e8).round() as u64, (p[1] * 1e8).round() as u64)
    }
    let mut seen: HashSet<(u64, u64)> = HashSet::new();
    let mut work: Vec<Vec<f64>> = vec![vec![2.0 / 3.0, 1.0 / 3.0]];
    while let Some(p) = work.pop() {
        let key = round_pt(&p);
        if !seen.insert(key) {
            continue;
        }
        for op in [&EvenU0 as &dyn UpdateOperator, &EvenU1 as &dyn UpdateOperator] {
            if let Ok(q) = op.apply(&p) {
                work.push(q);
            }
        }
    }
    let expected: HashSet<(u64, u64)> = [
        vec![2.0 / 3.0, 1.0 / 3.0],
        vec![0.5, 0.5],
        vec![1.0, 0.0],
        vec![0.0, 1.0],
    ]
    .iter()
    .map(|p| round_pt(p))
    .collect();
    assert_eq!(seen, expected, "Even Process orbit closure mismatch");
}

// ─── Noisy 2-state HMM (paper §6) ────────────────────────────────────────

#[test]
fn cv2_hmm_a_closed_form_at_mu() {
    // H1: U_0(μ)_0 = 1 - α - β + 2αβ on a small grid.
    let mu = vec![0.5, 0.5];
    let cases: &[(f64, f64)] = &[
        (0.1, 0.1),
        (0.2, 0.3),
        (0.3, 0.45),
        (0.05, 0.4),
    ];
    for (alpha, beta) in cases {
        let op = HmmUpdate {
            alpha: *alpha,
            beta: *beta,
            symbol: 0,
        };
        let out = op.apply(&mu).expect("admissible at interior");
        let a_formula = 1.0 - *alpha - *beta + 2.0 * *alpha * *beta;
        assert!(
            (out[0] - a_formula).abs() < EPS_TIGHT,
            "a mismatch at (α,β)=({alpha},{beta}): U_0(μ)_0 = {}, formula = {a_formula}",
            out[0]
        );
    }
}

#[test]
fn cv2_hmm_reference_numerical_point() {
    // H5: at (α, β) = (0.2, 0.3) and μ = (.5, .5),
    //     r = U_{01}(μ)_0 ≈ 0.4469.
    let mu = vec![0.5, 0.5];
    let alpha = 0.2;
    let beta = 0.3;
    let u01 = hmm_update_word(&mu, "01", alpha, beta).expect("admissible");
    assert!(
        (u01[0] - 0.4469).abs() < EPS_NUMERIC,
        "r at (0.2, 0.3) = {}, expected ≈ 0.4469",
        u01[0]
    );
}

#[test]
fn cv2_hmm_closed_form_tv_matches_direct() {
    // H6: paper Eq 6.4 closed form matches the direct U_{01} − U_{10} TV
    // across the smooth-regime grid (0.05, 0.45)².
    let mu = vec![0.5, 0.5];
    let grid: Vec<f64> = (0..9).map(|i| 0.05 + 0.05 * i as f64).collect();
    for &alpha in &grid {
        for &beta in &grid {
            let u01 = hmm_update_word(&mu, "01", alpha, beta).unwrap();
            let u10 = hmm_update_word(&mu, "10", alpha, beta).unwrap();
            let direct = tv(&u01, &u10);
            let closed = hmm_closed_form_tv(alpha, beta);
            assert!(
                (direct - closed).abs() < EPS_TIGHT,
                "closed-form mismatch at (α,β)=({alpha},{beta}): direct={direct}, closed={closed}"
            );
        }
    }
}

#[test]
fn cv2_hmm_unknown_symbol_errors() {
    let op = HmmUpdate {
        alpha: 0.2,
        beta: 0.3,
        symbol: 7, // invalid
    };
    let mu = vec![0.5, 0.5];
    let r = op.apply(&mu);
    assert!(
        matches!(r, Err(UpdateError::Inadmissible(_)) | Err(UpdateError::UnknownSymbol(_))),
        "expected an error for symbol=7, got {r:?}"
    );
}

// ─── Trait-object dispatch — CV3 will rely on this ───────────────────────

#[test]
fn cv2_trait_dispatch_works_for_both_families() {
    // CV3's commutator orchestrator takes &[Box<dyn UpdateOperator>] —
    // verify EvenU0/EvenU1 and HmmUpdate are all boxable and callable
    // through the same dyn interface.
    let mu = vec![0.5, 0.5];
    let ops: Vec<Box<dyn UpdateOperator>> = vec![
        Box::new(EvenU1),
        Box::new(HmmUpdate {
            alpha: 0.2,
            beta: 0.3,
            symbol: 0,
        }),
    ];
    for op in &ops {
        let _ = op.apply(&mu).expect("both admissible at μ");
    }
}

// ─── Domain swap (§8 GP discipline) ──────────────────────────────────────

#[test]
fn cv2_ds_hmm_numerical_identical_across_call_sites() {
    // HMM update is API-blind to field name — three identical call sites
    // produce bit-identical numerical results.
    let mu = vec![0.5, 0.5];
    let op = HmmUpdate {
        alpha: 0.2,
        beta: 0.3,
        symbol: 0,
    };
    let r0 = op.apply(&mu).unwrap();
    let r1 = op.apply(&mu).unwrap();
    let r2 = op.apply(&mu).unwrap();
    assert_eq!(r0, r1);
    assert_eq!(r1, r2);
}
