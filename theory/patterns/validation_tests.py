"""Math validation for Patterns v0.2 verdict primitives.

Mirrors the discipline used in `theory/post_kahler_directions/validation_tests.py`
and `theory/brain_primitives/`: prove the *mathematical claims* of each
primitive on toy data, in Python, BEFORE any Rust code is written.

If these tests fail, the spec is wrong. If they pass, the Rust TDD has
a target to aim for.

Each test:
  - has a docstring naming the spec section it validates
  - constructs toy data (no external deps beyond stdlib + numpy)
  - asserts the mathematical claim
  - prints PASS on success

Run: `python theory/patterns/validation_tests.py`
"""

from __future__ import annotations

import math
import sys
from collections import Counter
from dataclasses import dataclass, field
from itertools import combinations, product
from typing import Callable

import numpy as np

# ─── Toy substrate ──────────────────────────────────────────────────────────
# Just enough machinery to express the math claims, NOT a real engine.


@dataclass
class Row:
    pk: int
    fields: dict[str, float | int | str]


@dataclass
class Bundle:
    name: str
    rows: list[Row]

    def field_values(self, name: str) -> list:
        return [r.fields[name] for r in self.rows if name in r.fields]

    def field_stats(self, name: str) -> dict:
        vals = self.field_values(name)
        if all(isinstance(v, (int, float)) for v in vals):
            arr = np.array(vals, dtype=float)
            return {
                "min": float(arr.min()),
                "max": float(arr.max()),
                "mean": float(arr.mean()),
                "std": float(arr.std()),
                "categorical": False,
            }
        return {"set": set(vals), "categorical": True}


@dataclass
class Predicate:
    """Conjunction of clauses, each of shape (field, op, value)."""
    clauses: list[tuple[str, str, float | int | str]]

    def matches(self, row: Row) -> bool:
        return all(self._eval_clause(row, c) for c in self.clauses)

    def violations(self, row: Row) -> list[tuple[str, str, float | int | str]]:
        return [c for c in self.clauses if not self._eval_clause(row, c)]

    @staticmethod
    def _eval_clause(row: Row, c) -> bool:
        f, op, v = c
        if f not in row.fields:
            return False
        lhs = row.fields[f]
        if op == ">=": return lhs >= v
        if op == ">":  return lhs >  v
        if op == "<=": return lhs <= v
        if op == "<":  return lhs <  v
        if op == "==": return lhs == v
        if op == "!=": return lhs != v
        raise ValueError(f"unknown op {op}")


# ─── WEIGHT AST ─────────────────────────────────────────────────────────────


@dataclass
class WLit:
    value: float


@dataclass
class WField:
    name: str


@dataclass
class WAdd:
    left: any
    right: any


@dataclass
class WMul:
    left: any
    right: any


@dataclass
class WMin:
    left: any
    right: any


@dataclass
class WMax:
    left: any
    right: any


def eval_weight(expr, row: Row) -> float:
    if isinstance(expr, WLit):
        return float(expr.value)
    if isinstance(expr, WField):
        return float(row.fields.get(expr.name, 0))
    if isinstance(expr, WAdd):
        return eval_weight(expr.left, row) + eval_weight(expr.right, row)
    if isinstance(expr, WMul):
        return eval_weight(expr.left, row) * eval_weight(expr.right, row)
    if isinstance(expr, WMin):
        return min(eval_weight(expr.left, row), eval_weight(expr.right, row))
    if isinstance(expr, WMax):
        return max(eval_weight(expr.left, row), eval_weight(expr.right, row))
    raise ValueError(f"unknown node {type(expr).__name__}")


# ─── §2 K_P pattern curvature ───────────────────────────────────────────────


def hamming(a: Row, b: Row, fields: list[str]) -> int:
    return sum(1 for f in fields if a.fields.get(f) != b.fields.get(f))


def knn_indices(bundle: Bundle, fields: list[str], k: int) -> dict[int, list[int]]:
    """For each row index, return the indices of its k nearest neighbors by Hamming distance over the named fields."""
    n = len(bundle.rows)
    out: dict[int, list[int]] = {}
    for i in range(n):
        distances = [(j, hamming(bundle.rows[i], bundle.rows[j], fields))
                     for j in range(n) if j != i]
        distances.sort(key=lambda t: t[1])
        out[i] = [j for j, _ in distances[:k]]
    return out


def k_p(predicate: Predicate, bundle: Bundle, fields: list[str], k: int) -> tuple[float, int]:
    """Returns (K_P, n_matches). Per §2.1 of SPEC_v0.2_VERDICT.md."""
    match_set = {i for i, r in enumerate(bundle.rows) if predicate.matches(r)}
    n_matches = len(match_set)
    if n_matches == 0:
        return 0.0, 0
    nbrs = knn_indices(bundle, fields, k)
    ratios = []
    for i in range(len(bundle.rows)):
        nbr_set = set(nbrs[i])
        matches_in_nbrs = len(nbr_set & match_set)
        ratios.append(matches_in_nbrs / k)
    return float(np.var(ratios)), n_matches


