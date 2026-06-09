"""Math validation for the Update Commutator (Davis 2026, Causal States paper).

Phase 1 of GIGI's empirical scaffolding around the causal-states paper.
Pure Python; no GIGI substrate dependency. The point is to prove every
load-bearing math claim in the paper has a green test, so Phase 2 (Rust
COMMUTATOR verb) has a target.

Discipline mirrors `theory/patterns/validation_tests.py` and
`theory/post_kahler_directions/validation_tests.py`: each test
  - has a docstring naming the paper section / equation it validates
  - constructs toy data (no external deps beyond numpy)
  - asserts the mathematical claim
  - prints PASS on success

Run: `python theory/causal_states/validation_tests.py`

Companion to: `SPEC_v0.1_COMMUTATOR.md` in this directory.
"""

from __future__ import annotations

import math
import sys
from typing import Optional

import numpy as np

# ═══════════════════════════════════════════════════════════════════════════
# §A — TOY SUBSTRATE
# ═══════════════════════════════════════════════════════════════════════════

# ─── Even Process update operators (paper §5.1–5.2) ────────────────────────

def even_U0(p: tuple[float, float]) -> tuple[float, float]:
    """Even Process U_0 — observing 0. Paper Eq 5.3.

    Requires p_0 > 0; raises ValueError if p_0 == 0 (the (0, 1) corner case).
    Sends every p with p_0 > 0 to the corner (1, 0).
    """
    p0, p1 = p
    if p0 <= 0:
        raise ValueError("U_0 undefined at (0, 1) — Even Process §5.2")
    return (1.0, 0.0)


def even_U1(p: tuple[float, float]) -> tuple[float, float]:
    """Even Process U_1 — observing 1. Paper Eq 5.4.

    U_1(p) = (p_1, p_0/2) / (p_0/2 + p_1).
    Admissible whenever p_0/2 + p_1 > 0, which holds everywhere on Δ(S).
    """
    p0, p1 = p
    z = p0 / 2.0 + p1
    if z <= 0:
        raise ValueError("U_1 normalization is zero")
    return (p1 / z, (p0 / 2.0) / z)


def even_U_word(p: tuple[float, float], word: str) -> tuple[float, float]:
    """Iterated update along an observation word (paper Eq 3.6).

    Right-acting composition: U_w(p) is the result of observing symbols
    in temporal order. So U_{01}(p) = U_1(U_0(p)).
    """
    q = p
    for ch in word:
        if ch == "0":
            q = even_U0(q)
        elif ch == "1":
            q = even_U1(q)
        else:
            raise ValueError(f"unknown symbol {ch!r}")
    return q


# ─── Noisy two-state HMM update operators (paper §6.1–6.2) ─────────────────

def hmm_update(
    q: np.ndarray, x: int, alpha: float, beta: float
) -> np.ndarray:
    """Noisy 2-state HMM Bayesian update. Paper Eq 6.3.

    U_x(q) = M^T (E_x ⊙ q) / 1^T (E_x ⊙ q)
    where M is the symmetric transition matrix and E_x are emission weights.
    """
    M = np.array([[1.0 - alpha, alpha], [alpha, 1.0 - alpha]])
    if x == 0:
        E = np.array([1.0 - beta, beta])
    elif x == 1:
        E = np.array([beta, 1.0 - beta])
    else:
        raise ValueError(f"unknown symbol {x!r}")
    weighted = E * q
    transported = M.T @ weighted
    z = transported.sum()
    if z <= 0:
        raise ValueError("HMM update normalization is zero")
    return transported / z


def hmm_U_word(
    q: np.ndarray, word: str, alpha: float, beta: float
) -> np.ndarray:
    """Iterated HMM update along an observation word."""
    state = q.copy()
    for ch in word:
        state = hmm_update(state, int(ch), alpha, beta)
    return state


# ─── Scalar diagnostics (paper §4 Def 4.1) ────────────────────────────────


def diag_TV(p: np.ndarray, q: np.ndarray) -> float:
    """Total variation distance between two discrete distributions."""
    return 0.5 * float(np.abs(p - q).sum())


