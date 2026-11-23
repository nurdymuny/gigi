"""WP2 (21 November 2026), part 3: interval proof of the large-sector half on a box closer to the edge.

Statement proved (if the run ends with ALL PROVED): for every (x, y, t) in the box D = [XMIN, XMAX] x [YMIN, YMAX] x
[TMIN, TMAX] (t > 2, x <= 0, y > 0) and every sector j = 2l with l >= L0, every non-twist charge block B_{j,q} of H + GG^*
satisfies lambda_min(B_{j,q}) > lambda_j (the twist eigenvalue).  So she is the floor of every sector j >= 2 L0 on D.

Method (block diagonal dominance at order l, made exact on the small blocks):
  (I) Near-end blocks, n0 := l - q in {-1, ..., N1} (top) and n0' := l + q in {-1, ..., N1} (bottom); n0 = -2 is the
      twist and n0' = -2 its mirror (the 1 x 1 block of (-1,-1) at m = -l, entry exactly lambda_j: the conjugate copy of
      the twist, excluded exactly as in ricci_su2_exact_edge.phi_A).  Each has <= 9
      states, a at xi_a = n0 + c_a (top; m = l - xi) or eta_a = n0' - c_a (bottom; m = -l + eta).  With U = 1/l the
      matrix M = (B - lambda_j I)/l has entries
        diag:  4 xi (2 - xi U)(1 - 1/t) + K_a + U (E_a(xi) + R_a)              (top; exact identity S3 of edge_wp2_symbolic)
               the same with a -> -a, xi -> eta                                (bottom; w_{-a} = -w_a, C_{-a-a} = C_aa)
        ladder: sqrt(xi U (2 - xi U + U)) * (2i Omn[a',a])  between (xi, a) and (xi-1, a')    (top; real, S2)
                sqrt(eta U (2 - eta U + U)) * (2i Op[a',a])  between (eta, a) and (eta-1, a')  (bottom)
        same-charge: U * C[a',a].
      The entries are taken from the exact symbolic rows (sympy, lambdified onto mpmath.iv).  M is proved positive
      definite on each sub-box by an interval LDL^T (all pivots > 0; inclusion isotonicity covers every point matrix,
      which is real symmetric).  The ladder entries are O(sqrt U) against O(1) pivots, so no l^(1/2)-loss occurs.
      Requires 2 L0 >= N1 + 2 (every state of these blocks exists).
  (II) All other blocks (N1 < n0 and N1 < n0'): every row has min(xi, 2l - xi) >= N1 - 1 =: X0, and the plain
      Gershgorin row quantity G of scripts/ricci_su2_family_large_j.py (mirror-symmetric, checked symbolically here)
      is proved positive: integer xi in [X0, XA] by FA, xi >= XA by FB (relaxed), as in that script.
usage: python scripts/edge_wp2_interval_proof.py OUT.json L0 N1 XA XMIN XMAX YMIN YMAX TMIN TMAX
"""
import json
import sys
import time
from pathlib import Path

import mpmath
import numpy as np
import sympy as sp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_family_large_j as LJ          # noqa: E402
import edge_wp2_symbolic as WS                 # noqa: E402

iv = mpmath.iv
iv.prec = 80
TYPES = LJ.TYPES


def exact_entry_functions():
    """Real exact matrices 2i Omn, 2i Op, C (functions of X, Y, T on mpmath.iv), and their sparsity patterns."""
    ch, w, C, Op, Omn = WS.exact_rows()
    x, yp, t = WS.x, WS.yp, WS.t
    mods = [{"sqrt": iv.sqrt}, "mpmath"]
    out = {}
    for name, M in (("dn", (2 * sp.I * Omn).applyfunc(sp.simplify)), ("up", (2 * sp.I * Op).applyfunc(sp.simplify)),
                    ("C", C)):
        ent = {}
        for i in range(9):
            for k in range(9):
                e = sp.simplify(M[i, k])
                if e != 0:
                    assert sp.simplify(sp.im(e.subs(x, sp.Symbol("xr", real=True)))) == 0, (name, i, k, e)
                    ent[(ch[i], ch[k])] = sp.lambdify((x, yp, t), e, modules=mods)
        out[name] = ent
    return out


