"""The wave test (docs/research/wave-test-preregistration-2026-11-20.md, fixed before this script was written).

Linearised, gauge-fixed wave equation on the frozen Berger field of (t, 1, 1): a'' = -(H + G G^*) a, a(0) = 0,
a'(0) = heat-smoothed delta at e in the component theta^1 (x) tau_1. Peter-Weyl synthesis
  a_{c'}(g, tau) = sum_j d_j tr( [Phi_j(tau)]_{c'c} K_j D^j(g) ),
K_j = exp(-sigma Delta_j), Delta_j = -sum_i rho_i^2 / g_i, Phi_j(tau) = sin(sqrt(L) tau)/sqrt(L) on the spectrum of
L = H + G G^* in sector j (sinh / tau for negative / zero eigenvalues), D^j(exp X) = expm(rho(X)).
Gates: W1 (causality, code check), W2 (late growth rate = sqrt(-min_j lambda_t(j)) within 2%), W3 (>= 99% of the late
L2 norm in the twist pairs). Reported: arrival times, twist-takeover time at each point, carrying sectors.
usage: python scripts/ricci_su2_wave_test.py OUT.json
"""
import json
import sys
from pathlib import Path

import numpy as np
from scipy.linalg import expm

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_sector_gauge_check as C      # noqa: E402
import ricci_su2_spin_sector as S             # noqa: E402

JMAX, SIGMA, COMP = 60, 0.01, 1 * 3 + 1          # Amendment 1          # component (frame 1, Lie 1)
TS = (3.0, 4.5, 6.0, 8.0)
TAU = np.linspace(0.0, 30.0, 3001)
POINTS = [("transverse", 1, th) for th in (0.5, 1.0, 1.5, 2.0, 2.5)] + [("fibre", 0, 0.5), ("fibre", 0, 1.0)]


def lam(t, j):
    return (j * (j + 2) - 2 * (t - 3) * j - 2 * (t - 2) * (t + 4)) / t


def phi(ev, tau):
    """sin(sqrt(ev) tau)/sqrt(ev), continued to sinh and tau."""
    out = np.empty((len(tau), len(ev)))
    for k, e in enumerate(ev):
        if e > 1e-12:
            w = np.sqrt(e); out[:, k] = np.sin(w * tau) / w
        elif e < -1e-12:
            w = np.sqrt(-e); out[:, k] = np.sinh(w * tau) / w
        else:
            out[:, k] = tau
    return out


