"""Symbolic proof (all j >= 0, all t > 0) of the two Landau facts about the twist b = f_top (x) beta,
beta = (theta^1 + i theta^2) (x) (tau_1 + i tau_2), on the Berger sphere (t, 1, 1):
  X_+ b = (X_1 + i X_2) b = 0   and   X_0 b = i kappa b,  kappa = (2t - 4 - j)/sqrt(t) = -2(q - t)/sqrt(t), q = j/2 + 2,
with X_i = rho_i / sqrt(g_i) (x) I + I (x) Omega_i (the Weitzenbock covariant derivative); also R0(t) beta = (12 - 8t) beta
(R0(t) from scripts/ricci_su2_weitzenbock_symbolic_t.py), so the fibre / base / curvature split is exact for all j, t.
At the top weight f: rho_0 f = -i j f, rho_1 f = sqrt(j) g, rho_2 f = i sqrt(j) g (conventions of
scripts/ricci_su2_sector_formula.py), so (rho_1 + i rho_2) f = 0 identically and both facts reduce to j-independent
9 x 9 identities in t for Omega_i = I (x) [m_i]x + (-Gamma_i) (x) I on frame (x) Lie:
  (Omega_1 + i Omega_2) beta = 0,   Omega_0 beta = i (2t - 4)/sqrt(t) beta.
usage: python scripts/ricci_su2_landau_symbolic.py OUT.txt
"""
import sys
from pathlib import Path

import sympy as sp

sys.path.insert(0, str(Path(__file__).resolve().parent))
_argv, sys.argv = sys.argv, [sys.argv[0]]
import ricci_su2_gap_certificates_t as GC      # noqa: E402
sys.argv = _argv

t, j = sp.symbols("t j", positive=True)


def main():
    s, c, m, f, gmat = GC.background(sp.sqrt, sp.Integer(1), t)
    I3 = sp.eye(3)
    Om = [GC.kron(I3, GC.cross_mat(m[i])) + GC.kron(-sp.Matrix(gmat[i]), I3) for i in range(3)]
    beta = sp.zeros(9, 1)
    for idx, v in zip((3 * 1 + 1, 3 * 1 + 2, 3 * 2 + 1, 3 * 2 + 2), (-1, -sp.I, -sp.I, 1)):
        beta[idx] = v
    plus = sp.simplify((Om[1] + sp.I * Om[2]) * beta)
    zero_part = sp.simplify(Om[0] * beta - sp.I * (2 * t - 4) / sp.sqrt(t) * beta)
    # rho part at the top weight: (rho_1 + i rho_2) f = sqrt(j) g + i * i sqrt(j) g = 0; rho_0 f / sqrt(t) = -i j/sqrt(t) f
    rho_plus = sp.sqrt(j) + sp.I * (sp.I * sp.sqrt(j))
    kappa = sp.simplify(-j / sp.sqrt(t) + (2 * t - 4) / sp.sqrt(t))
    lines = [f"(rho_1 + i rho_2) f coefficient: {sp.simplify(rho_plus)}",
             f"(Omega_1 + i Omega_2) beta = {list(plus)}  -> zero: {plus == sp.zeros(9, 1)}",
             f"Omega_0 beta - i(2t-4)/sqrt(t) beta = {list(zero_part)}  -> zero: {zero_part == sp.zeros(9, 1)}",
             f"hence X_+ b = 0 and X_0 b = i kappa b with kappa = {kappa} = (2t - 4 - j)/sqrt(t), for all j >= 0, t > 0"]
    import json
    R = json.loads((Path(__file__).resolve().parent.parent / "research/su2-floor-family-20261119/weitzenbock_symbolic_t.json").read_text())["R0"]
    R0 = sp.Matrix(9, 9, lambda a, b: sp.sympify(R[a][b], locals={"t": t}))
    r0res = sp.simplify(R0 * beta - (12 - 8 * t) * beta)
    lam = (j * (j + 2) - 2 * (t - 3) * j - 2 * (t - 2) * (t + 4)) / t
    base = sp.factor(sp.simplify(lam - kappa ** 2 - (12 - 8 * t)))
    lines += [f"R0(t) beta - (12 - 8t) beta = {list(r0res)}  -> zero: {r0res == sp.zeros(9, 1)}",
              f"hence the split: fibre |X_0 b|^2 = kappa^2, curvature <b, R0 b> = 12 - 8t, base = lambda - kappa^2 - (12 - 8t) = {base}"]
    txt = "\n".join(lines)
    print(txt)
    Path(sys.argv[1]).write_text(txt + "\n")


if __name__ == "__main__":
    main()
