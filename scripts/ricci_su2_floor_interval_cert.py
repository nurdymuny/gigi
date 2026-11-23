"""Interval-in-t certification of the floor family (docs/research/floor-family-squashing-2026-11-19.md).

For a sector j and a slice t in [a, b] of the Berger squashing g = (t, 1, 1): the matrix A(t) - (lambda_t(j) + delta),
A = H + G G^*, is enclosed for ALL t in [a, b] at once (mpmath interval context, t and lambda_t(j) as intervals),
realified, and its lower-triangle midpoint M factored LDL^T. With eps = rho + backward error (as in the Floor Theorem
certificate), exactly four negative pivots and eps < delta prove, for every t in [a, b]: A(t) has at most two
eigenvalues below lambda_t(j) + delta - eps; lambda_t(j) is an exact eigenvalue of multiplicity two
(scripts/ricci_su2_sector_formula_squashed.py, symbolic in t); so lambda_t(j) is the lowest eigenvalue, twice, and
every other is at least lambda_t(j) + delta - eps > lambda_t(j). Its eigenvector lies in the gauge slice, so it is the
floor there. The assembly and count code are taken verbatim from scripts/ricci_su2_gap_certificates_t.py.

Error budget: outward-rounded (scripts/ricci_su2_outward.py; repair of the external review, 20 November 2026, finding 1).
usage: python scripts/ricci_su2_floor_interval_cert.py OUT.json TMIN TMAX SECTORS [NPROC]
"""
import json
import multiprocessing as mp
import os
import sys
import time
from itertools import product
from pathlib import Path

import mpmath
import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
_argv, sys.argv = sys.argv, [sys.argv[0]]
import ricci_su2_gap_certificates_t as GC      # noqa: E402  (module defaults; only its helpers are used)
import ricci_su2_outward as O                 # noqa: E402
sys.argv = _argv
background, PAIRS, cross_mat = GC.background, GC.PAIRS, GC.cross_mat
PREC = 96