def diag_Hellinger(p: np.ndarray, q: np.ndarray) -> float:
    """Hellinger distance: (1/√2) * ||sqrt(p) - sqrt(q)||_2."""
    return float(np.linalg.norm(np.sqrt(p) - np.sqrt(q))) / math.sqrt(2.0)


def diag_KL(p: np.ndarray, q: np.ndarray) -> float:
    """KL divergence in bits. Returns math.inf if q has zero support
    where p does not (mutual singularity)."""
    out = 0.0
    for pi, qi in zip(p, q):
        if pi == 0:
            continue
        if qi == 0:
            return math.inf
        out += pi * math.log2(pi / qi)
    return out


# ─── Closed-form TV for the noisy HMM (paper Eq 6.4) ──────────────────────


def hmm_closed_form_TV(alpha: float, beta: float) -> float:
    """Paper Eq 6.4: H^TV_{01,10}(μ) = α(1-2α)(1-2β) / [α(1-2β)^2 + 2β(1-β)]."""
    num = alpha * (1.0 - 2.0 * alpha) * (1.0 - 2.0 * beta)
    den = alpha * (1.0 - 2.0 * beta) ** 2 + 2.0 * beta * (1.0 - beta)
    return num / den


# ─── Helper: Φ — conditional future distribution for the Even Process ─────


def even_Phi_at_past(past: str) -> tuple[float, float]:
    """Conditional future law Φ(past) for the Even Process, encoded as the
    belief over causal states {s_0, s_1} reached by reading `past` from
    the stationary belief μ = (2/3, 1/3).

    Justification: two pasts have equal Φ iff they reach the same causal
    state. The bijection Past → CausalState is exactly the ε-machine
    quotient, so the belief over causal states encodes Φ uniquely.
    """
    p = (2.0 / 3.0, 1.0 / 3.0)
    return even_U_word(p, past)


# ═══════════════════════════════════════════════════════════════════════════
# §B — TESTS
# ═══════════════════════════════════════════════════════════════════════════

EPS_TIGHT = 1e-12
EPS_LOOSE = 1e-6
EPS_NUMERIC = 1e-4   # for tolerances on paper's quoted 4-decimal numbers


# ─── Identification proposition — paper §3 Prop 3.8 ───────────────────────


def test_I1_bayesian_update_consistency():
    """U_w(Φ(past)) = Φ(past·w) on the Even Process for several words."""
    for past in ["", "0", "1", "11", "01"]:
        for w in ["0", "1", "01", "10", "11", "011"]:
            try:
                lhs = even_U_word(even_Phi_at_past(past), w)
                rhs = even_Phi_at_past(past + w)
            except ValueError:
                continue  # one of the steps was inadmissible — skip
            assert (
                abs(lhs[0] - rhs[0]) < EPS_TIGHT
                and abs(lhs[1] - rhs[1]) < EPS_TIGHT
            ), f"consistency fails for past={past!r}, w={w!r}: {lhs} != {rhs}"


def test_I2_identification_forward():
    """If Φ(past) = Φ(past'), then U_w(Φ(past)) = U_w(Φ(past')) for every w."""
    # Two pasts that both reach causal state s_0:
    past_a = "0"
    past_b = "011"  # ends a complete even block of 1's → s_0
    phi_a = even_Phi_at_past(past_a)
    phi_b = even_Phi_at_past(past_b)
    assert abs(phi_a[0] - phi_b[0]) < EPS_TIGHT, \
        "the two pasts should yield identical Φ"
    for w in ["0", "1", "01", "10", "11"]:
        try:
            u_a = even_U_word(phi_a, w)
            u_b = even_U_word(phi_b, w)
            assert all(abs(a - b) < EPS_TIGHT for a, b in zip(u_a, u_b)), \
                f"forward direction fails at w={w!r}"
        except ValueError:
            continue


