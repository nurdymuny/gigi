"""Minimality of the sector bottoms (Proposition on the bottom of every sector) and the gap on the Poincare sphere.

At the Berger metric (3, 1, 1), sector of spin j/2, H = Hessian of E = 1/2 |F|^2 against the L2 mass, G = gauge map
(xi -> e^_i xi + [M_i, xi]). On the gauge-orthogonal slice G^T b = 0, so <b, H b> = <b, (H + G G^T) b>.

1. Weitzenbock identity (checked here, j-independent): H + G G^T = sum_i X_i^* X_i + R0 (x) I, with
   X_i = rho_i / sqrt(g_i) (x) I + Omega_i, Omega_i = ad(M_i) on the Lie index and -Gamma_i on the frame index.
2. All-j bound: sum_i |X_i v|^2 >= (sqrt(j(j+2)/3) - omega)^2 |v|^2 whenever sqrt(j(j+2)/3) >= omega, where
   omega^2 = sum_i ||Omega_i||^2 (reverse triangle inequality in R^3, and sum_i |rho_i v|^2 / g_i >= j(j+2)|v|^2/3
   because g_0 = 3 >= g_1 = g_2 = 1 and -sum rho_i^2 = j(j+2)). Hence every gauge-orthogonal eigenvalue of sector j
   is >= (sqrt(j(j+2)/3) - omega)^2 + min eig R0.
3. Computer-assisted range: for j = 0..JMAX, the smallest eigenvalue of H + G G^T equals lambda(j) = (j(j+2)-14)/3
   (twice: top and bottom weight) and the next is larger by a margin far above round-off. Since the eigenvector of
   the Proposition lies in the slice, lambda(j) is the lowest gauge-orthogonal value for every j <= JMAX.
4. The spin-0 slice exactly (sympy): -14/3 (x2), 16 - 2 sqrt 73, 22/3 (x2), 16 + 2 sqrt 73.

usage: python scripts/ricci_su2_sector_minimality.py [OUT.json] [JMAX]
"""
import json
import sys
import time
from pathlib import Path

import numpy as np
import sympy as sp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_sector_gauge_check as C        # noqa: E402
import ricci_su2_spin_sector as S               # noqa: E402
import ricci_su2_left_invariant_sector as LI    # noqa: E402

G_BERGER = [3, 1, 1]
lam = lambda j: (j * (j + 2) - 14) / 3          # noqa: E731


def omegas():
    s_, c, m, _ = S.background(G_BERGER)
    Gam = lambda i, jj, k: 0.5 * (c[i, jj, k] - c[jj, k, i] + c[k, i, jj])   # noqa: E731
    return [np.kron(np.eye(3), S.cross_mat(m[i])) + np.kron(-np.array([[Gam(i, a, b) for b in range(3)] for a in range(3)]),
                                                            np.eye(3)) for i in range(3)], s_


def weitzenbock(js=(0, 1, 2, 5, 9)):
    Om, s_ = omegas()
    R0 = None; dev_const = dev_j = 0.0
    for j in js:
        H, Gm = C.hessian_and_gauge(G_BERGER, j); d = j + 1
        rho = S.spin_rep(j)
        X = [np.kron(np.eye(9), rho[i] / s_[i]) + np.kron(Om[i], np.eye(d)) for i in range(3)]
        R = H + Gm @ Gm.conj().T - sum(x.conj().T @ x for x in X)
        Rj = R[::d, ::d]
        dev_const = max(dev_const, float(np.abs(R - np.kron(Rj, np.eye(d))).max()))
        if R0 is None:
            R0 = Rj
        dev_j = max(dev_j, float(np.abs(Rj - R0).max()))
    omega = float(np.sqrt(sum(np.linalg.norm(o, 2) ** 2 for o in Om)))
    return R0.real, omega, dev_const, dev_j


