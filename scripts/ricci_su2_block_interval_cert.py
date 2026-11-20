"""Certify the floor law in EVERY sector j <= JMAX, for every t in [TMIN, TMAX], block by block.

Ingredients (all proved elsewhere):
  * Weitzenbock identity H + G G^* = sum_i X_i^* X_i + R0(t) for every t > 0, symbolic in t
    (scripts/ricci_su2_weitzenbock_symbolic_t.py; R0(t) entries read from its receipt).
  * The fibre charge q = m + c_f + c_l is conserved; the sector of spin l = j/2 is the direct sum of charge blocks
    of size <= 9 (scripts/ricci_su2_charge_block_direct.py, validated against the full sector matrices).
  * The twist is the only state of charge q = +-(l + 2); its block is 1 x 1 with entry lambda_t(j) exactly
    (Proposition 6.1, symbolic in j and t).
For every other block and every t-slice [a, b]: B(t) - lambda_t(j) - eta is enclosed for all t in the slice at once
(mpmath interval context; t, sqrt t, the ladder coefficients and R0(t) as intervals), realified, and its midpoint M
factored LDL^T. With rho = min(Frobenius, max row sum) of the radii and the backward-error bound, all pivots positive
and eps < eta give lambda_min(B(t)) >= lambda_t(j) + eta - eps > lambda_t(j) for every t in the slice. Together:
lambda_t(j) is the lowest eigenvalue of H + G G^* in sector j, multiplicity exactly two (the two twist blocks).

Error budget: outward-rounded (scripts/ricci_su2_outward.py; repair of Astra's review, 20 November 2026, finding 1).
usage: python scripts/ricci_su2_block_interval_cert.py OUT.json TMIN TMAX JMAX [NPROC] [JMIN]
"""
import json
import multiprocessing as mp
import sys
import time
from pathlib import Path

import mpmath
import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
_argv, sys.argv = sys.argv, [sys.argv[0]]
import ricci_su2_gap_certificates_t as GC      # noqa: E402
sys.argv = _argv
import ricci_su2_charge_block_direct as D      # noqa: E402
import ricci_su2_outward as O                  # noqa: E402

PREC = 64
HERE = Path(__file__).resolve().parent
R0_STR = json.loads((HERE.parent / "research/su2-floor-family-20261119/weitzenbock_symbolic_t.json").read_text())["R0"]
iv = mpmath.iv


def lam_float(t, j):
    return (j * (j + 2) - 2 * (t - 3) * j - 2 * (t - 2) * (t + 4)) / t


# ---- complex interval helpers: (re, im) pairs of iv numbers ----
def cz(x):
    return (iv.mpf(x), iv.mpf(0))


def cadd(a, b):
    return (a[0] + b[0], a[1] + b[1])


def cmul(a, b):
    return (a[0] * b[0] - a[1] * b[1], a[0] * b[1] + a[1] * b[0])


def cconj(a):
    return (a[0], -a[1])


