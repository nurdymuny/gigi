"""Small-j half of the all-sector floor proof for the circle-symmetric family on a 3-D domain (21 November 2026).

For every sector j < JMAX and every non-twist charge block q, certify lambda_min(B_{j,q}(x, y, t)) > lambda_j(x, y, t)
for ALL (x, y, t) in the box D, by adaptive subdivision of D. On each sub-box:
  * the block (real symmetric: 2i Omega_+- is real in this family) is enclosed entrywise in interval arithmetic from the
    symbolic rows (scripts/ricci_su2_exact_edge.py construction; w, C, 2i Omega_+- lambdified with interval sqrt);
  * A = B - (lambda_j + eta) I, with lambda_j enclosed and eta = half the double-precision margin at the box centre;
  * stored midpoints, outward radii and rho (scripts/ricci_su2_outward.py), LDL^T, a posteriori residual
    ||M - L D L^T||_F (outward); certified if every pivot is positive and rho + residual < eta.
A sub-box that fails is split along its widest (relative to D) side, up to a depth limit.
The interval block at a point is first checked against the double-precision block (EE.Blocks).
usage: python scripts/ricci_su2_family_block_cert.py OUT.json JMIN JMAX XMIN XMAX YMIN YMAX TMIN TMAX [NPROC]
"""
import json
import sys
import time
from pathlib import Path

import mpmath
import numpy as np
import sympy as sp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_outward as O                  # noqa: E402

iv = mpmath.iv
PREC = 64
_S = {}


def symbolic():
    if _S:
        return _S
    import ricci_su2_edge_polynomial as EP     # reuse its symbolic row construction
    import ricci_su2_exact_edge as EE
    from ricci_su2_exact_edge import GC, cross
    x, y, t = EE.x, EE.y, EE.t
    s_, c, _, _, gmat = GC.background(sp.sqrt, sp.Integer(1), t)
    m = [sp.Matrix([x, 0, 0]), sp.Matrix([0, y, 0]), sp.Matrix([0, 0, y])]
    I3 = sp.eye(3)
    Om = [sp.kronecker_product(I3, cross(m[i])) + sp.kronecker_product(-sp.Matrix(gmat[i]), I3) for i in range(3)]
    Ric = sp.diag(2 * t, 4 - 2 * t, 4 - 2 * t); T = sp.zeros(9, 9)
    for i in range(3):
        for k in range(3):
            F = cross(m[i]) * m[k]
            for l in range(3):
                F = F - c[(i, k, l)] * m[l]
            T[3 * i:3 * i + 3, 3 * k:3 * k + 3] = cross(F)
    R0 = sp.kronecker_product(Ric, I3) - 2 * T
    r2 = sp.sqrt(2)
    U3 = sp.Matrix([[0, 1, 0], [1 / r2, 0, 1 / r2], [-sp.I / r2, 0, sp.I / r2]]); U = sp.kronecker_product(U3, U3)
    tr = lambda M: (U.H * M * U).applyfunc(sp.simplify)          # noqa: E731
    Omc = [tr(O_) for O_ in Om]
    w = [sp.simplify(Omc[0][a, a] / sp.I) for a in range(9)]
    Lp = (2 * sp.I * (Omc[1] + sp.I * Omc[2])).applyfunc(sp.simplify)      # real
    Lm = (2 * sp.I * (Omc[1] - sp.I * Omc[2])).applyfunc(sp.simplify)
    C = (-(Omc[1] * Omc[1] + Omc[2] * Omc[2]) + tr(R0)).applyfunc(sp.simplify)
    for M_ in (Lp, Lm, C):
        assert all(sp.simplify(sp.im(e)) == 0 for e in M_), "block not real"
    sv = sp.symbols("sv", positive=True); mS, lS, jS = sp.symbols("m l j", real=True)
    sub = lambda e: sp.simplify(sp.sympify(e).subs(t, sv ** 2))          # noqa: E731
    w = [sub(e) for e in w]; Lp = Lp.applyfunc(sub); Lm = Lm.applyfunc(sub); C = C.applyfunc(sub)
    lam = (x + (jS + 2) / sv) ** 2 + 2 * sv * x - y ** 2
    dshift = [sp.expand((w[a] - 2 * mS / sv) ** 2 + 4 * (lS * (lS + 1) - mS ** 2) + C[a, a] - lam) for a in range(9)]
    x_, y_ = x, y
    x, y, t = x_, y_, sv
    mods = {"sqrt": iv.sqrt}
    _S["dshift"] = [sp.lambdify((x_, y_, sv, mS, lS, jS), e, modules=[mods, "mpmath"]) for e in dshift]
    _S.update(ch=[(a, b) for a in (-1, 0, 1) for b in (-1, 0, 1)],
              w=[sp.lambdify((x, y, t), e, modules=[mods, "mpmath"]) for e in w],
              C=[[sp.lambdify((x, y, t), C[i, k], modules=[mods, "mpmath"]) for k in range(9)] for i in range(9)],
              Lp=[[sp.lambdify((x, y, t), Lp[i, k], modules=[mods, "mpmath"]) for k in range(9)] for i in range(9)],
              Lm=[[sp.lambdify((x, y, t), Lm[i, k], modules=[mods, "mpmath"]) for k in range(9)] for i in range(9)],
              Cnz=[[C[i, k] != 0 for k in range(9)] for i in range(9)],
              Lpnz=[[Lp[i, k] != 0 for k in range(9)] for i in range(9)],
              Lmnz=[[Lm[i, k] != 0 for k in range(9)] for i in range(9)])
    # derivatives in (x, y, s) for the mean-value certificate
    V = (x_, y_, sv)
    _S["d_dshift"] = [[sp.lambdify((x_, y_, sv, mS, lS, jS), sp.diff(e, v), modules=[mods, "mpmath"]) for e in dshift] for v in V]
    _S["dC"] = [[[sp.lambdify((x_, y_, sv), sp.diff(C[i, k], v), modules=[mods, "mpmath"]) for k in range(9)] for i in range(9)] for v in V]
    _S["dLp"] = [[[sp.lambdify((x_, y_, sv), sp.diff(Lp[i, k], v), modules=[mods, "mpmath"]) for k in range(9)] for i in range(9)] for v in V]
    _S["dLm"] = [[[sp.lambdify((x_, y_, sv), sp.diff(Lm[i, k], v), modules=[mods, "mpmath"]) for k in range(9)] for i in range(9)] for v in V]
    return _S


