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
//! Diagnostics (CV1):
//! - [`tv`] — total variation distance between two discrete distributions
//! - [`hellinger`] — Hellinger distance (1/√2 · ‖√p - √q‖₂)
//! - [`kl`] — KL divergence in bits, returning [`KlValue::Divergent`] when
//!   `q` has zero support where `p` does not (Even-Process sofic regime).
//!
//! Update operators (CV2):
//! - [`UpdateOperator`] — trait for `Δ(S) → Δ(S)` with typed admissibility
//! - [`EvenU0`], [`EvenU1`] — Even Process updates (paper Eq 5.3–5.4)
//! - [`HmmUpdate`] — noisy 2-state HMM Bayesian update (paper Eq 6.3)
//! - [`even_update_word`] / [`hmm_update_word`] — iterated update along
//!   an observation word, right-acting composition (paper Eq 3.6)
//! - [`hmm_closed_form_tv`] — paper Eq 6.4, closed-form TV diagnostic
//!
//! ## Math claims this module makes
//!
//! All primitives validated against paper §4–6 by the test files
//! `tests/causal_states_cv1_diagnostics.rs` and
//! `tests/causal_states_cv2_operators.rs`, mirroring the Python sibling
//! `theory/causal_states/validation_tests.py` (36/36 green).
//!
//! ## Scope
//!
//! CV1+CV2 are diagnostics + operators. The commutator orchestrator
//! (CV3), HTTP envelope (CV4), and empirical scan (Phase 3) land in
//! subsequent sub-phases — see `theory/causal_states/SPEC_v0.1_COMMUTATOR.md`.

// ─── Diagnostics (CV1) ───────────────────────────────────────────────────

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

// ─── Update operators (CV2) ──────────────────────────────────────────────

