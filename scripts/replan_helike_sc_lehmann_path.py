"""RECEDING-HORIZON Lehmann homotopy on base B3' (registration section 12). Floating point; not a bound.

Section 11 found short index paths under the Ritz proxy, but none survived real Lehmann steps: the optimal plans ride
each admissibility margin to its edge, and the real top-index bounds sit below the proxy. Here every executed step is
real, and the plan is recomputed from the real state after each one:

1. Current real states: {(N, k): rho} with rho a real Lehmann bound (or exact base level at k = 0).
2. Plan (dynamic programming as in plan_helike_sc_lehmann_path.plan): first moves from the real states, later states
   from the proxy l_i = mu_i - LOSS; fewest steps to a step with mu_11 - LOSS >= rho_b.
3. Execute only the first move with a real Lehmann step at (s', rho, N). The real l_1..l_N give the new real states
   {(N', k'): l_{N'+1}} for 12 <= N' <= N - 1.
4. PASS when a real step gives l_11 >= rho_b. FAIL if no plan exists from the real states or 40 real steps are used.

Model, basis and operators: the cached operators of plan_helike_sc_lehmann_path.build (same construction).

usage: python scripts/replan_helike_sc_lehmann_path.py [--controls 0.004,0.002] [--cache DIR] [--out PREFIX]
"""
import argparse
import json

import numpy as np

import dryrun_helike_sc_lehmann_homotopy as dr
import estimate_helike_sc_gbase as gb
from plan_helike_sc_lehmann_path import S_GRID, NMIN, build

DELTA = 0.02
LOSSES = (0.01, 0.03, 0.06)          # proxy loss for planned (not yet executed) steps; any PASS counts


def plan_first_move(MU, rhob, delta, loss, init, max_steps):
    """init: {(N, k0): rho} (all at one k0). Returns (n_steps, N_trial, k_next) of the first move, or None."""
    K, NM = MU.shape
    INF = 10 ** 9
    k0 = next(iter(init))[1]
    best = np.full((NM, K), INF, dtype=np.int64)
    first = {}                                   # (N, k) -> (N_trial_first, k_first) of the first move on its path
    for (N, k) in init:
        best[N, k] = 0
    rho = lambda N, k: init[(N, k)] if k == k0 else MU[k, N] - loss
    goal = None
    for N in range(NM - 2, NMIN - 1, -1):
        ks = np.where(best[N] < INF)[0]
        if not len(ks):
            continue
        col = MU[:, N - 1]
        arr = np.full(K, INF, dtype=np.int64); fN = np.full(K, -1); fk = np.full(K, -1)
        for k in ks:
            r = rho(N, k)
            if not np.isfinite(r):
                continue
            kmax = np.searchsorted(col, r - delta, side="right") - 1
            if kmax <= k:
                continue
            c = best[N, k] + 1
            seg = slice(k + 1, kmax + 1)
            better = c < arr[seg]
            arr[seg] = np.where(better, c, arr[seg])
            a, b = (N, -1) if k == k0 else first[(N, int(k))]
            fN[seg] = np.where(better, a, fN[seg]); fk[seg] = np.where(better, b, fk[seg])
        idx = np.arange(K)
        fk = np.where(fk == -1, idx, fk)          # first move from an init state lands at k' itself
        hit = np.where((arr < INF) & (MU[:, 10] - loss >= rhob))[0]
        for kp in hit:
            if arr[kp] <= max_steps and (goal is None or arr[kp] < goal[0]):
                goal = (int(arr[kp]), int(fN[kp]), int(fk[kp]))
        for Np in range(NMIN, N):
            imp = arr < best[Np]
            if imp.any():
                best[Np] = np.where(imp, arr, best[Np])
                for kp in np.where(imp)[0]:
                    first[(Np, int(kp))] = (int(fN[kp]), int(fk[kp]))
    return goal


def run(E, Dm, D2m, MU, rhob, loss, log):
    init = {(N, 0): float(E[N]) for N in range(NMIN, MU.shape[1] - 1)}
    used = 0; trail = []
    while used < dr.MAX_STEPS:
        g = plan_first_move(MU, rhob, DELTA, loss, init, dr.MAX_STEPS - used)
        if g is None:
            return f"FAIL (no plan from the real state after {used} steps)", trail
        nplan, N, kp = g
        k0 = next(iter(init))[1]; rho = init[(N, k0)]; s = float(S_GRID[kp])
        lo, mu, resid, nneg = dr.lehmann(E, Dm, D2m, s, rho, N)
        used += 1
        trail.append(dict(step=used, s=s, N=N, rho=rho, l11=float(lo[10]), mu11=float(mu[10]),
                          max_resid=float(resid.max()), planned_remaining=nplan - 1))
        log(f"    real step {used:2d}: s = {s:.4f}  N = {N:3d}  rho = {rho:.4f}  l_11 = {lo[10]:.4f}"
            f" (mu_11 {mu[10]:.4f}, rho_b {rhob:.4f})  max resid {resid.max():.1e}  plan said {nplan} steps")
        if lo[10] >= rhob:
            return f"PASS (real l_11 = {lo[10]:.4f} >= rho_b = {rhob:.4f} at s = {s:.4f}, {used} real steps)", trail
        init = {(Np, kp): float(lo[Np]) for Np in range(NMIN, N) if np.isfinite(lo[Np])}
        if not init:
            return f"FAIL (no real bounds at s = {s:.4f})", trail
    return "FAIL (40 real steps used)", trail


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--controls", default="0.004,0.002")
    ap.add_argument("--cache", default=None)
    ap.add_argument("--out", default=None)
    a = ap.parse_args()
    lines, res = [], []
    def log(s):
        print(s, flush=True); lines.append(s)
    log(f"Receding-horizon Lehmann homotopy on B3' (L_u = 5, L_v = 2), DELTA = {DELTA}; floating point, not a bound")
    for g2, E0 in gb.CONTROLS:
        if str(g2) not in a.controls.split(","):
            continue
        rhob = E0 + gb.RHO_B_REL
        log(f"\n=== control g2 = {g2}")
        E, Dm, D2m, MU = build(g2, E0, a.cache, log)
        out = dict(g2=g2, runs=[])
        for loss in LOSSES:
            log(f"  [planning LOSS = {loss}]")
            verdict, trail = run(E, Dm, D2m, MU, rhob, loss, log)
            log(f"  [planning LOSS = {loss}] {verdict}")
            out["runs"].append(dict(loss=loss, verdict=verdict, trail=trail))
        res.append(out)
        if a.out:
            json.dump(res, open(a.out + ".json", "w"), indent=1)
            open(a.out + ".log", "w").write("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
