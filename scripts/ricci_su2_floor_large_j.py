"""The floor law for every sector j >= 2*L0 and every t in [TMIN, TMAX]: an analytic Gershgorin bound, verified by
interval branch-and-bound. (Large-j half of the all-sector proof; the small-j half is
scripts/ricci_su2_block_interval_cert.py.)

Closed-form rows (scripts/ricci_su2_block_rows_symbolic.py; exact): in the charge block of H + G G^*, the state
(m, a), a = (c_f, c_l), c = c_f + c_l, has
  diagonal  D = (c(t-2) - 2m)^2/t + 4(l(l+1) - m^2) + C_a(t),
  couplings C_aa' (same m, |C_aa'| = 4t), and ladder couplings of modulus 2 sqrt(2t) sqrt((l-m)(l+m+1)) (to m+1, n_-(a)
  of them) and 2 sqrt(2t) sqrt((l+m)(l-m+1)) (to m-1, n_+(a) of them).
With x = l - m and lambda = lambda_t(2l), the Gershgorin quantity D - (row radius) - lambda equals exactly
  G = 4x(2l - x)(1 - 1/t) + 4l(2 - c)(t - 2)/t + E - 2 sqrt(2t) [n_- sqrt(x(2l - x + 1)) + n_+ sqrt((2l - x)(x + 1))],
  E = c^2 (t-2)^2/t + 4c(t-2)x/t + C_a - R_a + 2(t-2)(t+4)/t,   R_a = sum_{a' != a} |C_aa'|.
The row radius used counts every possible neighbour, so it is an upper bound. The map (m, a) -> (-m, -a) preserves G,
so x in [0, l] suffices. If G > 0 for every row except the twist (x = 0, a = (1,1), alone in its block), every
non-twist block lies above lambda by Gershgorin, and lambda_t(j) is the floor of the sector with multiplicity two.
Regions, with u = 1/l in (0, 1/L0]:
  A: integer x = 0..X0. F_A = G/l = 4x(2 - xu)(1 - 1/t) + 4(2-c)(t-2)/t + uE
        - 2 sqrt(2t) sqrt(u) [n_- sqrt(x(2 - xu + u)) + n_+ sqrt((2 - xu)(x+1))],  on (u, t).
  B: x in [X0, l]. With xi = x/l in [0, 1], v = 1/x in (0, 1/X0], F_B = G/(l x) =
        4(2 - xi)(1 - 1/t) + v 4(2-c)(t-2)/t + u v (c^2 (t-2)^2/t + K_a) + u 4c(t-2)/t
        - 2 sqrt(2t) sqrt(u v) [n_- sqrt(2 - xi + u) + n_+ sqrt(2 - xi) sqrt(1 + v)],  K_a = C_a - R_a + 2(t-2)(t+4)/t,
     on (xi, u, v, t) independently (a relaxation: it covers every actual (x, l)).
Both are proved positive by bisection with mpmath interval arithmetic (outward rounding).
usage: python scripts/ricci_su2_floor_large_j.py OUT.json L0 X0 TMIN TMAX
"""
import json
import sys
import time
from pathlib import Path

import mpmath

iv = mpmath.iv
iv.prec = 80

# per type a = (c_f, c_l): C_a(t) diagonal, R_a(t) = sum |C_aa'| off the diagonal, n_+ and n_- neighbour counts
# (from research/su2-floor-family-20261119/block_rows_symbolic.json; checked against it in main())
TYPES = {(-1, -1): ("12-6*t", "0", 2, 0), (-1, 0): ("t+4", "4*t", 2, 1), (-1, 1): ("6*t-4", "4*t", 1, 1),
         (0, -1): ("5*t", "4*t", 2, 1), (0, 0): ("6*t", "8*t", 2, 2), (0, 1): ("5*t", "4*t", 1, 2),
         (1, -1): ("6*t-4", "4*t", 1, 1), (1, 0): ("t+4", "4*t", 1, 2), (1, 1): ("12-6*t", "0", 0, 2)}


def ev(expr, t):
    return eval(expr, {"__builtins__": {}}, {"t": t})


def FA(x, a, U, T):
    cf, cl = a; c = cf + cl
    Ca, Ra, npl, nmi = TYPES[a]
    Ca, Ra = ev(Ca, T), ev(Ra, T)
    one = iv.mpf(1)
    E = c * c * (T - 2) ** 2 / T + 4 * c * (T - 2) * x / T + Ca - Ra + 2 * (T - 2) * (T + 4) / T
    lin = 4 * x * (2 - x * U) * (one - one / T) + 4 * (2 - c) * (T - 2) / T + U * E
    sq = 2 * iv.sqrt(2 * T) * iv.sqrt(U) * (nmi * iv.sqrt(x * (2 - x * U + U)) + npl * iv.sqrt((2 - x * U) * (x + 1)))
    return lin - sq


