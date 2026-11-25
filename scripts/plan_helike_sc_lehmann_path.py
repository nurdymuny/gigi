"""PATH SEARCH for the Lehmann homotopy on base B3' (registration section 11). Floating point; not a bound.

Question: does ANY admissible index path exist from the exact base spectrum (s = 0) to a Lehmann step giving
l_11 >= rho_b, within 40 steps? The registered greedy rule (section 9.3) failed (section 10); this replaces the rule by
an exhaustive search over the s-grid, with Ritz values as proxies for the Lehmann bounds.

Model and basis: identical to scripts/dryrun_helike_sc_lehmann_homotopy.py (imported).

State (N, k): the last Lehmann step was at s_k with N trial states, and rho = l_{N+1}(s_k) is the bound for the next
step (at k = 0, rho = E_{N+1}, the exact base level). Proxy: l_i(s_k) = mu_i(s_k) - LOSS for k > 0.
Move: (N, k) -> step at k' > k, admissible when mu_N(s_k') <= rho - DELTA; it yields l_1..l_N at s_k', and the next
state (N', k') with 12 <= N' <= N - 1 (keeping rho, j = N + 1, is dominated and omitted). Goal: a step at k' with
mu_11(s_k') - LOSS >= rho_b. Dynamic programming in decreasing N (N strictly decreases), minimising steps.

A found path is then VALIDATED: executed with real Lehmann steps (real l_j for every next rho, real admissibility
mu_N <= rho - DELTA) and judged by the real l_11 >= rho_b.

usage: python scripts/plan_helike_sc_lehmann_path.py [--controls 0.004,0.002] [--cache DIR] [--out PREFIX]
"""
import argparse
import json
import math
import os
import time

import numpy as np
from scipy.linalg import eigh

import dryrun_helike_sc_lehmann_homotopy as dr
import estimate_helike_sc_gbase as gb

S_GRID = np.linspace(0.0, 1.0, 801)
NMIN = 12                          # section 9.5: N' >= 12
CONFIGS = ((0.0, 0.0), (0.02, 0.0), (0.02, 0.01))   # (DELTA, LOSS); the last is the registered judging config


def build(g2, E0rel, cache, log):
    f = os.path.join(cache, f"ops_g{g2}.npz") if cache else None
    if f and os.path.exists(f):
        z = np.load(f)
        return z["E"], z["Dm"], z["D2m"], z["MU"]
    t0 = time.time(); rng = np.random.default_rng(2)
    m = gb.setup(g2); rhob = E0rel + gb.RHO_B_REL
    P, _ = dr.fit_quartic(m, rng)
    C3 = max(gb.cubic_constant(m, np.random.default_rng(2)), dr.cubic_constant_poly(P, rng))
    vlo, _, beta = gb.curvature(m, math.hypot(dr.LU, dr.LV), np.random.default_rng(2), n=300)
    w2 = m["w2"]; Lt = math.hypot(dr.LU, dr.LV)
    ws, wt = w2[:3].min() - 2 * C3 * Lt, w2[3:].min() - 2 * C3 * Lt
    D = dict(P)
    for k in range(6):
        e = tuple(2 if i == k else 0 for i in range(6))
        D[e] = D.get(e, 0.0) - 0.5 * (ws if k < 3 else wt)
    ecut = rhob + 5.0
    while True:
        ub = dr.Block(ws, dr.LU, beta, vlo, ecut); vb = dr.Block(wt, dr.LV, beta, 0.0, ecut)
        pairs = sorted(((a + b, i, j) for i, a in enumerate(ub.E) for j, b in enumerate(vb.E)), key=lambda x: x[0])
        if len(pairs) > dr.NBASIS and pairs[dr.NBASIS][0] <= ecut + min(ub.E.min(), vb.E.min()):
            break
        ecut += 3.0
    cut = pairs[dr.NBASIS][0]
    pairs = [p for p in pairs if p[0] < cut - 1e-12]
    E = np.array([p[0] for p in pairs]); I = [p[1] for p in pairs]; J = [p[2] for p in pairs]
    uidx = sorted(set(I)); vidx = sorted(set(J))
    upos = {k: n for n, k in enumerate(uidx)}; vpos = {k: n for n, k in enumerate(vidx)}
    Ui = np.array([upos[i] for i in I]); Vi = np.array([vpos[j] for j in J])
    Um = ub.monomials(uidx); Vm = vb.monomials(vidx)
    Dm = dr.product_operator(D, ub, vb, Ui, Vi, Um, Vm)
    D2m = dr.product_operator(dr.pmul(D, D), ub, vb, Ui, Vi, Um, Vm)
    Dm, D2m = 0.5 * (Dm + Dm.T), 0.5 * (D2m + D2m.T)
    MU = np.array([eigh(np.diag(E) + s * Dm, eigvals_only=True, subset_by_index=[0, dr.KMAX - 1]) for s in S_GRID])
    log(f"  built: {len(E)} basis functions, C3 {C3:.5f}, {time.time() - t0:.0f} s")
    if f:
        os.makedirs(cache, exist_ok=True); np.savez(f, E=E, Dm=Dm, D2m=D2m, MU=MU)
    return E, Dm, D2m, MU


