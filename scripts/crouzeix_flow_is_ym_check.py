"""Check: the Crouzeix curvature flow  dA/dt = [A, AA* - A*A]  is the Yang-Mills gradient flow for constant connections.

With A = X + iY (X, Y Hermitian), kappa(A) = AA* - A*A = -2i[X, Y], and the flow splits into
dX/dt = -2 grad_X E, dY/dt = -2 grad_Y E for E(X, Y) = 1/2 ||[X, Y]||_F^2: the Yang-Mills energy of the constant
connection i X du + i Y dv on the 2-torus. Gradient formulas are checked by central finite differences, the flow
identity on random complex matrices. Also prints the two turn-reading coefficients (arc - chord = 1/24, tangent -
arc = 1/12 at leading order).

usage: python scripts/crouzeix_flow_is_ym_check.py [OUT.json]
"""
import json
import sys

import numpy as np


def comm(a, b):
    return a @ b - b @ a


def main():
    rng = np.random.default_rng(20261119)
    E = lambda X, Y: 0.5 * np.linalg.norm(comm(X, Y)) ** 2           # noqa: E731
    ip = lambda a, b: float(np.real(np.trace(a.conj().T @ b)))        # noqa: E731
    out = {"trials": []}
    for n in (2, 3, 4, 6, 8):
        A = rng.normal(size=(n, n)) + 1j * rng.normal(size=(n, n))
        X, Y = (A + A.conj().T) / 2, (A - A.conj().T) / 2j
        kap = A @ A.conj().T - A.conj().T @ A
        Ad = comm(A, kap)
        Xd, Yd = (Ad + Ad.conj().T) / 2, (Ad - Ad.conj().T) / 2j
        gX, gY = comm(Y, comm(Y, X)), comm(X, comm(X, Y))
        D = rng.normal(size=(n, n)) + 1j * rng.normal(size=(n, n)); D = D + D.conj().T; h = 1e-6
        fdX = (E(X + h * D, Y) - E(X - h * D, Y)) / (2 * h)
        fdY = (E(X, Y + h * D) - E(X, Y - h * D)) / (2 * h)
        out["trials"].append({
            "n": n,
            "kappa_equals_minus_2i_XY": float(np.abs(kap + 2j * comm(X, Y)).max()),
            "gradX_fd_rel_err": abs(fdX - ip(gX, D)) / max(1.0, abs(fdX)),
            "gradY_fd_rel_err": abs(fdY - ip(gY, D)) / max(1.0, abs(fdY)),
            "flow_minus_ym_descent_X": float(np.abs(Xd + 2 * gX).max()),
            "flow_minus_ym_descent_Y": float(np.abs(Yd + 2 * gY).max()),
        })
    th = 1e-2
    out["arc_minus_chord_over_th3"] = (th - 2 * np.sin(th / 2)) / th ** 3
    out["tangent_minus_arc_over_th3"] = (2 * np.tan(th / 2) - th) / th ** 3
    out["max_flow_identity_error"] = max(max(t["flow_minus_ym_descent_X"], t["flow_minus_ym_descent_Y"]) for t in out["trials"])
    print(json.dumps(out, indent=1))
    if len(sys.argv) > 1:
        open(sys.argv[1], "w").write(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
