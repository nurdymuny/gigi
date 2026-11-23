"""Chirality-aware Landau bound for the floor law (spectral-skeleton note, section 6 lead). Double precision.

H + G G^* = -X0^2 - X1^2 - X2^2 + R0 with X_i anti-Hermitian. Since |X_+ v|^2, |X_- v|^2 >= 0,
  -X1^2 - X2^2 = X_+^* X_+ + iB = X_-^* X_- - iB,  B = [X1, X2],  so  -X1^2 - X2^2 >= c iB  for every c in [-1, 1].
The fibre charge Q = J0 + R_frame + R_Lie commutes with H + G G^*, X0 and iB, so on each charge block
  lambda_min(H + G G^*)|_Q >= max_{c in [-1,1]} lambda_min( -X0^2 + c iB + R0 )|_Q.
The sector bound is the minimum over blocks; if it equals lambda_t(j), the twist is the floor of the sector.
usage: python scripts/ricci_su2_chiral_bound.py OUT.json
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_spin_sector as S            # noqa: E402
import ricci_su2_sector_minimality as Mn     # noqa: E402
import ricci_su2_sector_gauge_check as C     # noqa: E402

lam = lambda t, j: (j * (j + 2) - 2 * (t - 3) * j - 2 * (t - 2) * (t + 4)) / t     # noqa: E731


def rot0():
    R = np.zeros((3, 3), complex); R[1, 2] = -1j; R[2, 1] = 1j
    return R


def block_bound(M0, iB, maxit=60):
    """max over c in [-1, 1] of lambda_min(M0 + c iB); concave in c (minimum of linear functions): golden search."""
    f = lambda c: np.linalg.eigvalsh(M0 + c * iB)[0]                           # noqa: E731
    lo, hi = -1.0, 1.0; g = (np.sqrt(5) - 1) / 2
    a, b = hi - g * (hi - lo), lo + g * (hi - lo); fa, fb = f(a), f(b)
    for _ in range(maxit):
        if fa < fb:
            lo, a, fa = a, b, fb; b = lo + g * (hi - lo); fb = f(b)
        else:
            hi, b, fb = b, a, fa; a = hi - g * (hi - lo); fa = f(a)
    best = max((f(-1.0), -1.0), (f(1.0), 1.0), (fa, a), (fb, b))
    return best


def sector(t, j):
    Mn.G_BERGER = [t, 1, 1]
    Om, s_ = Mn.omegas(); R0, _, _, _ = Mn.weitzenbock(js=(0, 1, 2))
    d = j + 1; rho = S.spin_rep(j); I3 = np.eye(3)
    X = [np.kron(np.eye(9), rho[i] / s_[i]) + np.kron(Om[i], np.eye(d)) for i in range(3)]
    iB = 1j * (X[1] @ X[2] - X[2] @ X[1]); iB = 0.5 * (iB + iB.conj().T)
    M0 = -X[0] @ X[0] + np.kron(R0, np.eye(d)); M0 = 0.5 * (M0 + M0.conj().T)
    Q = np.kron(np.eye(9), 0.5j * rho[0]) + np.kron(np.kron(rot0(), I3), np.eye(d)) + np.kron(np.kron(I3, rot0()), np.eye(d))
    q, U = np.linalg.eigh(0.5 * (Q + Q.conj().T))
    out = []
    for qv in np.unique(np.round(q, 8)):
        P = U[:, np.abs(q - qv) < 1e-6]
        val, c = block_bound(P.conj().T @ M0 @ P, P.conj().T @ iB @ P)
        out.append((float(qv), float(val), float(c)))
    H, G = C.hessian_and_gauge([t, 1, 1], j)
    true_floor = float(np.linalg.eigvalsh(H + G @ G.conj().T)[0])
    bound = min(v for _, v, _ in out)
    return {"t": t, "j": j, "lambda": lam(t, j), "true_floor": true_floor, "chiral_bound": bound,
            "gap_bound_minus_lambda": bound - lam(t, j), "blocks": out}


def main():
    rows = []
    for t in (2.0, 2.5, 2.75, 3.0, 4.0, 6.0, 10.0):
        line = []
        for j in list(range(0, 13)) + [20, 24, 30, 40]:
            r = sector(t, j); rows.append(r)
            line.append(f"{j}:{r['gap_bound_minus_lambda']:+.3f}")
        print(f"t={t}: chiral bound - lambda_t(j): " + " ".join(line), flush=True)
    Path(sys.argv[1]).write_text(json.dumps(rows, indent=1) + "\n")


if __name__ == "__main__":
    main()
