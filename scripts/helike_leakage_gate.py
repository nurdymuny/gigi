"""Relative cutoff-leakage gate, as registered in docs/research/helike-leakage-gate-amendment-2026-11-24.md.

The leakage only enters the certificate additively, through ||(H-E) eta psi|| <= r + l. The gate is therefore

    l_upper <= KAPPA * r_lower,   KAPPA = 1/1000,

with r_lower a RIGOROUS lower bound on ||(H-E)psi||/||psi|| (an interval enclosure with provenance), never a
floating estimate. Without one the gate is OPEN. The absolute 1e-10 line is kept only as a labelled diagnostic.
The tail-squared (<= 1e-12) and normalisation-width (<= 1e-10) budgets still gate.

All comparisons are exact: binary64 values and dyadic Arb endpoints are converted exactly to Fractions.
helike_cutoff_bounds.classify is deliberately untouched: it replays the historical receipt under its own rule.
"""
from fractions import Fraction
import math

KAPPA = Fraction(1, 1000)
ABSOLUTE_DIAGNOSTIC = Fraction(1, 10**10)
TAIL_SQ_BUDGET = Fraction(1, 10**12)
NORM_WIDTH_BUDGET = Fraction(1, 10**10)
DIAGNOSTIC_LABEL = 'diagnostic only (absolute 1e-10 line, withdrawn as a gate by the 2026-11-24 amendment)'


def exact(x):
    """Exact Fraction of a binary64 value or Fraction; refuses non-finite input."""
    if isinstance(x, Fraction):
        return x
    if isinstance(x, bool) or not isinstance(x, (int, float)) or not math.isfinite(x):
        raise ValueError(f'finite number required, got {x!r}')
    return Fraction(x)


def dyadic(d):
    """Exact value of a serialised Arb endpoint {'mantissa': str, 'exponent': int}."""
    m, e = int(d['mantissa']), int(d['exponent'])
    return Fraction(m) * (Fraction(2) ** e)


class RigorousLower:
    """A rigorous lower bound with its provenance. A bare float is not one."""
    __slots__ = ('value', 'source', 'sha256', 'field', 'status')

    def __init__(self, value, source, sha256, field, status):
        if status != 'INTERVAL_ENCLOSURE':
            raise ValueError(f'a rigorous lower bound needs an interval enclosure, not status {status!r}')
        if not (isinstance(sha256, str) and len(sha256) == 64):
            raise ValueError('provenance hash required')
        v = exact(value)
        if v < 0:
            raise ValueError('a residual norm lower bound cannot be negative')
        self.value, self.source, self.sha256, self.field, self.status = v, source, sha256, field, status


def relative_gate(leakage_upper, r_lower):
    """'PASS' if l_upper <= KAPPA * r_lower, 'FAIL' if not, 'OPEN' without a positive rigorous r_lower."""
    value = r_lower.value if isinstance(r_lower, RigorousLower) else None
    if value is None or value == 0:
        return 'OPEN'
    return 'PASS' if exact(leakage_upper) <= KAPPA * value else 'FAIL'


def residual_bound(r_upper, leakage_upper, q_lower):
    """Projected residual bound R = (r + l)/q (registered construction)."""
    q = exact(q_lower)
    if q <= 0:
        raise ValueError('q_lower must be positive')
    return (exact(r_upper) + exact(leakage_upper)) / q


def temple_enlargement(r_upper, leakage_upper, q_lower):
    """How much the leakage enlarges R^2 in a Kato-Temple bound: (2 r l + l^2)/q^2."""
    q = exact(q_lower)
    if q <= 0:
        raise ValueError('q_lower must be positive')
    r, l = exact(r_upper), exact(leakage_upper)
    return (2 * r * l + l * l) / (q * q)


def classify_state(tail_sq_upper, norm_sq_lower, norm_sq_upper, leakage_upper, r_lower):
    tail_ok = exact(tail_sq_upper) <= TAIL_SQ_BUDGET
    norm_ok = exact(norm_sq_upper) - exact(norm_sq_lower) <= NORM_WIDTH_BUDGET
    gate = relative_gate(leakage_upper, r_lower)
    absolute = exact(leakage_upper) <= ABSOLUTE_DIAGNOSTIC
    ratio = (exact(leakage_upper) / r_lower.value) if isinstance(r_lower, RigorousLower) and r_lower.value > 0 else None
    return {
        'tail_budget_passed': tail_ok,
        'norm_width_budget_passed': norm_ok,
        'relative_gate': gate,
        'leakage_over_r_lower_upper': None if ratio is None else float(ratio) if ratio == 0 else math.nextafter(float(ratio), math.inf),
        'absolute_1e-10_diagnostic': absolute,
        'absolute_1e-10_diagnostic_label': DIAGNOSTIC_LABEL,
        'state_passed': tail_ok and norm_ok and gate == 'PASS',
    }