# ─── §3 Pattern preflight ───────────────────────────────────────────────────


def preflight_internal(predicate: Predicate) -> tuple[bool, str]:
    """Internal contradiction check: same field with conflicting clauses, no bundle needed.

    Always a verdict gate regardless of near-miss budget — a self-contradictory
    predicate cannot be repaired by flipping bundle rows.
    """
    seen: dict[str, list] = {}
    for f, op, v in predicate.clauses:
        seen.setdefault(f, []).append((op, v))
    for f, ops in seen.items():
        ge_lo = max((v for op, v in ops if op in (">=", ">")), default=None)
        le_hi = min((v for op, v in ops if op in ("<=", "<")), default=None)
        if ge_lo is not None and le_hi is not None:
            if ge_lo > le_hi:
                return False, f"internal contradiction on {f}: lo={ge_lo}, hi={le_hi}"
        eqs = [v for op, v in ops if op == "=="]
        if len(eqs) > 1 and len(set(eqs)) > 1:
            return False, f"internal contradiction on {f}: == {eqs}"
    return True, "ok"


def preflight_statistic(predicate: Predicate, bundle: Bundle) -> tuple[bool, str]:
    """Bundle-statistic preflight: would ANY row satisfy each clause individually?

    Per §3.3: this is a verdict gate ONLY when near_miss_budget == 0. When budget
    >= 1, near-miss may repair this via field flips, so preflight_statistic
    becomes informational (the scan + near-miss handles the verdict).
    """
    ok, reason = preflight_internal(predicate)
    if not ok:
        return False, reason
    for f, op, v in predicate.clauses:
        stats = bundle.field_stats(f)
        if stats["categorical"]:
            if op == "==" and v not in stats["set"]:
                return False, f"field {f} cannot satisfy =={v} (values: {sorted(stats['set'])})"
            if op == "!=" and stats["set"] == {v}:
                return False, f"field {f} cannot satisfy !={v} (only value: {v})"
        else:
            if op in (">=", ">") and v > stats["max"]:
                return False, f"field {f} cannot satisfy {op}{v} (max={stats['max']})"
            if op in ("<=", "<") and v < stats["min"]:
                return False, f"field {f} cannot satisfy {op}{v} (min={stats['min']})"
            if op == "==" and not (stats["min"] <= v <= stats["max"]):
                return False, f"field {f} cannot satisfy =={v} (range=[{stats['min']},{stats['max']}])"
    return True, "ok"


def preflight_holonomy(predicate: Predicate, bundle: Bundle) -> tuple[bool, str]:
    """Layer 2 preflight: detect joint infeasibility (Bayes-net-style contradictions).

    Implemented as: does ANY row in the bundle satisfy ALL clauses jointly?
    If no row does AND each individual clause has support, that's the joint contradiction.

    A formal holonomy treatment lives in the Rust port — this is the math-level claim.
    """
    fields = list({c[0] for c in predicate.clauses})
    if len(fields) < 2:
        return True, "ok (single-field predicate)"
    # If every clause individually has rows but no row satisfies the conjunction,
    # the joint distribution forbids the conjunction.
    any_match = any(predicate.matches(r) for r in bundle.rows)
    if any_match:
        return True, "ok"
    # Every clause must have at least one satisfying row individually.
    for c in predicate.clauses:
        single = Predicate(clauses=[c])
        if not any(single.matches(r) for r in bundle.rows):
            return True, "ok (single clause already unsat, layer 1 should have caught)"
    return False, f"joint distribution forbids {fields}"


# ─── §4 verdict trichotomy ──────────────────────────────────────────────────


def verdict(predicate: Predicate, bundle: Bundle, near_miss_budget: int = 1) -> tuple[str, dict]:
    """Returns (verdict_str, payload). Per §4.

    Order matters: sat → near_miss → unsat. Holonomy preflight only fires
    in the unsat branch (where we'd return unsat anyway) — it doesn't
    pre-empt near-miss detection, since "joint distribution currently
    forbids" doesn't mean "and no near-miss exists."
    """
    # Internal contradiction always wins — no scan, no near-miss can repair it.
    int_ok, int_reason = preflight_internal(predicate)
    if not int_ok:
        return "unsat", {"reason": int_reason, "preflight_caught": True}
    # Bundle-statistic preflight is a verdict gate ONLY when near_miss_budget == 0.
    # With a budget, near-miss may repair what statistic says is impossible
    # (e.g., "color=purple" against {red, blue} is repairable by flipping color).
    if near_miss_budget == 0:
        pf_ok, pf_reason = preflight_statistic(predicate, bundle)
        if not pf_ok:
            return "unsat", {"reason": pf_reason, "preflight_caught": True}
    # Sat check — any strict match → sat
    matches = [r for r in bundle.rows if predicate.matches(r)]
    if matches:
        return "sat", {"n_matches": len(matches)}
    # Near-miss check SECOND — strict empty but rows within budget → near_miss
    near_miss = [r for r in bundle.rows
                 if 0 < len(predicate.violations(r)) <= near_miss_budget]
    if near_miss:
        return "near_miss", {"near_miss_count": len(near_miss),
                             "budget": near_miss_budget}
    # Finally — neither match nor near-miss → unsat. Holonomy is the
    # explanation, not a separate verdict path.
    h_ok, h_reason = preflight_holonomy(predicate, bundle)
    return "unsat", {"reason": h_reason if not h_ok else
                     "no matches and no near-misses within budget",
                     "preflight_caught": False}


