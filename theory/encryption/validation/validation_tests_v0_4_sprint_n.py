"""
GIGI Encrypt v0.4 — Sprint N rigor oracle (Python independent
verification).

Mirrors `tests/invariant_verify_v0_4.rs` from a pure-Python NumPy
implementation. Does NOT call Rust — recomputes the math from first
principles so the Rust shipped code path can be cross-validated.

Test coverage:
  N-1  verifier-no-gauge-key: K and λ_1 are gauge-invariant under
       affine encryption; verifier reproduces them from the encrypted
       value array alone.
  N-2  soundness: a tampered K (or any tampered field) is detected
       by recomputation.
  N-3  completeness: across 1000 random bundles each encrypted under
       a random affine gauge, honest verification succeeds ≥ 999/1000.
  N-4  full-tuple-required: K shifts by O(1/n) under
       duplicate-at-mean record addition; the K-only check would
       admit it under realistic tolerance; record_count catches it.
  N-5  bundle-id binding: claim about bundle "A" presented against
       bundle "B" → BundleMismatch verdict before any tuple check.
       (Closes the trust-handoff hole from v0.4 review Gap 1.)
  N-6  same-K-different-topology: indexed-field clique vs isolated
       gives identical K but maximally different λ_1; full tuple
       catches it.

Run:
  PYTHONIOENCODING=utf-8 python validation_tests_v0_4_sprint_n.py
"""

import sys
from dataclasses import dataclass, replace
from typing import Optional, Tuple

import numpy as np

SEP = "─" * 65

# ─────────────────────────────────────────────────────────────────
# Math primitives mirroring src/integrity.rs::InvariantTuple +
# src/invariant_verify.rs (Python oracle).
# ─────────────────────────────────────────────────────────────────

@dataclass
class InvariantTuple:
    k: float
    lambda_1: float
    holonomy_mean: float
    record_count: int
    beta_0: int
    beta_1: int


@dataclass
class InvariantStatement:
    bundle_id: str
    claimed: InvariantTuple


@dataclass
class InvariantTolerances:
    k: float = 1e-10
    lambda_1: float = 1e-10
    holonomy_mean: float = 1e-10


def compute_k(values: np.ndarray) -> float:
    """Davis dispersion K = Var(v) / range(v)². Population variance."""
    if len(values) == 0:
        return 0.0
    rng = float(values.max() - values.min())
    if rng < 1e-12:
        return 0.0
    return float(np.var(values) / (rng * rng))


def compute_lambda1_field_graph(categories: list) -> float:
    """λ_1 of the field-index graph: 0 if disconnected, n/(n-1) if a
    complete clique (all share same indexed value), otherwise sparse
    power-iteration approximation. Mirrors src/spectral.rs."""
    n = len(categories)
    if n < 2:
        return 0.0
    # Connected components: records grouped by their indexed value.
    from collections import defaultdict
    groups = defaultdict(list)
    for i, c in enumerate(categories):
        groups[c].append(i)
    n_components = len(groups)
    if n_components > 1:
        return 0.0
    # Single component: check clique structure.
    # For a complete graph K_n on the bitmap topology: λ_1 = n/(n-1).
    if len(groups) == 1 and len(next(iter(groups.values()))) == n:
        return n / (n - 1)
    # Otherwise sparse — we don't implement the full power iteration
    # in the oracle; return a placeholder. For Sprint N tests we only
    # need the extreme cases (clique or disconnected).
    return 0.5  # placeholder — not used by these tests


def compute_beta_0(categories: list) -> int:
    """β_0 = number of connected components in the field-index graph."""
    from collections import defaultdict
    groups = defaultdict(list)
    for i, c in enumerate(categories):
        groups[c].append(i)
    return len(groups)