def overall(states, expected):
    """PASS only if every expected state passes; FAIL if any fails; otherwise OPEN. Missing states refuse."""
    if len(states) != expected:
        raise ValueError(f'expected {expected} states, got {len(states)}')
    if all(s['state_passed'] for s in states):
        return 'PASS'
    if any(s['relative_gate'] == 'FAIL' or (not s['state_passed'] and s['relative_gate'] == 'PASS') for s in states):
        return 'FAIL'
    return 'OPEN'


# ---------------------------------------------------------------------------------------------------------------
# Flux-grid domain gate, amendment §5.7 (docs/research/helike-flux-method-amendment-2026-11-25.md), registered
# 26 November before code and before any grid value. The relative rule above tightens as r shrinks with the basis;
# the grid needs the cut's energy shift against its gap tolerance instead:
#
#     e_cut = l + 2 t r <= 3e-4   (one tenth of the 0.003 gap tolerance),
#
# l, t, r the rigorous UPPER bounds on leakage, tail and residual. The tail and normalisation budgets are unchanged.
# The ratio l / r_lower is kept in the receipt as relative_ratio_reported and never decides.

CUT_ENERGY_BUDGET = Fraction(3, 10**4)


class RigorousUpper:
    """A rigorous upper bound with its provenance (the residual upper bound r from an interval enclosure)."""
    __slots__ = ('value', 'source', 'sha256', 'field', 'status')

    def __init__(self, value, source, sha256, field, status):
        if status != 'INTERVAL_ENCLOSURE':
            raise ValueError(f'a rigorous upper bound needs an interval enclosure, not status {status!r}')
        if not (isinstance(sha256, str) and len(sha256) == 64):
            raise ValueError('provenance hash required')
        v = exact(value)
        if v < 0:
            raise ValueError('a norm upper bound cannot be negative')
        self.value, self.source, self.sha256, self.field, self.status = v, source, sha256, field, status


def cut_energy(leakage_upper, tail_upper, r_upper):
    """e_cut = l + 2 t r, exactly; None if r has no rigorous enclosure."""
    if not isinstance(r_upper, RigorousUpper):
        return None
    l, t, r = exact(leakage_upper), exact(tail_upper), r_upper.value
    e_cut = l + 2 * t * r
    return e_cut


def cut_gate(e_cut):
    if e_cut is None:
        return 'OPEN'
    return 'PASS' if e_cut <= CUT_ENERGY_BUDGET else 'FAIL'


def classify_state_cut(tail_sq_upper, norm_sq_lower, norm_sq_upper, leakage_upper, tail_upper, r_upper, r_lower):
    """One gated state under §5.7. r_lower only feeds the reported ratio."""
    tail_ok = exact(tail_sq_upper) <= TAIL_SQ_BUDGET
    norm_ok = exact(norm_sq_upper) - exact(norm_sq_lower) <= NORM_WIDTH_BUDGET
    e_cut = cut_energy(leakage_upper, tail_upper, r_upper)
    gate = cut_gate(e_cut)
    ratio = (exact(leakage_upper) / r_lower.value) if isinstance(r_lower, RigorousLower) and r_lower.value > 0 else None
    return {
        'tail_budget_passed': tail_ok,
        'norm_width_budget_passed': norm_ok,
        'cut_energy_upper': None if e_cut is None else math.nextafter(float(e_cut), math.inf) if Fraction(float(e_cut)) < e_cut else float(e_cut),
        'cut_gate': gate,
        'relative_ratio_reported': None if ratio is None else math.nextafter(float(ratio), math.inf),
        'relative_ratio_label': 'reported only (§5.7); not a gate',
        'state_passed': tail_ok and norm_ok and gate == 'PASS',
    }


def overall_cut(rows):
    """PASS only if every gated state passes; FAIL if any check fails with the enclosures resolved; else OPEN."""
    if all(r['state_passed'] for r in rows):
        return 'PASS'
    if any(r['cut_gate'] != 'OPEN' and not r['state_passed'] for r in rows):
        return 'FAIL'
    return 'OPEN'