def frame_iv(Tv):
    """Omega_i and R0 in the charge basis, as 9x9 complex-interval matrices, for t in the interval Tv."""
    s, c, m, f, gmat = GC.background(iv.sqrt, iv.mpf(1), Tv)
    one, zero = iv.mpf(1), iv.mpf(0)

    def cross(v):
        return [[zero, -v[2], v[1]], [v[2], zero, -v[0]], [-v[1], v[0], zero]]
    Om = []
    for i in range(3):
        cm = cross([x if not isinstance(x, int) else iv.mpf(x) for x in m[i]])
        G = [[gmat[i][a][b] if not isinstance(gmat[i][a][b], int) else iv.mpf(gmat[i][a][b]) for b in range(3)] for a in range(3)]
        O = [[zero] * 9 for _ in range(9)]
        for a in range(3):
            for b in range(3):
                for x in range(3):
                    for y in range(3):
                        v = (cm[x][y] if a == b else zero) - (G[a][b] if x == y else zero)
                        O[3 * a + x][3 * b + y] = O[3 * a + x][3 * b + y] + v
        Om.append(O)
    ns = {"sqrt": iv.sqrt, "t": Tv}
    R0 = [[eval(R0_STR[a][b].replace("sqrt", "sqrt"), {"__builtins__": {}}, ns) if R0_STR[a][b] != "0" else zero
           for b in range(9)] for a in range(9)]
    # charge basis of a 3-vector index under the rotation about axis 0: columns c = -1, 0, +1
    r2 = iv.sqrt(iv.mpf(2))
    U3 = [[cz(0), cz(1), cz(0)],
          [(one / r2, zero), cz(0), (one / r2, zero)],
          [(zero, -one / r2), cz(0), (zero, one / r2)]]
    charges = [-1, 0, 1]
    U = [[cmul(U3[a // 3][b // 3], U3[a % 3][b % 3]) for b in range(9)] for a in range(9)]
    ch = [(charges[b // 3], charges[b % 3]) for b in range(9)]

    def change(Mr):      # U^* M U, M real
        out = [[cz(0)] * 9 for _ in range(9)]
        MU = [[cz(0)] * 9 for _ in range(9)]
        for a in range(9):
            for b in range(9):
                acc = cz(0)
                for k in range(9):
                    if not (Mr[a][k] == 0):
                        acc = cadd(acc, cmul((Mr[a][k], zero), U[k][b]))
                MU[a][b] = acc
        for a in range(9):
            for b in range(9):
                acc = cz(0)
                for k in range(9):
                    acc = cadd(acc, cmul(cconj(U[k][a]), MU[k][b]))
                out[a][b] = acc
        return out
    return s, [change(O) for O in Om], change(R0), ch


def block_iv(j, q, Tv, fd):
    s, Omc, R0c, ch = fd
    l = j / 2
    states = [(q - ch[k][0] - ch[k][1], k) for k in range(9) if abs(q - ch[k][0] - ch[k][1]) <= l + 1e-9]
    n = len(states)
    targets, cols = {}, []
    for i in range(3):
        Xi = {}
        for col, (mm, k) in enumerate(states):
            if i == 0:
                acts = {mm: (iv.mpf(0), iv.mpf(-2 * mm))}
            else:
                up = l * (l + 1) - mm * (mm + 1); dn = l * (l + 1) - mm * (mm - 1)
                acts = {}
                if up > 0:
                    a_ = iv.sqrt(iv.mpf(up)); acts[mm + 1] = (iv.mpf(0), -a_) if i == 1 else (-a_, iv.mpf(0))
                if dn > 0:
                    a_ = iv.sqrt(iv.mpf(dn)); acts[mm - 1] = (iv.mpf(0), -a_) if i == 1 else (a_, iv.mpf(0))
            for mp_, cf in acts.items():
                key = (mp_, k); r = targets.setdefault(key, len(targets))
                v = (cf[0] / s[i], cf[1] / s[i])
                Xi[(r, col)] = cadd(Xi.get((r, col), cz(0)), v)
            for kp in range(9):
                v = Omc[i][kp][k]
                key = (mm, kp); r = targets.setdefault(key, len(targets))
                Xi[(r, col)] = cadd(Xi.get((r, col), cz(0)), v)
        cols.append(Xi)
    B = [[cz(0)] * n for _ in range(n)]
    for Xi in cols:
        byrow = {}
        for (r, c_), v in Xi.items():
            byrow.setdefault(r, []).append((c_, v))
        for r, ent in byrow.items():
            for a, va in ent:
                for b, vb in ent:
                    B[a][b] = cadd(B[a][b], cmul(cconj(va), vb))
    for a, (mm, k) in enumerate(states):
        for b, (m2, k2) in enumerate(states):
            if mm == m2:
                B[a][b] = cadd(B[a][b], R0c[k][k2])
    return states, B


def certify_block(j, q, a, b, eta, fd):
    mpmath.iv.prec = PREC; mpmath.mp.prec = PREC
    Tv = iv.mpf([mpmath.mpf(a), mpmath.mpf(b)])
    _, B = block_iv(j, q, Tv, fd)
    n = len(B); N = 2 * n
    lam = (iv.mpf(j * (j + 2)) - 2 * (Tv - 3) * j - 2 * (Tv - 2) * (Tv + 4)) / Tv
    mu = lam + iv.mpf(eta)
    A = [[None] * N for _ in range(N)]
    for x in range(n):
        for y in range(n):
            re, im = B[x][y]
            A[2 * x][2 * y] = re; A[2 * x][2 * y + 1] = -im; A[2 * x + 1][2 * y] = im; A[2 * x + 1][2 * y + 1] = re
    for x in range(N):
        A[x][x] = A[x][x] - mu
    # stored midpoints and OUTWARD radii about them (ricci_su2_outward; Astra's review, finding 1)
    mid = [[None] * N for _ in range(N)]; bud = O.Budget()
    for r in range(N):
        for c_ in range(r + 1):
            m_, r1 = O.mid_rad(A[r][c_])
            mid[r][c_] = mid[c_][r] = m_
            bud.add(r, c_, r1)
    rho = bud.rho()
    L = [[mpmath.mpf(0)] * N for _ in range(N)]; d = []
    for k in range(N):
        dk = mid[k][k] - mpmath.fsum(L[k][s_] ** 2 * d[s_] for s_ in range(k))
        if dk <= 0:
            return False, float(dk), float(rho)
        d.append(dk)
        for i2 in range(k + 1, N):
            L[i2][k] = (mid[i2][k] - mpmath.fsum(L[i2][s_] * L[k][s_] * d[s_] for s_ in range(k))) / dk
        L[k][k] = mpmath.mpf(1)
    frob = O.backward_frob((((i2, k2), [(L[i2][s_], d[s_], L[k2][s_]) for s_ in range(min(i2, k2) + 1)])
                            for i2 in range(N) for k2 in range(N)))
    eps, _ = O.eps_upper(rho, frob, N, PREC)
    return bool(eps < mpmath.mpf(eta)), float(eps), float(rho)


def task(arg):
    j, tmin, tmax = arg
    t0 = time.time()
    l = j / 2
    qs = [q for q in np.arange(-l - 2, l + 2 + 1e-9, 1.0) if abs(abs(q) - (l + 2)) > 1e-9]
    fdcache = {}
    nslices, fails, worst = 0, [], np.inf
    for q in qs:
        pend = [(tmin + (tmax - tmin) * k / 4, tmin + (tmax - tmin) * (k + 1) / 4) for k in range(4)]
        while pend:
            a, b = pend.pop()
            ts = np.linspace(a, b, 3)
            marg = min(np.linalg.eigvalsh(D.block(t, j, q, D.frame_data(t))[1])[0] - lam_float(t, j) for t in ts)
            if marg <= 0:
                fails.append((float(q), a, b, "margin<=0")); continue
            eta = 0.5 * marg
            if (a, b) not in fdcache:
                mpmath.iv.prec = PREC
                fdcache[(a, b)] = frame_iv(iv.mpf([mpmath.mpf(a), mpmath.mpf(b)]))
            ok, eps, _ = certify_block(j, q, a, b, eta, fdcache[(a, b)])
            if ok:
                nslices += 1; worst = min(worst, eta - eps)
            elif b - a < 1e-4:
                fails.append((float(q), a, b, eps))
            else:
                m_ = 0.5 * (a + b); pend += [(a, m_), (m_, b)]
    return {"j": j, "blocks": len(qs), "slices": nslices, "fails": fails, "min_margin_eta_minus_eps": float(worst),
            "seconds": round(time.time() - t0, 1)}


def main():
    out = Path(sys.argv[1]); tmin, tmax, jmax = float(sys.argv[2]), float(sys.argv[3]), int(sys.argv[4])
    nproc = int(sys.argv[5]) if len(sys.argv) > 5 else 4
    jmin = int(sys.argv[6]) if len(sys.argv) > 6 else 0
    rows = []
    with mp.get_context("spawn").Pool(nproc) as pool:
        for r in pool.imap_unordered(task, [(j, tmin, tmax) for j in range(jmax, jmin - 1, -1)]):
            rows.append(r)
            print(f"j={r['j']}: {r['blocks']} blocks, {r['slices']} certified slices, fails {len(r['fails'])}, "
                  f"min margin {r['min_margin_eta_minus_eps']:.3g}, {r['seconds']} s", flush=True)
            out.write_text(json.dumps({"t_range": [tmin, tmax], "jmin": jmin, "jmax": jmax, "rows": sorted(rows, key=lambda x: x["j"])}, indent=1) + "\n")
    allok = len(rows) == jmax - jmin + 1 and all(not r["fails"] for r in rows)
    out.write_text(json.dumps({"t_range": [tmin, tmax], "jmin": jmin, "jmax": jmax, "all_certified": allok,
                               "rows": sorted(rows, key=lambda x: x["j"])}, indent=1) + "\n")
    print("ALL CERTIFIED" if allok else "NOT ALL CERTIFIED")


if __name__ == "__main__":
    main()
