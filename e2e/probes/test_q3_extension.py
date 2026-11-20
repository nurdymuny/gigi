"""Q3 extension and Amendment 3 (docs/HALCYON_Q3_RUN_SPEC.md, "Amendment 3 and the extension run").
Written and committed before any extension correlator is computed.

Every point k0-k10 is scored with the verb rule after Amendment 3 (Q3-B selection, Q3-C truncation of
C(t0) directions without a 3-sigma t1 signal, Q1-1..Q1-4 plateau, jackknifed plateau error), exactly as
test_q3_scaling.py does otherwise (a^2 sigma from LTW eq. 14, refused where aL sqrt(sigma) < 3; the
ratio's error from one joint blocked jackknife over both channels).

Gates on Q3-C, judged on k7-k10 only:
  C-harm  where the Amendment-2 GEVP (no Q3-C) gives a plateau, the Q3-C value is within 1 sigma of it
          (sigma of the Amendment-2 value); a Q3-C refusal there is a failure
  C-job   k8 0++: the Q3-C plateau exists and agrees with LTW am0++ = 1.212(22) within 2 combined sigma
  C-null  every channel's shuffled projected series refuses at k7-k10 (a channel with no projected series
          is listed as vacuous, not counted as a pass)
Q3 gates with the extension: Q3-a at the four LTW betas (k0, k2, k4, k9); Q3-v at 2.2986 (k0, k7),
2.3715 (k2, k3), 2.4265 (k4, k5), 2.5115 (k9, k10); Q3-s over every beta passing both; Q3-r (k6);
Q3-n at every point (vacuous channels listed). k1 is scored and reported, not evidence for Q3-C.

usage: python test_q3_extension.py PROBE_EXE RUN_DIR
"""
import json
import subprocess
import sys
from pathlib import Path

import numpy as np

probe, run = sys.argv[1], Path(sys.argv[2])
BLOCK, T0, T1 = 100, 1, 2
LTW = {2.2986: (0.3667, 0.0018, 1.212, 0.022), 2.3715: (0.2879, 0.0013, 0.994, 0.018),
       2.4265: (0.2388, 0.0009, 0.834, 0.013), 2.5115: (0.1768, 0.0008, 0.658, 0.010)}
CONT, CONT_ERR = 3.78, 0.07
POINTS = [(0, 2.2986, 10), (1, 2.2986, 14), (2, 2.3715, 12), (3, 2.3715, 16), (4, 2.4265, 16),
          (5, 2.4265, 20), (6, 2.3, 16), (7, 2.2986, 12), (8, 2.2986, 14), (9, 2.5115, 20),
          (10, 2.5115, 24)]
PAIRS = {2.2986: (0, 7), 2.3715: (2, 3), 2.4265: (4, 5), 2.5115: (9, 10)}
EXT = (7, 8, 9, 10)


def call(payload, tag):
    i, o = run / f"{tag}.in.json", run / f"{tag}.out.json"
    i.write_text(json.dumps(payload))
    subprocess.run([probe, str(i), str(o)], check=True)
    i.unlink()
    return json.loads(o.read_text())


def proj_corr(series):
    q = series - series.mean()
    L = q.shape[1]
    A = np.stack([np.mean(q * np.roll(q, -tau, axis=1), axis=1) for tau in range(L)], axis=1)
    return A, q.mean(axis=1)


def plateau_from(A, m, idx, win, w):
    C = A[idx].mean(0) - m[idx].mean() ** 2
    lo, hi = win
    me = np.arccosh((C[lo - 1:hi] + C[lo + 1:hi + 2]) / (2 * C[lo:hi + 1]))
    return float((me * w).sum() / w.sum())


def pull(a, ea, b, eb):
    return (a - b) / np.hypot(ea, eb)


