//! Phase 4.1 — Property-based invariants.
//!
//! Universal inequalities the substrate must satisfy on every input
//! drawn from `Δ²`, not just the paper's named numerical anchors.
//!
//! ## Diagnostic inequalities (any p, q ∈ Δ²)
//!
//!   - 0 ≤ TV(p, q) ≤ 1
//!   - 0 ≤ Hellinger(p, q) ≤ 1
//!   - Hellinger² ≤ TV         (Le Cam lower bound)
//!   - TV ≤ √2 · Hellinger     (Le Cam upper bound)
//!   - KL_bits ≥ 2 · TV² / ln 2  (Pinsker, in bits)
//!
//! ## Operator invariants
//!
//!   - commutator(U, U, p) ≡ (0, 0, Finite(0))
//!     (self-commutator must vanish exactly).
//!   - TV(commutator(a, b, p)) ≡ TV(commutator(b, a, p))
//!     (commutator diagnostics are symmetric in the operator pair).
//!   - Hellinger(commutator(a, b, p)) ≡ Hellinger(commutator(b, a, p))
//!
//! ## Classifier invariants
//!
//!   - sofic → never reclassifies to smooth under any band tightening.
//!   - smooth → never reclassifies to sofic under any band tightening.
//!     (monotonicity in the bands).
//!
//! Test inputs are deterministically generated from a linear-congruential
//! seed — `Date.now()`/`rand` would break reproducibility.

#![cfg(feature = "causal_states")]

use gigi::causal_states::{
    classify_regime, commutator, hellinger, hmm_closed_form_tv, kl, tv, Commutator, EvenU0,
    EvenU1, HmmUpdate, KlValue, Regime, RegimeBands,
};

const EPS_TIGHT: f64 = 1e-12;
const EPS_LOOSE: f64 = 1e-9;
const LN2: f64 = std::f64::consts::LN_2;