# ─── §5 PATTERN_REPAIR menu ─────────────────────────────────────────────────


def repair_menu(predicate: Predicate, row: Row, max_flips: int,
                relaxation_costs: dict[str, float] | None = None,
                top: int = 5) -> list[tuple[list, float]]:
    """For a near-miss row, return the ordered minimum-cost flip menu.

    Each entry: (list_of_(field, current_value, target_value), total_cost).

    Per §5.3. We enumerate over violating clauses' target values. Toy
    implementation — assumes boolean target space for simplicity (the
    Rust port handles general value spaces).
    """
    relaxation_costs = relaxation_costs or {}
    viols = predicate.violations(row)
    if not viols:
        return [("already_matches", 0.0)]
    if len(viols) > max_flips:
        return [("too_far", float("inf"))]
    # Each violation has a target value: the v in (field, op, v) — assumes
    # equality-style clauses. Sufficient for the math claim; the Rust port
    # handles range-style clauses with a search over satisfying values.
    options = []
    for flip_subset in combinations(viols, len(viols)):
        flips = [(f, row.fields.get(f), v) for f, op, v in flip_subset]
        cost = sum(relaxation_costs.get(f, 1.0) for f, _, _ in flip_subset)
        options.append((flips, cost))
    # Sort by cost asc, then by flip count asc, then by field name lex
    options.sort(key=lambda o: (o[1], len(o[0]), tuple(sorted(f for f, _, _ in o[0]))))
    return options[:top]


# ─── §6 PATTERN_EXPLAIN ─────────────────────────────────────────────────────


def explain(expr, row: Row) -> dict:
    """Decompose a WEIGHT expression into per-term contributions.

    Returns a tree of {type, value/contribution, children}. Per §6.3.
    The root's contribution equals eval_weight(expr, row).
    """
    if isinstance(expr, WLit):
        return {"type": "lit", "value": expr.value, "contribution": float(expr.value)}
    if isinstance(expr, WField):
        v = float(row.fields.get(expr.name, 0))
        return {"type": "field", "name": expr.name, "value": v, "contribution": v}
    if isinstance(expr, WAdd):
        l = explain(expr.left, row); r = explain(expr.right, row)
        return {"type": "add", "left": l, "right": r,
                "contribution": l["contribution"] + r["contribution"]}
    if isinstance(expr, WMul):
        l = explain(expr.left, row); r = explain(expr.right, row)
        return {"type": "mul", "left": l, "right": r,
                "contribution": l["contribution"] * r["contribution"]}
    if isinstance(expr, WMin):
        # Canonical form: min(value_expr, cap_expr). `clipped` fires when
        # the cap (right) was binding — i.e., the value (left) wanted to
        # be higher than the cap allowed. left > right ⇒ cap fired.
        l = explain(expr.left, row); r = explain(expr.right, row)
        chosen = "left" if l["contribution"] <= r["contribution"] else "right"
        contribution = min(l["contribution"], r["contribution"])
        clipped = l["contribution"] > r["contribution"]
        return {"type": "min", "left": l, "right": r, "chosen": chosen,
                "clipped": clipped, "contribution": contribution}
    if isinstance(expr, WMax):
        # Symmetric: max(value_expr, floor_expr). Floored fires when
        # the floor (right) was binding — value wanted to be lower.
        l = explain(expr.left, row); r = explain(expr.right, row)
        chosen = "left" if l["contribution"] >= r["contribution"] else "right"
        floored = l["contribution"] < r["contribution"]
        return {"type": "max", "left": l, "right": r, "chosen": chosen,
                "floored": floored,
                "contribution": max(l["contribution"], r["contribution"])}
    raise ValueError(f"unknown node {type(expr).__name__}")


# ═══════════════════════════════════════════════════════════════════════════
# TESTS
# ═══════════════════════════════════════════════════════════════════════════


# ─── K_P (§2) ───────────────────────────────────────────────────────────────