rows, gates, a2 = {}, {}, {}
for k, beta, L in POINTS:
    e = json.loads((run / f"k{k}_b{beta}_L{L}.ensemble.json").read_text())
    out = {"beta": beta, "L": L, "seed": e["seed"], "n_meas": e["n_meas"], "threads": e.get("threads", 0),
           "mean_plaquette": float(np.mean(e["mean_plaquette"]))}
    ser, vacuous = {}, []
    for ch in ("scalar", "torelon"):
        r = call({"ops": e[ch], "t0": T0, "t1": T1, "block": BLOCK}, f"ext_k{k}_{ch}_gevp")
        out[ch] = {"gevp": r["gevp"], "plateau": r.get("plateau"), "plateau_as_fixed": r.get("plateau_as_fixed"),
                   "m_eff_t": r.get("m_eff_t"), "m_eff_err_t": r.get("m_eff_err_t")}
        if "vector" in (r["gevp"] or {}):
            v = np.array(r["gevp"]["vector"])
            s = np.einsum("a,act->ct", v, np.array(e[ch]))
            ser[ch] = s
            rng = np.random.default_rng(e["seed"] + 7)
            flat = s.flatten()
            rng.shuffle(flat)
            out[ch]["null"] = call({"p": flat.reshape(s.shape).tolist(), "block": BLOCK}, f"ext_k{k}_{ch}_null")["plateau"]
        else:
            vacuous.append(ch)
        if k in EXT:  # the Amendment-2 arm, for C-harm and for reporting whether Q3-C was needed
            ra = call({"ops": e[ch], "t0": T0, "t1": T1, "block": BLOCK, "truncate": False}, f"ext_k{k}_{ch}_a2")
            a2[(k, ch)] = ra.get("plateau") if "vector" in (ra["gevp"] or {}) else {"refused": ra["gevp"].get("refused")}
    if k == 6:
        out["level0_scalar_alone"] = call({"p": e["scalar"][0], "block": BLOCK}, "ext_k6_scalar_level0")["plateau"]
    sp, tp = out["scalar"].get("plateau") or {}, out["torelon"].get("plateau") or {}
    if "mass" in tp:
        a2s = (tp["mass"] + np.pi / (3 * L)) / L
        out["a2sigma"], out["a2sigma_err"] = a2s, tp["error"] / L
        out["a_sqrt_sigma"], out["a_sqrt_sigma_err"] = np.sqrt(a2s), tp["error"] / L / (2 * np.sqrt(a2s))
        out["aL_sqrt_sigma"] = L * np.sqrt(a2s)
        out["sigma_refused"] = bool(out["aL_sqrt_sigma"] < 3)
    if "mass" in sp and "mass" in tp and not out.get("sigma_refused"):
        As, ms = proj_corr(ser["scalar"])
        At, mt = proj_corr(ser["torelon"])
        win_s, win_t = (sp["t_lo"], sp["t_hi"]), (tp["t_lo"], tp["t_hi"])
        ws = np.array([1 / out["scalar"]["m_eff_err_t"][t] ** 2 for t in range(win_s[0], win_s[1] + 1)])
        wt = np.array([1 / out["torelon"]["m_eff_err_t"][t] ** 2 for t in range(win_t[0], win_t[1] + 1)])
        n = len(ms)
        B = n // BLOCK

        def ratio(idx):
            m0 = plateau_from(As, ms, idx, win_s, ws)
            a2_ = (plateau_from(At, mt, idx, win_t, wt) + np.pi / (3 * L)) / L
            return m0 / np.sqrt(a2_)
        js = np.array([ratio(np.r_[0:b * BLOCK, (b + 1) * BLOCK:n]) for b in range(B)])
        out["ratio"], out["ratio_err"] = ratio(np.arange(n)), float(np.sqrt((B - 1) / B * ((js - js.mean()) ** 2).sum()))
    out["null_vacuous"] = vacuous
    rows[k] = out
    gates[f"Q3-n_k{k}"] = all("refused" in (out[ch].get("null") or {}) for ch in ("scalar", "torelon") if ch not in vacuous)
    print(json.dumps({x: out.get(x) for x in ("beta", "L", "a_sqrt_sigma", "a_sqrt_sigma_err", "ratio", "ratio_err")}
                     | {"am0": sp.get("mass"), "am0_err": sp.get("error"), "scalar": sp.get("refused") or out["scalar"]["gevp"].get("refused"),
                        "torelon": tp.get("refused") or out["torelon"]["gevp"].get("refused"), "vacuous": vacuous}), flush=True)

