"""S3a: split each SU(2) force's residual into slow and fast directions, and test whether
the linearised flow filter predicts S3's trajectory errors.
Spec: docs/RICCI_SU2_PG_FLOW_FILTER_SPEC.md. Research script, not an engine verb.

usage: python scripts/ricci_su2_pg_flow_filter.py LEVEL TRIAL [RECEIPT.json]
  TRIAL  native | pg
"""
import json
import sys
import time
from pathlib import Path

import numpy as np
from scipy.linalg import cholesky, eigh, expm, schur, solve_triangular

HERE = Path(__file__).resolve().parent
REPO = HERE.parent
sys.path.insert(0, str(HERE))
import ricci_su2_pg_flow as FL                     # noqa: E402
from ricci_su2_pg_flow import compose, inverse, qlog, T_END  # noqa: E402

KCACHE = REPO / "outputs" / "su2-numrange"
NUMRANGE = REPO / "research" / "su2-numrange-20261118"
FLOW = REPO / "research" / "su2-flow-20261118"
TIMES = [0.0, 0.005, 0.010, 0.015, 0.020]
S2A = {("native", 0): 0.262459, ("native", 1): 0.2474, ("pg", 0): 0.3625, ("pg", 1): 0.0743}
NSTEPS = {0: 32, 1: 64}


def exact_velocity(lat, met, sgn, t, eps):
    Lm, L0, Lp = (lat.exact_links(met(s)) * sgn for s in (t - eps, t, t + eps))
    Li = inverse(L0)
    return (qlog(compose(Li, Lp)) - qlog(compose(Li, Lm))) / (2 * eps), L0


def main():
    level, trial = int(sys.argv[1]), sys.argv[2]
    receipt = Path(sys.argv[3]) if len(sys.argv) > 3 else None
    if receipt and receipt.exists():
        raise SystemExit(f"refusing to overwrite {receipt}")
    t0 = time.time()
    lat = FL.Lattice(level, trial)
    met = FL.Metric()
    native = np.asarray(lat.raw["links"], float)
    sgn = np.sign(np.sum(lat.exact_links(met(0.0)) * native, axis=1))[:, None]

    # ---- residuals along the exact path
    rows, rhat = [], []
    K = np.load(KCACHE / f"L{level}_{trial}_K.npy")
    mp0 = lat.metric_pieces(met(0.0))
    B0 = lat.mass(native, mp0).toarray()
    Lc = cholesky(B0, lower=True)
    c_eps = None
    for t in TIMES:
        g = met(t)
        v_ex, L = exact_velocity(lat, met, sgn, t, 1e-6)
        if t == 0.0:
            v_ex5, _ = exact_velocity(lat, met, sgn, t, 1e-5)
            c_eps = float(np.linalg.norm(v_ex5 - v_ex) / np.linalg.norm(v_ex))
        v_lat = lat.velocity(L, g)
        r = (v_lat - v_ex).ravel()
        mp = lat.metric_pieces(g)
        Bt = lat.mass(L, mp)
        rho = float(np.sqrt(r @ (Bt @ r)) / np.sqrt(v_ex.ravel() @ (Bt @ v_ex.ravel())))
        rhat.append(Lc.T @ r)
        rows.append({"t": t, "rho": rho, "vol": float(mp["vol"]),
                     "seconds": round(time.time() - t0, 1)})
        print(json.dumps(rows[-1]), flush=True)

    # ---- whitened operator, C-K
    G = solve_triangular(Lc, K, lower=True)
    G = solve_triangular(Lc, G.T, lower=True).T
    w_left = float(eigh(0.5 * (G + G.T), eigvals_only=True)[0])
    s2c = json.loads((NUMRANGE / f"L{level}_{trial}.json").read_text())["W_left"]
    c_k = abs(w_left - s2c) / abs(s2c)

    # ---- linear filter: y' = -G y + rhat(t), rhat linear between samples, exact
    N = G.shape[0]
    y = np.zeros(N)
    pred = []
    for k in range(len(TIMES) - 1):
        h = TIMES[k + 1] - TIMES[k]
        a, b = rhat[k], (rhat[k + 1] - rhat[k]) / h
        M = np.zeros((N + 2, N + 2))
        M[:N, :N] = -G
        M[:N, N] = b
        M[:N, N + 1] = a
        M[N, N + 1] = 1.0
        z = expm(M * h) @ np.r_[y, 0.0, 1.0]
        y = z[:N]
        pred.append(float(np.linalg.norm(y) / np.sqrt(rows[k + 1]["vol"])))
    integral = sum(0.5 * (rhat[k] + rhat[k + 1]) * (TIMES[k + 1] - TIMES[k]) for k in range(len(TIMES) - 1))
    e_naive = float(np.linalg.norm(integral) / np.sqrt(rows[-1]["vol"]))

    # ---- slow share at t = 0
    T, Z, sdim = schur(G, output="real", sort=lambda re, im: re < 1.0 / T_END)
    Qs = Z[:, :sdim]
    r0 = rhat[0]
    f_slow = float(np.linalg.norm(Qs.T @ r0) ** 2 / np.linalg.norm(r0) ** 2)

    # ---- measured S3 errors
    meas = json.loads((FLOW / f"L{level}_{trial}_N{NSTEPS[level]}_q6.json").read_text())["rows"]
    E = [m["E"] for m in meas]
    rel = [abs(p - e) / e for p, e in zip(pred, E)]
    out = {"level": level, "trial": trial, "dof": N,
           "C_S2a_rho0": rows[0]["rho"], "C_S2a_reference": S2A[(trial, level)],
           "C_S2a_rel": abs(rows[0]["rho"] - S2A[(trial, level)]) / S2A[(trial, level)],
           "C_eps": c_eps, "C_K_rel": c_k, "W_left": w_left,
           "rho": [r["rho"] for r in rows],
           "E_measured": E, "E_pred": pred, "model_rel_error": rel,
           "G_model_pass": bool(max(rel) < 0.25),
           "E_naive": e_naive, "filter_factor_W": pred[-1] / e_naive,
           "slow_dim": int(sdim), "f_slow": f_slow,
           "seconds": round(time.time() - t0, 1)}
    print(json.dumps(out), flush=True)
    if receipt:
        receipt.parent.mkdir(parents=True, exist_ok=True)
        receipt.write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