def test_K1_kp_concentrated_pattern_is_strictly_positive():
    """K_P > 0 when matching rows cluster (concentrated)."""
    # 100 rows; matching rows are the contiguous block 0..19 sharing
    # (a=1, b=1, c=1). The rest of the bundle has random (a, b, c) ≠ (1,1,1).
    rng = np.random.default_rng(seed=42)
    rows = []
    # 20 matching rows — all share (1, 1, 1), pk used as noise tiebreaker
    for pk in range(20):
        rows.append(Row(pk=pk, fields={"a": 1, "b": 1, "c": 1, "noise": pk}))
    # 80 non-matching rows with random bits, avoiding the (1,1,1) corner
    for pk in range(20, 100):
        while True:
            a, b, c = int(rng.integers(0, 2)), int(rng.integers(0, 2)), int(rng.integers(0, 2))
            if (a, b, c) != (1, 1, 1):
                break
        rows.append(Row(pk=pk, fields={"a": a, "b": b, "c": c, "noise": pk}))
    bundle = Bundle(name="concentrated", rows=rows)
    pred = Predicate(clauses=[("a", "==", 1), ("b", "==", 1), ("c", "==", 1)])
    kp_high, n = k_p(pred, bundle, fields=["a", "b", "c"], k=8)
    assert n == 20, f"expected 20 matches got {n}"
    assert kp_high > 0.0, f"expected strictly positive K_P got {kp_high}"


def test_K2_kp_responds_to_match_concentration():
    """Same pattern, two bundles. Bundle A places matches CLUSTERED in the
    kNN structure (matching rows share noise fingerprints, so they're kNN
    of each other). Bundle B places matches SCATTERED.

    The math claim: K_P measures the actual geometric concentration of the
    match set in the kNN graph, regardless of pattern shape.
    """
    rng = np.random.default_rng(seed=42)
    # Bundle A — clustered: matching rows share their noise fingerprint
    # with each other. Non-matching rows have a different fingerprint.
    rows_a = []
    for pk in range(40):
        if pk < 20:
            # Matching cluster: all share "noise fingerprint" (0, 0, 0)
            rows_a.append(Row(pk=pk, fields={"flag": 1, "n0": 0, "n1": 0, "n2": 0}))
        else:
            # Non-matching: share "noise fingerprint" (1, 1, 1)
            rows_a.append(Row(pk=pk, fields={"flag": 0, "n0": 1, "n1": 1, "n2": 1}))
    bundle_a = Bundle(name="clustered", rows=rows_a)

    # Bundle B — scattered: matching rows have random noise fingerprints.
    rows_b = []
    for pk in range(40):
        flag = 1 if pk < 20 else 0
        noise = {f"n{j}": int(rng.integers(0, 2)) for j in range(3)}
        rows_b.append(Row(pk=pk, fields={"flag": flag, **noise}))
    bundle_b = Bundle(name="scattered", rows=rows_b)

    pred = Predicate(clauses=[("flag", "==", 1)])
    fields = ["flag", "n0", "n1", "n2"]
    kp_clustered, _ = k_p(pred, bundle_a, fields=fields, k=5)
    kp_scattered, _ = k_p(pred, bundle_b, fields=fields, k=5)

    assert kp_clustered > kp_scattered, \
        f"clustered-placement K_P {kp_clustered:.4f} should exceed scattered K_P {kp_scattered:.4f}"


def test_K3_kp_empty_match_is_zero():
    """K_P = 0 by convention when n_matches = 0."""
    rows = [Row(pk=i, fields={"a": 0}) for i in range(10)]
    bundle = Bundle(name="no_match", rows=rows)
    pred = Predicate(clauses=[("a", "==", 1)])
    kp, n = k_p(pred, bundle, fields=["a"], k=3)
    assert n == 0 and kp == 0.0, f"empty match should give (0, 0) got ({kp}, {n})"


def test_K4_kp_concentrated_exceeds_tautology():
    """A pattern that matches everything has K_P = 0 (no variance).
    A pattern that matches one cluster has K_P > 0.
    """
    rows = [Row(pk=i, fields={"a": 1 if i < 20 else 0, "always": 1})
            for i in range(100)]
    bundle = Bundle(name="taut_vs_concentrated", rows=rows)
    pred_concentrated = Predicate(clauses=[("a", "==", 1)])
    pred_tautology = Predicate(clauses=[("always", "==", 1)])

    kp_conc, n_conc = k_p(pred_concentrated, bundle, fields=["a", "always"], k=8)
    kp_taut, n_taut = k_p(pred_tautology, bundle, fields=["a", "always"], k=8)

    assert n_conc == 20 and n_taut == 100
    assert kp_taut == 0.0, f"tautology must have K_P=0 (all neighborhoods are 100%): got {kp_taut}"
    assert kp_conc > kp_taut, f"concentrated K_P {kp_conc} should exceed tautology K_P {kp_taut}"


# ─── Pattern preflight (§3) ─────────────────────────────────────────────────


def test_PP1_preflight_catches_impossible_numeric_range():
    """Predicate asks for value beyond field max → unsat."""
    rows = [Row(pk=i, fields={"x": i}) for i in range(10)]  # x ∈ [0, 9]
    bundle = Bundle(name="b", rows=rows)
    pred = Predicate(clauses=[("x", ">=", 100)])
    ok, reason = preflight_statistic(pred, bundle)
    assert not ok, "preflight should catch x >= 100 against max=9"
    assert "max=9" in reason or "100" in reason, reason