def dblock_upper(j, q, X, Y, Sv, vi):
    """Upper bound of the Frobenius norm of d(B - lambda I)/d v_i over the box (v = x, y, s)."""
    S = symbolic(); l = sp.Rational(j, 2)
    states = [(q - a - b, k) for k, (a, b) in enumerate(S["ch"]) if abs(q - a - b) <= l]
    idx = {st: n for n, st in enumerate(states)}
    tot = iv.mpf(0)
    for (mm, a), k in idx.items():
        e = ivv(S["d_dshift"][vi][a](X, Y, Sv, iv.mpf(float(mm)), iv.mpf(float(l)), iv.mpf(j)))
        mag = max(abs(mpmath.mpf(e.a)), abs(mpmath.mpf(e.b))); tot += iv.mpf(mag) ** 2
        for (m2, a2), k2 in idx.items():
            e = None
            if m2 == mm and a2 != a and S["Cnz"][a2][a]:
                e = ivv(S["dC"][vi][a2][a](X, Y, Sv))
            elif m2 == mm + 1 and S["Lmnz"][a2][a]:
                e = iv.sqrt(iv.mpf(float((l - mm) * (l + mm + 1)))) * ivv(S["dLm"][vi][a2][a](X, Y, Sv))
            elif m2 == mm - 1 and S["Lpnz"][a2][a]:
                e = iv.sqrt(iv.mpf(float((l + mm) * (l - mm + 1)))) * ivv(S["dLp"][vi][a2][a](X, Y, Sv))
            if e is not None:
                mag = max(abs(mpmath.mpf(e.a)), abs(mpmath.mpf(e.b))); tot += iv.mpf(mag) ** 2
    return mpmath.mpf(iv.sqrt(tot).b)