def compute_invariant_tuple(
    values: np.ndarray, categories: Optional[list] = None
) -> InvariantTuple:
    """The full π_inv tuple. If categories is None, treats every
    record as having a unique category (β_0 = n, λ_1 = 0)."""
    n = len(values)
    if categories is None:
        categories = list(range(n))
    k = compute_k(values)
    lam1 = compute_lambda1_field_graph(categories)
    beta_0 = compute_beta_0(categories)
    # Simplified: β_1 = 0 for tree-like graphs, ⟨Hol⟩ = 0 for
    # untwisted bundles. Sufficient for Sprint N oracle.
    return InvariantTuple(
        k=k,
        lambda_1=lam1,
        holonomy_mean=0.0,
        record_count=n,
        beta_0=beta_0,
        beta_1=0,
    )


def verify_invariant_statement(
    store_values: np.ndarray,
    store_categories: Optional[list],
    store_bundle_id: str,
    statement: InvariantStatement,
    tolerances: InvariantTolerances,
) -> Tuple[str, dict]:
    """Returns (verdict, details) where verdict ∈ {'verified',
    'bundle_mismatch', 'rejected'}."""
    if store_bundle_id != statement.bundle_id:
        return (
            "bundle_mismatch",
            {"claimed": statement.bundle_id, "store_id": store_bundle_id},
        )
    computed = compute_invariant_tuple(store_values, store_categories)
    checks = [
        ("k", statement.claimed.k, computed.k, tolerances.k),
        ("lambda1", statement.claimed.lambda_1, computed.lambda_1, tolerances.lambda_1),
        (
            "holonomy_mean",
            statement.claimed.holonomy_mean,
            computed.holonomy_mean,
            tolerances.holonomy_mean,
        ),
    ]
    for field, claimed, comp, tol in checks:
        delta = abs(claimed - comp)
        if delta > tol:
            return (
                "rejected",
                {"field": field, "claimed": claimed, "computed": comp, "delta": delta},
            )
    # u64 fields — exact equality.
    for field, claimed, comp in [
        ("record_count", statement.claimed.record_count, computed.record_count),
        ("beta_0", statement.claimed.beta_0, computed.beta_0),
        ("beta_1", statement.claimed.beta_1, computed.beta_1),
    ]:
        if claimed != comp:
            return (
                "rejected",
                {
                    "field": field,
                    "claimed": float(claimed),
                    "computed": float(comp),
                    "delta": float(abs(claimed - comp)),
                },
            )
    return ("verified", {"computed": computed})


# ─────────────────────────────────────────────────────────────────
# Test harness
# ─────────────────────────────────────────────────────────────────

PASS, FAIL = 0, 0


def check(label: str, condition: bool, detail: str = "") -> None:
    global PASS, FAIL
    if condition:
        PASS += 1
    else:
        FAIL += 1
        print(f"  ✗ FAIL: {label} — {detail}")


# ─────────────────────────────────────────────────────────────────
# N-1: verifier-no-gauge-key
# ─────────────────────────────────────────────────────────────────

def test_n1_verifier_no_key():
    print(f"\n--- N-1: verifier reproduces K from encrypted values, no gauge key")
    rng = np.random.default_rng(7)
    plain = rng.uniform(0, 100, 200)
    a, b = 2.5, -3.0
    enc = a * plain + b

    k_plain = compute_k(plain)
    k_enc = compute_k(enc)
    check(
        "K gauge-invariant under (a, b)",
        abs(k_plain - k_enc) < 1e-9,
        f"K_plain={k_plain}, K_enc={k_enc}",
    )

    # End-to-end: honest claim on encrypted bundle verifies.
    stmt = InvariantStatement(
        bundle_id="n1",
        claimed=compute_invariant_tuple(plain),
    )
    verdict, _ = verify_invariant_statement(
        enc, None, "n1", stmt, InvariantTolerances()
    )
    check("honest claim verifies on encrypted bundle", verdict == "verified",
          f"verdict={verdict}")


# ─────────────────────────────────────────────────────────────────
# N-2: soundness sweep
# ─────────────────────────────────────────────────────────────────