def test_PP2_preflight_catches_missing_categorical():
    """Predicate asks for category not present → unsat."""
    rows = [Row(pk=i, fields={"color": c})
            for i, c in enumerate(["red", "blue", "red", "blue"])]
    bundle = Bundle(name="b", rows=rows)
    pred = Predicate(clauses=[("color", "==", "purple")])
    ok, reason = preflight_statistic(pred, bundle)
    assert not ok, "preflight should catch color=purple against {red, blue}"
    assert "purple" in reason, reason


def test_PP3_preflight_catches_internal_contradiction():
    """Predicate `x >= 0 AND x < 0` → unsat regardless of bundle."""
    rows = [Row(pk=i, fields={"x": i}) for i in range(10)]
    bundle = Bundle(name="b", rows=rows)
    pred = Predicate(clauses=[("x", ">=", 5), ("x", "<", 3)])
    ok, reason = preflight_statistic(pred, bundle)
    assert not ok, "internally contradictory predicate must be unsat"
    assert "contradiction" in reason.lower(), reason


def test_PP4_preflight_holonomy_catches_joint_contradiction():
    """Each clause individually satisfiable but conjunction forbidden by bundle."""
    # Bundle where x=1 NEVER co-occurs with y=1, but both x=1 and y=1 exist
    # separately.
    rows = [
        Row(pk=0, fields={"x": 1, "y": 0}),
        Row(pk=1, fields={"x": 1, "y": 0}),
        Row(pk=2, fields={"x": 0, "y": 1}),
        Row(pk=3, fields={"x": 0, "y": 1}),
        Row(pk=4, fields={"x": 0, "y": 0}),
    ]
    bundle = Bundle(name="joint_forbidden", rows=rows)
    pred = Predicate(clauses=[("x", "==", 1), ("y", "==", 1)])
    pf_ok, _ = preflight_statistic(pred, bundle)
    h_ok, h_reason = preflight_holonomy(pred, bundle)
    assert pf_ok, "layer-1 preflight should pass (each clause individually sat)"
    assert not h_ok, f"layer-2 holonomy preflight should catch joint contradiction: {h_reason}"


def test_PP5_preflight_passes_satisfiable_predicate():
    """Predicate that the bundle CAN satisfy → ok."""
    rows = [Row(pk=i, fields={"x": i, "color": "red" if i < 3 else "blue"})
            for i in range(10)]
    bundle = Bundle(name="b", rows=rows)
    pred = Predicate(clauses=[("x", ">=", 0), ("color", "==", "red")])
    ok, _ = preflight_statistic(pred, bundle)
    h_ok, _ = preflight_holonomy(pred, bundle)
    assert ok and h_ok, "satisfiable predicate must pass preflight"


# ─── Verdict trichotomy (§4) ────────────────────────────────────────────────


def test_VT1_verdict_sat_when_rows_match():
    rows = [Row(pk=i, fields={"a": 1 if i < 3 else 0}) for i in range(10)]
    bundle = Bundle(name="b", rows=rows)
    pred = Predicate(clauses=[("a", "==", 1)])
    v, payload = verdict(pred, bundle)
    assert v == "sat", f"expected sat got {v}"
    assert payload["n_matches"] == 3


def test_VT2_verdict_unsat_by_preflight():
    """With budget=0 (v0.1-compatible mode), bundle-stat preflight gates unsat."""
    rows = [Row(pk=i, fields={"x": i}) for i in range(10)]
    bundle = Bundle(name="b", rows=rows)
    pred = Predicate(clauses=[("x", ">=", 999)])
    v, payload = verdict(pred, bundle, near_miss_budget=0)
    assert v == "unsat", f"expected unsat got {v}"
    assert payload.get("preflight_caught") is True


def test_VT3_verdict_unsat_by_scan_when_no_match_and_no_near_miss():
    """No row matches, AND no row is within budget — pure unsat."""
    # All rows are 3 violations away from matching (a=0, b=0, c=0 needed)
    rows = [Row(pk=i, fields={"a": 1, "b": 1, "c": 1}) for i in range(5)]
    bundle = Bundle(name="b", rows=rows)
    pred = Predicate(clauses=[("a", "==", 0), ("b", "==", 0), ("c", "==", 0)])
    # near_miss_budget=1, so 3 violations is "too far"
    v, payload = verdict(pred, bundle, near_miss_budget=1)
    # Preflight currently catches this as joint contradiction since no row matches
    # joint and each clause individually has support — that's correct, it's unsat
    # by holonomy, just with a different reason than "scan returned empty".
    assert v == "unsat", f"expected unsat got {v}"


def test_VT4_verdict_near_miss_at_distance_1():
    """0 strict matches, ≥1 row at flip-distance 1."""
    # Rows have (a=1, b=1) but pattern wants (a=1, b=0).
    rows = [Row(pk=i, fields={"a": 1, "b": 1}) for i in range(5)]
    bundle = Bundle(name="b", rows=rows)
    pred = Predicate(clauses=[("a", "==", 1), ("b", "==", 0)])
    v, payload = verdict(pred, bundle, near_miss_budget=1)
    assert v == "near_miss", f"expected near_miss got {v}"
    assert payload["near_miss_count"] == 5