/// Errors a [`UpdateOperator`] can produce.
///
/// Typed admissibility is load-bearing: CV3's commutator orchestrator
/// distinguishes "boundary collapse" from "you handed me garbage."
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateError {
    /// Belief is on a boundary where this update is not defined
    /// (e.g. Even Process U_0 at `(0, 1)`).
    Inadmissible(&'static str),
    /// Normalization constant non-positive — usually means caller
    /// passed a non-probability vector.
    ZeroNorm,
    /// Observation word contains a symbol this operator does not
    /// recognize.
    UnknownSymbol(char),
}

/// Bayesian belief update `U: Δ(S) → Δ(S)`.
///
/// Domain-blind by API: the operator sees a probability vector, has no
/// field names. CV3's commutator orchestrator composes pairs of these
/// via `&[Box<dyn UpdateOperator>]`.
pub trait UpdateOperator {
    /// Apply this update to `belief`, returning the new belief or a
    /// typed admissibility error.
    fn apply(&self, belief: &[f64]) -> Result<Vec<f64>, UpdateError>;
}

/// Even Process `U_0` — observation `0`. Paper Eq 5.3.
///
/// Sends every belief with `p_0 > 0` to the corner `(1, 0)`. Undefined
/// at `(0, 1)` — returns [`UpdateError::Inadmissible`].
pub struct EvenU0;

impl UpdateOperator for EvenU0 {
    fn apply(&self, belief: &[f64]) -> Result<Vec<f64>, UpdateError> {
        if belief.len() != 2 {
            return Err(UpdateError::Inadmissible(
                "Even Process operators require a 2-state belief",
            ));
        }
        if belief[0] <= 0.0 {
            return Err(UpdateError::Inadmissible(
                "Even U_0 undefined at (0, 1) — paper §5.2",
            ));
        }
        Ok(vec![1.0, 0.0])
    }
}

/// Even Process `U_1` — observation `1`. Paper Eq 5.4.
///
/// `U_1(p) = (p_1, p_0/2) / (p_0/2 + p_1)`. Admissible whenever
/// `p_0/2 + p_1 > 0`, which holds everywhere on `Δ(S)`.
pub struct EvenU1;

impl UpdateOperator for EvenU1 {
    fn apply(&self, belief: &[f64]) -> Result<Vec<f64>, UpdateError> {
        if belief.len() != 2 {
            return Err(UpdateError::Inadmissible(
                "Even Process operators require a 2-state belief",
            ));
        }
        let z = belief[0] / 2.0 + belief[1];
        if z <= 0.0 {
            return Err(UpdateError::ZeroNorm);
        }
        Ok(vec![belief[1] / z, (belief[0] / 2.0) / z])
    }
}

/// Noisy 2-state HMM Bayesian update. Paper Eq 6.3.
///
/// Symmetric transition matrix with crossover probability `α`, emissions
/// with confusion probability `β`. `symbol ∈ {0, 1}` is the observation.
///
/// `U_x(q) = M^T (E_x ⊙ q) / 1^T (E_x ⊙ q)`
///   with `M = [[1-α, α], [α, 1-α]]`, `E_0 = (1-β, β)`, `E_1 = (β, 1-β)`.
#[derive(Debug, Clone, Copy)]
pub struct HmmUpdate {
    pub alpha: f64,
    pub beta: f64,
    pub symbol: u8,
}

impl UpdateOperator for HmmUpdate {
    fn apply(&self, belief: &[f64]) -> Result<Vec<f64>, UpdateError> {
        if belief.len() != 2 {
            return Err(UpdateError::Inadmissible(
                "HMM update requires a 2-state belief",
            ));
        }
        let e = match self.symbol {
            0 => [1.0 - self.beta, self.beta],
            1 => [self.beta, 1.0 - self.beta],
            other => {
                // u8 → represent as the lossless decimal char so callers
                // can pattern-match the symbol they passed.
                let ch = std::char::from_digit(u32::from(other), 10).unwrap_or('?');
                return Err(UpdateError::UnknownSymbol(ch));
            }
        };
        // weighted = E_x ⊙ q  (Hadamard product)
        let weighted = [e[0] * belief[0], e[1] * belief[1]];
        // transported = M^T · weighted (M symmetric → M^T = M)
        let t0 = (1.0 - self.alpha) * weighted[0] + self.alpha * weighted[1];
        let t1 = self.alpha * weighted[0] + (1.0 - self.alpha) * weighted[1];
        let z = t0 + t1;
        if z <= 0.0 {
            return Err(UpdateError::ZeroNorm);
        }
        Ok(vec![t0 / z, t1 / z])
    }
}

/// Apply a sequence of Even-Process single-symbol updates encoded as a
/// `0/1` string (right-acting composition; paper Eq 3.6).
///
/// `even_update_word(p, "01") = U_1(U_0(p))` — observation order matches
/// reading the string left-to-right.
pub fn even_update_word(initial: &[f64], word: &str) -> Result<Vec<f64>, UpdateError> {
    let mut state = initial.to_vec();
    for ch in word.chars() {
        state = match ch {
            '0' => EvenU0.apply(&state)?,
            '1' => EvenU1.apply(&state)?,
            other => return Err(UpdateError::UnknownSymbol(other)),
        };
    }
    Ok(state)
}

/// Apply a sequence of HMM single-symbol updates encoded as a `0/1` string.
///
/// Same right-acting convention as [`even_update_word`].
pub fn hmm_update_word(
    initial: &[f64],
    word: &str,
    alpha: f64,
    beta: f64,
) -> Result<Vec<f64>, UpdateError> {
    let mut state = initial.to_vec();
    for ch in word.chars() {
        let symbol = match ch {
            '0' => 0u8,
            '1' => 1u8,
            other => return Err(UpdateError::UnknownSymbol(other)),
        };
        let op = HmmUpdate {
            alpha,
            beta,
            symbol,
        };
        state = op.apply(&state)?;
    }
    Ok(state)
}

/// Paper Eq 6.4: closed-form TV of the commutator at the noisy 2-state HMM.
///
/// `H^TV_{01,10}(μ) = α(1-2α)(1-2β) / [α(1-2β)² + 2β(1-β)]`
///
/// Valid on `(α, β) ∈ (0, 1/2)²`; matches direct calculation to machine
/// precision (validated by H6 test).
pub fn hmm_closed_form_tv(alpha: f64, beta: f64) -> f64 {
    let num = alpha * (1.0 - 2.0 * alpha) * (1.0 - 2.0 * beta);
    let den = alpha * (1.0 - 2.0 * beta).powi(2) + 2.0 * beta * (1.0 - beta);
    num / den
}