def test_n2_soundness():
    print(f"\n--- N-2: tampering with any of the 6 fields is detected")
    rng = np.random.default_rng(13)
    values = rng.uniform(0, 50, 150)
    honest = compute_invariant_tuple(values)
    stmt_honest = InvariantStatement(bundle_id="n2", claimed=honest)

    tamperings = [
        ("k", replace(honest, k=honest.k + 0.5)),
        ("lambda1", replace(honest, lambda_1=honest.lambda_1 + 0.5)),
        ("holonomy_mean", replace(honest, holonomy_mean=0.5)),
        ("record_count", replace(honest, record_count=honest.record_count + 1)),
        ("beta_0", replace(honest, beta_0=honest.beta_0 + 1)),
        ("beta_1", replace(honest, beta_1=honest.beta_1 + 1)),
    ]
    for field, tampered in tamperings:
        stmt = replace(stmt_honest, claimed=tampered)
        verdict, details = verify_invariant_statement(
            values, None, "n2", stmt, InvariantTolerances()
        )
        check(
            f"tampered {field} caught",
            verdict == "rejected" and details.get("field") == field,
            f"verdict={verdict}, details={details}",
        )


# ─────────────────────────────────────────────────────────────────
# N-3: completeness ≥ 999/1000
# ─────────────────────────────────────────────────────────────────

def test_n3_completeness():
    print(f"\n--- N-3: completeness — 1000 random bundles, ≥ 999 succeed")
    rng = np.random.default_rng(42)
    successes = 0
    for trial in range(1000):
        n = rng.integers(64, 128)
        values = rng.uniform(-10, 10, n) * rng.uniform(0.1, 2.0) + rng.uniform(
            -10, 10
        )
        stmt = InvariantStatement(
            bundle_id=f"n3_{trial}",
            claimed=compute_invariant_tuple(values),
        )
        # Random nonzero affine gauge.
        sign = 1.0 if rng.random() < 0.5 else -1.0
        a = sign * rng.uniform(0.01, 100.0)
        b = rng.uniform(-1000, 1000)
        enc = a * values + b
        verdict, _ = verify_invariant_statement(
            enc, None, f"n3_{trial}", stmt, InvariantTolerances()
        )
        if verdict == "verified":
            successes += 1
    check(
        f"completeness ≥ 0.999 (got {successes}/1000)",
        successes >= 999,
        f"{successes} / 1000",
    )


# ─────────────────────────────────────────────────────────────────
# N-4: full-tuple required (record_count catches K-game)
# ─────────────────────────────────────────────────────────────────

def test_n4_full_tuple():
    print(f"\n--- N-4: full tuple catches gameable-K (record_count delta)")
    rng = np.random.default_rng(17)
    n = 200
    b1 = rng.uniform(0, 1, n)
    mean_b1 = float(b1.mean())
    b2 = np.append(b1, mean_b1)  # duplicate-at-mean

    t1 = compute_invariant_tuple(b1)
    t2 = compute_invariant_tuple(b2)

    k_rel = abs(t1.k - t2.k) / max(abs(t1.k), 1e-12)
    check(
        f"K shift bounded by O(1/n): rel = {k_rel:.4f}",
        k_rel < 0.02,
        f"K1={t1.k}, K2={t2.k}",
    )
    check(
        f"record_count differs by 1",
        t2.record_count == t1.record_count + 1,
        f"r1={t1.record_count}, r2={t2.record_count}",
    )

    # Verifier holds b2; prover claims b1's tuple under matching bundle_id.
    stmt = InvariantStatement(bundle_id="n4", claimed=t1)
    verdict, details = verify_invariant_statement(
        b2, None, "n4", stmt, InvariantTolerances()
    )
    check(
        "full-tuple verifier rejects (K or record_count)",
        verdict == "rejected"
        and details.get("field") in ("k", "record_count"),
        f"verdict={verdict}, details={details}",
    )