def run_t(t):
    g = [t, 1, 1]
    ld = lam(t, np.arange(JMAX + 1)); lmin = float(ld.min())
    sig = {p: np.zeros((len(TAU), 9), complex) for p in range(len(POINTS))}
    twist_sig = {p: np.zeros((len(TAU), 9), complex) for p in range(len(POINTS))}
    norm2_T = 0.0; twist2_T = 0.0; by_sector_T = {}
    for j in range(JMAX + 1):
        d = j + 1; rho = S.spin_rep(j)
        H, G = C.hessian_and_gauge(g, j)
        L = H + G @ G.conj().T; L = 0.5 * (L + L.conj().T)
        ev, V = np.linalg.eigh(L)
        twist = np.abs(ev - lam(t, j)) < 1e-8 * max(1.0, abs(lam(t, j)))
        if twist.sum() != 2:
            raise SystemExit(f"t={t} j={j}: twist pair not found ({twist.sum()})")
        Delta = -sum(rho[i] @ rho[i] / g[i] for i in range(3)); Delta = 0.5 * (Delta + Delta.conj().T)
        K = expm(-SIGMA * Delta)                                   # d x d
        # initial velocity in sector j: columns k: (K e_k) (x) e_COMP  -> matrix X0 of shape (9d, d)
        X0 = np.zeros((9 * d, d), complex)
        X0[COMP * d:(COMP + 1) * d, :] = K
        Y = V.conj().T @ X0                                        # modal coefficients, (9d, d)
        P = phi(ev, TAU)                                           # (ntau, 9d)
        # field: a_{c'}(g) = d * sum_{w, k} [V diag(P) Y]_{(c', w), k} D(g)_{k, w}
        for p, (_, axis, th) in enumerate(POINTS):
            Dg = expm(th * rho[axis])
            M = Y @ Dg                                            # (9d, d): sum_k Y_{n,k} D_{k,w}
            Vr = V.reshape(9, d, 9 * d)                            # [c', w, n]
            Wn = np.einsum("cwn,nw->cn", Vr, M)                    # (9, 9d)
            sig[p] += d * P @ Wn.T
            twist_sig[p] += d * P[:, twist] @ Wn[:, twist].T
        # L2 norm at tau_end: sum_j d_j ||F_j||^2 / d_j^2 with F_j = d_j * (V P Y)  -> d_j ||P Y||_F^2
        PY = P[-1][:, None] * Y
        n2 = d * float(np.sum(np.abs(PY) ** 2)); t2 = d * float(np.sum(np.abs(PY[twist]) ** 2))
        norm2_T += n2; twist2_T += t2; by_sector_T[j] = n2
    res = {"t": t, "lambda_min": lmin, "predicted_rate": float(np.sqrt(-lmin)), "points": []}
    w1_ok = True; w2_ok = True
    for p, (kind, axis, th) in enumerate(POINTS):
        amp = np.linalg.norm(sig[p], axis=1); tamp = np.linalg.norm(twist_sig[p], axis=1)
        d_low = th; d_high = th * (np.sqrt(t) if axis == 0 else 1.0)
        ref = amp[(TAU >= d_high) & (TAU <= d_high + 1)].max()
        pre = amp[TAU <= d_low - 0.75]
        w1 = float(pre.max() / ref) if pre.size else None
        if w1 is not None:
            w1_ok &= w1 <= 1e-4
        sel = (TAU >= 20) & (TAU <= 30)
        slope = float(np.polyfit(TAU[sel], np.log(amp[sel]), 1)[0])
        w2_ok &= abs(slope / np.sqrt(-lmin) - 1) <= 0.02
        arr = float(TAU[np.argmax(amp > 1e-3 * ref)])
        share = tamp / np.maximum(amp, 1e-300)
        take = TAU[share > 0.9]
        res["points"].append({"kind": kind, "theta": th, "distance_bounds": [th, th * (np.sqrt(t) if axis == 0 else 1.0)],
                              "W1_pre_arrival_ratio": w1, "arrival_time": arr, "late_log_slope": slope,
                              "twist_share_at_end": float(share[-1]),
                              "twist_takeover_time": float(take[0]) if take.size else None})
    w3 = twist2_T / norm2_T
    top = sorted(by_sector_T.items(), key=lambda kv: -kv[1])[:4]
    res.update({"W1": bool(w1_ok), "W2": bool(w2_ok), "W3_twist_share_L2": w3, "W3": bool(w3 >= 0.99),
                "carrying_sectors_at_end": [(j, v / norm2_T) for j, v in top]})
    return res


def main():
    out = [run_t(t) for t in TS]
    for r in out:
        print(f"t={r['t']}: W1 {r['W1']}  W2 {r['W2']} (pred {r['predicted_rate']:.4f}, slopes "
              f"{[round(p['late_log_slope'], 4) for p in r['points']]})  W3 {r['W3']} ({r['W3_twist_share_L2']:.6f}); "
              f"carriers {[(j, round(s, 3)) for j, s in r['carrying_sectors_at_end']]}", flush=True)
        for p in r["points"]:
            print(f"     {p['kind']:10s} theta={p['theta']}: pre-arrival {p['W1_pre_arrival_ratio']}, arrival "
                  f"{p['arrival_time']:.2f} (distance in {p['distance_bounds']}), twist >90% from "
                  f"{p['twist_takeover_time']}", flush=True)
    summary = {"W1_all": all(r["W1"] for r in out), "W2_all": all(r["W2"] for r in out), "W3_all": all(r["W3"] for r in out)}
    print(summary)
    Path(sys.argv[1]).write_text(json.dumps({"summary": summary, "runs": out}, indent=1) + "\n")


if __name__ == "__main__":
    main()
