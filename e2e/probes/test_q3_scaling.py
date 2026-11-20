"""Q3 gates (docs/HALCYON_Q3_RUN_SPEC.md, research/q3-scaling-20261119/RUN_LOG.md). Written and
committed before any Q3 ensemble finished.

Per point: the engine probe runs the GEVP over the four smearing levels at (t0, t1) = (1, 2) for the
0++ operator and for the flux loop, projects, and applies EFFECTIVE_MASS (amended rule, plateau error by
the jackknife of the fit). Here:
  a^2 sigma = (am_P + pi/(3L)) / L      (LTW eq. 14); refused where aL sqrt(sigma) < 3
  m0++/sqrt(sigma) with its error from one joint blocked jackknife (blocks of 100) over both channels:
  in every leave-a-block-out sample both projected correlators are recomputed, the cosh m_eff taken,
  and each plateau averaged over its fixed window with its fixed weights (Amendment Q1-3).

Gates: Q3-a (LTW Table 5 within 2 combined sigma at k = 0, 2, 4), Q3-v (the two volumes agree within
2 sigma), Q3-s (c + d a^2 sigma over the betas passing a and v; c within 2 sigma of 3.78(7); the
falsifier: monotone drift and chi2/dof > 2), Q3-r (k = 6: level 0 alone refuses or lies > 3 sigma above
the GEVP value), Q3-n (the shuffled projected series refuses, both channels, every point).

usage: python test_q3_scaling.py PROBE_EXE RUN_DIR
"""
import json
import subprocess
import sys
from pathlib import Path

import numpy as np

probe, run = sys.argv[1], Path(sys.argv[2])
BLOCK, T0, T1 = 100, 1, 2
LTW = {2.2986: (0.3667, 0.0018, 1.212, 0.022), 2.3715: (0.2879, 0.0013, 0.994, 0.018),
       2.4265: (0.2388, 0.0009, 0.834, 0.013)}
CONT, CONT_ERR = 3.78, 0.07
POINTS = [(0, 2.2986, 10), (1, 2.2986, 14), (2, 2.3715, 12), (3, 2.3715, 16), (4, 2.4265, 16),
          (5, 2.4265, 20), (6, 2.3, 16)]


def call(payload, tag):
    i, o = run / f"{tag}.in.json", run / f"{tag}.out.json"
    i.write_text(json.dumps(payload))
    subprocess.run([probe, str(i), str(o)], check=True)
    i.unlink()
    return json.loads(o.read_text())


def proj_corr(series):
    """Shift-invariant connected correlator rows: per-config a_c(tau), m_c (as the engine)."""
    q = series - series.mean()
    L = q.shape[1]
    A = np.stack([np.mean(q * np.roll(q, -tau, axis=1), axis=1) for tau in range(L)], axis=1)
    return A, q.mean(axis=1)


def plateau_from(A, m, idx, win, w):
    C = A[idx].mean(0) - m[idx].mean() ** 2
    lo, hi = win
    me = np.arccosh((C[lo - 1:hi] + C[lo + 1:hi + 2]) / (2 * C[lo:hi + 1]))
    return float((me * w).sum() / w.sum())


rows, gates = {}, {}
for k, beta, L in POINTS:
    f = run / f"k{k}_b{beta}_L{L}.ensemble.json"
    e = json.loads(f.read_text())
    out = {"beta": beta, "L": L, "seed": e["seed"], "n_meas": e["n_meas"],
           "mean_plaquette": float(np.mean(e["mean_plaquette"]))}
    ser = {}
    for ch in ("scalar", "torelon"):
        r = call({"ops": e[ch], "t0": T0, "t1": T1, "block": BLOCK}, f"k{k}_{ch}_gevp")
        out[ch] = {"gevp": r["gevp"], "plateau": r.get("plateau"), "plateau_as_fixed": r.get("plateau_as_fixed"),
                   "m_eff_t": r.get("m_eff_t"), "m_eff_err_t": r.get("m_eff_err_t")}
        if "vector" in (r["gevp"] or {}):
            v = np.array(r["gevp"]["vector"])
            s = np.einsum("a,act->ct", v, np.array(e[ch]))
            ser[ch] = s
            rng = np.random.default_rng(e["seed"] + 7)
            flat = s.flatten()
            rng.shuffle(flat)
            out[ch]["null"] = call({"p": flat.reshape(s.shape).tolist(), "block": BLOCK}, f"k{k}_{ch}_null")["plateau"]
    if k == 6:
        out["level0_scalar_alone"] = call({"p": e["scalar"][0], "block": BLOCK}, "k6_scalar_level0")["plateau"]
    sp, tp = out["scalar"].get("plateau") or {}, out["torelon"].get("plateau") or {}
    if "mass" in tp:
        a2s = (tp["mass"] + np.pi / (3 * L)) / L
        out["a2sigma"], out["a2sigma_err"] = a2s, tp["error"] / L
        out["a_sqrt_sigma"], out["a_sqrt_sigma_err"] = np.sqrt(a2s), tp["error"] / L / (2 * np.sqrt(a2s))
        out["aL_sqrt_sigma"] = L * np.sqrt(a2s)
        out["sigma_refused"] = out["aL_sqrt_sigma"] < 3
    if "mass" in sp and "mass" in tp and not out.get("sigma_refused"):
        # one joint blocked jackknife over both channels
        As, ms = proj_corr(ser["scalar"])
        At, mt = proj_corr(ser["torelon"])
        win_s, win_t = (sp["t_lo"], sp["t_hi"]), (tp["t_lo"], tp["t_hi"])
        ws = np.array([1 / out["scalar"]["m_eff_err_t"][t] ** 2 for t in range(win_s[0], win_s[1] + 1)])
        wt = np.array([1 / out["torelon"]["m_eff_err_t"][t] ** 2 for t in range(win_t[0], win_t[1] + 1)])
        n = len(ms)
        B = n // BLOCK

        def ratio(idx):
            m0 = plateau_from(As, ms, idx, win_s, ws)
            a2 = (plateau_from(At, mt, idx, win_t, wt) + np.pi / (3 * L)) / L
            return m0 / np.sqrt(a2)
        full = ratio(np.arange(n))
        js = np.array([ratio(np.r_[0:b * BLOCK, (b + 1) * BLOCK:n]) for b in range(B)])
        out["ratio"], out["ratio_err"] = full, float(np.sqrt((B - 1) / B * ((js - js.mean()) ** 2).sum()))
    rows[k] = out
    gates[f"Q3-n_k{k}"] = all("refused" in (out[ch].get("null") or {"refused": 1}) for ch in ("scalar", "torelon"))
    print(json.dumps({x: out.get(x) for x in ("beta", "L", "a_sqrt_sigma", "a_sqrt_sigma_err", "ratio", "ratio_err")}
                     | {"am0": sp.get("mass"), "am0_err": sp.get("error"), "scalar": sp.get("refused"),
                        "torelon": tp.get("refused")}), flush=True)


