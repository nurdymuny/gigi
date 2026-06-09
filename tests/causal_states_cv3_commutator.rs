//! Causal States CV3 — commutator orchestrator.
//!
//! Given an ordered pair of update operators `(a, b)` and a base belief
//! `p ∈ Δ(S)`, compute:
//!
//!   forward  = b.apply(a.apply(p))   (observation order "ab", right-acting)
//!   backward = a.apply(b.apply(p))   (observation order "ba")
//!   Ω        = (forward, backward) with TV / Hellinger / KL diagnostics
//!
//! This is the core load-bearing primitive of the paper: every regime
//! classifier, scatter plot, and ε-machine identification proof reduces
//! to "what is the magnitude of Ω at this base belief?"
//!
//! Math claims this test file pins:
//!
//! Sofic regime (Even Process):
//!   - @ μ = (2/3, 1/3) the pair (U_0, U_1) saturates:
//!     TV = 1, Hellinger = 1, KL = Divergent.
//!   - @ (1, 0) the (U_0, U_1) pair raises PathInadmissible.
//!
//! Smooth regime (noisy 2-state HMM):
//!   - @ μ = (.5, .5), (α, β) = (0.2, 0.3):
//!     TV ≈ 0.1062, Hellinger ≈ 0.0752, KL ≈ 0.0327 bits (paper H5).
//!   - Closed-form Eq 6.4 sanity:
//!     commutator(HMM_0, HMM_1, μ).tv ≡ hmm_closed_form_tv(α, β)
//!     across the smooth-regime grid (paper H6).
//!
//! Trivial regime (iid Bernoulli, here encoded by HMM with α = 1/2):
//!   Pair commutes pointwise. TV = Hellinger = 0, KL = Finite(0).
//!   This is the "commutator vanishes on iid" claim that GIGI's
//!   empirical scaffolding has to recover for the paper's thesis.

#![cfg(feature = "causal_states")]

use gigi::causal_states::{
    commutator, hmm_closed_form_tv, Commutator, CommutatorError, EvenU0, EvenU1, HmmUpdate,
    KlValue, UpdateError, WhichPath,
};

const EPS_TIGHT: f64 = 1e-12;
const EPS_NUMERIC: f64 = 1e-4;

// ─── Sofic regime: Even Process saturates ────────────────────────────────

#[test]
fn cv3_even_process_at_mu_saturates() {
    // forward  = U_1(U_0(μ)) = U_1(1, 0) = (0, 1)
    // backward = U_0(U_1(μ)) = U_0(0.5, 0.5) = (1, 0)
    let mu = vec![2.0 / 3.0, 1.0 / 3.0];
    let omega = commutator(&EvenU0, &EvenU1, &mu).expect("admissible at μ");

    assert!(
        (omega.forward[0] - 0.0).abs() < EPS_TIGHT
            && (omega.forward[1] - 1.0).abs() < EPS_TIGHT,
        "forward = {:?}, expected (0, 1)",
        omega.forward
    );
    assert!(
        (omega.backward[0] - 1.0).abs() < EPS_TIGHT
            && (omega.backward[1] - 0.0).abs() < EPS_TIGHT,
        "backward = {:?}, expected (1, 0)",
        omega.backward
    );

    assert!((omega.tv - 1.0).abs() < EPS_TIGHT, "TV = {}, expected 1", omega.tv);
    assert!((omega.hellinger - 1.0).abs() < EPS_TIGHT, "Hel = {}, expected 1", omega.hellinger);
    assert!(matches!(omega.kl, KlValue::Divergent),
            "KL should be Divergent in sofic regime, got {:?}", omega.kl);
}

#[test]
fn cv3_even_process_at_corner_raises() {
    // @ (1, 0): U_1(1,0) = (0,1), then U_0(0,1) is inadmissible.
    // forward path:  U_1(U_0(1,0)) — U_0 is admissible here, returns (1,0); then U_1(1,0)=(0,1). OK.
    // backward path: U_0(U_1(1,0)) — U_1(1,0)=(0,1), then U_0(0,1) is INADMISSIBLE.
    // So backward should raise PathInadmissible.
    let p = vec![1.0, 0.0];
    let r = commutator(&EvenU0, &EvenU1, &p);
    assert!(
        matches!(
            r,
            Err(CommutatorError::PathInadmissible {
                which: WhichPath::Backward,
                error: UpdateError::Inadmissible(_),
            })
        ),
        "@ (1, 0) the backward path U_0(U_1(.)) should be inadmissible, got {r:?}"
    );
}

