"""Floor paper, Section 8 ("The round sphere"): at g = (1,1,1) the Levi-Civita connection is the meron, and the
construction gives -2, 1, 6, 7, 10 as the lowest gauge-orthogonal eigenvalues of the sectors j = 0..4 (the published
fluctuation spectrum, Kumar-Lechtenfeld-Picanco Costa 2021), with exactly one negative eigenvalue (Laquer's index).
Uses the project's sector builder (ricci_su3_test2_family.hessian_and_gauge), as the other floor scripts do.
usage: python scripts/ricci_su2_round_sphere_check.py OUT.json     (exits non-zero if any check fails)
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su3_test2_family as T2   # noqa: E402

EXPECTED = {0: -2.0, 1: 1.0, 2: 6.0, 3: 7.0, 4: 10.0}


def slice_spectrum(j):
    S, C2, eps = T2.su2_data()
    t = 1.0; x = (2 - t) / np.sqrt(t); y = np.sqrt(t)            # the Berger field at t = 1: the meron
    g = [t, 1.0, 1.0]; M = [x * np.eye(3)[0], y * np.eye(3)[1], y * np.eye(3)[2]]
    rho = S.spin_rep(j)
    H, G = T2.hessian_and_gauge(C2, g, eps, M, rho)
    u, s, vh = np.linalg.svd(G.conj().T, full_matrices=True)    # G^* : forms -> functions; slice = ker G^*
    rank = int((s > 1e-10 * max(s.max(), 1.0)).sum())
    V = vh.conj().T[:, rank:]
    A = V.conj().T @ H @ V; A = 0.5 * (A + A.conj().T)
    return np.linalg.eigvalsh(A)


def main():
    out = {"sectors": {}, "pass": True}
    neg = 0
    for j, want in EXPECTED.items():
        ev = slice_spectrum(j)
        neg += int((ev < -1e-9).sum())
        ok = abs(ev[0] - want) < 1e-9
        out["sectors"][j] = {"lowest": float(ev[0]), "expected": want, "abs_error": float(abs(ev[0] - want)),
                             "lowest_five": [float(v) for v in ev[:5]], "pass": bool(ok)}
        out["pass"] &= bool(ok)
    out["negative_eigenvalues_j0_to_4"] = neg
    out["index_one"] = neg == 1
    out["pass"] &= out["index_one"]
    Path(sys.argv[1]).parent.mkdir(parents=True, exist_ok=True)
    Path(sys.argv[1]).write_text(json.dumps(out, indent=1))
    print(json.dumps({j: (r["lowest"], r["pass"]) for j, r in out["sectors"].items()}), "index one:", out["index_one"])
    if not out["pass"]:
        sys.exit(1)


if __name__ == "__main__":
    main()
