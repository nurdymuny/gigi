"""The Weitzenbock identity H + G G^* = sum_i X_i^* X_i + R0(t) on the Berger sphere (t, 1, 1), proved in U(su(2))
with t symbolic (same PBW argument as scripts/ricci_su2_gap_certificates.py, which proves it at a fixed t).
Prints R0(t) entrywise and its eigenvalues; writes them to OUT.json.
usage: python scripts/ricci_su2_weitzenbock_symbolic_t.py OUT.json
"""
import json
import sys
from pathlib import Path

import sympy as sp

sys.path.insert(0, "./scripts")
_argv, sys.argv = sys.argv, [sys.argv[0]]
import ricci_su2_gap_certificates_t as GC      # noqa: E402
sys.argv = _argv

t = sp.symbols("t", positive=True); xx, yy = sp.symbols("x y", real=True)


def main():
    s, c, m0_, f0_, gmat = GC.background(sp.sqrt, sp.Integer(1), t)
    m = [sp.Matrix([xx,0,0]), sp.Matrix([0,yy,0]), sp.Matrix([0,0,yy])]
    f = {}
    for i_ in range(3):
        for k_ in range(3):
            v = sp.Matrix(GC.cross_mat(m[i_])) * m[k_]
            for l_ in range(3): v = v - c[(i_,k_,l_)]*m[l_]
            f[(i_,k_)] = v
    print("Berger m check", m0_)
    I3 = sp.eye(3)
    Op, kron, PAIRS, cross_mat = GC.Op, GC.kron, GC.PAIRS, GC.cross_mat

    def blockmat(rows, cols, placements):
        M = sp.zeros(3 * rows, 3 * cols)
        for (bi, bj), blk in placements:
            M[3 * bi:3 * bi + 3, 3 * bj:3 * bj + 3] += sp.Matrix(blk)
        return M
    D = Op(9, 9)
    for p, (i, jj) in enumerate(PAIRS):
        D.add((i,), blockmat(3, 3, [((p, jj), I3 / s[i])]))
        D.add((), blockmat(3, 3, [((p, jj), cross_mat(m[i]))]))
        D.add((jj,), blockmat(3, 3, [((p, i), -I3 / s[jj])]))
        D.add((), blockmat(3, 3, [((p, i), -sp.Matrix(cross_mat(m[jj])))]))
        for k in range(3):
            D.add((), blockmat(3, 3, [((p, k), -c[(i, jj, k)] * I3)]))
    crossterm = Op(9, 9)
    for (i, jj) in PAIRS:
        blk = -sp.Matrix(cross_mat(f[(i, jj)]))
        crossterm.add((), blockmat(3, 3, [((i, jj), blk), ((jj, i), blk.T)]))
    Gop = Op(9, 3)
    for i in range(3):
        Gop.add((i,), blockmat(3, 1, [((i, 0), I3 / s[i])]))
        Gop.add((), blockmat(3, 1, [((i, 0), cross_mat(m[i]))]))
    X = []
    for i in range(3):
        om = kron(I3, cross_mat(m[i])) + kron(-sp.Matrix(gmat[i]), I3)
        X.append(Op(9, 9, {(i,): sp.eye(9) / s[i], (): om}))
    lhs = (D.dag() * D) + crossterm + (Gop * Gop.dag())
    rhs = Op(9, 9)
    for x in X:
        rhs = rhs + x.dag() * x
    diff = (lhs - rhs).normal()
    residual = [str(w) for w, mm in diff.t.items() if len(w) > 0 and sp.simplify(mm) != sp.zeros(9, 9)]
    R0 = sp.simplify(diff.t.get((), sp.zeros(9, 9)))
    Ric = sp.diag(2*t,4-2*t,4-2*t); T = sp.zeros(9,9)
    for i_ in range(3):
        for k_ in range(3): T[3*i_:3*i_+3,3*k_:3*k_+3] = sp.Matrix(GC.cross_mat(f[(i_,k_)]))
    print("R0 - (Ric x I - 2T) =", sp.simplify(R0 - (sp.kronecker_product(Ric,I3) - 2*T)))
    ev = {str(sp.factor(sp.simplify(k))): int(v) for k, v in R0.eigenvals().items()}
    print("identity holds in U(su(2)) for all t > 0:", not residual, residual)
    print("R0(t) eigenvalues:", ev)
    Path(sys.argv[1]).write_text(json.dumps({"identity_holds_all_t": not residual, "residual_words": residual,
                                             "R0": [[str(R0[a, b]) for b in range(9)] for a in range(9)],
                                             "R0_eigenvalues": ev}, indent=1) + "\n")


if __name__ == "__main__":
    main()