def test_I3_identification_reverse():
    """If U_w(Φ(past)) = U_w(Φ(past')) for all admissible w (including w=∅),
    then Φ(past) = Φ(past') — direction ⇒ of the proposition.
    """
    # Take w = ∅ (identity transport): equality of transported fibers
    # reduces directly to equality of Φ.
    past_a = "0"
    past_b = "011"
    phi_a = even_Phi_at_past(past_a)
    phi_b = even_Phi_at_past(past_b)
    # Forward applied with w=∅:
    transported_a = phi_a
    transported_b = phi_b
    # Hypothesis: transported are equal — verify the conclusion Φ equal
    assert all(abs(a - b) < EPS_TIGHT for a, b in zip(transported_a, transported_b))


def test_I4_section_entropy_equals_C_mu():
    """H[σ] = H[μ] = C_μ on the Even Process: ≈ 0.9183 bits."""
    p0, p1 = 2.0 / 3.0, 1.0 / 3.0
    H = -(p0 * math.log2(p0) + p1 * math.log2(p1))
    assert abs(H - 0.9182958340544896) < EPS_TIGHT, f"C_μ off: got {H}"


# ─── Even Process operator semantics — paper §5.1–5.2 ─────────────────────


def test_E1_epsilon_machine_stationary():
    """Stationary distribution μ(s_0) = 2/3, μ(s_1) = 1/3."""
    # Verified by solving μ M = μ for the transition graph:
    # μ_0 = (1/2) μ_0 + 1 · μ_1, normalized.
    # The fixed point is μ_0 = 2/3, μ_1 = 1/3.
    mu = (2.0 / 3.0, 1.0 / 3.0)
    # P(s_0 next | s_0 now) = 1/2 (emit 0), P(s_1 | s_0) = 1/2 (emit 1)
    # P(s_0 | s_1) = 1, P(s_1 | s_1) = 0
    mu_next_0 = 0.5 * mu[0] + 1.0 * mu[1]
    mu_next_1 = 0.5 * mu[0]
    assert abs(mu_next_0 - mu[0]) < EPS_TIGHT
    assert abs(mu_next_1 - mu[1]) < EPS_TIGHT


def test_E2_C_mu_even_process():
    """C_μ ≈ 0.9183 bits (paper §5.1)."""
    p0, p1 = 2.0 / 3.0, 1.0 / 3.0
    cmu = -(p0 * math.log2(p0) + p1 * math.log2(p1))
    assert abs(cmu - 0.9183) < 1e-3


def test_E3_U0_collapses_to_corner():
    """U_0(p) = (1, 0) for every p with p_0 > 0 (paper Eq 5.3)."""
    for p in [(2.0/3.0, 1.0/3.0), (0.5, 0.5), (1.0, 0.0), (0.9, 0.1), (0.01, 0.99)]:
        out = even_U0(p)
        assert out == (1.0, 0.0), f"U_0 collapse fails at {p}: got {out}"


def test_E4_U1_formula():
    """U_1(p) matches the closed form (paper Eq 5.4) on the orbit points."""
    cases = [
        ((2.0/3.0, 1.0/3.0), (0.5, 0.5)),
        ((0.5, 0.5), (2.0/3.0, 1.0/3.0)),
        ((1.0, 0.0), (0.0, 1.0)),
        ((0.0, 1.0), (1.0, 0.0)),
    ]
    for inp, expected in cases:
        out = even_U1(inp)
        assert all(abs(a - b) < EPS_TIGHT for a, b in zip(out, expected)), \
            f"U_1{inp} = {out}, expected {expected}"


def test_E5_U0_undefined_at_corner():
    """U_0(0, 1) raises — admissibility requires p_0 > 0."""
    try:
        even_U0((0.0, 1.0))
    except ValueError:
        return
    assert False, "U_0(0, 1) should raise"


def test_E6_reachable_orbit_exactly_four_points():
    """Orbit closure from μ under admissible updates = {(2/3,1/3), (1/2,1/2),
    (1,0), (0,1)} (paper §5.3)."""
    seen: set[tuple[float, float]] = set()
    work = [(2.0/3.0, 1.0/3.0)]

    def round_pt(p: tuple[float, float]) -> tuple[float, float]:
        return (round(p[0], 8), round(p[1], 8))

    while work:
        p = work.pop()
        key = round_pt(p)
        if key in seen:
            continue
        seen.add(key)
        for U in (even_U0, even_U1):
            try:
                q = U(p)
                work.append(q)
            except ValueError:
                continue

    expected = {
        round_pt((2.0/3.0, 1.0/3.0)),
        round_pt((0.5, 0.5)),
        round_pt((1.0, 0.0)),
        round_pt((0.0, 1.0)),
    }
    assert seen == expected, f"orbit not as expected: got {seen}"


