"""Large-j half of the all-sector floor proof for the circle-symmetric family (21 November 2026).

Family: SU(2) on (t, 1, 1), m_0 = x e_0, m_1 = y e_1, m_2 = y e_2. Charge-block rows (closed form, symbolic; see
scripts/ricci_su2_exact_edge.py), for type a = (c_f, c_l), c = c_f + c_l, at m = l - xi:
  w_a: (1,1) sqrt t - x - 2/sqrt t; (1,0) (t-2)/sqrt t; (0,1) -x; (1,-1) sqrt t + x - 2/sqrt t; (0,0) 0; and w_{-a} = -w_a.
  C_aa: (+-1,+-1) 4 sqrt(t) x - t - y^2 + 4; (+-1,0) -t + 2y^2 + 4; (+-1,-+1) -+4 sqrt(t) x ... (see TABLE below);
  same-charge couplings of modulus co = 2|y||sqrt(t) x - t - 2|/sqrt t; ladder moduli sqrt2 y or sqrt2 sqrt t.
Gershgorin quantity (diagonal - lambda_{2l} - full radius), exact in l, xi:
  G = 4 xi (2l - xi)(1 - 1/t) + l K_a + E_a(xi) - 2 sqrt(xi(2l - xi + 1)) P-_a - 2 sqrt((2l - xi)(xi + 1)) P+_a,
  K_a = 4 - 8/t - 4(w_a + x)/sqrt t,
  E_a = 4 w_a xi/sqrt t + w_a^2 - x^2 - 4/t - 4x/sqrt t + C_aa - R_a - 2 sqrt(t) x + y^2,
with lambda_{2l} = (x + (2l+2)/sqrt t)^2 + 2 sqrt(t) x - y^2 the twist. The mirror (m, a) -> (-m, -a) preserves G
(w_{-a} = -w_a, C, R symmetric, P+_a = P-_{-a}), so xi in [0, l] suffices; the twist is (1,1) at xi = 0.
Regions (u = 1/l <= 1/L0): A integer xi <= X0: F_A = G/l on (u, x, y, t); B xi >= X0 relaxed:
  F_B = G/(l xi) on (xi/l in [0,1], u, v = 1/xi in (0, 1/X0], x, y, t). Proved positive by interval bisection.
The formula for G is checked against direct block rows (numeric) before the proof.
usage: python scripts/ricci_su2_family_large_j.py OUT.json L0 X0 XMIN XMAX YMIN YMAX TMIN TMAX
"""
import json
import sys
import time
from pathlib import Path

import mpmath
import numpy as np

iv = mpmath.iv
iv.prec = 80
TYPES = [(a, b) for a in (-1, 0, 1) for b in (-1, 0, 1)]


def ingredients(X, Y, T, absf, sqrtf):
    st = sqrtf(T)
    w = {(1, 1): st - X - 2 / st, (1, 0): (T - 2) / st, (0, 1): -X, (1, -1): st + X - 2 / st, (0, 0): 0 * X}
    for a in list(w):
        w[(-a[0], -a[1])] = -w[a]
    Caa = {(1, 1): 4 * st * X - T - Y * Y + 4, (1, 0): -T + 2 * Y * Y + 4, (0, 1): 4 * T + Y * Y,
           (1, -1): -4 * st * X - T + 3 * Y * Y + 4, (0, 0): 4 * T + 2 * Y * Y}
    for a in list(Caa):
        Caa[(-a[0], -a[1])] = Caa[a]
    co = absf(2 * Y * (st * X - T - 2) / st)
    R = {(1, 1): 0 * co, (-1, -1): 0 * co, (0, 0): 2 * co}
    for a in TYPES:
        R.setdefault(a, co)
    r2 = sqrtf(2 * (1 + 0 * T))
    ay = absf(Y)
    Pp = {(-1, -1): r2 * (ay + st), (-1, 0): r2 * (ay + st), (-1, 1): r2 * st, (0, -1): r2 * (ay + st), (0, 0): r2 * (ay + st),
          (0, 1): r2 * st, (1, -1): r2 * ay, (1, 0): r2 * ay, (1, 1): 0 * st}
    Pm = {a: Pp[(-a[0], -a[1])] for a in TYPES}
    return w, Caa, R, Pp, Pm


def G_exact(l, xi, a, X, Y, T):
    w, Caa, R, Pp, Pm = ingredients(X, Y, T, abs, np.sqrt)
    st = np.sqrt(T)
    K = 4 - 8 / T - 4 * (w[a] + X) / st
    E = 4 * w[a] * xi / st + w[a] ** 2 - X ** 2 - 4 / T - 4 * X / st + Caa[a] - R[a] - 2 * st * X + Y ** 2
    return (4 * xi * (2 * l - xi) * (1 - 1 / T) + l * K + E - 2 * np.sqrt(max(xi * (2 * l - xi + 1), 0)) * Pm[a]
            - 2 * np.sqrt(max((2 * l - xi) * (xi + 1), 0)) * Pp[a])