def FB(a, XI, U, V, T):
    cf, cl = a; c = cf + cl
    Ca, Ra, npl, nmi = TYPES[a]
    K = ev(Ca, T) - ev(Ra, T) + 2 * (T - 2) * (T + 4) / T
    one = iv.mpf(1)
    val = (4 * (2 - XI) * (one - one / T) + V * 4 * (2 - c) * (T - 2) / T + U * V * (c * c * (T - 2) ** 2 / T + K)
           + U * 4 * c * (T - 2) / T)
    sq = 2 * iv.sqrt(2 * T) * iv.sqrt(U * V) * (nmi * iv.sqrt(2 - XI + U) + npl * iv.sqrt(2 - XI) * iv.sqrt(1 + V))
    return val - sq


def prove(f, box, maxdepth=40):
    """Bisect the box (list of (lo, hi)) until f's interval lower bound is > 0 on every piece. Returns (ok, pieces, min lower bound)."""
    stack = [(box, 0)]; pieces = 0; worst = mpmath.mpf("inf")
    while stack:
        b, dpt = stack.pop()
        val = f(*[iv.mpf([mpmath.mpf(lo), mpmath.mpf(hi)]) for lo, hi in b])
        lo = mpmath.mpf(val.a)
        if lo > 0:
            pieces += 1; worst = min(worst, lo); continue
        if dpt >= maxdepth:
            return False, pieces, float(lo), b
        k = max(range(len(b)), key=lambda i: (b[i][1] - b[i][0]) / max(abs(b[i][1]), 1e-300) if b[i][1] != b[i][0] else -1)
        mid = 0.5 * (b[k][0] + b[k][1])
        stack.append(([*b[:k], (b[k][0], mid), *b[k + 1:]], dpt + 1))
        stack.append(([*b[:k], (mid, b[k][1]), *b[k + 1:]], dpt + 1))
    return True, pieces, float(worst), None


def main():
    out = Path(sys.argv[1]); L0, X0 = int(sys.argv[2]), int(sys.argv[3]); tmin, tmax = float(sys.argv[4]), float(sys.argv[5])
    # check the type table against the symbolic receipt
    import sympy as sp
    rec = json.loads((Path(__file__).resolve().parent.parent / "research/su2-floor-family-20261119/block_rows_symbolic.json").read_text())
    ts = sp.symbols("t", positive=True)
    for k, (cf, cl) in enumerate(rec["charges"]):
        Ca, Ra, npl, nmi = TYPES[(cf, cl)]
        C = [sp.sympify(rec["C"][k][b], locals={"t": ts}) for b in range(9)]
        assert sp.simplify(C[k] - sp.sympify(Ca, locals={"t": ts})) == 0, (cf, cl)
        assert sp.simplify(sum(sp.Abs(C[b]) for b in range(9) if b != k) - sp.sympify(Ra, locals={"t": ts})) == 0, (cf, cl)
        n_plus = sum(1 for b in range(9) if sp.simplify(sp.sympify(rec["Omega_plus"][b][k], locals={"t": ts})) != 0)
        n_minus = sum(1 for b in range(9) if sp.simplify(sp.sympify(rec["Omega_minus"][b][k], locals={"t": ts})) != 0)
        assert (n_plus, n_minus) == (npl, nmi), ((cf, cl), n_plus, n_minus)
        mods = [sp.simplify(sp.Abs(sp.sympify(rec[w][b][k], locals={"t": ts}))) for w in ("Omega_plus", "Omega_minus")
                for b in range(9) if sp.simplify(sp.sympify(rec[w][b][k], locals={"t": ts})) != 0]
        assert all(sp.simplify(mm - sp.sqrt(2 * ts)) == 0 for mm in mods), (cf, cl)
    print("type table matches the symbolic rows", flush=True)
    t0 = time.time(); res = {"L0": L0, "X0": X0, "t_range": [tmin, tmax], "regionA": {}, "regionB": {}}
    ok_all = True
    for a in TYPES:
        for x in range(0, X0 + 1):
            if x == 0 and a == (1, 1):
                continue                                         # the twist itself
            ok, n, worst, bad = prove(lambda U, T: FA(x, a, U, T), [(0.0, 1.0 / L0), (tmin, tmax)])
            res["regionA"][f"{a} x={x}"] = {"ok": ok, "pieces": n, "min_lower_bound": worst, "bad_box": bad}
            ok_all &= ok
        ok, n, worst, bad = prove(lambda XI, U, V, T: FB(a, XI, U, V, T),
                                  [(0.0, 1.0), (0.0, 1.0 / L0), (0.0, 1.0 / X0), (tmin, tmax)])
        res["regionB"][str(a)] = {"ok": ok, "pieces": n, "min_lower_bound": worst, "bad_box": bad}
        ok_all &= ok
        print(a, "A:", all(v["ok"] for k, v in res["regionA"].items() if k.startswith(str(a))), " B:", ok, n, f"{worst:.3g}",
              round(time.time() - t0, 1), "s", flush=True)
    res["all_proved"] = ok_all
    res["seconds"] = round(time.time() - t0, 1)
    out.write_text(json.dumps(res, indent=1, default=str) + "\n")
    print("ALL PROVED" if ok_all else "NOT PROVED", flush=True)


if __name__ == "__main__":
    import _receipt_status                     # exit 1 if the receipt is missing, stale or records a failed check
    _receipt_status.begin()
    main()
    _receipt_status.exit_on_failure(sys.argv[1])