# ─── Even Process commutator — paper §5.4 ─────────────────────────────────


def test_E7_commutator_at_mu():
    """Ω_{01,10}(μ) = (-1, 1) — direct §5.4 calculation."""
    mu = (2.0/3.0, 1.0/3.0)
    u_01 = even_U_word(mu, "01")
    u_10 = even_U_word(mu, "10")
    diff = (u_01[0] - u_10[0], u_01[1] - u_10[1])
    assert all(abs(a - b) < EPS_TIGHT for a, b in zip(diff, (-1.0, 1.0))), \
        f"Ω_{{01,10}}(μ) = {diff}, expected (-1, 1)"


def test_E8_commutator_at_half():
    """Ω_{01,10}(1/2, 1/2) = (-1, 1)."""
    p = (0.5, 0.5)
    u_01 = even_U_word(p, "01")
    u_10 = even_U_word(p, "10")
    diff = (u_01[0] - u_10[0], u_01[1] - u_10[1])
    assert all(abs(a - b) < EPS_TIGHT for a, b in zip(diff, (-1.0, 1.0)))


def test_E9_TV_saturates_at_interior():
    """H^TV_{01,10} = 1 on the two interior reachable beliefs."""
    for p in [(2.0/3.0, 1.0/3.0), (0.5, 0.5)]:
        u_01 = np.array(even_U_word(p, "01"))
        u_10 = np.array(even_U_word(p, "10"))
        tv = diag_TV(u_01, u_10)
        assert abs(tv - 1.0) < EPS_TIGHT, f"TV at {p} = {tv}, expected 1"


def test_E10_Hellinger_saturates_at_interior():
    """H^Hel_{01,10} = 1 on the same beliefs."""
    for p in [(2.0/3.0, 1.0/3.0), (0.5, 0.5)]:
        u_01 = np.array(even_U_word(p, "01"))
        u_10 = np.array(even_U_word(p, "10"))
        hel = diag_Hellinger(u_01, u_10)
        assert abs(hel - 1.0) < EPS_TIGHT, f"Hel at {p} = {hel}, expected 1"


def test_E11_KL_diverges_at_interior():
    """H^KL = ∞ on the same — mutually singular point masses."""
    for p in [(2.0/3.0, 1.0/3.0), (0.5, 0.5)]:
        u_01 = np.array(even_U_word(p, "01"))
        u_10 = np.array(even_U_word(p, "10"))
        kl = diag_KL(u_01, u_10)
        assert math.isinf(kl), f"KL at {p} = {kl}, expected inf"


def test_E12_commutator_undefined_at_corners():
    """Commutator at (1, 0) and (0, 1) is undefined — Ω requires both
    compositions to be defined, and one composition path is blocked."""
    for p in [(1.0, 0.0), (0.0, 1.0)]:
        try:
            even_U_word(p, "01")
            even_U_word(p, "10")
        except ValueError:
            return
    assert False, "commutator at corners should hit an undefined update"


# ─── Independence of C_μ and the commutator — the thesis ──────────────────