def check_against_rows():
    """G_exact equals (diagonal - lambda - Gershgorin radius) of the actual block rows (numeric, random points)."""
    import ricci_su2_exact_edge as EE
    bl = EE.Blocks(); rng = np.random.default_rng(11); worst = 0.0
    for _ in range(30):
        X, Y, T = rng.uniform(-3, 1), rng.uniform(0.2, 3), rng.uniform(2, 10); j = int(rng.integers(2, 30)); l = j / 2
        d = bl.data(X, Y, T); lam = EE.lam_tw(X, Y, T, j)
        for q in np.arange(-l - 2, l + 2 + 1e-9, 1.0):
            states, B = bl.block(d, j, q, T)
            for k, (m, ai) in enumerate(states):
                a = bl.ch[ai]; xi = l - m
                if (a == (1, 1) and xi == 0) or (a == (-1, -1) and xi == 2 * l):
                    continue
                gersh = B[k, k].real - lam - (np.abs(B[k]).sum() - abs(B[k, k]))
                # G_exact counts every possible neighbour (an upper bound on the radius): G_exact <= gersh
                worst = max(worst, G_exact(l, xi, a, X, Y, T) - gersh)
    return worst


def FA(xi, a, U, X, Y, T):
    w, Caa, R, Pp, Pm = ingredients(X, Y, T, abs, iv.sqrt)
    st = iv.sqrt(T); one = iv.mpf(1)
    K = 4 - 8 / T - 4 * (w[a] + X) / st
    E = 4 * w[a] * xi / st + w[a] ** 2 - X ** 2 - 4 / T - 4 * X / st + Caa[a] - R[a] - 2 * st * X + Y ** 2
    lin = 4 * xi * (2 - xi * U) * (one - one / T) + K + U * E
    sq = 2 * iv.sqrt(U) * (Pm[a] * iv.sqrt(xi * (2 - xi * U + U)) + Pp[a] * iv.sqrt((2 - xi * U) * (xi + 1)))
    return lin - sq


def FB(a, XN, U, V, X, Y, T):
    w, Caa, R, Pp, Pm = ingredients(X, Y, T, abs, iv.sqrt)
    st = iv.sqrt(T); one = iv.mpf(1)
    K = 4 - 8 / T - 4 * (w[a] + X) / st
    E0 = w[a] ** 2 - X ** 2 - 4 / T - 4 * X / st + Caa[a] - R[a] - 2 * st * X + Y ** 2
    val = 4 * (2 - XN) * (one - one / T) + K * V + U * 4 * w[a] / st + U * V * E0
    sq = 2 * iv.sqrt(U * V) * (Pm[a] * iv.sqrt(2 - XN + U) + Pp[a] * iv.sqrt((2 - XN) * (1 + V)))
    return val - sq


def prove(f, box, maxdepth=60, maxpieces=400000):
    w0 = [max(hi - lo, 1e-300) for lo, hi in box]          # split by width relative to the starting box
    stack = [(box, 0)]; pieces = 0; worst = mpmath.mpf("inf")
    while stack:
        b, dpt = stack.pop()
        val = f(*[iv.mpf([mpmath.mpf(lo), mpmath.mpf(hi)]) for lo, hi in b])
        lo = mpmath.mpf(val.a)
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
    out = Path(sys.argv[1]); L0, X0 = int(sys.argv[2]), int(sys.argv[3])
    xmin, xmax, ymin, ymax, tmin, tmax = map(float, sys.argv[4:10])
    t0 = time.time()
    chk = check_against_rows()
    print("G_exact - (actual Gershgorin quantity), max over random rows (must be <= ~1e-9):", chk, flush=True)
    res = {"L0": L0, "X0": X0, "domain": {"x": [xmin, xmax], "y": [ymin, ymax], "t": [tmin, tmax]},
           "G_formula_lower_bound_check": chk, "regionA": {}, "regionB": {}}
    ok_all = chk <= 1e-9
    for a in TYPES:
        for xi in range(0, X0 + 1):
            if xi == 0 and a == (1, 1):
                continue
            ok, n, worst, bad = prove(lambda U, X, Y, T: FA(xi, a, U, X, Y, T),
                                      [(0.0, 1.0 / L0), (xmin, xmax), (ymin, ymax), (tmin, tmax)])
            res["regionA"][f"{a} xi={xi}"] = {"ok": ok, "pieces": n, "min_lower_bound": worst, "bad": bad}
            ok_all &= ok
            if not ok:
                print("A fails", a, xi, worst, bad, flush=True)
        ok, n, worst, bad = prove(lambda XN, U, V, X, Y, T: FB(a, XN, U, V, X, Y, T),
                                  [(0.0, 1.0), (0.0, 1.0 / L0), (0.0, 1.0 / X0), (xmin, xmax), (ymin, ymax), (tmin, tmax)])
        res["regionB"][str(a)] = {"ok": ok, "pieces": n, "min_lower_bound": worst, "bad": bad}
        ok_all &= ok
        print(a, "A ok:", all(v["ok"] for k, v in res["regionA"].items() if k.startswith(str(a))), "B:", ok, n,
              f"{worst:.3g}", round(time.time() - t0, 1), "s", flush=True)
    res["all_proved"] = bool(ok_all); res["seconds"] = round(time.time() - t0, 1)
    out.write_text(json.dumps(res, indent=1, default=str) + "\n")
    print("ALL PROVED" if ok_all else "NOT PROVED", flush=True)


if __name__ == "__main__":
    main()
    import _receipt_status                     # exit 1 if the receipt records a failed check
    _receipt_status.exit_on_failure(sys.argv[1])