# Q3-a, Q3-v, Q3-s
passed_beta = []
for beta, (kc, kv) in PAIRS.items():
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
        passed_beta.append((beta, rc["a2sigma"], rc["ratio"], rc["ratio_err"]))

scaling = {"points": passed_beta}
if len(passed_beta) >= 2:
    _, x, y, s = (np.array(c) for c in zip(*passed_beta))
    W = np.diag(1 / s ** 2)
    X = np.stack([np.ones_like(x), x], axis=1)
    cov = np.linalg.inv(X.T @ W @ X)
    c, d = cov @ X.T @ W @ y
    chi2 = float(((y - (c + d * x)) / s) ** 2 @ np.ones_like(y))
    dof = len(y) - 2
    monotone = bool(np.all(np.diff(y[np.argsort(x)]) > 0) or np.all(np.diff(y[np.argsort(x)]) < 0))
    drift = bool(abs(y.max() - y.min()) > np.hypot(*sorted(s)[-2:]))
    chi2_dof = chi2 / dof if dof > 0 else None
    scaling |= {"c": float(c), "c_err": float(np.sqrt(cov[0, 0])), "d": float(d), "d_err": float(np.sqrt(cov[1, 1])),
                "chi2": chi2, "dof": dof, "chi2_dof": chi2_dof,
                "falsifier": bool(monotone and drift and chi2_dof is not None and chi2_dof > 2),
                "pull_vs_LTW_continuum": float(pull(c, np.sqrt(cov[0, 0]), CONT, CONT_ERR))}
    gates["Q3-s"] = bool((not scaling["falsifier"]) and abs(scaling["pull_vs_LTW_continuum"]) <= 2)
else:
    gates["Q3-s"] = False
    scaling["note"] = "fewer than two betas passed Q3-a and Q3-v"

r6 = rows[6]
g6 = r6["scalar"].get("plateau") or {}
l0 = r6.get("level0_scalar_alone") or {}
gates["Q3-r"] = bool("mass" in g6 and ("refused" in l0 or pull(l0["mass"], l0["error"], g6["mass"], g6["error"]) > 3))

# Amendment 3 gates
harm = {}
for k in EXT:
    for ch in ("scalar", "torelon"):
        pa, pc = a2[(k, ch)] or {}, rows[k][ch].get("plateau") or {}
        if "mass" in pa:
            ok = "mass" in pc and abs(pc["mass"] - pa["mass"]) <= pa["error"]
            harm[f"k{k}_{ch}"] = {"a2": pa, "q3c": pc, "pass": bool(ok)}
        else:
            harm[f"k{k}_{ch}"] = {"a2": pa, "q3c": pc, "pass": None}
gates["C-harm"] = all(v["pass"] for v in harm.values() if v["pass"] is not None)
p8 = rows[8]["scalar"].get("plateau") or {}
gates["C-job"] = bool("mass" in p8 and abs(pull(p8["mass"], p8["error"], 1.212, 0.022)) <= 2)
gates["C-null"] = all(gates[f"Q3-n_k{k}"] for k in EXT)
amend3 = {"C-harm_detail": harm,
          "C-job_k8": {"q3c": p8, "a2_scalar": a2[(8, "scalar")],
                       "a2_refused_at_k8": "refused" in (a2[(8, "scalar")] or {"refused": 1})},
          "null_vacuous": {k: rows[k]["null_vacuous"] for k in EXT if rows[k]["null_vacuous"]}}

rec = {"kind": "Q3 extension + Amendment 3 (Q3-C)", "spec": "docs/HALCYON_Q3_RUN_SPEC.md",
       "gates": gates, "all_pass": all(gates.values()), "scaling": scaling, "amendment3": amend3,
       "rows": rows}
(run / "q3ext_receipt.json").write_text(json.dumps(rec, indent=1, default=float) + "\n")
print(json.dumps({"gates": gates, "scaling": scaling, "C-job_k8": amend3["C-job_k8"],
                  "null_vacuous": amend3["null_vacuous"]}, indent=1, default=float))