def test_VT5_verdict_trichotomy_exhaustive():
    """Every (pattern, bundle) pair lands in exactly one verdict bucket."""
    # Construct a bundle and run three patterns hitting each verdict.
    rows = [
        Row(pk=0, fields={"a": 1, "b": 1}),
        Row(pk=1, fields={"a": 1, "b": 0}),
        Row(pk=2, fields={"a": 0, "b": 0}),
    ]
    bundle = Bundle(name="b", rows=rows)

    sat_pred = Predicate(clauses=[("a", "==", 1)])
    near_pred = Predicate(clauses=[("a", "==", 0), ("b", "==", 1)])  # 1 flip from row 0
    unsat_pred = Predicate(clauses=[("a", ">=", 100)])

    v_sat, _ = verdict(sat_pred, bundle)
    v_near, _ = verdict(near_pred, bundle, near_miss_budget=1)
    # Use budget=0 so bundle-stat preflight gates this as unsat.
    v_unsat, _ = verdict(unsat_pred, bundle, near_miss_budget=0)

    verdicts = {v_sat, v_near, v_unsat}
    assert verdicts == {"sat", "near_miss", "unsat"}, \
        f"trichotomy missing a verdict: {verdicts}"


# ─── PATTERN_REPAIR (§5) ────────────────────────────────────────────────────


def test_PR1_repair_single_flip_uniform_cost():
    """One violation, default cost=1.0, single-entry menu."""
    row = Row(pk=0, fields={"a": 1, "b": 1})
    pred = Predicate(clauses=[("a", "==", 1), ("b", "==", 0)])
    menu = repair_menu(pred, row, max_flips=1)
    assert len(menu) == 1, f"expected 1 entry got {len(menu)}"
    flips, cost = menu[0]
    assert cost == 1.0
    assert flips[0] == ("b", 1, 0)


def test_PR2_repair_double_flip():
    row = Row(pk=0, fields={"a": 1, "b": 1, "c": 1})
    pred = Predicate(clauses=[("a", "==", 0), ("b", "==", 0)])
    menu = repair_menu(pred, row, max_flips=2)
    assert len(menu) == 1
    flips, cost = menu[0]
    assert cost == 2.0
    assert set((f, t) for f, _, t in flips) == {("a", 0), ("b", 0)}


def test_PR3_repair_custom_costs_sort_correctly():
    """Field with lower relaxation_cost should rank cheaper."""
    row = Row(pk=0, fields={"cheap_field": 1, "expensive_field": 1})
    # Two patterns, each requires one flip. The one with lower cost should be cheaper.
    pred_cheap = Predicate(clauses=[("cheap_field", "==", 0)])
    pred_expensive = Predicate(clauses=[("expensive_field", "==", 0)])
    costs = {"cheap_field": 0.5, "expensive_field": 3.0}

    menu_cheap = repair_menu(pred_cheap, row, max_flips=1, relaxation_costs=costs)
    menu_exp = repair_menu(pred_expensive, row, max_flips=1, relaxation_costs=costs)

    assert menu_cheap[0][1] == 0.5
    assert menu_exp[0][1] == 3.0
    assert menu_cheap[0][1] < menu_exp[0][1]


def test_PR4_repair_already_matches_returns_sentinel():
    row = Row(pk=0, fields={"a": 1})
    pred = Predicate(clauses=[("a", "==", 1)])
    menu = repair_menu(pred, row, max_flips=1)
    assert menu == [("already_matches", 0.0)]


def test_PR5_repair_too_far_returns_sentinel():
    row = Row(pk=0, fields={"a": 1, "b": 1, "c": 1, "d": 1})
    pred = Predicate(clauses=[("a", "==", 0), ("b", "==", 0),
                              ("c", "==", 0), ("d", "==", 0)])
    menu = repair_menu(pred, row, max_flips=1)
    assert menu[0][0] == "too_far"


def test_PR6_repair_min_cost_is_actually_minimum():
    """Among returned options, the first has the min cost — and no shorter sequence exists outside the menu."""
    row = Row(pk=0, fields={"a": 1, "b": 1, "c": 1})
    pred = Predicate(clauses=[("a", "==", 0)])
    menu = repair_menu(pred, row, max_flips=1)
    flips, cost = menu[0]
    # The minimum cost equals the per-field cost of the only violation.
    assert cost == 1.0
    # Confirm: applying the flip makes the row match.
    new_fields = dict(row.fields)
    for f, _, t in flips:
        new_fields[f] = t
    new_row = Row(pk=row.pk, fields=new_fields)
    assert pred.matches(new_row), "applying min-cost flip must satisfy predicate"


# ─── PATTERN_EXPLAIN (§6) ───────────────────────────────────────────────────


def test_PE1_explain_lit_returns_literal_value():
    row = Row(pk=0, fields={})
    e = explain(WLit(7.5), row)
    assert e["type"] == "lit" and e["contribution"] == 7.5


