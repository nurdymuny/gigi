"""The threshold t*: where the twist stops being the floor. Exact characterisation.

In sector j = 4 the competitor is the charge q = 0 block (<= 9 x 9, closed-form rows from
scripts/ricci_su2_block_rows_symbolic.py). t* is the largest t in (2, 3) where det(B_q(t) - lambda_t(4)) = 0, so t* is
a root of an explicit polynomial in s = sqrt(t). Prints the factored determinant, the root, and a check against
bisection (2.6980269405...). Also lists t_2 = sqrt(22) - 2, the stretch where the j = 2 twist goes free.
usage: python scripts/ricci_su2_tstar_exact.py OUT.txt
"""
import json
import sys
from pathlib import Path

import sympy as sp

HERE = Path(__file__).resolve().parent
s = sp.symbols("s", positive=True)       # s = sqrt(t)
t = s ** 2


def main():
    rec = json.loads((HERE.parent / "research/su2-floor-family-20261119/block_rows_symbolic.json").read_text())
    ts = sp.symbols("t", positive=True)
    P = lambda M: sp.Matrix(9, 9, lambda a, b: sp.sympify(M[a][b], locals={"t": ts}).subs(ts, t))   # noqa: E731
    C, Op, Om, ch = P(rec["C"]), P(rec["Omega_plus"]), P(rec["Omega_minus"]), rec["charges"]
    lines = []
    for j, q in ((4, 0), (4, 1), (1, sp.Rational(1, 2))):
        l = sp.Rational(j, 2)
        states = [(q - cf - cl, a) for a, (cf, cl) in enumerate(ch) if abs(q - cf - cl) <= l]
        n = len(states); B = sp.zeros(n, n); idx = {st: k for k, st in enumerate(states)}
        for (m, a), k in idx.items():
            c = ch[a][0] + ch[a][1]
            B[k, k] += (c * (t - 2) - 2 * m) ** 2 / t + 4 * (l * (l + 1) - m ** 2)
            for (m2, a2), k2 in idx.items():
                if m2 == m:
                    B[k2, k] += C[a2, a]
                if m2 == m + 1:
                    B[k2, k] += 2 * sp.I * sp.sqrt((l - m) * (l + m + 1)) * Om[a2, a]
                if m2 == m - 1:
                    B[k2, k] += 2 * sp.I * sp.sqrt((l + m) * (l - m + 1)) * Op[a2, a]
        lam = (j * (j + 2) - 2 * (t - 3) * j - 2 * (t - 2) * (t + 4)) / t
        M = (B - lam * sp.eye(n)).applyfunc(sp.simplify)
        num = sp.numer(sp.together(sp.simplify(M.det(method="berkowitz"))))
        fl = sp.factor_list(sp.expand(num))
        lines.append(f"j={j} q={q}: block size {n}; det(B - lambda_t(j)) numerator factors:")
        for fac, mult in fl[1]:
            roots = sp.Poly(fac, s).nroots(n=40, maxsteps=200)
            rt = sorted(sp.re(r) ** 2 for r in roots if abs(sp.im(r)) < 1e-25 and sp.re(r) > 0)
            lines.append(f"   degree {sp.degree(fac, s)} (mult {mult}): {sp.factor(fac)}")
            lines.append(f"      positive real roots t = s^2: {[sp.N(r, 20) for r in rt]}")
    lines.append(f"t_2 = sqrt(22) - 2 = {sp.N(sp.sqrt(22) - 2, 20)}")
    txt = "\n".join(lines); print(txt)
    Path(sys.argv[1]).write_text(txt + "\n")


if __name__ == "__main__":
    main()
