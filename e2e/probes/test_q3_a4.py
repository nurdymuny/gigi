"""Amendment 4 (docs/HALCYON_Q3_RUN_SPEC.md, "Amendment 4"): scoring of the fresh points k11-k19.
Written and committed while k11-k19 sample, before any of their correlators is computed.

Verb rule: probe "rule": "a4" (Amendment-2 GEVP, Q3-C only as a fallback, no GEVP plateau slice before t1).
Counterfactual for A4-start: "a4_nostart" (A4-1 alone). Everything else as test_q3_extension.py: a^2 sigma
from LTW eq. 14, refused where aL sqrt(sigma) < 3; the ratio's error from one joint blocked jackknife over
both channels; Q3-a / Q3-v / Q3-s as fixed in the run spec.

Judged only on k11-k19:
  A4-path   every value reports its path; fallback values listed (report)
  A4-null   every channel's shuffled projected series refuses, for the Amendment-2 vector and the Q3-C
            vector wherever each exists (stop if not)
  A4-start  every channel where A4-2 moved the window or produced a refusal, with the A4-1-alone value
            (record, not pass/fail)
  Q3-a, Q3-v at (k11,k12) (k13,k14) (k15,k16) (k17,k18); Q3-s over the β passing both
  k17 replicate: does k17's am0++ reproduce k9's excess (> 2 sigma above LTW)?  (report)
  k19: a sqrt(sigma) against LTW at beta 2.2986 and the A4-start record there (report)
  outcome: success >= 3 β pass Q3-a and Q3-v (and A4-null); partial 2; failure < 2
k0-k10 are rescored under A4 and reported; they are not evidence. Q3-r is rescored on k6 and reported.

usage: python test_q3_a4.py PROBE_EXE RUN_DIR
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
OLD = [(0, 2.2986, 10), (1, 2.2986, 14), (2, 2.3715, 12), (3, 2.3715, 16), (4, 2.4265, 16),
       (5, 2.4265, 20), (6, 2.3, 16), (7, 2.2986, 12), (8, 2.2986, 14), (9, 2.5115, 20),
       (10, 2.5115, 24)]
FRESH = [(11, 2.2986, 10), (12, 2.2986, 12), (13, 2.3715, 12), (14, 2.3715, 16), (15, 2.4265, 16),
         (16, 2.4265, 20), (17, 2.5115, 20), (18, 2.5115, 24), (19, 2.2986, 14)]
PAIRS = {2.2986: (11, 12), 2.3715: (13, 14), 2.4265: (15, 16), 2.5115: (17, 18)}
CH = ("scalar", "torelon")


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
    return float((a - b) / np.hypot(ea, eb))


def shuffled_null(e, ch, vec, tag):
    s = np.einsum("a,act->ct", np.array(vec), np.array(e[ch]))
    rng = np.random.default_rng(e["seed"] + 7)
    flat = s.flatten()
    rng.shuffle(flat)
    return call({"p": flat.reshape(s.shape).tolist(), "block": BLOCK}, tag)["plateau"]


def score(k, beta, L, fresh):
    pre = "a4" if fresh else "a4re"
    e = json.loads((run / f"k{k}_b{beta}_L{L}.ensemble.json").read_text())
    out = {"beta": beta, "L": L, "seed": e["seed"], "n_meas": e["n_meas"],
           "mean_plaquette": float(np.mean(e["mean_plaquette"]))}
    ser = {}
    for ch in CH:
        r = call({"ops": e[ch], "t0": T0, "t1": T1, "block": BLOCK, "rule": "a4"}, f"{pre}_k{k}_{ch}")
        g = r["gevp"] or {}
        out[ch] = {"gevp": g, "plateau": r.get("plateau"), "plateau_as_fixed": r.get("plateau_as_fixed"),
                   "m_eff_t": r.get("m_eff_t"), "m_eff_err_t": r.get("m_eff_err_t")}
        if "vector" in g:
            ser[ch] = np.einsum("a,act->ct", np.array(g["vector"]), np.array(e[ch]))
        if fresh:
            # A4-start counterfactual: A4-1 alone
            rn = call({"ops": e[ch], "t0": T0, "t1": T1, "block": BLOCK, "rule": "a4_nostart"}, f"{pre}_k{k}_{ch}_nostart")
            out[ch]["nostart"] = {"gevp_path": (rn["gevp"] or {}).get("path"),
                                  "refused": (rn["gevp"] or {}).get("refused"), "plateau": rn.get("plateau")}
            # A4-null: the shuffled projected series for the Amendment-2 vector and the Q3-C vector
            nulls = {}
            for rule in ("a2", "q3c"):
                rr = call({"ops": e[ch], "t0": T0, "t1": T1, "block": BLOCK, "rule": rule}, f"{pre}_k{k}_{ch}_{rule}")
                gg = rr["gevp"] or {}
                nulls[rule] = (shuffled_null(e, ch, gg["vector"], f"{pre}_k{k}_{ch}_{rule}_null")
                               if "vector" in gg else {"no_vector": gg.get("refused")})
            out[ch]["nulls"] = nulls
    if k == 6:
        out["level0_scalar_alone"] = call({"p": e["scalar"][0], "block": BLOCK}, f"{pre}_k6_scalar_level0")["plateau"]
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
    print(json.dumps({x: out.get(x) for x in ("beta", "L", "a_sqrt_sigma", "a_sqrt_sigma_err", "ratio", "ratio_err")}
                     | {"am0": sp.get("mass"), "am0_err": sp.get("error"),
                        "paths": [out[ch]["gevp"].get("path") for ch in CH],
                        "refused": [out[ch]["plateau"].get("refused") if out[ch]["plateau"] else out[ch]["gevp"].get("refused")
                                    for ch in CH]}), flush=True)
    return out


rows = {k: score(k, b, L, True) for k, b, L in FRESH}
rescored = {k: score(k, b, L, False) for k, b, L in OLD}
gates = {}

# A4-path
fallback = [f"k{k}_{ch}" for k in rows for ch in CH if rows[k][ch]["gevp"].get("path") == "q3c_fallback"]
# A4-null
null_fail, null_record = [], {}
for k in rows:
    for ch in CH:
        for rule, nl in rows[k][ch]["nulls"].items():
            null_record[f"k{k}_{ch}_{rule}"] = nl
            if "no_vector" not in nl and "refused" not in nl:
                null_fail.append(f"k{k}_{ch}_{rule}")
gates["A4-null"] = not null_fail
# A4-start
start = {}
for k in rows:
    for ch in CH:
        a4p = rows[k][ch]["plateau"] or {}
        ns = rows[k][ch]["nostart"]
        nsp = ns.get("plateau") or {}
        moved = ("mass" in a4p) != ("mass" in nsp) or \
            ("mass" in a4p and (a4p["t_lo"], a4p["t_hi"]) != (nsp["t_lo"], nsp["t_hi"]))
        if moved:
            start[f"k{k}_{ch}"] = {"a4": a4p or {"refused": rows[k][ch]["gevp"].get("refused")},
                                   "a4_without_A4-2": nsp or {"refused": ns.get("refused")}}

# Q3-a, Q3-v, Q3-s
passed_beta, per_beta = [], {}
for beta, (kc, kv) in PAIRS.items():
    rc, rv = rows[kc], rows[kv]
    sig, sige, m0, m0e = LTW[beta]
    sp = rc["scalar"].get("plateau") or {}
    ok_a = "mass" in sp and "a_sqrt_sigma" in rc and not rc.get("sigma_refused") \
        and abs(pull(rc["a_sqrt_sigma"], rc["a_sqrt_sigma_err"], sig, sige)) <= 2 \
        and abs(pull(sp["mass"], sp["error"], m0, m0e)) <= 2
    spv = rv["scalar"].get("plateau") or {}
    v_refused = not ("mass" in spv and "a_sqrt_sigma" in rv and not rv.get("sigma_refused"))
    ok_v = ok_a and not v_refused \
        and abs(pull(sp["mass"], sp["error"], spv["mass"], spv["error"])) <= 2 \
        and abs(pull(rc["a_sqrt_sigma"], rc["a_sqrt_sigma_err"], rv["a_sqrt_sigma"], rv["a_sqrt_sigma_err"])) <= 2
    gates[f"Q3-a_beta{beta}"], gates[f"Q3-v_beta{beta}"] = bool(ok_a), bool(ok_v)
    per_beta[beta] = {"Q3-v_by_refusal": bool(ok_a and v_refused)}
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
                "falsifier_applicable": dof > 0,
                "pull_vs_LTW_continuum": pull(c, np.sqrt(cov[0, 0]), CONT, CONT_ERR)}
    gates["Q3-s"] = bool((not scaling["falsifier"]) and abs(scaling["pull_vs_LTW_continuum"]) <= 2)
else:
    gates["Q3-s"] = False
    scaling["note"] = "fewer than two betas passed Q3-a and Q3-v"

n_pass = len(passed_beta)
outcome = ("success" if n_pass >= 3 else "partial" if n_pass == 2 else "failure") if gates["A4-null"] else "stop: A4-null failed"

# k17 replicate of k9
p17 = rows[17]["scalar"].get("plateau") or {}
k17 = {"am0": p17, "pull_vs_LTW": pull(p17["mass"], p17["error"], 0.658, 0.010) if "mass" in p17 else None}
k17["reproduces_k9_excess"] = bool(k17["pull_vs_LTW"] is not None and k17["pull_vs_LTW"] > 2)
# k19
r19 = rows[19]
k19 = {"a_sqrt_sigma": r19.get("a_sqrt_sigma"), "a_sqrt_sigma_err": r19.get("a_sqrt_sigma_err"),
       "pull_a_sqrt_sigma_vs_LTW": pull(r19["a_sqrt_sigma"], r19["a_sqrt_sigma_err"], 0.3667, 0.0018) if "a_sqrt_sigma" in r19 else None,
       "am0": r19["scalar"].get("plateau"),
       "pull_am0_vs_LTW": pull(r19["scalar"]["plateau"]["mass"], r19["scalar"]["plateau"]["error"], 1.212, 0.022)
       if "mass" in (r19["scalar"].get("plateau") or {}) else None,
       "torelon_plateau": r19["torelon"].get("plateau"), "A4-start": {c: start.get(f"k19_{c}") for c in CH}}
# k18 (fresh k10): was the large-volume a sqrt(sigma) excess there again? (A4-2 does not address it)
r18 = rows[18]
k18 = {"pull_a_sqrt_sigma_vs_LTW": pull(r18["a_sqrt_sigma"], r18["a_sqrt_sigma_err"], 0.1768, 0.0008) if "a_sqrt_sigma" in r18 else None}
# Q3-r rescored (k6)
r6 = rescored[6]
g6 = r6["scalar"].get("plateau") or {}
l0 = r6.get("level0_scalar_alone") or {}
q3r_rescored = bool("mass" in g6 and ("refused" in l0 or pull(l0["mass"], l0["error"], g6["mass"], g6["error"]) > 3))

rec = {"kind": "Q3 Amendment 4", "spec": "docs/HALCYON_Q3_RUN_SPEC.md (Amendment 4)",
       "gates": gates, "outcome": outcome, "n_beta_passing": n_pass, "per_beta": per_beta, "scaling": scaling,
       "A4-path_fallback": fallback, "A4-null_failures": null_fail, "A4-null_record": null_record,
       "A4-start": start, "k17_replicate": k17, "k18_large_volume_sigma": k18, "k19": k19,
       "rescored_not_evidence": {"Q3-r_k6": q3r_rescored,
                                 "rows": {k: {x: v.get(x) for x in ("beta", "L", "a_sqrt_sigma", "a_sqrt_sigma_err",
                                                                    "ratio", "ratio_err")}
                                          | {ch: {"plateau": v[ch].get("plateau"), "path": v[ch]["gevp"].get("path"),
                                                  "gevp_refused": v[ch]["gevp"].get("refused")} for ch in CH}
                                          for k, v in rescored.items()}},
       "rows": rows}
(run / "q3a4_receipt.json").write_text(json.dumps(rec, indent=1, default=float) + "\n")
print(json.dumps({"gates": gates, "outcome": outcome, "scaling": scaling, "A4-path_fallback": fallback,
                  "A4-start": list(start), "k17_replicate": k17, "k18": k18, "k19": k19,
                  "Q3-r_k6_rescored": q3r_rescored}, indent=1, default=float))
