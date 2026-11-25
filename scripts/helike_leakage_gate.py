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