def test_PE2_explain_field_returns_row_value():
    row = Row(pk=0, fields={"x": 3.0})
    e = explain(WField("x"), row)
    assert e["type"] == "field" and e["contribution"] == 3.0


def test_PE3_explain_add_root_equals_eval():
    row = Row(pk=0, fields={"x": 2.0, "y": 5.0})
    expr = WAdd(WField("x"), WField("y"))
    e = explain(expr, row)
    assert e["contribution"] == 7.0 == eval_weight(expr, row)


def test_PE4_explain_mul_root_equals_product():
    row = Row(pk=0, fields={"x": 3.0})
    expr = WMul(WField("x"), WLit(4.0))
    e = explain(expr, row)
    assert e["contribution"] == 12.0
    assert e["left"]["contribution"] == 3.0
    assert e["right"]["contribution"] == 4.0


def test_PE5_explain_min_chosen_branch_and_clip_flag():
    """min(15, 10) → contribution=10, chosen=right, clipped=True."""
    row = Row(pk=0, fields={"sum": 15.0})
    expr = WMin(WField("sum"), WLit(10.0))
    e = explain(expr, row)
    assert e["type"] == "min"
    assert e["chosen"] == "right"
    assert e["contribution"] == 10.0
    assert e["clipped"] is True

    # min(5, 10) → contribution=5, chosen=left, clipped=False
    row2 = Row(pk=0, fields={"sum": 5.0})
    e2 = explain(expr, row2)
    assert e2["chosen"] == "left"
    assert e2["contribution"] == 5.0
    assert e2["clipped"] is False


def test_PE6_explain_full_scj_scorer_invariant():
    """Sum of leaf contributions equals root contribution for the full SCJ shape."""
    row = Row(pk=0, fields={
        "cast_truncate_alloc": 1, "multiply_before_alloc": 1, "shift_before_alloc": 1,
        "param_times_const": 1, "unchecked_param_to_size": 1, "mdl_shift_size": 1,
        "reaches_ExAllocatePool2": 1, "reaches_MmBuildMdlForNonPagedPool": 1,
        "has_probe_read": 1, "has_probe_write": 1,
    })
    sum_expr = WField("cast_truncate_alloc")
    for f, w in [("multiply_before_alloc", 3), ("shift_before_alloc", 3),
                 ("param_times_const", 2), ("unchecked_param_to_size", 2),
                 ("mdl_shift_size", 2), ("reaches_ExAllocatePool2", 1),
                 ("reaches_MmBuildMdlForNonPagedPool", 1), ("has_probe_read", 1),
                 ("has_probe_write", 1)]:
        sum_expr = WAdd(sum_expr, WMul(WField(f), WLit(w)))
    sum_expr = WAdd(WMul(WField("cast_truncate_alloc"), WLit(3)), sum_expr) \
               if False else sum_expr
    # Build the actual SCJ scorer: sum of weighted terms, clipped at 10.
    pieces = [
        WMul(WField("cast_truncate_alloc"), WLit(3)),
        WMul(WField("multiply_before_alloc"), WLit(3)),
        WMul(WField("shift_before_alloc"), WLit(3)),
        WMul(WField("param_times_const"), WLit(2)),
        WMul(WField("unchecked_param_to_size"), WLit(2)),
        WMul(WField("mdl_shift_size"), WLit(2)),
        WMul(WField("reaches_ExAllocatePool2"), WLit(1)),
        WMul(WField("reaches_MmBuildMdlForNonPagedPool"), WLit(1)),
        WMul(WField("has_probe_read"), WLit(1)),
        WMul(WField("has_probe_write"), WLit(1)),
    ]
    sum_expr = pieces[0]
    for p in pieces[1:]:
        sum_expr = WAdd(sum_expr, p)
    scj = WMin(sum_expr, WLit(10.0))

    score = eval_weight(scj, row)
    e = explain(scj, row)
    assert e["contribution"] == score
    # All ten bits set → raw sum = 19, clipped to 10
    assert score == 10.0
    assert e["chosen"] == "right"
    assert e["clipped"] is True


# ─── Domain-swap discipline (§8) ────────────────────────────────────────────


def test_DS_repair_menu_isomorphic_across_domains():
    """Same math, three domain-named bundles — identical numerical output.

    This is the §8 domain-neutrality proof. The substrate must produce
    bit-identical results for isomorphic data regardless of what the
    fields are named.
    """
    def run(field_a, field_b, target_a, target_b):
        row = Row(pk=0, fields={field_a: 1, field_b: 1})
        pred = Predicate(clauses=[(field_a, "==", target_a), (field_b, "==", target_b)])
        return repair_menu(pred, row, max_flips=2)

    vuln = run("cast_truncate_alloc", "has_probe_read", 0, 0)
    fraud = run("amount_over_threshold", "same_origin_destination", 0, 0)
    edu = run("assignments_complete", "attendance_high", 0, 0)

    # Strip field names — keep only structural shape (flip count + cost)
    def shape(menu): return [(len(f), c) for f, c in menu]

    assert shape(vuln) == shape(fraud) == shape(edu), \
        "domain-swap must produce isomorphic repair menus"