def count_interval(j, Tv, mu_iv):
    iv = mpmath.iv
    iv = mpmath.iv
    iv.prec = PREC
    mpmath.mp.prec = PREC
    one = iv.mpf(1)
    s, c, m, f, gmat = background(iv.sqrt, one, Tv)
    d = j + 1
    half = mpmath.mpf(j) / 2
    mvals = [half - w for w in range(d)]
    # rho_x = -i (Jp + Jp^T), rho_y = -(Jp - Jp^T), rho_z = -2 i Jz ; Jp[w-1, w] = sqrt(s(s+1) - m(m+1))
    jp = {}
    for w in range(1, d):
        mm = mvals[w]
        jp[(w - 1, w)] = iv.sqrt(iv.mpf(half * (half + 1) - mm * (mm + 1)))
    rho = [{}, {}, {}]                       # entries (w, w') -> (re, im)
    for (a, b), v in jp.items():
        rho[0][(a, b)] = (0, -v); rho[0][(b, a)] = (0, -v)
        rho[1][(a, b)] = (-v, 0); rho[1][(b, a)] = (v, 0)
    for w in range(d):
        rho[2][(w, w)] = (0, -2 * iv.mpf(mvals[w]))
    idx = lambda fr, lie, w: w * 9 + fr * 3 + lie       # noqa: E731  band-preserving (w-major)
    n = 9 * d

    def cadd(dct, key, val):
        a = dct.get(key, (0, 0))
        dct[key] = (a[0] + val[0], a[1] + val[1])

    # D rows (p, lie, w); columns (frame, lie, w)
    Dm = {}
    for p, (i, jj) in enumerate(PAIRS):
        for a in range(3):
            for w in range(d):
                row = idx(p, a, w)
                for (sgn, fr, gi, mi) in ((1, jj, i, i), (-1, i, jj, jj)):
                    for (w1, w2), z in rho[gi].items():
                        if w1 == w:
                            cadd(Dm, (row, idx(fr, a, w2)), (sgn * z[0] / s[gi], sgn * z[1] / s[gi]))
                    cm = cross_mat(m[mi])
                    for a2 in range(3):
                        if not (isinstance(cm[a][a2], int) and cm[a][a2] == 0):
                            cadd(Dm, (row, idx(fr, a2, w)), (sgn * cm[a][a2], 0))
                for k in range(3):
                    if not (isinstance(c[(i, jj, k)], int) and c[(i, jj, k)] == 0):
                        cadd(Dm, (row, idx(k, a, w)), (-c[(i, jj, k)], 0))
    Gm = {}                                    # rows (frame, lie, w), cols (lie, w)
    for i in range(3):
        for a in range(3):
            for w in range(d):
                row = idx(i, a, w)
                for (w1, w2), z in rho[i].items():
                    if w1 == w:
                        cadd(Gm, (row, w2 * 3 + a), (z[0] / s[i], z[1] / s[i]))
                cm = cross_mat(m[i])
                for a2 in range(3):
                    if not (isinstance(cm[a][a2], int) and cm[a][a2] == 0):
                        cadd(Gm, (row, w * 3 + a2), (cm[a][a2], 0))

    def gram(S, conj_left):
        """sum_k conj(S[k, x]) S[k, y]  (conj_left) or  sum_k S[x, k] conj(S[y, k])."""
        by = {}
        for (r, cc), z in S.items():
            by.setdefault(r if conj_left else cc, []).append((cc if conj_left else r, z))
        out = {}
        for _, entries in by.items():
            for x, zx in entries:
                for y, zy in entries:
                    if conj_left:   # conj(zx) * zy
                        val = (zx[0] * zy[0] + zx[1] * zy[1], zx[0] * zy[1] - zx[1] * zy[0])
                    else:           # zx * conj(zy)
                        val = (zx[0] * zy[0] + zx[1] * zy[1], zx[1] * zy[0] - zx[0] * zy[1])
                    cadd(out, (x, y), val)
        return out
    A = gram(Dm, True)
    for (x, y), z in gram(Gm, False).items():
        cadd(A, (x, y), z)
    for (i, jj) in PAIRS:
        blk = cross_mat(f[(i, jj)])
        for a, a2, w in product(range(3), range(3), range(d)):
            if not (isinstance(blk[a][a2], int) and blk[a][a2] == 0):
                cadd(A, (idx(i, a, w), idx(jj, a2, w)), (-blk[a][a2], 0))
                cadd(A, (idx(jj, a2, w), idx(i, a, w)), (-blk[a][a2], 0))
    # realify: C^n -> R^2n, index 2x (re), 2x+1 (im)
    N = 2 * n
    mid, bud = {}, O.Budget()
    Ar = {}
    for (x, y), z in A.items():
        re, im = z
        for (r_, c_, v) in ((2 * x, 2 * y, re), (2 * x, 2 * y + 1, -im if not isinstance(im, int) else 0),
                            (2 * x + 1, 2 * y, im), (2 * x + 1, 2 * y + 1, re)):
            if isinstance(v, int) and v == 0:
                continue
            Ar[(r_, c_)] = Ar.get((r_, c_), 0) + v
    for x in range(N):
        Ar[(x, x)] = Ar.get((x, x), 0) - mu_iv
    band = 0
    # M is the symmetric matrix built from the LOWER triangle (and diagonal) of the enclosure: that is exactly
    # what the factorization reads. The exact matrix is symmetric and each lower interval encloses its entry, so
    # ||exact - M||_2 <= ||exact - M||_F <= sqrt(sum_diag r^2 + 2 sum_lower r^2) =: rho.
    for (r_, c_), v in Ar.items():
        band = max(band, abs(r_ - c_))
        if r_ < c_:
            continue
        # stored midpoint and OUTWARD radius about it (ricci_su2_outward; the external review, finding 1)
        m_, r1 = O.mid_rad(v if not isinstance(v, int) else iv.mpf(v))
        mid[(r_, c_)] = m_
        bud.add(r_, c_, r1)
    # LDL^T without pivoting, banded, at PREC bits
    Lrows = [dict() for _ in range(N)]
    dvals = []
    for k in range(N):
        lo = max(0, k - band)
        dk = mid.get((k, k), mpmath.mpf(0)) - mpmath.fsum(Lrows[k].get(t, 0) ** 2 * dvals[t] for t in range(lo, k))
        dvals.append(dk)
        if dk == 0:
            raise RuntimeError("zero pivot: refusing to certify")
        for i2 in range(k + 1, min(N, k + band + 1)):
            aik = mid.get((i2, k), mpmath.mpf(0))
            sm = mpmath.fsum(Lrows[i2].get(t, 0) * Lrows[k].get(t, 0) * dvals[t] for t in range(max(lo, i2 - band), k))
            if aik != 0 or sm != 0:
                Lrows[i2][k] = (aik - sm) / dk
        Lrows[k][k] = mpmath.mpf(1)
    # || |L||D||L^T| ||_F, outward
    terms = {}
    for i2 in range(N):
        for t, lit in Lrows[i2].items():
            for k2 in range(max(0, t), min(N, t + band + 1)):
                lkt = Lrows[k2].get(t)
                if lkt is not None:
                    terms.setdefault((i2, k2), []).append((lit, dvals[t], lkt))
    # a posteriori residual of the stored factors, outward (replaces the Higham-type bound)
    res_iv = O.residual_frob(N, lambda i2, k2: mid.get((i2, k2), mpmath.mpf(0)), Lrows, dvals, band)
    # rigorous: |E| <= R entrywise, R symmetric non-negative => ||E||_2 <= ||R||_2 <= max row sum of R
    rho = bud.rho()
    eps, bwd = O.eps_residual(rho, res_iv)
    neg = sum(1 for dv in dvals if dv < 0)
    return {"neg": neg, "eps": float(eps), "rho": float(rho), "residual": float(bwd),
            "min_abs_pivot": float(min(abs(dv) for dv in dvals)), "N": N}


