"""Wave test, exploratory diagnostics (NOT pre-registered; run after the gated test, whose receipt is wave_test.json).

Why W2 (pointwise late growth rate) failed at t = 4.5, 6, 8 while W3 passed:
  (a) the growth rate of the global L2 norm over tau in [20, 30] (sectors are orthogonal, so no cancellation);
  (b) at each listening point, the per-sector contributions at tau = 20, 25, 30 (sign and size along the dominant
      component), to see whether near-equal exponentials of different sectors interfere;
  (c) the stable twist-takeover time: the first tau after arrival from which the twist share stays in [0.9, 1.1]
      to tau = 30 (the reported 'takeover' in wave_test.json is meaningless before arrival, where both parts are tiny).
usage: python scripts/ricci_su2_wave_diagnostics.py OUT.json
"""
import json
import sys
from pathlib import Path

import numpy as np
from scipy.linalg import expm

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_sector_gauge_check as C      # noqa: E402
import ricci_su2_spin_sector as S             # noqa: E402
import ricci_su2_wave_test as W               # noqa: E402


def run_t(t):
    g = [t, 1, 1]; TAU = W.TAU
    per_sector = {p: {} for p in range(len(W.POINTS))}
    tot = {p: np.zeros((len(TAU), 9), complex) for p in range(len(W.POINTS))}
    tw = {p: np.zeros((len(TAU), 9), complex) for p in range(len(W.POINTS))}
    norm2 = np.zeros(len(TAU))
    for j in range(W.JMAX + 1):
        d = j + 1; rho = S.spin_rep(j)
        H, G = C.hessian_and_gauge(g, j)
        L = H + G @ G.conj().T; L = 0.5 * (L + L.conj().T)
        ev, V = np.linalg.eigh(L)
        twist = np.abs(ev - W.lam(t, j)) < 1e-8 * max(1.0, abs(W.lam(t, j)))
        Delta = -sum(rho[i] @ rho[i] / g[i] for i in range(3)); Delta = 0.5 * (Delta + Delta.conj().T)
        K = expm(-W.SIGMA * Delta)
        X0 = np.zeros((9 * d, d), complex); X0[W.COMP * d:(W.COMP + 1) * d, :] = K
        Y = V.conj().T @ X0
        P = W.phi(ev, TAU)
        norm2 += d * (P ** 2) @ np.sum(np.abs(Y) ** 2, axis=1)
        for p, (_, axis, th) in enumerate(W.POINTS):
            Dg = expm(th * rho[axis])
            Wn = np.einsum("cwn,nw->cn", V.reshape(9, d, 9 * d), Y @ Dg)
            contrib = d * P @ Wn.T
            tot[p] += contrib; tw[p] += d * P[:, twist] @ Wn[:, twist].T
            if j <= 8:
                per_sector[p][j] = contrib
    sel = (TAU >= 20) & (TAU <= 30)
    out = {"t": t, "predicted_rate": float(np.sqrt(-min(W.lam(t, j) for j in range(W.JMAX + 1)))),
           "L2_norm_late_log_slope": float(np.polyfit(TAU[sel], 0.5 * np.log(norm2[sel]), 1)[0]), "points": []}
    for p, (kind, axis, th) in enumerate(W.POINTS):
        amp = np.linalg.norm(tot[p], axis=1)
        share = np.linalg.norm(tw[p], axis=1) / np.maximum(amp, 1e-300)
        ok = (share >= 0.9) & (share <= 1.1)
        stable = None
        for k in range(len(TAU) - 1, -1, -1):
            if not ok[k]:
                stable = float(TAU[k + 1]) if k + 1 < len(TAU) else None
                break
        else:
            stable = 0.0
        # dominant component at tau = 30, and per-sector projections on it at 20, 25, 30
        c = int(np.argmax(np.abs(tot[p][-1])))
        snaps = {}
        for tt in (20.0, 25.0, 30.0):
            k = int(np.argmin(np.abs(TAU - tt)))
            ref = tot[p][k, c]
            snaps[tt] = {j: float((v[k, c] / ref).real) for j, v in per_sector[p].items() if abs(v[k, c]) > 1e-3 * abs(ref)}
        sl = float(np.polyfit(TAU[sel], np.log(amp[sel]), 1)[0])
        out["points"].append({"kind": kind, "theta": th, "late_log_slope": sl, "stable_twist_takeover": stable,
                              "dominant_component": c, "sector_fractions_along_it": snaps})
    return out


def main():
    res = [run_t(t) for t in W.TS]
    for r in res:
        print(f"t={r['t']}: predicted {r['predicted_rate']:.4f}, global L2 slope {r['L2_norm_late_log_slope']:.4f}")
        for p in r["points"]:
            fr = p["sector_fractions_along_it"][30.0]
            print(f"   {p['kind']:10s} th={p['theta']}: point slope {p['late_log_slope']:.4f}; stable twist takeover "
                  f"{p['stable_twist_takeover']}; sector fractions at tau=30: {{{', '.join(f'{j}: {v:+.3f}' for j, v in fr.items())}}}")
    Path(sys.argv[1]).write_text(json.dumps(res, indent=1) + "\n")


if __name__ == "__main__":
    main()
