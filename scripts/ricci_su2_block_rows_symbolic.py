"""Closed-form rows of the charge blocks (large-j argument). Symbolic in t; exact in l, m.

A = -X0^2 + (-X1^2 - X2^2) + R0,  X_i = rho'_i/sqrt(g_i) (x) I + I (x) Omega_i,  rho' = -2i (J_z, J_x, J_y).
In the charge basis of frame (x) Lie (charges c(a) = c_f + c_l), Omega_0 is diagonal, Omega_0 = i diag(w_a), so
  -X0^2 |m,a> = (w_a - 2m/sqrt(t))^2 |m,a>,
and with rho'_+- = -2i J_+-, Omega_+- = Omega_1 +- i Omega_2:
  -X1^2 - X2^2 = 4(l(l+1) - m^2) + 2i (J_+ (x) Omega_- + J_- (x) Omega_+) - (Omega_1^2 + Omega_2^2).
So the block of charge q has diagonal D(m,a) = (w_a - 2m/sqrt t)^2 + 4(l(l+1) - m^2) + C_aa, constant couplings
C_aa' (c(a') = c(a)), and ladder couplings 2i sqrt((l-m)(l+m+1)) (Omega_-)_{a'a} to (m+1, a'), 2i sqrt((l+m)(l-m+1))
(Omega_+)_{a'a} to (m-1, a'), with C = -(Omega_1^2 + Omega_2^2) + R0.
Writes w_a, C, Omega_+-, the charges and a numerical rebuild check against ricci_su2_charge_block_direct.block.
usage: python scripts/ricci_su2_block_rows_symbolic.py OUT.json
"""
import json
import sys
from pathlib import Path

import numpy as np
import sympy as sp

sys.path.insert(0, str(Path(__file__).resolve().parent))
_argv, sys.argv = sys.argv, [sys.argv[0]]
import ricci_su2_gap_certificates_t as GC      # noqa: E402
sys.argv = _argv
import ricci_su2_charge_block_direct as D      # noqa: E402

t = sp.symbols("t", positive=True)


def symbolic_pieces():
    s, c, m, f, gmat = GC.background(sp.sqrt, sp.Integer(1), t)
    I3 = sp.eye(3)
    Om = [GC.kron(I3, GC.cross_mat(m[i])) + GC.kron(-sp.Matrix(gmat[i]), I3) for i in range(3)]
    R = json.loads((Path(__file__).resolve().parent.parent / "research/su2-floor-family-20261119/weitzenbock_symbolic_t.json").read_text())["R0"]
    R0 = sp.Matrix(9, 9, lambda a, b: sp.sympify(R[a][b], locals={"t": t}))
    r2 = sp.sqrt(2)
    U3 = sp.Matrix([[0, 1, 0], [1 / r2, 0, 1 / r2], [-sp.I / r2, 0, sp.I / r2]])     # columns: charge -1, 0, +1
    U = sp.kronecker_product(U3, U3)
    ch = [(cf, cl) for cf in (-1, 0, 1) for cl in (-1, 0, 1)]
    tr = lambda M: sp.simplify(U.H * M * U)                                       # noqa: E731
    Omc = [tr(O) for O in Om]
    C = sp.simplify(-(Omc[1] * Omc[1] + Omc[2] * Omc[2]) + tr(R0))
    Op, Omn = sp.simplify(Omc[1] + sp.I * Omc[2]), sp.simplify(Omc[1] - sp.I * Omc[2])
    offd = sp.simplify(Omc[0] - sp.diag(*[Omc[0][a, a] for a in range(9)]))
    w = [sp.simplify(Omc[0][a, a] / sp.I) for a in range(9)]
    return ch, w, C, Op, Omn, offd


def rebuild(tv, j, q, ch, w, C, Op, Omn):
    """The block of charge q from the closed-form rows, numerically."""
    l = j / 2
    sub = lambda e: complex(e.subs(t, tv))                                        # noqa: E731
    Cn = np.array([[sub(C[a, b]) for b in range(9)] for a in range(9)])
    Opn = np.array([[sub(Op[a, b]) for b in range(9)] for a in range(9)])
    Omnn = np.array([[sub(Omn[a, b]) for b in range(9)] for a in range(9)])
    wn = [float(sp.re(w[a].subs(t, tv))) for a in range(9)]
    states = [(q - ch[a][0] - ch[a][1], a) for a in range(9) if abs(q - ch[a][0] - ch[a][1]) <= l + 1e-9]
    idx = {s_: k for k, s_ in enumerate(states)}; n = len(states)
    B = np.zeros((n, n), complex)
    for (mm, a), k in idx.items():
        B[k, k] += (wn[a] - 2 * mm / np.sqrt(tv)) ** 2 + 4 * (l * (l + 1) - mm * mm)
        for (m2, a2), k2 in idx.items():
            if m2 == mm:
                B[k2, k] += Cn[a2, a]
            if m2 == mm + 1:
                B[k2, k] += 2j * np.sqrt(max((l - mm) * (l + mm + 1), 0)) * Omnn[a2, a]
            if m2 == mm - 1:
                B[k2, k] += 2j * np.sqrt(max((l + mm) * (l - mm + 1), 0)) * Opn[a2, a]
    return states, B


def main():
    ch, w, C, Op, Omn, offd = symbolic_pieces()
    print("Omega_0 off-diagonal in the charge basis is zero:", offd == sp.zeros(9, 9))
    print("w_a (Omega_0 = i diag w):", [sp.simplify(x) for x in w])
    worst = 0.0
    for tv in (2.75, 3.0, 4.0, 7.3):
        for j in (0, 1, 2, 5, 8, 13):
            l = j / 2
            for q in np.arange(-l - 2, l + 2 + 1e-9, 1.0):
                _, B1 = rebuild(tv, j, q, ch, w, C, Op, Omn)
                _, B2 = D.block(tv, j, q)
                if B1.size:
                    worst = max(worst, float(np.max(np.abs(np.linalg.eigvalsh(0.5 * (B1 + B1.conj().T)) - np.linalg.eigvalsh(B2)))))
    print("closed-form rows rebuild the validated blocks (max eigenvalue difference):", worst)
    out = {"charges": ch, "w": [str(x) for x in w], "C": [[str(C[a, b]) for b in range(9)] for a in range(9)],
           "Omega_plus": [[str(Op[a, b]) for b in range(9)] for a in range(9)],
           "Omega_minus": [[str(Omn[a, b]) for b in range(9)] for a in range(9)], "rebuild_max_eig_diff": worst}
    Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
