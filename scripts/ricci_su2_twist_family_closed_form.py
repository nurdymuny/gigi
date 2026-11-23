"""The twist's eigenvalue in closed form across the circle-symmetric family (step 1 of the map, 21 November 2026).

Family: SU(2) on the Berger metric (t, 1, 1), left-invariant field with m_0 = x e_0, m_1 = y e_1, m_2 = y e_2
(gauge vectors, orthonormal coframe). It contains every background tested so far:
  Berger field x = (2 - t)/sqrt(t), y = sqrt(t);  A = s theta: x = 2s/sqrt(t), y = 2s;  the squashed meron (numerical).
The twist is alone in its charge block for every member (counting), so its eigenvalue is the diagonal entry
  lambda = (w - j/sqrt t)^2 + 2j + C,   w = <beta, Omega_0 beta>/i,   C = <beta, (-(Omega_1^2 + Omega_2^2) + R0) beta>,
with R0 = Ric (x) I - 2[F, .] (the Weitzenbock constant term). Computed symbolically in x, y, t; then checked against
the general numerical builder (the twist's Rayleigh value) at random (x, y, t, j).
usage: python scripts/ricci_su2_twist_family_closed_form.py OUT.txt
"""
import sys
from pathlib import Path

import numpy as np
import sympy as sp

sys.path.insert(0, str(Path(__file__).resolve().parent))
_argv, sys.argv = sys.argv, [sys.argv[0]]
import ricci_su2_gap_certificates_t as GC      # noqa: E402
sys.argv = _argv

x, y = sp.symbols("x y", real=True)
t = sp.symbols("t", positive=True)
j = sp.symbols("j", nonnegative=True)


def cross(v):
    return sp.Matrix([[0, -v[2], v[1]], [v[2], 0, -v[0]], [-v[1], v[0], 0]])


def closed_form():
    s, c, _, _, gmat = GC.background(sp.sqrt, sp.Integer(1), t)
    m = [sp.Matrix([x, 0, 0]), sp.Matrix([0, y, 0]), sp.Matrix([0, 0, y])]
    I3 = sp.eye(3)
    Om = [sp.kronecker_product(I3, cross(m[i])) + sp.kronecker_product(-sp.Matrix(gmat[i]), I3) for i in range(3)]
    # Ricci of (t, 1, 1) in this normalisation, and the field strength
    Ric = sp.diag(2 * t, 4 - 2 * t, 4 - 2 * t)
    T = sp.zeros(9, 9)
    for i in range(3):
        for k in range(3):
            Fik = cross(m[i]) * m[k]
            for l in range(3):
                Fik = Fik - c[(i, k, l)] * m[l]
            T[3 * i:3 * i + 3, 3 * k:3 * k + 3] = cross(Fik)
    R0 = sp.kronecker_product(Ric, I3) - 2 * T
    r2 = sp.sqrt(2)
    U3 = sp.Matrix([[0, 1, 0], [1 / r2, 0, 1 / r2], [-sp.I / r2, 0, sp.I / r2]])      # columns: charge -1, 0, +1
    beta = sp.kronecker_product(U3[:, 2], U3[:, 2])                                  # charge (+1, +1)
    bH = beta.H
    w = sp.simplify((bH * Om[0] * beta)[0] / sp.I)
    C = sp.simplify((bH * (-(Om[1] * Om[1] + Om[2] * Om[2]) + R0) * beta)[0])
    lam = sp.expand((w - j / sp.sqrt(t)) ** 2 + 2 * j + C)
    return w, C, lam, R0, beta


def numeric_twist(xv, yv, tv, jv):
    import ricci_su3_test2_family as T2
    S, C2, eps = T2.su2_data()
    g = [tv, 1.0, 1.0]; M = [xv * np.eye(3)[0], yv * np.eye(3)[1], yv * np.eye(3)[2]]
    rho = S.spin_rep(jv); d = jv + 1
    H, G = T2.hessian_and_gauge(C2, g, eps, M, rho)
    parts = (1j * C2[0].T, 1j * 2 * T2.adm(np.eye(3)[0], eps), 1j * rho[0])
    signs = T2.fit_signs(parts, H, G @ G.conj().T, 3, 3, d)
    Q = T2.assemble_Q(parts, signs[0], 3, 3, d)
    if abs(np.linalg.eigvalsh(Q).max() - (jv + 4)) > 1e-8:
        Q = -Q
    row = T2.classify_point(H, G, Q, jv + 4)
    return row["extremal"], row


def main():
    w, C, lam, R0, beta = closed_form()
    lines = [f"w (fibre rotation of the twist's frame) = {w}", f"C = {sp.factor(C)}",
             f"lambda(x, y, t, j) = {sp.simplify(lam)}"]
    # Berger and A = s theta specialisations
    lb = sp.simplify(lam.subs({x: (2 - t) / sp.sqrt(t), y: sp.sqrt(t)}))
    target = (j * (j + 2) - 2 * (t - 3) * j - 2 * (t - 2) * (t + 4)) / t
    lines.append(f"Berger (x = (2-t)/sqrt t, y = sqrt t): lambda - lambda_t(j) = {sp.simplify(lb - target)}")
    s = sp.symbols("s", real=True)
    lines.append(f"A = s theta (x = 2s/sqrt t, y = 2s): lambda = {sp.factor(sp.simplify(lam.subs({x: 2 * s / sp.sqrt(t), y: 2 * s})))}")
    # numeric check at random points
    rng = np.random.default_rng(7); worst = 0.0
    f = sp.lambdify((x, y, t, j), lam, "numpy")
    for _ in range(40):
        xv, yv = rng.uniform(-3, 3), rng.uniform(-3, 3); tv = rng.uniform(0.3, 9); jv = int(rng.integers(0, 9))
        num, _ = numeric_twist(xv, yv, tv, jv)
        worst = max(worst, abs(num - f(xv, yv, tv, jv)) / max(1, abs(num)))
    lines.append(f"numeric check, 40 random (x, y, t, j): max relative difference {worst:.2e}")
    txt = "\n".join(lines); print(txt)
    Path(sys.argv[1]).write_text(txt + "\n")


if __name__ == "__main__":
    main()