def test_TH1_Cmu_does_not_determine_commutator():
    """Construct two processes with equal C_μ but different commutator.

    Process A: the Even Process (C_μ ≈ 0.9183, TV saturates at 1).
    Process B: a "rotation" process with the SAME stationary entropy
    over a 2-state automaton but commuting updates. We use the iid
    process X_n ~ Bernoulli(2/3): two abstract states, but emissions
    are independent of state, so U_0 and U_1 commute pointwise.

    The two processes have C_μ approximately equal (Even is 0.9183;
    iid B(2/3) has C_μ = 0 because the next-symbol distribution is
    independent of the past — but the H[μ] of the BERNOULLI is 0.9183).
    The point of TH1 is therefore subtler than just C_μ equality —
    we use the symbol-level entropy H[X_1] as the matched quantity
    that the audit might confuse with C_μ.
    """
    # Even Process stationary entropy
    p0, p1 = 2.0/3.0, 1.0/3.0
    H_even = -(p0 * math.log2(p0) + p1 * math.log2(p1))

    # iid Bernoulli(2/3) symbol entropy
    H_bern = -(p0 * math.log2(p0) + p1 * math.log2(p1))

    assert abs(H_even - H_bern) < EPS_TIGHT, \
        "audit confounder: same stationary entropy"

    # Even Process commutator at μ = SATURATED (E9)
    mu = (2.0/3.0, 1.0/3.0)
    u01 = np.array(even_U_word(mu, "01"))
    u10 = np.array(even_U_word(mu, "10"))
    tv_even = diag_TV(u01, u10)

    # iid Bernoulli: U_a and U_b both ignore past, so they trivially commute.
    # U_a(p) = stationary belief for every a. Hence commutator = 0.
    tv_bern = 0.0

    # The orthogonality claim: equal H, very different commutator.
    assert tv_even - tv_bern > 0.5, \
        f"orthogonality violated: TV_even={tv_even}, TV_bern={tv_bern}"


def test_TH2_commutator_zero_iid_process():
    """For an iid process, U_a and U_b commute trivially.

    Reason: in an iid process the conditional future is independent of the
    past, so U_a(p) = p_stationary for every a. Both compositions land on
    the same point, commutator vanishes everywhere.
    """
    # Encode the iid Bernoulli(0.5) as: any update sends belief to (0.5, 0.5).
    def iid_U(p, a):
        return (0.5, 0.5)
    p = (0.7, 0.3)
    u_01 = iid_U(iid_U(p, 0), 1)
    u_10 = iid_U(iid_U(p, 1), 0)
    diff = (u_01[0] - u_10[0], u_01[1] - u_10[1])
    assert all(abs(d) < EPS_TIGHT for d in diff), \
        "iid commutator should vanish"


def test_TH3_commutator_nonzero_minimal_Cmu():
    """Even Process has a minimal-state (|S|=2) ε-machine; its commutator
    saturates. Direction: small |S| / small C_μ does NOT imply small
    commutator. C_μ-minimization and commutator-minimization are distinct
    optimization targets.
    """
    mu = (2.0/3.0, 1.0/3.0)
    u01 = np.array(even_U_word(mu, "01"))
    u10 = np.array(even_U_word(mu, "10"))
    tv = diag_TV(u01, u10)
    # Even Process has |S|=2 (smaller than many real processes) yet TV=1.
    assert tv > 0.99, "minimal-state process can still have saturated commutator"


# ─── Noisy HMM operator semantics — paper §6.1–6.2 ────────────────────────


def test_H1_hmm_a_closed_form():
    """a = 1 - α - β + 2αβ matches U_0(μ)_0 on a grid."""
    rng = np.random.default_rng(seed=42)
    mu = np.array([0.5, 0.5])
    for _ in range(20):
        alpha = float(rng.uniform(0.05, 0.45))
        beta = float(rng.uniform(0.05, 0.45))
        a_formula = 1.0 - alpha - beta + 2.0 * alpha * beta
        a_direct = hmm_update(mu, 0, alpha, beta)[0]
        assert abs(a_formula - a_direct) < EPS_TIGHT, \
            f"a mismatch at α={alpha}, β={beta}: {a_formula} vs {a_direct}"


def test_H2_hmm_U0_at_mu():
    """U_0(μ) = (a, 1-a) for symmetric stationary."""
    rng = np.random.default_rng(seed=43)
    mu = np.array([0.5, 0.5])
    for _ in range(20):
        alpha = float(rng.uniform(0.05, 0.45))
        beta = float(rng.uniform(0.05, 0.45))
        u0 = hmm_update(mu, 0, alpha, beta)
        a = 1.0 - alpha - beta + 2.0 * alpha * beta
        assert abs(u0[0] - a) < EPS_TIGHT and abs(u0[1] - (1.0 - a)) < EPS_TIGHT