def mirror_check():
    """Symbolic: G_exact(l, 2l - xi, a) == G_exact(l, xi, -a) for all types (so rows xi > l reduce to xi < l)."""
    l, xi = sp.symbols("l xi", positive=True)
    x, yp, t = WS.x, WS.yp, WS.t
    w, Caa, R, Pp, Pm = LJ.ingredients(x, yp, t, sp.Abs, sp.sqrt)
    st = sp.sqrt(t)

    def G(L, XI, a):
        K = 4 - 8 / t - 4 * (w[a] + x) / st
        E = 4 * w[a] * XI / st + w[a] ** 2 - x ** 2 - 4 / t - 4 * x / st + Caa[a] - R[a] - 2 * st * x + yp ** 2
        return (4 * XI * (2 * L - XI) * (1 - 1 / t) + L * K + E - 2 * sp.sqrt(XI * (2 * L - XI + 1)) * Pm[a]
                - 2 * sp.sqrt((2 * L - XI) * (XI + 1)) * Pp[a])
    return all(sp.simplify(sp.expand(G(l, 2 * l - xi, a) - G(l, xi, (-a[0], -a[1])))) == 0 for a in TYPES)


class NearEnd:
    def __init__(self):
        self.E = exact_entry_functions()

    def matrix(self, end, n0, U, X, Y, T):
        w, Caa, R, Pp, Pm = LJ.ingredients(X, Y, T, abs, iv.sqrt)
        st = iv.sqrt(T); one = iv.mpf(1)
        if end == "top":
            pos = {a: n0 + a[0] + a[1] for a in TYPES}
            lad = self.E["dn"]          # (xi, a) -> (xi - 1, a'): raw entry 2i Omn[a', a] (m -> m + 1)
        else:
            pos = {a: n0 - a[0] - a[1] for a in TYPES}
            lad = self.E["up"]          # (eta, a) -> (eta - 1, a'): m -> m - 1, raw entry 2i Op[a', a]
        st_ = sorted([a for a in TYPES if pos[a] >= 0], key=lambda a: -pos[a])        # eliminate high xi first
        n = len(st_); M = [[iv.mpf(0)] * n for _ in range(n)]
        for i, a in enumerate(st_):
            b = a if end == "top" else (-a[0], -a[1])
            z = pos[a]
            K = 4 - 8 / T - 4 * (w[b] + X) / st
            ER = 4 * w[b] * z / st + w[b] ** 2 - X ** 2 - 4 / T - 4 * X / st + Caa[b] - 2 * st * X + Y ** 2   # E_b + R_b
            M[i][i] = 4 * z * (2 - z * U) * (one - one / T) + K + U * ER
            for k, a2 in enumerate(st_):
                if k == i:
                    continue
                if pos[a2] == z and (a2, a) in self.E["C"]:
                    M[i][k] = M[i][k] + U * self.E["C"][(a2, a)](X, Y, T)
                if pos[a2] == z - 1 and (a2, a) in lad:
                    v = lad[(a2, a)](X, Y, T) * iv.sqrt(z * U * (2 - z * U + U))
                    M[i][k] = M[i][k] + v; M[k][i] = M[k][i] + v
        return st_, M

    def ldl_min_pivot(self, end, n0, U, X, Y, T):
        _, M = self.matrix(end, n0, U, X, Y, T)
        n = len(M); A = [row[:] for row in M]; worst = None
        for k in range(n):
            p = A[k][k]
            if not (mpmath.mpf(p.a) > 0):
                return mpmath.mpf(p.a)
            worst = mpmath.mpf(p.a) if worst is None else min(worst, mpmath.mpf(p.a))
            for i in range(k + 1, n):
                f = A[i][k] / p
                for j2 in range(k + 1, i + 1):
                    A[i][j2] = A[i][j2] - f * A[j2][k]
        return worst


def check_matrix_numeric(ne, rng):
    """M(U, x, y, t) * l + lambda_j I equals the actual block (Blocks.block), numerically, both ends."""
    import ricci_su2_exact_edge as EE
    bl = EE.Blocks(); worst = 0.0
    for _ in range(12):
        X, Y, T = rng.uniform(-3, 0), rng.uniform(0.5, 3), rng.uniform(3, 10); j = int(rng.integers(30, 60)); l = j / 2
        d = bl.data(X, Y, T); lam = EE.lam_tw(X, Y, T, j)
        for end in ("top", "bottom"):
            for n0 in range(-2 if end == "bottom" else -1, 8):     # includes the mirror twist (check only)
                q = l - n0 if end == "top" else -l + n0
                states, B = bl.block(d, j, q, T)
                order, M = ne.matrix(end, n0, iv.mpf(1 / l), iv.mpf(X), iv.mpf(Y), iv.mpf(T))
                Mn = np.array([[float(mpmath.mpf(e.mid)) for e in row] for row in M]) * l + lam * np.eye(len(M))
                lab = {}
                for k, (mm, ai) in enumerate(states):
                    lab[bl.ch[ai]] = k
                perm = [lab[a] for a in order]
                Bp = B.real[np.ix_(perm, perm)]
                worst = max(worst, float(np.abs(Bp - Mn).max() / max(1.0, np.abs(Bp).max())))
    return worst