def ivv(v):
    return v if isinstance(v, type(iv.mpf(0))) else iv.mpf(v)


def block_iv(j, q, X, Y, T, shifted=False):
    """T here is s = sqrt(t) (interval). shifted=True returns B - lambda_j I with the diagonal formed as one expression."""
    S = symbolic(); l = sp.Rational(j, 2)
    states = [(q - a - b, k) for k, (a, b) in enumerate(S["ch"]) if abs(q - a - b) <= l]
    idx = {st: n for n, st in enumerate(states)}; n = len(states)
    B = [[iv.mpf(0)] * n for _ in range(n)]
    for (mm, a), k in idx.items():
        if shifted:
            B[k][k] = ivv(S["dshift"][a](X, Y, T, iv.mpf(float(mm)), iv.mpf(float(l)), iv.mpf(j)))
        else:
            wa = ivv(S["w"][a](X, Y, T))
            B[k][k] = (wa - 2 * iv.mpf(mm) / T) ** 2 + 4 * iv.mpf(l * (l + 1) - mm ** 2) + ivv(S["C"][a][a](X, Y, T))
        for (m2, a2), k2 in idx.items():
            if m2 == mm and a2 != a and S["Cnz"][a2][a]:
                B[k2][k] = B[k2][k] + ivv(S["C"][a2][a](X, Y, T))
            if m2 == mm + 1 and S["Lmnz"][a2][a]:
                B[k2][k] = B[k2][k] + iv.sqrt(iv.mpf((l - mm) * (l + mm + 1))) * ivv(S["Lm"][a2][a](X, Y, T))
            if m2 == mm - 1 and S["Lpnz"][a2][a]:
                B[k2][k] = B[k2][k] + iv.sqrt(iv.mpf((l + mm) * (l - mm + 1))) * ivv(S["Lp"][a2][a](X, Y, T))
    return states, B


def lam_iv(j, X, Y, S):
    return (X + (j + 2) / S) ** 2 + 2 * S * X - Y ** 2


def margin_float(j, q, xv, yv, tv):
    mpmath.mp.prec = 53; iv.prec = 53
    _, B = block_iv(j, q, iv.mpf(xv), iv.mpf(yv), iv.mpf(np.sqrt(tv)), shifted=True)
    Bm = np.array([[float(mpmath.mpf(e.mid)) for e in row] for row in B])
    return float(np.linalg.eigvalsh(0.5 * (Bm + Bm.T))[0])


def certify_point(j, q, xc, yc, sc):
    """Certified lower bound on lambda_min(B - lambda I) at the point (xc, yc, s = sc) (interval point evaluation)."""
    mpmath.mp.prec = 53; iv.prec = 53
    _, Bf = block_iv(j, q, iv.mpf(xc), iv.mpf(yc), iv.mpf(sc), shifted=True)
    Bm = np.array([[float(mpmath.mpf(e.mid)) for e in row] for row in Bf])
    eta = 0.5 * float(np.linalg.eigvalsh(0.5 * (Bm + Bm.T))[0])
    if eta <= 0:
        return mpmath.mpf(-1)
    mpmath.mp.prec = PREC; iv.prec = PREC
    _, B = block_iv(j, q, iv.mpf(xc), iv.mpf(yc), iv.mpf(sc), shifted=True)
    n = len(B)
    mid = [[None] * n for _ in range(n)]; bud = O.Budget()
    for r in range(n):
        for c_ in range(r + 1):
            v = B[r][c_] - (iv.mpf(eta) if r == c_ else 0)
            m_, rr = O.mid_rad(v)
            mid[r][c_] = mid[c_][r] = m_; bud.add(r, c_, rr)
    rho = bud.rho()
    L = [[mpmath.mpf(0)] * n for _ in range(n)]; d = []
    for k in range(n):
        dk = mid[k][k] - mpmath.fsum(L[k][s_] ** 2 * d[s_] for s_ in range(k))
        if dk <= 0:
            return mpmath.mpf(-1)
        d.append(dk)
        for i2 in range(k + 1, n):
            L[i2][k] = (mid[i2][k] - mpmath.fsum(L[i2][s_] * L[k][s_] * d[s_] for s_ in range(k))) / dk
        L[k][k] = mpmath.mpf(1)
    Lrows = [{s_: L[i2][s_] for s_ in range(i2 + 1) if L[i2][s_] != 0} for i2 in range(n)]
    eps, _ = O.eps_residual(rho, O.residual_frob(n, lambda i2, k2: mid[i2][k2], Lrows, d))
    return mpmath.mpf(eta) - eps          # lambda_min(B - lambda I)(centre) >= eta - eps