def plan(MU, rhob, delta, loss, max_steps=dr.MAX_STEPS):
    K, NM = MU.shape                       # NM = KMAX; mu_i(k) = MU[k, i-1]
    INF = 10 ** 9
    best = np.full((NM, K), INF, dtype=np.int64)
    par = {}
    best[NMIN:NM - 1, 0] = 0               # state (N, 0), rho = E_{N+1}
    goal = (INF, None)
    rho = lambda N, k: MU[k, N] - (loss if k > 0 else 0.0)
    for N in range(NM - 2, NMIN - 1, -1):
        ks = np.where(best[N] < INF)[0]
        if not len(ks):
            continue
        arr = np.full(K, INF, dtype=np.int64); src = np.full(K, -1)
        col = MU[:, N - 1]
        for k in ks:
            thr = rho(N, k) - delta
            kmax = np.searchsorted(col, thr, side="right") - 1      # col is non-decreasing in s
            if kmax <= k:
                continue
            seg = slice(k + 1, kmax + 1); c = best[N, k] + 1
            better = c < arr[seg]
            arr[seg] = np.where(better, c, arr[seg]); src[seg] = np.where(better, k, src[seg])
        hit = np.where((arr < INF) & (MU[:, 10] - loss >= rhob))[0]
        for kp in hit:
            if arr[kp] < goal[0]:
                goal = (int(arr[kp]), (N, int(kp)))
        for Np in range(NMIN, N):
            imp = arr < best[Np]
            if imp.any():
                best[Np] = np.where(imp, arr, best[Np])
                for kp in np.where(imp)[0]:
                    par[(Np, int(kp))] = (N, int(src[kp]))
    if goal[1] is None or goal[0] > max_steps:
        return None, goal[0] if goal[1] else None
    # path: list of steps (s index, trial N, next N')
    N, kp = goal[1]
    steps = [(kp, N, None)]
    k = None
    # the step at kp came from state (N, src) where state (N, src) was produced at a step with trial count par
    state = None
    # rebuild: the goal step used trial N from some state (N, k0); find k0 = argmin source recorded via a re-scan
    col = MU[:, N - 1]
    cands = [k0 for k0 in np.where(best[N] < INF)[0] if k0 < kp and col[kp] <= rho(N, k0) - delta]
    k0 = min(cands, key=lambda c: best[N, c]); state = (N, int(k0))
    while state[1] != 0:
        prevN, prevk = par[state]
        steps.append((state[1], prevN, state[0]))
        state = (prevN, prevk)
    steps.reverse()
    return dict(n_steps=len(steps), N0=state[0], steps=[(float(S_GRID[k]), int(n), nx) for k, n, nx in steps]), goal[0]


def validate(E, Dm, D2m, MU, path, rhob, delta, log):
    N = path["N0"]; rho = E[N]                 # rho = E_{N+1}, exact base
    for s, Ntrial, Nnext in path["steps"]:
        assert Ntrial == N
        k = int(round(s * (len(S_GRID) - 1)))
        if MU[k, N - 1] > rho - delta:
            return f"REAL FAIL at s = {s:.4f}: mu_N = {MU[k, N - 1]:.4f} > rho - DELTA = {rho - delta:.4f}", None
        lo, mu, resid, nneg = dr.lehmann(E, Dm, D2m, s, rho, N)
        log(f"    real step s = {s:.4f}  N = {N:3d}  rho = {rho:.4f}  l_11 = {lo[10]:.4f} (mu_11 {mu[10]:.4f})"
            f"  max resid {resid.max():.1e}")
        if lo[10] >= rhob:
            return f"REAL PASS: l_11 = {lo[10]:.4f} >= rho_b = {rhob:.4f} at s = {s:.4f}", float(lo[10])
        if Nnext is None:
            return f"REAL FAIL: final l_11 = {lo[10]:.4f} < rho_b = {rhob:.4f}", float(lo[10])
        rho = lo[Nnext]; N = Nnext                # rho' = l_{N'+1}
        if not np.isfinite(rho):
            return f"REAL FAIL: no Lehmann bound at index {Nnext + 1} at s = {s:.4f}", None
    return "REAL FAIL: path ended", None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--controls", default="0.004,0.002")
    ap.add_argument("--cache", default=None)
    ap.add_argument("--out", default=None)
    a = ap.parse_args()
    lines, res = [], []
    def log(s):
        print(s, flush=True); lines.append(s)
    log("Lehmann path search on B3' (L_u = 5, L_v = 2), 801-point s-grid; floating point, not a bound")
    for g2, E0 in gb.CONTROLS:
        if str(g2) not in a.controls.split(","):
            continue
        rhob = E0 + gb.RHO_B_REL
        log(f"\n=== control g2 = {g2}: rho_b - E0 = {gb.RHO_B_REL}")
        E, Dm, D2m, MU = build(g2, E0, a.cache, log)
        log(f"  mu_11(1) - rho_b = {MU[-1, 10] - rhob:+.4f}; first s with mu_11 >= rho_b: "
            f"{S_GRID[np.argmax(MU[:, 10] >= rhob)] if (MU[:, 10] >= rhob).any() else 'none'}")
        out = dict(g2=g2, configs=[])
        for delta, loss in CONFIGS:
            path, nmin = plan(MU, rhob, delta, loss)
            tag = f"DELTA = {delta}, LOSS = {loss}"
            if path is None:
                log(f"  [{tag}] NO PATH" + (f" within 40 steps (shortest {nmin})" if nmin else ""))
                out["configs"].append(dict(delta=delta, loss=loss, path=None, shortest=nmin)); continue
            log(f"  [{tag}] PATH: {path['n_steps']} steps from N0 = {path['N0']}: " +
                " -> ".join(f"(s {s:.3f}, N {n})" for s, n, _ in path["steps"]))
            verdict, l11 = validate(E, Dm, D2m, MU, path, rhob, delta, log)
            log(f"  [{tag}] {verdict}")
            out["configs"].append(dict(delta=delta, loss=loss, path=path, validation=verdict, l11=l11))
        res.append(out)
        if a.out:
            json.dump(res, open(a.out + ".json", "w"), indent=1)
            open(a.out + ".log", "w").write("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