// ─── Smooth regime: noisy 2-state HMM ────────────────────────────────────

#[test]
fn cv3_hmm_reference_numerical_point() {
    // Paper H5: at μ = (.5, .5), (α, β) = (0.2, 0.3):
    //   TV ≈ 0.1062, Hel ≈ 0.0752, KL ≈ 0.0327 bits.
    let mu = vec![0.5, 0.5];
    let u_0 = HmmUpdate { alpha: 0.2, beta: 0.3, symbol: 0 };
    let u_1 = HmmUpdate { alpha: 0.2, beta: 0.3, symbol: 1 };

    let omega = commutator(&u_0, &u_1, &mu).expect("admissible at interior");

    assert!(
        (omega.tv - 0.1062).abs() < EPS_NUMERIC,
        "HMM TV @ (0.2, 0.3) = {}, expected ≈ 0.1062",
        omega.tv
    );
    assert!(
        (omega.hellinger - 0.0752).abs() < EPS_NUMERIC,
        "HMM Hel @ (0.2, 0.3) = {}, expected ≈ 0.0752",
        omega.hellinger
    );
    match omega.kl {
        KlValue::Finite(v) => assert!(
            (v - 0.0327).abs() < EPS_NUMERIC,
            "HMM KL @ (0.2, 0.3) = {v}, expected ≈ 0.0327"
        ),
        KlValue::Divergent => panic!("HMM KL should be finite in smooth regime"),
    }
}

#[test]
fn cv3_hmm_closed_form_tv_matches_commutator() {
    // Paper Eq 6.4: TV from the orchestrator must equal the closed form
    // across the smooth-regime grid.
    let mu = vec![0.5, 0.5];
    let grid: Vec<f64> = (0..9).map(|i| 0.05 + 0.05 * i as f64).collect();
    for &alpha in &grid {
        for &beta in &grid {
            let u_0 = HmmUpdate { alpha, beta, symbol: 0 };
            let u_1 = HmmUpdate { alpha, beta, symbol: 1 };
            let omega = commutator(&u_0, &u_1, &mu).unwrap();
            let closed = hmm_closed_form_tv(alpha, beta);
            assert!(
                (omega.tv - closed).abs() < EPS_TIGHT,
                "closed-form mismatch at (α,β)=({alpha},{beta}): orchestrator={}, closed={closed}",
                omega.tv
            );
        }
    }
}

// ─── Trivial regime: iid-like HMM (α = 1/2) commutes ─────────────────────

#[test]
fn cv3_hmm_at_alpha_half_is_iid_and_commutes() {
    // α = 1/2 → transition matrix M = [[.5, .5], [.5, .5]] is rank-one:
    // every belief lands on the stationary (.5, .5) after a single update.
    // Two updates from any p both end at (.5, .5), so the commutator
    // vanishes in all three diagnostics.
    let p = vec![0.7, 0.3]; // arbitrary base
    let u_0 = HmmUpdate { alpha: 0.5, beta: 0.3, symbol: 0 };
    let u_1 = HmmUpdate { alpha: 0.5, beta: 0.3, symbol: 1 };
    let omega = commutator(&u_0, &u_1, &p).unwrap();

    assert!(omega.tv.abs() < EPS_TIGHT, "iid TV should be 0, got {}", omega.tv);
    assert!(omega.hellinger.abs() < EPS_TIGHT,
            "iid Hellinger should be 0, got {}", omega.hellinger);
    match omega.kl {
        KlValue::Finite(v) => assert!(v.abs() < EPS_TIGHT,
                                       "iid KL should be 0, got {v}"),
        KlValue::Divergent => panic!("iid KL should be Finite(0), got Divergent"),
    }
}