def certify_mv(j, q, box):
    """Mean-value certificate on a box in (x, y, t): lower bound at the centre minus sum_i sup||d_i A||_F * halfwidth_i."""
    (x0, x1), (y0, y1), (t0, t1) = box
    s0, s1 = float(np.sqrt(t0)), float(np.sqrt(t1))
    xc, yc, sc = 0.5 * (x0 + x1), 0.5 * (y0 + y1), 0.5 * (s0 + s1)
    mu = certify_point(j, q, xc, yc, sc)
    if mu <= 0:
        return False, float(mu)
    mpmath.mp.prec = PREC; iv.prec = PREC
    X, Y = iv.mpf([mpmath.mpf(x0), mpmath.mpf(x1)]), iv.mpf([mpmath.mpf(y0), mpmath.mpf(y1)])
    Sv = iv.sqrt(iv.mpf([mpmath.mpf(t0), mpmath.mpf(t1)]))
    sl, sh = mpmath.mpf(Sv.a), mpmath.mpf(Sv.b)
    hw = [mpmath.mpf(x1 - x0) / 2, mpmath.mpf(y1 - y0) / 2, max(sh - mpmath.mpf(sc), mpmath.mpf(sc) - sl)]
    var = mpmath.fsum(dblock_upper(j, q, X, Y, Sv, i) * hw[i] for i in range(3))
    lower = mu - var
    return bool(lower > 0), float(lower)


def certify(j, q, box):
    return certify_mv(j, q, box)


def certify_direct(j, q, box):
    (x0, x1), (y0, y1), (t0, t1) = box
    eta = 0.5 * margin_float(j, q, 0.5 * (x0 + x1), 0.5 * (y0 + y1), 0.5 * (t0 + t1))
    if eta <= 0:
        return False, eta
    mpmath.mp.prec = PREC; iv.prec = PREC
    X, Y = iv.mpf([mpmath.mpf(x0), mpmath.mpf(x1)]), iv.mpf([mpmath.mpf(y0), mpmath.mpf(y1)])
    Sv = iv.sqrt(iv.mpf([mpmath.mpf(t0), mpmath.mpf(t1)]))           # s = sqrt t over the t-range of the box
    _, B = block_iv(j, q, X, Y, Sv, shifted=True)                     # B - lambda_j I, diagonal as one expression
    n = len(B); mu = iv.mpf(eta)
    mid = [[None] * n for _ in range(n)]; bud = O.Budget()
    for r in range(n):
        for c_ in range(r + 1):
            v = B[r][c_] - (mu if r == c_ else 0)
            m_, rr = O.mid_rad(v)
            mid[r][c_] = mid[c_][r] = m_; bud.add(r, c_, rr)
    rho = bud.rho()
    L = [[mpmath.mpf(0)] * n for _ in range(n)]; d = []
    for k in range(n):
        dk = mid[k][k] - mpmath.fsum(L[k][s_] ** 2 * d[s_] for s_ in range(k))
        if dk <= 0:
            return False, eta
        d.append(dk)
        for i2 in range(k + 1, n):
            L[i2][k] = (mid[i2][k] - mpmath.fsum(L[i2][s_] * L[k][s_] * d[s_] for s_ in range(k))) / dk
        L[k][k] = mpmath.mpf(1)
    Lrows = [{s_: L[i2][s_] for s_ in range(i2 + 1) if L[i2][s_] != 0} for i2 in range(n)]
    res = O.residual_frob(n, lambda i2, k2: mid[i2][k2], Lrows, d)
    eps, _ = O.eps_residual(rho, res)
    return bool(eps < mpmath.mpf(eta)), float(eta - eps)


