"""M3 at the Helike background (bundle-framing §14): how the fast fibre's zero-point energy moves Helike and the competitor.

Slow model (the reduced Hamiltonian, flat kinetic 1/2 |p_q|^2): H = 1/2 |p|^2 + V_sourced(q), q_bg = s q_MC.
Fast fibre: E0_fast(F), F = 2 g q the left-invariant field in helike-core units (energies in units of 1/R).
Unit map (checked here, K7): reduced energies are YM energies / 2, so the BO term in the reduced model is
    dV(q) = 1/2 E0_fast(2 g q),  Hess_q dV = 2 g^2 Hess_F E0_fast,
and to first order a normal mode u_k (unit in q, frequency w_k) moves by
    d w_k = g^2 (u_k . Hess_F E0_fast . u_k) / w_k.
Scheme: E0 -> E0 + kappa dC with dC = (b0 / 8 pi^2) Vol density(F) (a shift of 1/g^2 at the scale 1/R). Both the
zeta (mu R = 1) value and the per-unit-kappa coefficient are reported, so the ordering question can be read for any kappa.
usage: python scripts/helike_m3_background.py OUT.json [--s 1.15 --t 3 --jmax 130 --h 0.05]"""
import argparse
import copy
import json
import math
from pathlib import Path

import numpy as np

import helike_m3_zero_point as zp
import helike_m3_zeta as z
from gigi_grid_demos import model_V

B0 = 22.0 / 3.0


def hess_q(f, q0, h=1e-3):
    n = 9; H = np.zeros((n, n)); x0 = q0.reshape(-1)
    def F(x):
        return f(x.reshape(3, 3))
    for i in range(n):
        for j in range(i, n):
            def d(a, b):
                x = x0.copy(); x[i] += a; x[j] += b; return F(x)
            v = (d(h, h) - d(h, -h) - d(-h, h) + d(-h, -h)) / (4 * h * h)
            H[i, j] = H[j, i] = v
    return H


def normal_modes(s, t, g2):
    g = math.sqrt(g2)
    q0 = s * np.diag([1 / math.sqrt(t), 1.0, 1.0]) / g
    H = hess_q(lambda q: model_V(q, s, t, g, True), q0)
    w2, U = np.linalg.eigh(H)
    return q0, w2, U


