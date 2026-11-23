"""Outward-rounded error budget for the interval inertia certificates (repair of the external review, 20 November 2026,
finding 1).

The certificates replace an interval matrix by a point matrix M (the stored midpoints of its lower triangle), factor M
as L D L^T in floating point, and certify with eps = rho + gamma_N || |L||D||L^T| ||_F, where rho bounds
||exact - M||_2. Previously the radius was computed as (hi - lo)/2 times (1 + 2^(8 - PREC)), which does not cover the
rounding of the stored midpoint, and rho, the Frobenius term, gamma and eps were accumulated with round-to-nearest.

Here every quantity in the budget is an upper bound by construction:
  * mid_rad(v): m = a stored point (any rounding), r = upper bound of max |x - m| over x in v, from the interval
    v - [m, m] evaluated with outward rounding. So |exact - m| <= r for the exact entry in v.
  * Budget: radii squared and row sums, |L||D||L^T| and its Frobenius norm, gamma_N = N u / (1 - N u) and
    eps = rho + gamma_N * frob, all accumulated in mpmath's interval context; only upper endpoints are used.
Requires iv.prec <= mp.prec, so that interval endpoints convert to mpf exactly.
"""
import mpmath

iv = mpmath.iv
_IV = type(iv.mpf(0))


def _check_prec():
    if iv.prec > mpmath.mp.prec:
        raise RuntimeError("outward budget needs iv.prec <= mp.prec")


def up(x):
    """Upper endpoint of an interval (exact as an mpf when iv.prec <= mp.prec)."""
    return mpmath.mpf(x.b)


def mid_rad(v):
    """(m, r): a stored point m and r >= max |x - m| over x in the interval v (outward)."""
    _check_prec()
    if not isinstance(v, _IV):
        v = iv.mpf(v)
    a, b = mpmath.mpf(v.a), mpmath.mpf(v.b)
    m = (a + b) / 2
    w = v - iv.mpf(m)
    r = max(-mpmath.mpf(w.a), mpmath.mpf(w.b), mpmath.mpf(0))
    return m, r


class Budget:
    """Accumulates radii of the lower triangle (rho = min(Frobenius bound, max row sum)), outward."""

    def __init__(self):
        self.rad2 = iv.mpf(0)
        self.rows = {}

    def add(self, r_, c_, r):
        ri = iv.mpf(r)
        self.rad2 += ri * ri if r_ == c_ else 2 * ri * ri
        self.rows[r_] = self.rows.get(r_, iv.mpf(0)) + ri
        if r_ != c_:
            self.rows[c_] = self.rows.get(c_, iv.mpf(0)) + ri

    def rho(self):
        frob = up(iv.sqrt(self.rad2))
        if not self.rows:
            return frob
        return min(frob, max(up(v) for v in self.rows.values()))


def backward_frob(entries):
    """Upper bound of sqrt(sum_ik (sum_t |L_it| |d_t| |L_kt|)^2), given an iterable of ((i, k), [(l_it, d_t, l_kt), ...])."""
    tot = iv.mpf(0)
    for _, terms in entries:
        s = iv.mpf(0)
        for lit, dt, lkt in terms:
            s += iv.mpf(abs(lit)) * iv.mpf(abs(dt)) * iv.mpf(abs(lkt))
        tot += s * s
    return iv.sqrt(tot)


def eps_upper(rho, frob_iv, N, prec):
    """Upper bound of rho + gamma_N * frob, gamma_N = N u / (1 - N u), u = 2^-prec."""
    u = iv.mpf(2) ** (-prec)
    gamma = N * u / (1 - N * u)
    e = iv.mpf(rho) + gamma * frob_iv
    return up(e), up(gamma * frob_iv)


def residual_frob(N, mid_lower, Lrows, d, band=None):
    """A posteriori: upper bound of ||M - L D L^T||_F, with M the stored symmetric point matrix (lower triangle
    mid_lower(i, k), i >= k) and L (unit lower triangular, Lrows[i] = {t: L_it}), D = diag(d) the stored factors.
    L D L^T is then an exact real matrix congruent to D (Sylvester), and ||exact - L D L^T||_2 <= rho + this bound.
    No rounding-error constant of the factorization is needed (replaces the Higham-type bound; second review,
    20 November 2026)."""
    tot = iv.mpf(0)
    for i in range(N):
        lo = 0 if band is None else max(0, i - band)
        for k in range(lo, i + 1):
            s = iv.mpf(mid_lower(i, k))
            Li, Lk = Lrows[i], Lrows[k]
            for t, lit in Li.items():
                if t > k:
                    continue
                lkt = Lk.get(t)
                if lkt is None:
                    continue
                s -= iv.mpf(lit) * iv.mpf(d[t]) * iv.mpf(lkt)
            m = iv.mpf(max(abs(mpmath.mpf(s.a)), abs(mpmath.mpf(s.b))))     # |e_ik| <= m (s encloses e_ik)
            tot += m * m if i == k else 2 * m * m
    return iv.sqrt(tot)


def eps_residual(rho, res_iv):
    """Upper bound of rho + ||M - L D L^T||_F."""
    return up(iv.mpf(rho) + res_iv), up(res_iv)