def task(arg):
    j, D = arg
    t_start = time.time(); l = j / 2
    qs = [q for q in np.arange(-l - 2, l + 2 + 1e-9, 1.0) if abs(abs(q) - (l + 2)) > 1e-9]
    widths = [D[i][1] - D[i][0] for i in range(3)]
    nboxes, fails, worst = 0, [], float("inf")
    for q in qs:
        stack = [(D, 0)]
        while stack:
            box, dpt = stack.pop()
            ok, marg = certify(j, float(q), box)
            if ok:
                nboxes += 1; worst = min(worst, marg); continue
            if dpt >= 20:
                fails.append((float(q), box)); continue
            k = max(range(3), key=lambda i: (box[i][1] - box[i][0]) / widths[i])
            mid = 0.5 * (box[k][0] + box[k][1])
            b1 = [list(b) for b in box]; b2 = [list(b) for b in box]
            b1[k][1] = mid; b2[k][0] = mid
            stack.append(([tuple(b) for b in b1], dpt + 1)); stack.append(([tuple(b) for b in b2], dpt + 1))
    return {"j": j, "blocks": len(qs), "certified_boxes": nboxes, "fails": fails, "min_margin": worst,
            "seconds": round(time.time() - t_start, 1)}


def point_check():
    import ricci_su2_exact_edge as EE
    bl = EE.Blocks(); rng = np.random.default_rng(5); worst = 0.0
    for _ in range(20):
        xv, yv, tv = rng.uniform(-3, -1.5), rng.uniform(0.5, 3), rng.uniform(3, 10); j = int(rng.integers(1, 12))
        l = j / 2
        for q in np.arange(-l - 2, l + 2 + 1e-9, 1.0):
            _, Bd = bl.block(bl.data(xv, yv, tv), j, q, tv)
            if not Bd.size:
                continue
            mpmath.mp.prec = 53; iv.prec = 53
            _, Bi = block_iv(j, float(q), iv.mpf(xv), iv.mpf(yv), iv.mpf(np.sqrt(tv)), shifted=True)
            Bm = np.array([[float(mpmath.mpf(e.mid)) for e in row] for row in Bi])
            lam = EE.lam_tw(xv, yv, tv, j)
            worst = max(worst, float(np.abs(np.linalg.eigvalsh(Bd) - lam - np.linalg.eigvalsh(0.5 * (Bm + Bm.T))).max()))
    return worst


def main():
    out = Path(sys.argv[1]); jmin, jmax = int(sys.argv[2]), int(sys.argv[3])
    D = [(float(sys.argv[4]), float(sys.argv[5])), (float(sys.argv[6]), float(sys.argv[7])), (float(sys.argv[8]), float(sys.argv[9]))]
    nproc = int(sys.argv[10]) if len(sys.argv) > 10 else 6
    chk = point_check(); print("interval blocks vs double blocks (eigenvalues):", chk, flush=True)
    if chk > 1e-9:
        raise SystemExit("interval block construction disagrees with the double-precision blocks")
    import multiprocessing as mp
    rows = []
    with mp.Pool(nproc) as pool:
        for r in pool.imap_unordered(task, [(j, D) for j in range(jmax - 1, jmin - 1, -1)]):
            rows.append(r)
            print(f"j={r['j']}: {r['blocks']} blocks, {r['certified_boxes']} boxes, fails {len(r['fails'])}, "
                  f"min margin {r['min_margin']:.3g}, {r['seconds']} s", flush=True)
            out.write_text(json.dumps({"domain": D, "jmin": jmin, "jmax": jmax, "point_check": chk,
                                       "rows": sorted(rows, key=lambda z: z["j"])}, indent=1, default=str) + "\n")
    allok = len(rows) == jmax - jmin and all(not r["fails"] for r in rows)
    out.write_text(json.dumps({"domain": D, "jmin": jmin, "jmax": jmax, "point_check": chk, "all_certified": allok,
                               "rows": sorted(rows, key=lambda z: z["j"])}, indent=1, default=str) + "\n")
    print("ALL CERTIFIED" if allok else "NOT ALL CERTIFIED", flush=True)


if __name__ == "__main__":
    main()