def lam_iv(j, Tv):
    iv = mpmath.iv
    return (iv.mpf(j * (j + 2)) - 2 * (Tv - 3) * j - 2 * (Tv - 2) * (Tv + 4)) / Tv


def lam_float(j, t):
    return (j * (j + 2) - 2 * (t - 3) * j - 2 * (t - 2) * (t + 4)) / t


def gap_estimate(j, a, b):
    """Double-precision estimate of (next eigenvalue - lambda) on [a, b]; only used to choose delta."""
    import ricci_su2_sector_gauge_check as C
    best = np.inf
    for t in np.linspace(a, b, 3):
        H, G = C.hessian_and_gauge([t, 1, 1], j)
        ev = np.linalg.eigvalsh(H + G @ G.conj().T)
        best = min(best, ev[2] - lam_float(j, t))
    return best


def task(arg):
    j, a, b = arg
    mpmath.iv.prec = PREC; mpmath.mp.prec = PREC
    delta = 0.5 * gap_estimate(j, a, b)
    Tv = mpmath.iv.mpf([mpmath.mpf(a), mpmath.mpf(b)])
    r = count_interval(j, Tv, lam_iv(j, Tv) + mpmath.iv.mpf(delta))
    ok = (r["neg"] == 4) and (r["eps"] < delta)
    return dict(r, j=j, a=a, b=b, delta=delta, certified=bool(ok))


def main():
    out = Path(sys.argv[1]); tmin, tmax = float(sys.argv[2]), float(sys.argv[3])
    sectors = [int(x) for x in sys.argv[4].split(",")]
    nproc = int(sys.argv[5]) if len(sys.argv) > 5 else 6
    t0 = time.time()
    pending = []
    for j in sectors:
        edges = np.linspace(tmin, tmax, 30)
        pending += [(j, float(x), float(y)) for x, y in zip(edges[:-1], edges[1:])]
    done, failed_min = [], []
    with mp.get_context("spawn").Pool(nproc) as pool:
        while pending:
            nxt = []
            for r in pool.imap_unordered(task, pending):
                if r["certified"]:
                    done.append(r)
                elif r["b"] - r["a"] < 1e-4:
                    failed_min.append(r)
                else:
                    m = 0.5 * (r["a"] + r["b"])
                    nxt += [(r["j"], r["a"], m), (r["j"], m, r["b"])]
            print(f"round: certified {len(done)}, splitting {len(nxt) // 2}, gave up {len(failed_min)}, "
                  f"{time.time() - t0:.0f} s", flush=True)
            pending = nxt
    # coverage check: for each sector, the certified slices tile [tmin, tmax] with no gap
    cover = {}
    for j in sectors:
        sl = sorted((r["a"], r["b"]) for r in done if r["j"] == j)
        ok = bool(sl) and abs(sl[0][0] - tmin) < 1e-15 and abs(sl[-1][1] - tmax) < 1e-15 and all(
            abs(sl[i][1] - sl[i + 1][0]) < 1e-15 for i in range(len(sl) - 1))
        cover[str(j)] = {"slices": len(sl), "covered": ok,
                         "min_margin_delta_minus_eps": min(r["delta"] - r["eps"] for r in done if r["j"] == j)}
    res = {"kind": "interval-in-t certification of the floor family", "precision_bits": PREC,
           "t_range": [tmin, tmax], "sectors": sectors, "coverage": cover,
           "all_certified": all(c["covered"] for c in cover.values()) and not failed_min,
           "failed_at_min_width": failed_min, "slices": sorted(done, key=lambda r: (r["j"], r["a"])),
           "seconds": round(time.time() - t0, 1)}
    out.write_text(json.dumps(res, indent=1) + "\n")
    print(json.dumps({k: v for k, v in res.items() if k != "slices"}, indent=1))


if __name__ == "__main__":
    main()