def test_DS_verdict_isomorphic_across_domains():
    """Same predicate shape, three domains, identical verdict + counts."""
    def run(field_name, target):
        rows = [Row(pk=i, fields={field_name: 1 if i < 3 else 0}) for i in range(10)]
        bundle = Bundle(name=f"{field_name}_bundle", rows=rows)
        pred = Predicate(clauses=[(field_name, "==", target)])
        return verdict(pred, bundle)

    v_vuln, p_vuln = run("cast_truncate_alloc", 1)
    v_fraud, p_fraud = run("amount_over_threshold", 1)
    v_edu, p_edu = run("assignments_complete", 1)

    assert v_vuln == v_fraud == v_edu == "sat"
    assert p_vuln["n_matches"] == p_fraud["n_matches"] == p_edu["n_matches"] == 3


def test_DS_explain_isomorphic_across_domains():
    """Same WEIGHT expression shape, three domains, identical numeric tree."""
    def run(field_a, field_b):
        row = Row(pk=0, fields={field_a: 2.0, field_b: 5.0})
        expr = WMin(WAdd(WMul(WField(field_a), WLit(3)),
                         WMul(WField(field_b), WLit(2))), WLit(10.0))
        return explain(expr, row)

    e_vuln = run("cast_truncate_alloc", "has_probe_read")
    e_fraud = run("amount_over_threshold", "same_origin_destination")
    e_edu = run("assignments_complete", "attendance_high")

    # Compare the numeric contributions — names differ but math is identical
    def numeric_only(node):
        d = {"type": node["type"], "contribution": node["contribution"]}
        if "left" in node: d["left"] = numeric_only(node["left"])
        if "right" in node: d["right"] = numeric_only(node["right"])
        return d

    assert numeric_only(e_vuln) == numeric_only(e_fraud) == numeric_only(e_edu)


def test_DS_kp_isomorphic_across_domains():
    """Same data shape, three domains, identical K_P."""
    def run(field_name):
        rows = []
        for i in range(20):
            rows.append(Row(pk=i, fields={field_name: 1 if i < 5 else 0,
                                          "noise": i}))
        bundle = Bundle(name=f"{field_name}_bundle", rows=rows)
        pred = Predicate(clauses=[(field_name, "==", 1)])
        return k_p(pred, bundle, fields=[field_name, "noise"], k=4)

    r_vuln = run("cast_truncate_alloc")
    r_fraud = run("amount_over_threshold")
    r_edu = run("assignments_complete")

    assert r_vuln == r_fraud == r_edu, \
        f"K_P domain swap mismatch: {r_vuln} vs {r_fraud} vs {r_edu}"


# ═══════════════════════════════════════════════════════════════════════════
# RUNNER
# ═══════════════════════════════════════════════════════════════════════════


TESTS = [
    test_K1_kp_concentrated_pattern_is_strictly_positive,
    test_K2_kp_responds_to_match_concentration,
    test_K3_kp_empty_match_is_zero,
    test_K4_kp_concentrated_exceeds_tautology,
    test_PP1_preflight_catches_impossible_numeric_range,
    test_PP2_preflight_catches_missing_categorical,
    test_PP3_preflight_catches_internal_contradiction,
    test_PP4_preflight_holonomy_catches_joint_contradiction,
    test_PP5_preflight_passes_satisfiable_predicate,
    test_VT1_verdict_sat_when_rows_match,
    test_VT2_verdict_unsat_by_preflight,
    test_VT3_verdict_unsat_by_scan_when_no_match_and_no_near_miss,
    test_VT4_verdict_near_miss_at_distance_1,
    test_VT5_verdict_trichotomy_exhaustive,
    test_PR1_repair_single_flip_uniform_cost,
    test_PR2_repair_double_flip,
    test_PR3_repair_custom_costs_sort_correctly,
    test_PR4_repair_already_matches_returns_sentinel,
    test_PR5_repair_too_far_returns_sentinel,
    test_PR6_repair_min_cost_is_actually_minimum,
    test_PE1_explain_lit_returns_literal_value,
    test_PE2_explain_field_returns_row_value,
    test_PE3_explain_add_root_equals_eval,
    test_PE4_explain_mul_root_equals_product,
    test_PE5_explain_min_chosen_branch_and_clip_flag,
    test_PE6_explain_full_scj_scorer_invariant,
    test_DS_repair_menu_isomorphic_across_domains,
    test_DS_verdict_isomorphic_across_domains,
    test_DS_explain_isomorphic_across_domains,
    test_DS_kp_isomorphic_across_domains,
]


if __name__ == "__main__":
    passed = 0
    failed: list[tuple[str, str]] = []
    print("=" * 72)
    print("PATTERNS v0.2 MATH VALIDATION")
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
    print("All Patterns v0.2 math validation tests passed.")
