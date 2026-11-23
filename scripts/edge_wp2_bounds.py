"""WP2 (21 November 2026), part 2: sharper lower bounds for the non-twist charge blocks at large l, checked against the
true block minima, and the cutoffs L0 they give on domains approaching the edge.  All numbers here are COMPUTED
(double precision); the rigorous version of the chosen bound is scripts/edge_wp2_interval_proof.py.

Notation (exact, see edge_wp2_symbolic.py S1-S3): row (m = l - xi, a) of a charge block; margin quantity
  g(l, xi, a) = D - lambda_2l = 4 xi (2l - xi)(1 - 1/t) + l K_a + E_a(xi) + R_a.
Ladder couplings: to xi - 1 modulus 2 sqrt(xi (2l - xi + 1)) |Omn|, to xi + 1 modulus 2 sqrt((2l - xi)(xi + 1)) |Op|;
row sums P-_a, P+_a; same-charge row sums R_a.

Bounds on  beta(l) := min over non-twist blocks of sector 2l of [lambda_min(block) - lambda_2l]:
  (G0) plain Gershgorin (family_large_j.py): min_rows G_exact.
  (GW) weighted Gershgorin with an l-dependent diagonal similarity: weight s(xi) = prod_{k < min(xi, X1)} r_k for
       xi <= l, mirrored s(xi) = s(2l - xi), with r_k = kappa_k / sqrt(l) (k < X1).  Row bound
         G_w = g - R_a - 2 sqrt(xi (2l - xi + 1)) P-_a / r_{xi-1} - 2 sqrt((2l - xi)(xi + 1)) P+_a r_xi
       (r = 1 for indices >= X1).  Valid for any positive weights (similar matrix, Gershgorin on the real line).
       Ladder couplings O(sqrt l) are pushed into the small-xi rows as O(1) terms, against diagonal gaps O(l).
  (CM) comparison-matrix bound lambda_min(diag(B) - |offdiag(B)|) - lambda_2l: v*Bv >= |v|^T M(B) |v|, rigorous, and
       the best any positive diagonal scaling of Gershgorin can give (Perron vector weights).  Reference only.
  (EX) exact: lambda_min of the actual blocks (numpy eigvalsh).  This is what an exact treatment of the small
       near-end blocks (the interval LDL^T proof) achieves.
usage: python scripts/edge_wp2_bounds.py OUT.json
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_exact_edge as EE              # noqa: E402
import ricci_su2_family_large_j as LJ          # noqa: E402

TYPES = LJ.TYPES
BL = EE.Blocks()


def ingr(X, Y, T):
    w, Caa, R, Pp, Pm = LJ.ingredients(X, Y, T, abs, np.sqrt)
    st = np.sqrt(T)
    K = {a: 4 - 8 / T - 4 * (w[a] + X) / st for a in TYPES}
    E0 = {a: w[a] ** 2 - X ** 2 - 4 / T - 4 * X / st + Caa[a] - R[a] - 2 * st * X + Y ** 2 for a in TYPES}
    E1 = {a: 4 * w[a] / st for a in TYPES}
    return K, E0, E1, Pp, Pm


def row_bounds(l, X, Y, T, kappa=None, xis=None):
    """min over rows (xi in xis, xi <= l, all types, twist row excluded) of G_exact (kappa None) or G_w."""
    K, E0, E1, Pp, Pm = ingr(X, Y, T)
    if xis is None:
        lf = int(np.floor(l))
        xis = np.arange(0, min(lf, 80) + 1)
        if lf > 80:
            xis = np.unique(np.concatenate([xis, np.floor(np.geomspace(80, lf, 80)), [lf]]))
    xis = np.asarray(xis, float)
    X1 = 0 if kappa is None else len(kappa)
    best = np.inf
    for a in TYPES:
        g = 4 * xis * (2 * l - xis) * (1 - 1 / T) + l * K[a] + E0[a] + E1[a] * xis
        dn = 2 * np.sqrt(np.maximum(xis * (2 * l - xis + 1), 0)) * Pm[a]
        up = 2 * np.sqrt(np.maximum((2 * l - xis) * (xis + 1), 0)) * Pp[a]
        if X1:
            r_up = np.array([kappa[int(k)] / np.sqrt(l) if k < X1 else 1.0 for k in xis])
            r_dn = np.array([kappa[int(k) - 1] / np.sqrt(l) if 1 <= k <= X1 else 1.0 for k in xis])
            G = g - up * r_up - dn / r_dn
        else:
            G = g - up - dn
        if a == (1, 1):
            G = np.where(xis == 0, np.inf, G)
        best = min(best, float(G.min()))
    return best


def blocks_near_ends(l, X, Y, T, n0max=12):
    d = BL.data(X, Y, T); j = int(round(2 * l)); out = []
    qs = set()
    for n0 in range(-1, n0max + 1):
        for q in (l - n0, -(l - n0)):
            if abs(q) <= l + 2 + 1e-9:
                qs.add(float(q))
    for q in sorted(qs):
        _, B = BL.block(d, j, q, T)
        if B.size:
            out.append((q, B.real))
    return out


def all_blocks(l, X, Y, T):
    d = BL.data(X, Y, T); j = int(round(2 * l)); out = []
    for q in np.arange(-l - 1, l + 1 + 1e-9, 1.0):
        _, B = BL.block(d, j, q, T)
        if B.size:
            out.append((q, B.real))
    return out


def exact_and_cmp(l, X, Y, T, full=None):
    lam = EE.lam_tw(X, Y, T, int(round(2 * l)))
    full = (l <= 60) if full is None else full
    bl = all_blocks(l, X, Y, T) if full else blocks_near_ends(l, X, Y, T)
    ex = min(np.linalg.eigvalsh(B)[0] for _, B in bl) - lam
    cm = min(np.linalg.eigvalsh(np.diag(np.diag(B)) - np.abs(B - np.diag(np.diag(B))))[0] for _, B in bl) - lam
    return ex, cm


def kappa_family(theta, X1):
    return [theta / np.sqrt(k + 1) for k in range(X1)]


L_GRID = np.unique(np.concatenate([np.arange(1, 100.5, 0.5), np.round(np.geomspace(100, 1e6, 300) * 2) / 2]))


def cutoff(fun, Ls=L_GRID):
    """Smallest grid l with fun(l') > 0 for every grid l' >= l (inf if fun(max) <= 0).  Scans downward and stops at the
    first non-positive value (lazy)."""
    Ls = list(Ls)
    if fun(Ls[-1]) <= 0:
        return np.inf
    k = len(Ls) - 1
    while k > 0 and fun(Ls[k - 1]) > 0:
        k -= 1
    return float(Ls[k])


def main():
    t0 = time.time(); res = {}
    rng = np.random.default_rng(2026)
    # 1. random comparison: bounds are below the true minimum; how much sharper GW is than G0
    cmp_rows = []; viol = 0
    for _ in range(60):
        X, Y, T = rng.uniform(-3, 0), rng.uniform(0.5, 3), rng.uniform(3, 10)
        l = float(rng.choice([10, 20, 40, 80, 160, 320]))
        ex_full, cm = exact_and_cmp(l, X, Y, T, full=(l <= 80))
        g0 = row_bounds(l, X, Y, T)
        gw = max(row_bounds(l, X, Y, T, kappa_family(th, X1)) for th in (2, 3, 4, 6, 9) for X1 in (1, 2, 3)
                 if l > X1 + 1)
        ok = (g0 <= ex_full + 1e-8) and (gw <= ex_full + 1e-8) and (cm <= ex_full + 1e-8)
        viol += (not ok)
        cmp_rows.append({"x": X, "y": Y, "t": T, "l": l, "exact": ex_full, "comparison_matrix": cm, "G0": g0, "GW_best": gw,
                         "all_blocks": l <= 80, "bounds_below_exact": ok})
    res["random_comparison"] = cmp_rows
    res["random_comparison_violations"] = viol
    print("random comparison done; violations:", viol, round(time.time() - t0, 1), flush=True)
    # 1b. where is the exact minimum attained (near ends?) on random full computations
    far = 0; tot = 0
    for _ in range(25):
        X, Y, T = rng.uniform(-3, 0.3), rng.uniform(0.5, 3), rng.uniform(3, 10); l = float(rng.integers(13, 70))
        lam = EE.lam_tw(X, Y, T, int(2 * l))
        vals = [(np.linalg.eigvalsh(B)[0] - lam, q) for q, B in all_blocks(l, X, Y, T)]
        v, q = min(vals); tot += 1
        far += (l - abs(q) > 12)
    res["argmin_far_from_ends_count"] = {"far(n0>12)": far, "of": tot}
    # 2. edges x*(t, y) on a grid (A-edge, sectors j <= 12)
    tg = [3, 4, 5, 6, 7, 8, 9, 10]; yg = [0.5, 1.0, 1.5, 2.0, 2.5, 3.0]
    edges = {}
    for T in tg:
        for Y in yg:
            edges[(T, Y)] = EE.edge(BL.phi_A, Y, float(T))
    res["edge_grid"] = [{"t": T, "y": Y, "x_star": e} for (T, Y), e in edges.items()]
    print("edges:", {k: round(v, 4) for k, v in edges.items()}, round(time.time() - t0, 1), flush=True)
    # 3. cutoffs
    xs_rel = [0.0, 0.5]       # offsets below the top of the x-range sampled (and x = -3)
    res["cutoffs"] = {}
    for delta in (0.5, 0.25, 0.1):
        for mode in ("literal", "corrected"):
            rows = []
            for (T, Y), xe in edges.items():
                top = xe - delta if mode == "literal" else min(xe, 0.0) - delta
                rec = {"t": T, "y": Y, "x_top": top}
                if top >= 0:
                    rec.update(G0=np.inf, GW=np.inf, EX=np.inf, note="x_top >= 0: truth fails at large l (S5)")
                    rows.append(rec); continue
                xs = sorted({top - o for o in xs_rel if top - o >= -3} | {-3.0})
                L0 = {"G0": 0.0, "GW": 0.0, "EX": 0.0}; best_par = None
                for X in xs:
                    L0["G0"] = max(L0["G0"], cutoff(lambda L: row_bounds(L, X, Y, T)))
                    # GW: best (theta, X1) for this point (an interval proof needs one choice per box; reported)
                    cands = []
                    for th in (3, 4, 6, 9):
                        for X1 in (1, 2):
                            kap = kappa_family(th, X1)
                            cands.append((cutoff(lambda L: row_bounds(L, X, Y, T, kap) if L > X1 + 1 else -1.0), th, X1))
                    c = min(cands); L0["GW"] = max(L0["GW"], c[0])
                    if X == top:
                        best_par = c[1:]
                    exl = [1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64, 96, 128, 192, 256, 400, 600, 1000, 2000, 5000]
                    L0["EX"] = max(L0["EX"], cutoff(lambda L: exact_and_cmp(L, X, Y, T, full=(L <= 30))[0], exl))
                rec.update(L0); rec["GW_theta_X1_at_top"] = best_par
                rows.append(rec)
            fin = lambda k: max(r[k] for r in rows)      # noqa: E731
            res["cutoffs"][f"delta={delta} {mode}"] = {"rows": rows, "max_G0": fin("G0"), "max_GW": fin("GW"),
                                                       "max_EX": fin("EX")}
            print(f"delta={delta} {mode}: max L0  G0={fin('G0')}  GW={fin('GW')}  EX={fin('EX')}",
                  round(time.time() - t0, 1), flush=True)
    Path(sys.argv[1]).write_text(json.dumps(res, indent=1, default=str) + "\n")


if __name__ == "__main__":
    main()