def prove(f, box, maxdepth=60, maxpieces=2000000):
    w0 = [max(hi - lo, 1e-300) for lo, hi in box]
    stack = [(box, 0)]; pieces = 0; worst = mpmath.mpf("inf")
    while stack:
        b, dpt = stack.pop()
        lo = f(*[iv.mpf([mpmath.mpf(a), mpmath.mpf(c)]) for a, c in b])
        if lo > 0:
            pieces += 1; worst = min(worst, lo)
            if pieces > maxpieces:
                return False, pieces, float(worst), "too many pieces"
            continue
        if dpt >= maxdepth:
            return False, pieces, float(lo), b
        k = max(range(len(b)), key=lambda i: (b[i][1] - b[i][0]) / w0[i])
        mid = 0.5 * (b[k][0] + b[k][1])
        stack.append(([*b[:k], (b[k][0], mid), *b[k + 1:]], dpt + 1))
        stack.append(([*b[:k], (mid, b[k][1]), *b[k + 1:]], dpt + 1))
    return True, pieces, float(worst), None


def main():
    out = Path(sys.argv[1]); L0, N1, XA = int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
    xmin, xmax, ymin, ymax, tmin, tmax = map(float, sys.argv[5:11])
    assert 2 * L0 >= N1 + 2 and XA <= L0 and N1 - 1 <= XA and xmax <= 0 and tmin > 2 and ymin > 0
    t0 = time.time()
    res = {"L0": L0, "N1": N1, "XA": XA, "domain": {"x": [xmin, xmax], "y": [ymin, ymax], "t": [tmin, tmax]}}
    res["mirror_symbolic"] = mirror_check()
    print("mirror symmetry of G (symbolic):", res["mirror_symbolic"], flush=True)
    res["G_formula_lower_bound_check"] = LJ.check_against_rows()
    print("G_exact - actual Gershgorin quantity (<= ~1e-9):", res["G_formula_lower_bound_check"], flush=True)
    ne = NearEnd()
    res["near_end_matrix_vs_blocks_max_rel_diff"] = check_matrix_numeric(ne, np.random.default_rng(7))
    print("near-end M vs Blocks.block:", res["near_end_matrix_vs_blocks_max_rel_diff"], flush=True)
    ok_all = res["mirror_symbolic"] and res["G_formula_lower_bound_check"] <= 1e-9 and \
        res["near_end_matrix_vs_blocks_max_rel_diff"] <= 1e-9
    box4 = [(0.0, 1.0 / L0), (xmin, xmax), (ymin, ymax), (tmin, tmax)]
    res["near_end"] = {}
    for end in ("top", "bottom"):
        for n0 in range(-1, N1 + 1):
            ok, n, worst, bad = prove(lambda U, X, Y, T: ne.ldl_min_pivot(end, n0, U, X, Y, T), box4)
            res["near_end"][f"{end} n0={n0}"] = {"ok": ok, "pieces": n, "min_pivot_lower_bound": worst, "bad": bad}
            ok_all &= ok
            print(end, n0, ok, n, f"{worst:.3g}", bad if not ok else "", round(time.time() - t0, 1), "s", flush=True)
            if not ok:
                break
    res["bulk_A"] = {}; res["bulk_B"] = {}
    for a in TYPES:
        for xi in range(N1 - 1, XA + 1):
            ok, n, worst, bad = prove(lambda U, X, Y, T: mpmath.mpf(LJ.FA(xi, a, U, X, Y, T).a), box4)
            res["bulk_A"][f"{a} xi={xi}"] = {"ok": ok, "pieces": n, "min_lower_bound": worst, "bad": bad}
            ok_all &= ok
            if not ok:
                print("bulk A fails", a, xi, worst, bad, flush=True)
        ok, n, worst, bad = prove(lambda XN, U, V, X, Y, T: mpmath.mpf(LJ.FB(a, XN, U, V, X, Y, T).a),
                                  [(0.0, 1.0), (0.0, 1.0 / L0), (0.0, 1.0 / XA), (xmin, xmax), (ymin, ymax), (tmin, tmax)])
        res["bulk_B"][str(a)] = {"ok": ok, "pieces": n, "min_lower_bound": worst, "bad": bad}
        ok_all &= ok
        print("bulk", a, "A ok:", all(v["ok"] for k, v in res["bulk_A"].items() if k.startswith(str(a))), "B:", ok, n,
              f"{worst:.3g}", round(time.time() - t0, 1), "s", flush=True)
    res["all_proved"] = bool(ok_all); res["seconds"] = round(time.time() - t0, 1)
    out.write_text(json.dumps(res, indent=1, default=str) + "\n")
    print("ALL PROVED" if ok_all else "NOT PROVED", flush=True)


if __name__ == "__main__":
    main()
