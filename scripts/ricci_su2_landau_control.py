"""Control for the Landau-level test (docs/research/spectral-skeleton-2026-11-20.md, section 6).

X_+ b = 0 on the twist is evidence only if X_+ does not annihilate the other modes too. Over the same 90 cases as
ricci_su2_landau_check.py (t in {1.5, 2, 2.5, 3, 3.5, 4, 5, 6, 8}, j = 1..10), for every eigenvector of
H + G G^* in the sector:
  - the twist (top weight of the lambda_t(j) eigenspace):      |X_+ b|   (expected ~ 0)
  - its conjugate copy (bottom weight of the same eigenspace):  |X_- b'|  (expected ~ 0) and |X_+ b'| (expected > 0)
  - every other eigenvector v:                                  min |X_+ v| (expected bounded away from 0)
usage: python scripts/ricci_su2_landau_control.py OUT.json
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
    rows = []
    worst = {"X_plus_twist": 0.0, "X_minus_conjugate": 0.0, "min_X_plus_conjugate": np.inf,
             "min_X_plus_other": np.inf, "others_annihilated": 0}
    for t in (1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 5.0, 6.0, 8.0):
        Mn.G_BERGER = [t, 1, 1]
        Om, s_ = Mn.omegas()
        for j in range(1, 11):
            d = j + 1
            rho = S.spin_rep(j)
            X = [np.kron(np.eye(9), rho[i] / s_[i]) + np.kron(Om[i], np.eye(d)) for i in range(3)]
            Xp, Xm = X[1] + 1j * X[2], X[1] - 1j * X[2]
            H, G = C.hessian_and_gauge([t, 1, 1], j)
            w, V = np.linalg.eigh(H + G @ G.conj().T)
            twin = np.abs(w - lam(t, j)) < 1e-8
            E = V[:, twin]
            J0 = np.kron(np.eye(9), 0.5j * rho[0])
            wv, U = np.linalg.eigh(E.conj().T @ J0 @ E)
            top, bot = E @ U[:, -1], E @ U[:, 0]
            other = [float(np.linalg.norm(Xp @ V[:, k])) for k in range(len(w)) if not twin[k]]
            r = {"t": t, "j": j, "twin_dim": int(twin.sum()),
                 "X_plus_twist": float(np.linalg.norm(Xp @ top)),
                 "X_minus_conjugate": float(np.linalg.norm(Xm @ bot)),
                 "X_plus_conjugate": float(np.linalg.norm(Xp @ bot)),
                 "min_X_plus_other": min(other), "n_other": len(other),
                 "others_annihilated": sum(x < 1e-8 for x in other)}
            rows.append(r)
            worst["X_plus_twist"] = max(worst["X_plus_twist"], r["X_plus_twist"])
            worst["X_minus_conjugate"] = max(worst["X_minus_conjugate"], r["X_minus_conjugate"])
            worst["min_X_plus_conjugate"] = min(worst["min_X_plus_conjugate"], r["X_plus_conjugate"])
            worst["min_X_plus_other"] = min(worst["min_X_plus_other"], r["min_X_plus_other"])
            worst["others_annihilated"] += r["others_annihilated"]
    out = {"kind": "control for the Landau-level test: X_+ annihilates the twist and nothing else", "worst": worst,
           "rows": rows}
    Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + "\n")
    print(json.dumps(worst, indent=1))


if __name__ == "__main__":
    main()