def pull(a, ea, b, eb):
    return (a - b) / np.hypot(ea, eb)


passed_beta = []
for (kc, kv), beta in zip([(0, 1), (2, 3), (4, 5)], LTW):
    rc, rv = rows[kc], rows[kv]
    sig, sige, m0, m0e = LTW[beta]
    sp = rc["scalar"].get("plateau") or {}
    ok_a = "mass" in sp and "a_sqrt_sigma" in rc and not rc.get("sigma_refused") \
        and abs(pull(rc["a_sqrt_sigma"], rc["a_sqrt_sigma_err"], sig, sige)) <= 2 \
        and abs(pull(sp["mass"], sp["error"], m0, m0e)) <= 2
    spv = rv["scalar"].get("plateau") or {}
    ok_v = ok_a and "mass" in spv and "a_sqrt_sigma" in rv and not rv.get("sigma_refused") \
        and abs(pull(sp["mass"], sp["error"], spv["mass"], spv["error"])) <= 2 \
        and abs(pull(rc["a_sqrt_sigma"], rc["a_sqrt_sigma_err"], rv["a_sqrt_sigma"], rv["a_sqrt_sigma_err"])) <= 2
    gates[f"Q3-a_beta{beta}"], gates[f"Q3-v_beta{beta}"] = bool(ok_a), bool(ok_v)
    if ok_a and ok_v and "ratio" in rc:
        passed_beta.append((rc["a2sigma"], rc["ratio"], rc["ratio_err"]))

scaling = {"points": passed_beta}
if len(passed_beta) >= 2:
    x, y, s = (np.array(c) for c in zip(*passed_beta))
    W = np.diag(1 / s ** 2)
    X = np.stack([np.ones_like(x), x], axis=1)
    cov = np.linalg.inv(X.T @ W @ X)
    c, d = cov @ X.T @ W @ y
    chi2 = float(((y - (c + d * x)) / s) ** 2 @ np.ones_like(y))
    dof = max(len(y) - 2, 1)
    monotone = bool(np.all(np.diff(y[np.argsort(x)]) > 0) or np.all(np.diff(y[np.argsort(x)]) < 0))
    drift = bool(abs(y.max() - y.min()) > np.hypot(*sorted(s)[-2:]))
    scaling |= {"c": float(c), "c_err": float(np.sqrt(cov[0, 0])), "d": float(d), "chi2_dof": chi2 / dof,
                "falsifier": bool(monotone and drift and chi2 / dof > 2)}
    gates["Q3-s"] = (not scaling["falsifier"]) and abs(pull(c, np.sqrt(cov[0, 0]), CONT, CONT_ERR)) <= 2
else:
    gates["Q3-s"] = False
    scaling["note"] = "fewer than two betas passed Q3-a and Q3-v"

r6 = rows[6]
g6 = r6["scalar"].get("plateau") or {}
l0 = r6.get("level0_scalar_alone") or {}
gates["Q3-r"] = "mass" in g6 and ("refused" in l0 or pull(l0["mass"], l0["error"], g6["mass"], g6["error"]) > 3)

rec = {"kind": "Q3: m0++/sqrt(sigma) in the engine against LTW", "spec": "docs/HALCYON_Q3_RUN_SPEC.md",
       "gates": gates, "all_pass": all(gates.values()), "scaling": scaling, "rows": rows}
(run / "q3_receipt.json").write_text(json.dumps(rec, indent=1, default=float) + "\n")
print(json.dumps({"gates": gates, "scaling": scaling}, indent=1, default=float))
