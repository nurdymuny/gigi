"""The Landau-level test of the floor law (docs/research/spectral-skeleton-2026-11-20.md, test 1).

H + G G^* = sum_i X_i^* X_i + R0 (Weitzenbock), X_i = rho_i / sqrt(g_i) + Omega_i, on the Berger sphere (t, 1, 1).
For the twist b (the top-weight vector of the lambda_t(j) eigenspace), test:
  X_+ b = (X_1 + i X_2) b = 0        (annihilated by the holomorphic covariant derivative: lowest Landau level)
  X_0 b = i kappa b                   (definite charge along the Hopf fibre)
and decompose lambda_t(j) = |X_0 b|^2 + (|X_1 b|^2 + |X_2 b|^2) + <b, R0 b> against the closed forms
  fibre (j - 2(t-2))^2 / t,   base (Landau) 2(j + t),   shift 12 - 8t.
usage: python scripts/ricci_su2_landau_check.py OUT.json
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_sector_gauge_check as C     # noqa: E402
import ricci_su2_spin_sector as S            # noqa: E402
import ricci_su2_sector_minimality as Mn     # noqa: E402

lam = lambda t, j: (j * (j + 2) - 2 * (t - 3) * j - 2 * (t - 2) * (t + 4)) / t     # noqa: E731


def main():
    rows, worst = [], {"X_plus_b": 0.0, "X0_eig_residual": 0.0, "fibre": 0.0, "base": 0.0, "shift": 0.0, "sum": 0.0}
    for t in (1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 5.0, 6.0, 8.0):
        Mn.G_BERGER = [t, 1, 1]
        Om, s_ = Mn.omegas()
        R0, _, _, _ = Mn.weitzenbock(js=(0, 1, 2))
        for j in range(1, 11):
            d = j + 1; rho = S.spin_rep(j)
            X = [np.kron(np.eye(9), rho[i] / s_[i]) + np.kron(Om[i], np.eye(d)) for i in range(3)]
            H, G = C.hessian_and_gauge([t, 1, 1], j); A = H + G @ G.conj().T
            w, V = np.linalg.eigh(A); E = V[:, np.abs(w - lam(t, j)) < 1e-8]
            J0 = np.kron(np.eye(9), 0.5j * rho[0])
            wv, U = np.linalg.eigh(E.conj().T @ J0 @ E); b = E @ U[:, np.argmax(wv)]; b /= np.linalg.norm(b)
            x0 = X[0] @ b; kap = b.conj() @ x0
            fibre = float(np.linalg.norm(x0) ** 2); base = float(np.linalg.norm(X[1] @ b) ** 2 + np.linalg.norm(X[2] @ b) ** 2)
            shift = float((b.conj() @ np.kron(R0, np.eye(d)) @ b).real)
            pred = {"fibre": (j - 2 * (t - 2)) ** 2 / t, "base": 2 * (j + t), "shift": 12 - 8 * t}
            r = {"t": t, "j": j, "X_plus_b": float(np.linalg.norm((X[1] + 1j * X[2]) @ b)),
                 "X0_eig_residual": float(np.linalg.norm(x0 - kap * b)), "kappa": [float(kap.real), float(kap.imag)],
                 "fibre": fibre, "base": base, "shift": shift, "lambda": lam(t, j)}
            rows.append(r)
            for k in ("X_plus_b", "X0_eig_residual"):
                worst[k] = max(worst[k], r[k])
            for k in ("fibre", "base", "shift"):
                worst[k] = max(worst[k], abs(r[k] - pred[k]))
            worst["sum"] = max(worst["sum"], abs(fibre + base + shift - lam(t, j)))
    out = {"decomposition": "lambda_t(j) = (j - 2(t-2))^2/t + 2(j + t) + (12 - 8t)", "worst": worst, "rows": rows}
    Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + "\n")
    print(json.dumps(worst, indent=1))


if __name__ == "__main__":
    main()