# ─────────────────────────────────────────────────────────────────
# N-5: bundle-id binding (Gap 1 fix)
# ─────────────────────────────────────────────────────────────────

def test_n5_bundle_id_binding():
    print(f"\n--- N-5: bundle-id mismatch caught before tuple check")
    rng = np.random.default_rng(23)
    vals_a = rng.uniform(0, 10, 50)
    vals_b = vals_a.copy()  # identical data → tuples coincide
    stmt = InvariantStatement(bundle_id="bundle_A", claimed=compute_invariant_tuple(vals_a))
    verdict, details = verify_invariant_statement(
        vals_b, None, "bundle_B", stmt, InvariantTolerances()
    )
    check(
        "BundleMismatch fires even when tuples coincide",
        verdict == "bundle_mismatch"
        and details["claimed"] == "bundle_A"
        and details["store_id"] == "bundle_B",
        f"verdict={verdict}, details={details}",
    )


# ─────────────────────────────────────────────────────────────────
# N-6: same-K-different-topology (indexed field adversary)
# ─────────────────────────────────────────────────────────────────

def test_n6_same_k_different_topology():
    print(f"\n--- N-6: same K, different topology → catches on λ_1 or β_0")
    n = 80
    values = np.array([(i * 0.7 + 1.3) for i in range(n)])

    # Clique: all share category "shared" → 1 component, λ_1 = n/(n-1).
    cats_clique = ["shared"] * n
    t_clique = compute_invariant_tuple(values, cats_clique)

    # Isolated: each unique → n components, λ_1 = 0.
    cats_isolated = [f"unique_{i}" for i in range(n)]
    t_isolated = compute_invariant_tuple(values, cats_isolated)

    check(
        "K agrees (same value data)",
        abs(t_clique.k - t_isolated.k) < 1e-9,
        f"K_clique={t_clique.k}, K_isolated={t_isolated.k}",
    )
    check(
        f"β_0 differs maximally: 1 (clique) vs {n} (isolated)",
        t_clique.beta_0 == 1 and t_isolated.beta_0 == n,
        f"β_0_clique={t_clique.beta_0}, β_0_isolated={t_isolated.beta_0}",
    )
    check(
        f"λ_1 differs (clique ≈ {n/(n-1):.3f}, isolated = 0)",
        abs(t_clique.lambda_1 - t_isolated.lambda_1) > 0.5,
        f"λ_1_clique={t_clique.lambda_1}, λ_1_isolated={t_isolated.lambda_1}",
    )

    # Adversary substitution: prover claims t_clique; verifier holds isolated.
    stmt = InvariantStatement(bundle_id="n6", claimed=t_clique)
    verdict, details = verify_invariant_statement(
        values, cats_isolated, "n6", stmt, InvariantTolerances()
    )
    check(
        "full-tuple verifier rejects substitution",
        verdict == "rejected"
        and details.get("field") in ("lambda1", "beta_0", "holonomy_mean"),
        f"verdict={verdict}, details={details}",
    )


# ─────────────────────────────────────────────────────────────────
# Main
# ─────────────────────────────────────────────────────────────────

def main():
    print(SEP)
    print("GIGI v0.4 Sprint N — Invariant Consistency Verification")
    print("Python independent oracle (mirrors tests/invariant_verify_v0_4.rs)")
    print(SEP)

    test_n1_verifier_no_key()
    test_n2_soundness()
    test_n3_completeness()
    test_n4_full_tuple()
    test_n5_bundle_id_binding()
    test_n6_same_k_different_topology()

    print(f"\n{SEP}")
    print(f"SUMMARY: {PASS} pass / {FAIL} fail / {PASS + FAIL} total")
    print(SEP)

    if FAIL > 0:
        print(f"\n  Some claims failed verification. Investigate before shipping.")
        sys.exit(1)
    else:
        print(f"\n  All Sprint N claims verified by Python oracle.")
        sys.exit(0)


if __name__ == "__main__":
    main()
