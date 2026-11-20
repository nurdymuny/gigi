"""T1 (adjacent-ideas grid): Henrici's departure from normality of the linearised SU(2) lattice forces.

For each (level, trial), G = L^-1 K L^-T is the force in the native mass norm (B = L L^T), exactly as in S2c/S2d.
Measured:
  dep^2 = ||G||_F^2 - sum |lambda|^2         (Henrici's departure; from a real Schur form, so no cancellation
                                             against separately computed eigenvalues)
  kappa = G G^T - G^T G                      (the self-commutator; for A = X + iY, kappa = -2i[X, Y])
  E_YM  = ||kappa||_F^2 / 8                  (= 1/2 ||[X, Y]||_F^2, the Yang-Mills energy of the constant
                                             connection iX du + iY dv, scripts/crouzeix_flow_is_ym_check.py)
  Henrici's bound dep^2 <= sqrt((N^3 - N)/12) ||kappa||_F  (sanity check)
and, at level 0 only (--flow), the Crouzeix curvature flow dA/dt = [A, AA* - A*A] integrated from A = G/||G||_F,
to test the identity  int_0^inf ||kappa(A(t))||_F^2 dt = 1/2 dep^2(A_0)  on a real operator.

usage: python scripts/ricci_su2_henrici_t1.py OUT.json [--flow]
"""
import json
import sys
import time
from pathlib import Path

import numpy as np
from scipy.integrate import solve_ivp
from scipy.linalg import cholesky, schur, solve_triangular

HERE = Path(__file__).resolve().parent
REPO = HERE.parent
sys.path.insert(0, str(HERE))
import ricci_su2_pg_flow_cayley as CAY                    # noqa: E402
import ricci_su2_pg_force as F                            # noqa: E402
from ricci_su2_pg_stability import ORDER                  # noqa: E402
from test_ricci_operator_native import sparse_matrix      # noqa: E402

RUNS = [(0, "native"), (0, "pg"), (1, "native"), (1, "pg"), (2, "native"), (2, "pg"), (2, "pg_onering")]
SPEC = REPO / "research" / "su2-spectrum-n3-20261119"


def whitened(level, trial):
    raw = json.loads((F.BERGER / f"level{level}_berger_q{ORDER}.output.json").read_text())
    B = sparse_matrix(raw).toarray()
    K = CAY.load_K(level, trial).toarray()
    L = cholesky(B, lower=True)
    G = solve_triangular(L, K, lower=True)
    return solve_triangular(L, G.T, lower=True).T


def schur_eigs(T):
    """Eigenvalues of a real quasi-triangular Schur factor, from its 1x1 and 2x2 diagonal blocks."""
    n, lam, i = T.shape[0], [], 0
    while i < n:
        if i + 1 < n and abs(T[i + 1, i]) > 0:
            lam.extend(np.linalg.eigvals(T[i:i + 2, i:i + 2])); i += 2
        else:
            lam.append(T[i, i]); i += 1
    return np.array(lam, complex)


def measure(G):
    N = G.shape[0]
    fro2 = float(np.sum(G * G))
    T, _ = schur(G, output="real", check_finite=False)
    lam = schur_eigs(T)
    dep2 = fro2 - float(np.sum(np.abs(lam) ** 2))
    kap = G @ G.T - G.T @ G
    kF = float(np.linalg.norm(kap))
    Sym, Skw = 0.5 * (G + G.T), 0.5 * (G - G.T)
    return lam, {"dof": N, "fro2": fro2, "dep2": dep2, "dep": float(np.sqrt(max(dep2, 0.0))),
                 "dep_over_fro": float(np.sqrt(max(dep2, 0.0) / fro2)),
                 "kappa_F": kF, "E_YM": kF ** 2 / 8, "E_YM_over_fro4": kF ** 2 / 8 / fro2 ** 2,
                 "henrici_bound_ratio": dep2 / (np.sqrt((N ** 3 - N) / 12) * kF) if kF > 0 else None,
                 "sym_F": float(np.linalg.norm(Sym)), "skew_F": float(np.linalg.norm(Skw)),
                 "max_abs_imag": float(np.abs(lam.imag).max()), "min_real": float(lam.real.min()),
                 "max_abs_eig": float(np.abs(lam).max())}


def crouzeix_flow(G, t_end=400.0):
    """Integrate dA/dt = [A, AA^T - A^T A] (real A) from A0 = G/||G||_F; accumulate int ||kappa||_F^2 dt."""
    A0 = G / np.linalg.norm(G); n = A0.shape[0]

    def rhs(t, y):
        A = y[:-1].reshape(n, n)
        k = A @ A.T - A.T @ A
        return np.concatenate([(A @ k - k @ A).ravel(), [float(np.sum(k * k))]])
    ts = np.concatenate([[0.0], np.geomspace(1e-3, t_end, 40)])
    sol = solve_ivp(rhs, (0, t_end), np.concatenate([A0.ravel(), [0.0]]), method="RK45", rtol=1e-9, atol=1e-12,
                    t_eval=ts)
    _, d0 = measure(A0)
    target = 0.5 * d0["dep2"]
    traj = []
    for t, y in zip(sol.t, sol.y.T):
        A = y[:-1].reshape(n, n)
        traj.append({"t": float(t), "integral": float(y[-1]), "fraction_of_half_dep2": float(y[-1] / target),
                     "spectrum_drift": None})
    A_end = sol.y[:-1, -1].reshape(n, n)
    lam0 = np.sort_complex(np.linalg.eigvals(A0)); lam1 = np.sort_complex(np.linalg.eigvals(A_end))
    _, d1 = measure(A_end)
    return {"half_dep2_A0": target, "integral_at_t_end": float(sol.y[-1, -1]),
            "remaining_half_dep2_at_t_end": 0.5 * d1["dep2"],
            "identity_residual": float(sol.y[-1, -1] + 0.5 * d1["dep2"] - target) / target,
            "spectrum_drift_max": float(np.abs(lam0 - lam1).max()),
            "status": sol.message, "nfev": int(sol.nfev), "trajectory": traj}


def main():
    outp = Path(sys.argv[1]); do_flow = "--flow" in sys.argv
    res = {}
    for level, trial in RUNS:
        t0 = time.time()
        G = whitened(level, trial)
        lam, d = measure(G)
        saved = np.load(SPEC / f"L{level}_{trial}.eigs.npy")
        d["saved_spectrum_sumsq_rel_diff"] = float(abs(np.sum(np.abs(saved) ** 2) - np.sum(np.abs(lam) ** 2))
                                                   / np.sum(np.abs(lam) ** 2))
        d["saved_max_abs_imag"] = float(np.abs(saved.imag).max())
        if do_flow and level == 0:
            d["crouzeix_flow"] = crouzeix_flow(G)
        d["seconds"] = round(time.time() - t0, 1)
        res[f"L{level}_{trial}"] = d
        print(f"L{level}_{trial}", json.dumps({k: v for k, v in d.items() if k != "crouzeix_flow"}), flush=True)
        if "crouzeix_flow" in d:
            print("  flow", json.dumps({k: v for k, v in d["crouzeix_flow"].items() if k != "trajectory"}), flush=True)
        outp.write_text(json.dumps(res, indent=1) + "\n")


if __name__ == "__main__":
    main()
