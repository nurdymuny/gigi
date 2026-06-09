//! Causal States v0.1 — Update Commutator substrate.
//!
//! Companion to Davis (2026), *"Causal States as Predictive Sections:
//! ε-Machines and the Update Commutator on Belief-State Dynamics."*
//! Phase 2 of the empirical scaffolding around the paper. Phase 1 was
//! the Python math validation harness (`theory/causal_states/`); this
//! module is the Rust port of the load-bearing primitives.
//!
//! ## Surface
//!
//! - [`tv`] — total variation distance between two discrete distributions
//! - [`hellinger`] — Hellinger distance (1/√2 · ‖√p - √q‖₂)
//! - [`kl`] — KL divergence in bits, returning [`KlValue::Divergent`] when
//!   `q` has zero support where `p` does not (Even-Process sofic regime).
//!
//! ## Math claims this module makes
//!
//! All three diagnostics are validated against paper §4 Def 4.1 and the
//! reference numerical point at the noisy 2-state HMM `(α, β) = (0.2, 0.3)`
//! by the test file `tests/causal_states_cv1_diagnostics.rs` and its
//! Python sibling `theory/causal_states/validation_tests.py` (36/36).
//!
//! ## Scope
//!
//! v0.1 is **diagnostics only**. The update operator framework, the
//! commutator orchestrator, and the HTTP envelope land in subsequent
//! sub-phases (CV2–CV4) — see `theory/causal_states/SPEC_v0.1_COMMUTATOR.md`.

/// KL divergence value — finite when both distributions share support
/// where it matters, `Divergent` when mutually singular (paper §5.4,
/// Even Process sofic regime).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KlValue {
    Finite(f64),
    Divergent,
}

/// Total variation distance between two discrete probability distributions.
///
/// Paper §4 Def 4.1: `TV(p, q) = ½ Σ |p_i - q_i|`.
///
/// # Panics
///
/// Panics if `p.len() != q.len()`.
pub fn tv(p: &[f64], q: &[f64]) -> f64 {
    assert_eq!(p.len(), q.len(), "TV: distribution dimension mismatch");
    0.5 * p.iter().zip(q.iter()).map(|(a, b)| (a - b).abs()).sum::<f64>()
}

/// Hellinger distance between two discrete probability distributions.
///
/// Paper §4 Def 4.1: `H(p, q) = (1/√2) · ‖√p - √q‖₂`.
///
/// Bounded in `[0, 1]` for probability distributions.
///
/// # Panics
///
/// Panics if `p.len() != q.len()`.
pub fn hellinger(p: &[f64], q: &[f64]) -> f64 {
    assert_eq!(p.len(), q.len(), "Hellinger: distribution dimension mismatch");
    let sumsq: f64 = p
        .iter()
        .zip(q.iter())
        .map(|(a, b)| (a.sqrt() - b.sqrt()).powi(2))
        .sum();
    sumsq.sqrt() / std::f64::consts::SQRT_2
}

/// KL divergence between two discrete probability distributions, in bits.
///
/// Paper §4 Def 4.1: `KL(p ‖ q) = Σ p_i log₂(p_i / q_i)`.
///
/// Returns [`KlValue::Divergent`] when `q_i = 0` for any `i` with `p_i > 0`
/// (mutual singularity, the paper's Even-Process saturating regime).
/// Treats `p_i = 0` as contributing zero (`0 · log 0 := 0` convention).
///
/// # Panics
///
/// Panics if `p.len() != q.len()`.
pub fn kl(p: &[f64], q: &[f64]) -> KlValue {
    assert_eq!(p.len(), q.len(), "KL: distribution dimension mismatch");
    let mut acc = 0.0;
    for (pi, qi) in p.iter().zip(q.iter()) {
        if *pi == 0.0 {
            continue;
        }
        if *qi == 0.0 {
            return KlValue::Divergent;
        }
        acc += pi * (pi / qi).log2();
    }
    KlValue::Finite(acc)
}