def charge(u):
    """Charge about frame axis 0 of a q-displacement u (3x3): rows 1,2 rotate. 0, 1 or 2 by the generator's action."""
    U = u.reshape(3, 3)
    a = np.linalg.norm(U[0]) ** 2
    b = np.linalg.norm(U[1:] .T @ np.array([[1, 0], [0, 1]])) ** 2
    sym = U[1:]  # 2x3 block
    return {"row0_weight": float(a), "rows12_weight": float(b)}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--s", type=float, default=1.15)
    ap.add_argument("--t", type=float, default=3.0)
    ap.add_argument("--jmax", type=int, default=130)
    ap.add_argument("--h", type=float, default=0.05)
    ap.add_argument("--g2", default="0.002,0.004")
    a = ap.parse_args()
    work = Path(a.out).parent; work.mkdir(parents=True, exist_ok=True)
    s, t = a.s, a.t
    Fbg = np.diag([2 * s / math.sqrt(t), 2 * s, 2 * s])
    g2s = [float(v) for v in a.g2.split(",")]
    # normal modes are g-independent in direction (q scales by 1/g); take them from the first g2
    q0, w2, U = normal_modes(s, t, g2s[0])
    phys = [k for k in range(9) if w2[k] > 1e-6]
    # one representative per distinct frequency
    reps = []
    for k in phys:
        if not reps or abs(w2[k] - w2[reps[-1]]) > 1e-8:
            reps.append(k)
    fields = [Fbg.tolist()]
    for k in reps:
        u = U[:, k].reshape(3, 3)
        fields += [(Fbg + a.h * u).tolist(), (Fbg - a.h * u).tolist()]
    # the unsourced YM slow sector at F_bg vs the reduced unsourced model: unit check K7
    pts = zp.spectra(t, a.jmax, (), fields=fields, work=work)
    centre = pts[0]
    def cap(p, jc):
        p = copy.deepcopy(p); p["sectors"] = [x for x in p["sectors"] if x["j"] <= jc]; return p
    rep = {"s": s, "t": t, "jmax": a.jmax, "h": a.h, "F_bg": Fbg.tolist(), "modes": []}
    vol = 2 * math.pi ** 2 * math.sqrt(t)
    for i, k in enumerate(reps):
        plus, minus = pts[1 + 2 * i], pts[2 + 2 * i]
        rp, rm = z.zeta_fast(centre, plus), z.zeta_fast(centre, minus)
        rp2, rm2 = z.zeta_fast(cap(centre, a.jmax - 20), cap(plus, a.jmax - 20)), z.zeta_fast(cap(centre, a.jmax - 20), cap(minus, a.jmax - 20))
        H_zeta = (rp["dE0_fast"] + rm["dE0_fast"]) / a.h ** 2
        H_zeta_trunc = (rp2["dE0_fast"] + rm2["dE0_fast"]) / a.h ** 2
        H_C = (rp["dC"] + rm["dC"]) / a.h ** 2                       # Hess of the log coefficient (per unit kappa)
        dens = [p["energy_density"] for p in (centre, plus, minus)]
        H_C_pred = B0 / (8 * math.pi ** 2) * vol * (dens[1] + dens[2] - 2 * dens[0]) / a.h ** 2
        mult = int(sum(1 for kk in phys if abs(w2[kk] - w2[k]) < 1e-8))
        m = {"index": k, "mult": mult, "u": U[:, k].reshape(3, 3).round(6).tolist(),
             "H_zeta": H_zeta, "H_zeta_jmax_minus_20": H_zeta_trunc, "H_C": H_C, "H_C_pred_b0": H_C_pred,
             "fit_resid": max(rp["fit_resid"], rm["fit_resid"]), "per_g2": {}}
        for g2 in g2s:
            q0g, w2g, Ug = normal_modes(s, t, g2)
            wk = math.sqrt(w2g[k])
            m["per_g2"][str(g2)] = {"omega": wk, "d_omega_zeta": g2 * H_zeta / wk, "d_omega_per_kappa": g2 * H_C / wk}
        rep["modes"].append(m)
        print(json.dumps({kk: m[kk] for kk in ("index", "mult", "H_zeta", "H_zeta_jmax_minus_20", "H_C", "H_C_pred_b0", "fit_resid")}
                         | {"per_g2": m["per_g2"]}), flush=True)
    # K7: slow sector (j = 0) of the unsourced YM Hessian at F_bg vs the reduced unsourced model at q_bg (factor 2 in w)
    mu0 = np.sort(np.array([x for x in centre["sectors"] if x["j"] == 0][0]["slice"]))
    g = math.sqrt(g2s[0])
    Hf = hess_q(lambda q: model_V(q, s, t, g, False), q0)
    # compress onto the complement of colour rotations dq = q L_a (the j = 0 gauge directions; the YM slice does this)
    L = [np.array([[0, 0, 0], [0, 0, -1], [0, 1, 0]]), np.array([[0, 0, 1], [0, 0, 0], [-1, 0, 0]]), np.array([[0, -1, 0], [1, 0, 0], [0, 0, 0]])]
    R = np.stack([(q0 @ La).reshape(-1) for La in L], axis=1)
    Qf, _ = np.linalg.qr(np.concatenate([R, np.eye(9)], axis=1))
    P = Qf[:, 3:9]
    wr2 = np.sort(np.linalg.eigvalsh(P.T @ Hf @ P))
    rep["K7_units"] = {"ym_slow_mu": mu0.tolist(), "reduced_unsourced_w2": wr2.tolist(),
                       "max_abs_mu_minus_4w2": float(np.max(np.abs(np.sort(mu0) - 4 * np.sort(wr2)))) if len(mu0) == len(wr2) else None}
    print(json.dumps(rep["K7_units"]), flush=True)
    Path(a.out).write_text(json.dumps(rep, indent=1) + "\n")


if __name__ == "__main__":
    main()