def spin0_slice_exact():
    g = [sp.Integer(3), 1, 1]; s = [sp.sqrt(x) for x in g]
    c = lambda i, j, k: sp.Integer(LI.BR.get((i, j, k), 0)) * s[k] / (s[i] * s[j])          # noqa: E731
    Gam = lambda i, j, k: sp.Rational(1, 2) * (c(i, j, k) - c(j, k, i) + c(k, i, j))         # noqa: E731
    M = [sp.Matrix(3, 3, lambda k, jj: Gam(i, jj, k)) for i in range(3)]
    b = sp.symbols("b0:9"); B = [LI.so3(b[3 * i:3 * i + 3]) for i in range(3)]; A = [M[i] + B[i] for i in range(3)]
    E = 0
    for i, j in [(0, 1), (1, 2), (0, 2)]:
        F = A[i] * A[j] - A[j] * A[i] - sum((c(i, j, k) * A[k] for k in range(3)), sp.zeros(3))
        E += LI.ip(F, F) / 2
    H = sp.hessian(E, b).subs({x: 0 for x in b}); Mass = sp.hessian(sum(LI.ip(Bi, Bi) for Bi in B) / 2, b)
    H = H / Mass[0, 0]
    mv = [sp.Matrix([M[i][2, 1], M[i][0, 2], M[i][1, 0]]) for i in range(3)]
    Gmat = sp.Matrix.hstack(*[sp.Matrix.vstack(*[mv[i].cross(e) for i in range(3)]) for e in sp.eye(3).columnspace()])
    Q = sp.Matrix.hstack(*sp.GramSchmidt(Gmat.T.nullspace(), True))
    ev = (Q.T * H * Q).applyfunc(sp.nsimplify).eigenvals()
    return {str(sp.nsimplify(sp.simplify(k))): int(v) for k, v in ev.items()}


def main():
    outp = Path(sys.argv[1]) if len(sys.argv) > 1 else None
    JMAX = int(sys.argv[2]) if len(sys.argv) > 2 else 120
    t0 = time.time()
    R0, omega, dev_const, dev_j = weitzenbock()
    rmin = float(np.linalg.eigvalsh(R0).min())
    bound = lambda j: (np.sqrt(j * (j + 2) / 3) - omega) ** 2 + rmin if np.sqrt(j * (j + 2) / 3) >= omega else -np.inf  # noqa: E731
    J142 = next(j for j in range(1, 10000) if all(bound(k) >= 142 for k in range(j, j + 2000)))
    out = {"weitzenbock_R_is_constant_times_identity": dev_const, "weitzenbock_R_j_independent": dev_j,
           "omega": omega, "min_eig_R0": rmin, "all_j_bound_at_least_142_from_j": J142,
           "spin0_slice_exact": spin0_slice_exact()}
    rows = []
    for j in range(0, JMAX + 1):
        H, Gm = C.hessian_and_gauge(G_BERGER, j)
        ev = np.linalg.eigvalsh(H + Gm @ Gm.conj().T)
        nrm = float(np.abs(ev).max()); tol = 1e-12 * nrm * len(ev)
        nb = int((np.abs(ev - lam(j)) < 1e-8 * max(1.0, nrm)).sum())
        rows.append({"j": j, "bottom_minus_lambda": float(ev[0] - lam(j)), "multiplicity": nb,
                     "next_margin": float(ev[nb] - lam(j)), "roundoff_tolerance": tol,
                     "verified": bool(ev[0] >= lam(j) - tol and nb == 2 and ev[nb] - lam(j) > 100 * tol)})
    out["range"] = [0, JMAX]
    out["all_verified"] = all(r["verified"] for r in rows)
    out["min_next_margin"] = min(r["next_margin"] for r in rows)
    out["max_roundoff_tolerance"] = max(r["roundoff_tolerance"] for r in rows)
    out["per_j"] = rows
    out["seconds"] = round(time.time() - t0, 1)
    print(json.dumps({k: v for k, v in out.items() if k != "per_j"}, indent=1))
    if outp:
        outp.write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