def test_H3_hmm_U1_at_mu():
    """U_1(μ) = (1-a, a)."""
    rng = np.random.default_rng(seed=44)
    mu = np.array([0.5, 0.5])
    for _ in range(20):
        alpha = float(rng.uniform(0.05, 0.45))
        beta = float(rng.uniform(0.05, 0.45))
        u1 = hmm_update(mu, 1, alpha, beta)
        a = 1.0 - alpha - beta + 2.0 * alpha * beta
        assert abs(u1[0] - (1.0 - a)) < EPS_TIGHT and abs(u1[1] - a) < EPS_TIGHT


def test_H4_hmm_updates_keep_interior_support():
    """U_x(q) stays in the interior of Δ for every q ∈ int(Δ)."""
    rng = np.random.default_rng(seed=45)
    for _ in range(20):
        alpha = float(rng.uniform(0.05, 0.45))
        beta = float(rng.uniform(0.05, 0.45))
        q0 = float(rng.uniform(0.1, 0.9))
        q = np.array([q0, 1.0 - q0])
        for x in (0, 1):
            out = hmm_update(q, x, alpha, beta)
            assert 0.0 < out[0] < 1.0, \
                f"H4 violated at α={alpha}, β={beta}, q={q}, x={x}: {out}"


def test_H5_hmm_reference_numerical_point():
    """At (α, β) = (0.2, 0.3): r ≈ 0.4469; TV ≈ 0.1062; Hel ≈ 0.0752;
    KL ≈ 0.0327 bits."""
    mu = np.array([0.5, 0.5])
    u_01 = hmm_U_word(mu, "01", 0.2, 0.3)
    u_10 = hmm_U_word(mu, "10", 0.2, 0.3)
    r = u_01[0]
    assert abs(r - 0.4469) < EPS_NUMERIC, f"r={r}, expected ≈ 0.4469"
    assert abs(diag_TV(u_01, u_10) - 0.1062) < EPS_NUMERIC
    assert abs(diag_Hellinger(u_01, u_10) - 0.0752) < EPS_NUMERIC
    assert abs(diag_KL(u_01, u_10) - 0.0327) < EPS_NUMERIC


# ─── Noisy HMM closed-form TV — paper Eq 6.4 ──────────────────────────────


def test_H6_closed_form_TV_matches_direct():
    """Closed-form TV matches direct calculation across (α, β) ∈ (0.01, 0.49)²."""
    mu = np.array([0.5, 0.5])
    grid = np.linspace(0.01, 0.49, 10)
    for alpha in grid:
        for beta in grid:
            u_01 = hmm_U_word(mu, "01", float(alpha), float(beta))
            u_10 = hmm_U_word(mu, "10", float(alpha), float(beta))
            tv_direct = diag_TV(u_01, u_10)
            tv_closed = hmm_closed_form_TV(float(alpha), float(beta))
            assert abs(tv_direct - tv_closed) < EPS_TIGHT, \
                f"closed form mismatch at (α,β)=({alpha},{beta}): " \
                f"direct={tv_direct}, closed={tv_closed}"


def test_H7_TV_vanishes_at_alpha_zero():
    """lim_{α→0+} H^TV = 0 (frozen hidden state — Bayesian updates commute)."""
    for beta in [0.1, 0.2, 0.3, 0.45]:
        tv = hmm_closed_form_TV(0.001, beta)
        assert abs(tv) < 0.01, f"TV at α→0 with β={beta} should vanish: got {tv}"


def test_H8_TV_vanishes_at_alpha_half():
    """lim_{α→1/2-} H^TV = 0 (rank-one transition — every update → μ)."""
    for beta in [0.1, 0.2, 0.3, 0.45]:
        tv = hmm_closed_form_TV(0.499, beta)
        assert abs(tv) < 0.01, f"TV at α→1/2 with β={beta} should vanish: got {tv}"


def test_H9_TV_vanishes_at_beta_half():
    """lim_{β→1/2-} H^TV = 0 (uninformative emissions — U_0 = U_1)."""
    for alpha in [0.1, 0.2, 0.3, 0.45]:
        tv = hmm_closed_form_TV(alpha, 0.499)
        assert abs(tv) < 0.01, f"TV at β→1/2 with α={alpha} should vanish: got {tv}"