#[test]
fn cv3_hmm_at_beta_half_uninformative_emissions_also_commute() {
    // β = 1/2 → emissions are uninformative (E_0 = E_1 = (.5, .5)):
    // U_0 = U_1 identically. Composition trivially commutes.
    let p = vec![0.4, 0.6];
    let u_0 = HmmUpdate { alpha: 0.3, beta: 0.5, symbol: 0 };
    let u_1 = HmmUpdate { alpha: 0.3, beta: 0.5, symbol: 1 };
    let omega = commutator(&u_0, &u_1, &p).unwrap();
    assert!(omega.tv.abs() < EPS_TIGHT, "β=1/2 TV should be 0, got {}", omega.tv);
}

// ─── Path-direction discrimination ───────────────────────────────────────

#[test]
fn cv3_swapping_operators_swaps_forward_backward() {
    // commutator(a, b, p) returns forward = b(a(p)), backward = a(b(p)).
    // commutator(b, a, p) should return forward = a(b(p)), backward = b(a(p)).
    // I.e. forward and backward swap.
    let mu = vec![2.0 / 3.0, 1.0 / 3.0];
    let omega_ab = commutator(&EvenU0, &EvenU1, &mu).unwrap();
    let omega_ba = commutator(&EvenU1, &EvenU0, &mu).unwrap();

    for i in 0..2 {
        assert!(
            (omega_ab.forward[i] - omega_ba.backward[i]).abs() < EPS_TIGHT,
            "swap: forward[{i}] vs backward[{i}]: {} vs {}",
            omega_ab.forward[i],
            omega_ba.backward[i]
        );
        assert!(
            (omega_ab.backward[i] - omega_ba.forward[i]).abs() < EPS_TIGHT,
            "swap: backward[{i}] vs forward[{i}]: {} vs {}",
            omega_ab.backward[i],
            omega_ba.forward[i]
        );
    }
    // TV / Hellinger are symmetric in their arguments → unchanged.
    assert!((omega_ab.tv - omega_ba.tv).abs() < EPS_TIGHT);
    assert!((omega_ab.hellinger - omega_ba.hellinger).abs() < EPS_TIGHT);
}

// ─── Forward-path failure also raises ────────────────────────────────────

#[test]
fn cv3_forward_path_inadmissibility_propagates_with_which_tag() {
    // Pair the iid-on-symbol-0 operator with EvenU0 at base (0, 1):
    // forward  = EvenU0(HmmUpdate_0(0, 1)) — HMM is OK, gives some interior
    //                                       belief, then U_0 may be OK because
    //                                       result has p_0 > 0. Skip this case.
    //
    // Simpler: pair (EvenU0, EvenU1) at base (0, 1).
    // forward  = U_1(U_0(0, 1)) — U_0(0, 1) is inadmissible. FORWARD fails.
    let p = vec![0.0, 1.0];
    let r = commutator(&EvenU0, &EvenU1, &p);
    assert!(
        matches!(
            r,
            Err(CommutatorError::PathInadmissible {
                which: WhichPath::Forward,
                error: UpdateError::Inadmissible(_),
            })
        ),
        "@ (0, 1) the forward path U_1(U_0(.)) should be inadmissible, got {r:?}"
    );
}

// ─── Returned struct shape audit ─────────────────────────────────────────

#[test]
fn cv3_commutator_struct_shape() {
    // Ensure the public fields are exactly what CV4's HTTP envelope will
    // serialize: forward (Vec<f64>), backward (Vec<f64>), tv (f64),
    // hellinger (f64), kl (KlValue).
    let mu = vec![0.5, 0.5];
    let u_0 = HmmUpdate { alpha: 0.2, beta: 0.3, symbol: 0 };
    let u_1 = HmmUpdate { alpha: 0.2, beta: 0.3, symbol: 1 };
    let omega: Commutator = commutator(&u_0, &u_1, &mu).unwrap();

    // Fields are accessible (compile-time guarantee).
    let _f: &Vec<f64> = &omega.forward;
    let _b: &Vec<f64> = &omega.backward;
    let _t: f64 = omega.tv;
    let _h: f64 = omega.hellinger;
    let _k: KlValue = omega.kl;
    // forward and backward are probability vectors (sum to 1).
    assert!((omega.forward.iter().sum::<f64>() - 1.0).abs() < EPS_TIGHT);
    assert!((omega.backward.iter().sum::<f64>() - 1.0).abs() < EPS_TIGHT);
}
