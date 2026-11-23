"""The closed-form edge polynomial (21 November 2026).

The competitor that sets the twist's edge over most of the circle-symmetric family is the charge-0 block of sector
j = 4 (scripts/ricci_su2_exact_edge.py). Its rows are closed-form in (x, y, t) (same construction). With s = sqrt t,
  P(x, y, s) = numerator of det(B_{4,0}(x, y, s^2) - lambda_4(x, y, s^2) I)
is a polynomial; the edge (H + GG^* version) is the largest real root in x of the factor that changes sign there.
Checks: (1) restricted to the Berger path x = (2 - s^2)/s, y = s, the factorisation contains the degree-16 factor
3s^16 + 121s^14 + ... + 262144 of scripts/ricci_su2_tstar_exact.py (up to a constant); (2) the edge root of the
identified factor matches exact_edge.json's H + GG^* edges.
usage: python scripts/ricci_su2_edge_polynomial.py OUT.txt
"""
import json
import sys
import time
from pathlib import Path

import numpy as np
import sympy as sp
from sympy.polys.matrices import DomainMatrix

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_exact_edge as EE              # noqa: E402

s = sp.symbols("s", positive=True)


def symbolic_block():
    x, y, t = EE.x, EE.y, EE.t
    # rebuild the symbolic rows (as in EE.symbolic_rows, but keep sympy expressions)
    import sympy as _sp
    from ricci_su2_exact_edge import GC, cross
    s_, c, _, _, gmat = GC.background(_sp.sqrt, _sp.Integer(1), t)
    m = [_sp.Matrix([x, 0, 0]), _sp.Matrix([0, y, 0]), _sp.Matrix([0, 0, y])]
    I3 = _sp.eye(3)
    Om = [_sp.kronecker_product(I3, cross(m[i])) + _sp.kronecker_product(-_sp.Matrix(gmat[i]), I3) for i in range(3)]
    Ric = _sp.diag(2 * t, 4 - 2 * t, 4 - 2 * t)
    T = _sp.zeros(9, 9)
    for i in range(3):
        for k in range(3):
            Fik = cross(m[i]) * m[k]
            for l in range(3):
                Fik = Fik - c[(i, k, l)] * m[l]
            T[3 * i:3 * i + 3, 3 * k:3 * k + 3] = cross(Fik)
    R0 = _sp.kronecker_product(Ric, I3) - 2 * T
    r2 = _sp.sqrt(2)
    U3 = _sp.Matrix([[0, 1, 0], [1 / r2, 0, 1 / r2], [-_sp.I / r2, 0, _sp.I / r2]])
    U = _sp.kronecker_product(U3, U3)
    tr = lambda M: (U.H * M * U).applyfunc(_sp.simplify)          # noqa: E731
    Omc = [tr(O) for O in Om]
    w = [_sp.simplify(Omc[0][a, a] / _sp.I) for a in range(9)]
    Op, Omn = (Omc[1] + _sp.I * Omc[2]).applyfunc(_sp.simplify), (Omc[1] - _sp.I * Omc[2]).applyfunc(_sp.simplify)
    C = (-(Omc[1] * Omc[1] + Omc[2] * Omc[2]) + tr(R0)).applyfunc(_sp.simplify)
    ch = [(cf, cl) for cf in (-1, 0, 1) for cl in (-1, 0, 1)]
    j, q = 4, 0; l = sp.Rational(j, 2)
    states = [(q - a - b, k) for k, (a, b) in enumerate(ch) if abs(q - a - b) <= l]
    idx = {st: n for n, st in enumerate(states)}; n = len(states)
    B = _sp.zeros(n, n)
    for (mm, a), k in idx.items():
        B[k, k] += (w[a] - 2 * mm / _sp.sqrt(t)) ** 2 + 4 * (l * (l + 1) - mm ** 2)
        for (m2, a2), k2 in idx.items():
            if m2 == mm:
                B[k2, k] += C[a2, a]
            if m2 == mm + 1:
                B[k2, k] += 2 * _sp.I * _sp.sqrt((l - mm) * (l + mm + 1)) * Omn[a2, a]
            if m2 == mm - 1:
                B[k2, k] += 2 * _sp.I * _sp.sqrt((l + mm) * (l - mm + 1)) * Op[a2, a]
    lam = (x + (j + 2) / _sp.sqrt(t)) ** 2 + 2 * _sp.sqrt(t) * x - y ** 2
    M = (B - lam * _sp.eye(n)).subs(t, s ** 2)
    M = (M * s ** 2).applyfunc(lambda e: _sp.expand(_sp.simplify(e)))      # clear 1/s^2
    return M, n


def main():
    t0 = time.time(); lines = []
    M, n = symbolic_block()
    lines.append(f"block size {n}; entries built in {time.time() - t0:.1f} s")
    print(lines[-1], flush=True)
    K = sp.QQ.algebraic_field(sp.sqrt(2), sp.sqrt(3))[EE.x, EE.y, s]
    dM = DomainMatrix.from_Matrix(M).convert_to(K)
    lines.append(f"domain: {dM.domain}")
    print(lines[-1], flush=True)
    det = dM.det()
    P = sp.expand(dM.domain.to_sympy(det))
    lines.append(f"determinant computed in {time.time() - t0:.1f} s; total degree {sp.Poly(P, EE.x, EE.y, s).total_degree()}")
    print(lines[-1], flush=True)
    P = sp.expand(P)
    fl = sp.factor_list(P)
    lines.append(f"factors ({time.time() - t0:.1f} s):")
    for fac, mult in fl[1]:
        pol = sp.Poly(fac, EE.x, EE.y, s)
        lines.append(f"  degree (x,y,s) = {pol.degree(EE.x)}, {pol.degree(EE.y)}, {pol.degree(s)}; total {pol.total_degree()}; "
                     f"mult {mult}; terms {len(pol.terms())}")
        lines.append("    " + str(fac)[:3000])
    # Berger restriction
    berger = {EE.x: (2 - s ** 2) / s, EE.y: s}
    deg16 = 3*s**16 + 121*s**14 + 1066*s**12 - 2836*s**10 - 18624*s**8 + 20992*s**6 + 181248*s**4 - 425984*s**2 + 262144
    found = []
    for fac, mult in fl[1]:
        r = sp.factor(sp.together(fac.subs(berger)))
        num = sp.numer(r)
        q, rem = sp.div(sp.Poly(num, s), sp.Poly(deg16, s))
        if rem.is_zero:
            found.append(str(fac)[:80])
    lines.append(f"Berger restriction contains the degree-16 factor of t_A: {bool(found)} {found}")
    txt = "\n".join(lines); print(txt[-4000:])
    Path(sys.argv[1]).write_text(txt + "\n")
    Path(sys.argv[1]).with_suffix(".srepr").write_text(sp.srepr(P) + "\n")


if __name__ == "__main__":
    main()