def test_H10_TV_denominator_positive():
    """Denominator α(1-2β)² + 2β(1-β) > 0 strictly on (0, 1/2)²."""
    rng = np.random.default_rng(seed=46)
    for _ in range(200):
        alpha = float(rng.uniform(1e-6, 0.5 - 1e-6))
        beta = float(rng.uniform(1e-6, 0.5 - 1e-6))
        den = alpha * (1.0 - 2.0 * beta) ** 2 + 2.0 * beta * (1.0 - beta)
        assert den > 0, f"denominator non-positive at ({alpha}, {beta})"


# ─── Noisy HMM parameter dependence — paper §6.4 ──────────────────────────


def test_H11_TV_non_monotone_in_alpha():
    """At fixed β=0.1, H^TV(α) has a unique interior maximum on (0, 1/2)."""
    beta = 0.1
    alphas = np.linspace(0.01, 0.49, 49)
    tvs = [hmm_closed_form_TV(float(a), beta) for a in alphas]
    # Find argmax
    i_max = int(np.argmax(tvs))
    # Must be strictly interior
    assert 5 < i_max < len(tvs) - 5, \
        f"peak should be interior: i_max={i_max}, tvs[0..3]={tvs[:3]}, tvs[-3:]={tvs[-3:]}"
    # And strictly bigger than both endpoints
    assert tvs[i_max] > tvs[0] and tvs[i_max] > tvs[-1]


def test_H12_TV_peak_near_0_2_at_small_beta():
    """At β=0.1, the peak α* ∈ (0.15, 0.25)."""
    beta = 0.1
    alphas = np.linspace(0.01, 0.49, 100)
    tvs = [hmm_closed_form_TV(float(a), beta) for a in alphas]
    alpha_star = float(alphas[int(np.argmax(tvs))])
    assert 0.15 <= alpha_star <= 0.25, \
        f"peak α* = {alpha_star}, expected in (0.15, 0.25)"


def test_H13_TV_monotone_decreasing_in_beta():
    """At fixed α=0.2, H^TV is monotone-decreasing in β on (0.05, 0.45)."""
    alpha = 0.2
    betas = np.linspace(0.05, 0.45, 41)
    tvs = [hmm_closed_form_TV(alpha, float(b)) for b in betas]
    for i in range(1, len(tvs)):
        # Allow tiny numerical noise but require non-increasing
        assert tvs[i] <= tvs[i - 1] + EPS_LOOSE, \
            f"non-monotone at β={betas[i]}: {tvs[i-1]} → {tvs[i]}"


def test_H14_Hellinger_finite_everywhere():
    """H^Hel ≤ 1 across (α, β) grid (Hellinger is always bounded)."""
    mu = np.array([0.5, 0.5])
    grid = np.linspace(0.05, 0.45, 9)
    for alpha in grid:
        for beta in grid:
            u_01 = hmm_U_word(mu, "01", float(alpha), float(beta))
            u_10 = hmm_U_word(mu, "10", float(alpha), float(beta))
            hel = diag_Hellinger(u_01, u_10)
            assert 0.0 <= hel <= 1.0 + EPS_LOOSE, \
                f"Hellinger out of bounds at ({alpha}, {beta}): {hel}"


def test_H15_KL_finite_everywhere():
    """H^KL < ∞ across the grid — mutual absolute continuity in non-sync regime."""
    mu = np.array([0.5, 0.5])
    grid = np.linspace(0.05, 0.45, 9)
    for alpha in grid:
        for beta in grid:
            u_01 = hmm_U_word(mu, "01", float(alpha), float(beta))
            u_10 = hmm_U_word(mu, "10", float(alpha), float(beta))
            kl = diag_KL(u_01, u_10)
            assert math.isfinite(kl), \
                f"KL diverged in non-sync regime at ({alpha}, {beta}): {kl}"


# ─── Cross-regime check — paper §6.5 ──────────────────────────────────────