/// Deterministic LCG — same seed → same sequence, reproducible across runs.
struct Lcg(u64);
impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }
    fn next_u64(&mut self) -> u64 {
        // Numerical Recipes LCG constants.
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0
    }
    /// Uniform in `(0, 1)` — avoids exact 0 and 1 to keep distributions interior.
    fn next_f64(&mut self) -> f64 {
        let u = self.next_u64();
        // 53-bit mantissa, shift to (0, 1) and clamp away from boundary.
        let raw = ((u >> 11) as f64) / ((1u64 << 53) as f64);
        // raw ∈ [0, 1); map to (eps, 1 - eps).
        let eps = 1e-9;
        eps + raw * (1.0 - 2.0 * eps)
    }
    /// Random 2-state probability vector with both entries ≥ `eps`.
    fn next_belief(&mut self, eps: f64) -> Vec<f64> {
        let mut p0 = self.next_f64();
        // Clamp so both entries are ≥ eps.
        if p0 < eps {
            p0 = eps;
        }
        if p0 > 1.0 - eps {
            p0 = 1.0 - eps;
        }
        vec![p0, 1.0 - p0]
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §A — Diagnostic inequalities
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn p4_1_tv_in_unit_interval() {
    let mut rng = Lcg::new(0xc0ffee);
    for _ in 0..200 {
        let p = rng.next_belief(1e-6);
        let q = rng.next_belief(1e-6);
        let d = tv(&p, &q);
        assert!(d >= 0.0 && d <= 1.0 + EPS_TIGHT, "TV={d} out of [0, 1]");
    }
}

#[test]
fn p4_1_hellinger_in_unit_interval() {
    let mut rng = Lcg::new(0xdeadbeef);
    for _ in 0..200 {
        let p = rng.next_belief(1e-6);
        let q = rng.next_belief(1e-6);
        let h = hellinger(&p, &q);
        assert!(h >= 0.0 && h <= 1.0 + EPS_TIGHT, "Hellinger={h} out of [0, 1]");
    }
}

#[test]
fn p4_1_le_cam_lower_bound_hsquared_le_tv() {
    // For any p, q on the simplex: Hellinger² ≤ TV.
    let mut rng = Lcg::new(0xfeedface);
    for _ in 0..500 {
        let p = rng.next_belief(1e-6);
        let q = rng.next_belief(1e-6);
        let t = tv(&p, &q);
        let h = hellinger(&p, &q);
        assert!(
            h * h <= t + EPS_LOOSE,
            "Le Cam lower bound violated: H²={} > TV={t} (p={p:?}, q={q:?})",
            h * h
        );
    }
}

#[test]
fn p4_1_le_cam_upper_bound_tv_le_sqrt2_h() {
    // TV ≤ √2 · Hellinger.
    let mut rng = Lcg::new(0x1badb002);
    for _ in 0..500 {
        let p = rng.next_belief(1e-6);
        let q = rng.next_belief(1e-6);
        let t = tv(&p, &q);
        let h = hellinger(&p, &q);
        let bound = std::f64::consts::SQRT_2 * h;
        assert!(
            t <= bound + EPS_LOOSE,
            "Le Cam upper bound violated: TV={t} > √2·H={bound} (p={p:?}, q={q:?})",

        );
    }
}

#[test]
fn p4_1_pinsker_bits_inequality() {
    // KL_bits · ln 2 ≥ 2 · TV²  ⇔  KL_bits ≥ 2 · TV² / ln 2.
    // Holds when KL is finite (test only those samples).
    let mut rng = Lcg::new(0xcafebabe);
    let mut n_finite = 0;
    for _ in 0..500 {
        let p = rng.next_belief(1e-6);
        let q = rng.next_belief(1e-6);
        let t = tv(&p, &q);
        match kl(&p, &q) {
            KlValue::Finite(kl_bits) => {
                n_finite += 1;
                let lhs = kl_bits;
                let rhs = 2.0 * t * t / LN2;
                assert!(
                    lhs + EPS_LOOSE >= rhs,
                    "Pinsker violated: KL_bits={lhs} < 2·TV²/ln2={rhs} (TV={t}, p={p:?}, q={q:?})"
                );
            }
            KlValue::Divergent => {} // Pinsker trivial when KL = ∞
        }
    }
    assert!(n_finite > 400, "expected most samples to have finite KL; got {n_finite}/500");
}

#[test]
fn p4_1_kl_nonneg_when_finite() {
    // KL ≥ 0 (Gibbs' inequality).
    let mut rng = Lcg::new(0xbaadf00d);
    for _ in 0..500 {
        let p = rng.next_belief(1e-6);
        let q = rng.next_belief(1e-6);
        if let KlValue::Finite(v) = kl(&p, &q) {
            assert!(v >= -EPS_LOOSE, "KL = {v} < 0 violates Gibbs (p={p:?}, q={q:?})");
        }
    }
}

#[test]
fn p4_1_diagnostics_vanish_iff_equal() {
    // p == q → TV = Hellinger = 0, KL = Finite(0).
    let mut rng = Lcg::new(0x1234abcd);
    for _ in 0..50 {
        let p = rng.next_belief(1e-6);
        assert!(tv(&p, &p).abs() < EPS_TIGHT);
        assert!(hellinger(&p, &p).abs() < EPS_TIGHT);
        match kl(&p, &p) {
            KlValue::Finite(v) => assert!(v.abs() < EPS_TIGHT),
            KlValue::Divergent => panic!("KL(p||p) must be Finite(0), got Divergent"),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §B — Operator invariants
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn p4_1_self_commutator_vanishes() {
    // commutator(U, U, p) must give forward == backward; TV = Hel = 0;
    // KL = Finite(0).
    let mu = vec![0.5, 0.5];

    // Even Process U_1 (admissible everywhere).
    let omega = commutator(&EvenU1, &EvenU1, &mu).unwrap();
    assert!(omega.tv.abs() < EPS_TIGHT, "Even U_1 self: TV={}", omega.tv);
    assert!(omega.hellinger.abs() < EPS_TIGHT);
    assert!(matches!(omega.kl, KlValue::Finite(v) if v.abs() < EPS_TIGHT));

    // HMM at various (α, β).
    let mut rng = Lcg::new(0xdeed1ed);
    for _ in 0..30 {
        let alpha = 0.05 + rng.next_f64() * 0.4;
        let beta = 0.05 + rng.next_f64() * 0.4;
        let symbol = if rng.next_u64() % 2 == 0 { 0u8 } else { 1u8 };
        let op = HmmUpdate { alpha, beta, symbol };
        let omega = commutator(&op, &op, &mu).unwrap();
        assert!(omega.tv.abs() < EPS_TIGHT, "HMM self: TV={}", omega.tv);
        assert!(omega.hellinger.abs() < EPS_TIGHT);
        match omega.kl {
            KlValue::Finite(v) => assert!(v.abs() < EPS_TIGHT),
            KlValue::Divergent => panic!("HMM self-commutator KL must be Finite(0)"),
        }
    }
}

#[test]
fn p4_1_commutator_diagnostics_swap_symmetric() {
    // TV and Hellinger of the commutator are symmetric under operator
    // swap (because TV(a, b) = TV(b, a) and the two compositions trade
    // forward ↔ backward).
    let mut rng = Lcg::new(0x5eed1eaf);
    for _ in 0..50 {
        let alpha = 0.05 + rng.next_f64() * 0.4;
        let beta = 0.05 + rng.next_f64() * 0.4;
        let p = rng.next_belief(1e-3);
        let u_0 = HmmUpdate { alpha, beta, symbol: 0 };
        let u_1 = HmmUpdate { alpha, beta, symbol: 1 };
        let ab = commutator(&u_0, &u_1, &p).unwrap();
        let ba = commutator(&u_1, &u_0, &p).unwrap();
        assert!(
            (ab.tv - ba.tv).abs() < EPS_TIGHT,
            "TV asymmetric: ab={}, ba={}",
            ab.tv,
            ba.tv
        );
        assert!(
            (ab.hellinger - ba.hellinger).abs() < EPS_TIGHT,
            "Hellinger asymmetric: ab={}, ba={}",
            ab.hellinger,
            ba.hellinger
        );
        // forward(ab) == backward(ba).
        for i in 0..2 {
            assert!((ab.forward[i] - ba.backward[i]).abs() < EPS_TIGHT);
            assert!((ab.backward[i] - ba.forward[i]).abs() < EPS_TIGHT);
        }
    }
}

#[test]
fn p4_1_commutator_closed_form_matches_orchestrator_random() {
    // Eq 6.4 closed form ≡ orchestrator TV across random (α, β).
    let mut rng = Lcg::new(0xc1057ed);
    let mu = vec![0.5, 0.5];
    for _ in 0..200 {
        let alpha = 0.01 + rng.next_f64() * 0.48;
        let beta = 0.01 + rng.next_f64() * 0.48;
        let u_0 = HmmUpdate { alpha, beta, symbol: 0 };
        let u_1 = HmmUpdate { alpha, beta, symbol: 1 };
        let omega = commutator(&u_0, &u_1, &mu).unwrap();
        let closed = hmm_closed_form_tv(alpha, beta);
        assert!(
            (omega.tv - closed).abs() < EPS_TIGHT,
            "closed-form mismatch at (α,β)=({alpha}, {beta}): orchestrator={}, closed={closed}",
            omega.tv
        );
    }
}

#[test]
fn p4_1_even_process_orbit_admissibility_partition() {
    // The Even Process orbit has 4 points. For each point, the (U_0, U_1)
    // commutator is either fully admissible or fails on a specific path.
    // Verify the failure partition matches the paper.
    let points: &[(Vec<f64>, &str)] = &[
        (vec![2.0 / 3.0, 1.0 / 3.0], "mu_interior"),
        (vec![0.5, 0.5], "half_interior"),
        (vec![1.0, 0.0], "corner_10"),
        (vec![0.0, 1.0], "corner_01"),
    ];
    for (p, name) in points {
        let result = commutator(&EvenU0, &EvenU1, p);
        match name {
            &"mu_interior" | &"half_interior" => {
                let omega = result.expect("interior must be admissible");
                assert!((omega.tv - 1.0).abs() < EPS_TIGHT,
                        "interior saturation @ {name}: TV = {}", omega.tv);
            }
            &"corner_10" | &"corner_01" => {
                assert!(result.is_err(), "corner @ {name} must be inadmissible");
            }
            _ => unreachable!(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §C — Classifier monotonicity
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn p4_1_classifier_sofic_robust_to_band_tightening() {
    // KL Divergent → Sofic under EVERY band choice (the divergence test
    // fires first). Tighten/widen tv_low and tv_high arbitrarily; classifier
    // must stay Sofic.
    let mu = vec![2.0 / 3.0, 1.0 / 3.0];
    let omega = commutator(&EvenU0, &EvenU1, &mu).unwrap();
    let mut rng = Lcg::new(0xfa5701fc);
    for _ in 0..50 {
        let tv_low = rng.next_f64() * 0.5;
        let tv_high = tv_low + 0.01 + rng.next_f64() * (1.0 - tv_low - 0.01);
        let bands = RegimeBands { tv_low, tv_high };
        assert_eq!(
            classify_regime(&omega, bands),
            Regime::Sofic,
            "sofic broken under bands ({tv_low}, {tv_high})"
        );
    }
}

#[test]
fn p4_1_classifier_smooth_zero_tv_robust_to_band_widening() {
    // A finite-KL pair with TV = 0 (self-commutator) is Smooth under any
    // tv_low > 0 (and any tv_high > tv_low). Sweep the bands.
    let mu = vec![0.5, 0.5];
    let omega = commutator(&EvenU1, &EvenU1, &mu).unwrap();
    assert_eq!(omega.tv, 0.0);
    let mut rng = Lcg::new(0xb14c4571c);
    for _ in 0..50 {
        let tv_low = 0.001 + rng.next_f64() * 0.499;
        let tv_high = tv_low + 0.01 + rng.next_f64() * (1.0 - tv_low - 0.01);
        let bands = RegimeBands { tv_low, tv_high };
        assert_eq!(
            classify_regime(&omega, bands),
            Regime::Smooth,
            "self-commutator Smooth broken under bands ({tv_low}, {tv_high})"
        );
    }
}

#[test]
fn p4_1_classifier_borderline_squeezable_to_smooth_or_sofic() {
    // A finite-KL omega with TV in (tv_low_default, tv_high_default) is
    // Borderline under defaults. Tightening tv_low past TV makes it Sofic
    // (no — Smooth) — wait: with tv_low > TV, TV ≤ tv_low → Smooth.
    // And tv_high < TV makes it Sofic.
    let omega = Commutator {
        forward: vec![0.7, 0.3],
        backward: vec![0.1, 0.9],
        tv: 0.6,
        hellinger: 0.5,
        kl: KlValue::Finite(0.4),
    };
    assert_eq!(classify_regime(&omega, RegimeBands::default()), Regime::Borderline);
    // Tighten tv_low above 0.6 → Smooth.
    let smooth_bands = RegimeBands { tv_low: 0.65, tv_high: 0.95 };
    assert_eq!(classify_regime(&omega, smooth_bands), Regime::Smooth);
    // Drop tv_high below 0.6 → Sofic.
    let sofic_bands = RegimeBands { tv_low: 0.10, tv_high: 0.55 };
    assert_eq!(classify_regime(&omega, sofic_bands), Regime::Sofic);
}

// ═══════════════════════════════════════════════════════════════════════════
// §D — Numerical pathology
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn p4_1_extreme_belief_doesnt_panic() {
    // Beliefs very close to corners should not panic the commutator —
    // they should either succeed or return PathInadmissible cleanly.
    let extremes: Vec<Vec<f64>> = vec![
        vec![1e-9, 1.0 - 1e-9],
        vec![1.0 - 1e-9, 1e-9],
        vec![1e-15, 1.0 - 1e-15],
        vec![0.5 - 1e-12, 0.5 + 1e-12],
    ];
    let u_0 = HmmUpdate { alpha: 0.2, beta: 0.3, symbol: 0 };
    let u_1 = HmmUpdate { alpha: 0.2, beta: 0.3, symbol: 1 };
    for p in &extremes {
        // Must not panic. May return Err.
        let _ = commutator(&u_0, &u_1, p);
        let _ = commutator(&EvenU0, &EvenU1, p);
    }
}

#[test]
fn p4_1_extreme_hmm_parameters_dont_panic() {
    // (α, β) near the singular limits 0 and 1/2 — these are paper's
    // vanishing-TV anchors (H7/H8/H9). Should compute, not panic.
    let mu = vec![0.5, 0.5];
    let extremes: &[(f64, f64)] = &[
        (1e-9, 0.3),
        (0.5 - 1e-9, 0.3),
        (0.2, 1e-9),
        (0.2, 0.5 - 1e-9),
        (1e-15, 1e-15),
    ];
    for (alpha, beta) in extremes {
        let u_0 = HmmUpdate { alpha: *alpha, beta: *beta, symbol: 0 };
        let u_1 = HmmUpdate { alpha: *alpha, beta: *beta, symbol: 1 };
        let omega = commutator(&u_0, &u_1, &mu).unwrap();
        // TV ∈ [0, 1] still.
        assert!(omega.tv >= 0.0 && omega.tv <= 1.0 + EPS_TIGHT);
    }
}