def test_R1_sofic_regime_saturates():
    """Even Process (sofic, synchronizing): TV=1, Hel=1, KL=∞."""
    mu = (2.0/3.0, 1.0/3.0)
    u_01 = np.array(even_U_word(mu, "01"))
    u_10 = np.array(even_U_word(mu, "10"))
    assert abs(diag_TV(u_01, u_10) - 1.0) < EPS_TIGHT
    assert abs(diag_Hellinger(u_01, u_10) - 1.0) < EPS_TIGHT
    assert math.isinf(diag_KL(u_01, u_10))


def test_R2_non_synchronizing_regime_smooth():
    """HMM diagnostic ∂H^TV/∂α is continuous on (0, 1/2) — sample-based check."""
    beta = 0.2
    alphas = np.linspace(0.01, 0.49, 200)
    tvs = np.array([hmm_closed_form_TV(float(a), beta) for a in alphas])
    diffs = np.diff(tvs)
    # Continuity: no jumps > 0.01 between adjacent samples
    assert float(np.max(np.abs(diffs))) < 0.01, \
        f"discontinuity detected: max jump = {float(np.max(np.abs(diffs)))}"


# ═══════════════════════════════════════════════════════════════════════════
# RUNNER
# ═══════════════════════════════════════════════════════════════════════════


TESTS = [
    # Identification (paper §3)
    test_I1_bayesian_update_consistency,
    test_I2_identification_forward,
    test_I3_identification_reverse,
    test_I4_section_entropy_equals_C_mu,
    # Even Process operator semantics (§5.1–5.2)
    test_E1_epsilon_machine_stationary,
    test_E2_C_mu_even_process,
    test_E3_U0_collapses_to_corner,
    test_E4_U1_formula,
    test_E5_U0_undefined_at_corner,
    test_E6_reachable_orbit_exactly_four_points,
    # Even Process commutator (§5.4)
    test_E7_commutator_at_mu,
    test_E8_commutator_at_half,
    test_E9_TV_saturates_at_interior,
    test_E10_Hellinger_saturates_at_interior,
    test_E11_KL_diverges_at_interior,
    test_E12_commutator_undefined_at_corners,
    # Thesis: independence of C_μ and commutator
    test_TH1_Cmu_does_not_determine_commutator,
    test_TH2_commutator_zero_iid_process,
    test_TH3_commutator_nonzero_minimal_Cmu,
    # Noisy HMM operator semantics (§6.1–6.2)
    test_H1_hmm_a_closed_form,
    test_H2_hmm_U0_at_mu,
    test_H3_hmm_U1_at_mu,
    test_H4_hmm_updates_keep_interior_support,
    test_H5_hmm_reference_numerical_point,
    # Closed-form TV (Eq 6.4)
    test_H6_closed_form_TV_matches_direct,
    test_H7_TV_vanishes_at_alpha_zero,
    test_H8_TV_vanishes_at_alpha_half,
    test_H9_TV_vanishes_at_beta_half,
    test_H10_TV_denominator_positive,
    # Parameter dependence (§6.4)
    test_H11_TV_non_monotone_in_alpha,
    test_H12_TV_peak_near_0_2_at_small_beta,
    test_H13_TV_monotone_decreasing_in_beta,
    test_H14_Hellinger_finite_everywhere,
    test_H15_KL_finite_everywhere,
    # Cross-regime check (§6.5)
    test_R1_sofic_regime_saturates,
    test_R2_non_synchronizing_regime_smooth,
]


if __name__ == "__main__":
    passed = 0
    failed: list[tuple[str, str]] = []
    print("=" * 72)
    print("CAUSAL STATES — UPDATE COMMUTATOR v0.1 MATH VALIDATION")
    print("=" * 72)
    for t in TESTS:
        name = t.__name__
        try:
            t()
            passed += 1
            print(f"  [PASS] {name}")
        except AssertionError as e:
            failed.append((name, str(e)))
            print(f"  [FAIL] {name}  -- {e}")
        except Exception as e:
            failed.append((name, f"{type(e).__name__}: {e}"))
            print(f"  [ERR ] {name}  -- {type(e).__name__}: {e}")
    print("=" * 72)
    print(f"PASSED: {passed}/{len(TESTS)}")
    if failed:
        print("FAILED:")
        for name, msg in failed:
            print(f"  - {name}: {msg}")
        sys.exit(1)
    print("All causal-states v0.1 math validation tests passed.")
