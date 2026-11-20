"""Q2 gates (docs/HALCYON_Q2_RUN_SPEC.md): EFFECTIVE_MASS on 2D SU(2) Monte-Carlo ensembles against
the exact character-expansion answer, for the heat-kernel and the Wilson action.

The exact oracle is computed here, independently of the engine (scipy Bessel functions, character sum
to 2j = 59):

    <p(0) p(t)> = 1/4 * sum_{|j-k| = 1/2} lam_j^(Ns t) lam_k^(Ns (Nt - t)) / sum_j lam_j^(Ns Nt)
    heat kernel lam_j = exp(-2 j(j+1)/beta),  Wilson lam_j = I_{2j+1}(beta)/I_1(beta).

Gates: Q2-a (heat-kernel plateau within 2 sigma of exact), Q2-a' (|C - C_exact| <= 4 sigma at every
t <= Nt/2), Q2-w (the same on Wilson against Wilson's exact answer), Q2-r (Wilson plateau more than
3 sigma from the heat-kernel exact m), Q2-n (refusal on the plaquette channel and on the shuffled
Polyakov series, every ensemble).

usage: python test_q2_exact_2d.py ENSEMBLE_EXE PROBE_EXE OUT_DIR
"""
import json
import subprocess
import sys
from pathlib import Path

import numpy as np
from scipy.special import ive

ens_exe, probe_exe, out = sys.argv[1], sys.argv[2], Path(sys.argv[3])
out.mkdir(parents=True, exist_ok=True)
NS, NT, N_THERM, N_MEAS, EVERY, BLOCK, TWO_J = 2, 32, 2000, 20000, 5, 100, 59
BETAS = [3, 4, 6, 8]
SEEDS = {"heat_kernel": 202611190, "wilson": 202611191}


def lam(action, beta, two_j):
    if action == "heat_kernel":
        j = two_j / 2
        return np.exp(-2 * j * (j + 1) / beta)
    return ive(two_j + 1, beta) / ive(1, beta)       # exponentially scaled; the ratio is exact


def exact_C(action, beta):
    lj = np.array([lam(action, beta, k) for k in range(TWO_J + 1)])
    z = np.sum(lj ** (NS * NT))
    c = []
    for t in range(NT):
        s = 0.0
        for j in range(TWO_J + 1):
            for k in (j - 1, j + 1):
                if 0 <= k <= TWO_J:
                    s += lj[j] ** (NS * t) * lj[k] ** (NS * (NT - t))
        c.append(0.25 * s / z)
    return np.array(c)


def m_eff(c):
    return [None if not (0 < t < len(c) - 1) else float(np.arccosh((c[t - 1] + c[t + 1]) / (2 * c[t])))
            for t in range(len(c))]


def run(cmd):
    subprocess.run(cmd, check=True)


def probe(series, tag):
    i, o = out / f"{tag}.in.json", out / f"{tag}.out.json"
    i.write_text(json.dumps({"p": series, "block": BLOCK}))
    run([probe_exe, str(i), str(o)])
    i.unlink()
    return json.loads(o.read_text())


rows, gates = [], {}
for beta in BETAS:
    exact = {a: exact_C(a, beta) for a in ("heat_kernel", "wilson")}
    m_exact = {a: m_eff(exact[a]) for a in exact}
    m_lead = {a: float(-NS * np.log(lam(a, beta, 1))) for a in exact}
    for action in ("heat_kernel", "wilson"):
        tag = f"{action}_b{beta}"
        ens = out / f"{tag}.ensemble.json"
        if not ens.exists():
            run([ens_exe, action, str(beta), str(NS), str(NT), str(N_THERM), str(N_MEAS), str(EVERY),
                 str(SEEDS[action] + beta), str(ens)])
        e = json.loads(ens.read_text())
        pol, plq = e["polyakov"], e["plaquette_rows"]
        r_pol = probe(pol, f"{tag}.polyakov")
        r_plq = probe(plq, f"{tag}.plaquette")
        rng = np.random.default_rng(20260628 + 1)
        flat = np.array(pol).flatten()
        rng.shuffle(flat)
        r_shuf = probe(flat.reshape(len(pol), NT).tolist(), f"{tag}.shuffled")

        C, Ce = np.array(r_pol["C_t"], float), np.array(r_pol["C_err_t"], float)
        pulls = [float(abs(C[t] - exact[action][t]) / Ce[t]) for t in range(NT // 2 + 1)]
        pl = r_pol["plateau"]
        row = {"beta": beta, "action": action, "acceptance": e["acceptance"], "eps": e["eps"],
               "seconds_ensemble": e["seconds"], "seed": e["seed"],
               "m_leading_exact": m_lead, "plateau": pl, "max_pull_C": max(pulls), "pulls_C": pulls,
               "plaquette_channel": r_plq["plateau"], "shuffled": r_shuf["plateau"]}
        if "mass" in pl:
            w = np.array([1 / r_pol["m_eff_err_t"][t] ** 2 for t in range(pl["t_lo"], pl["t_hi"] + 1)])
            for a in exact:
                me = np.array([m_exact[a][t] for t in range(pl["t_lo"], pl["t_hi"] + 1)])
                row[f"m_exact_window_{a}"] = float((me * w).sum() / w.sum())
            row["pull_vs_own_exact"] = (pl["mass"] - row[f"m_exact_window_{action}"]) / pl["error"]
            row["pull_vs_heat_kernel_exact"] = (pl["mass"] - row["m_exact_window_heat_kernel"]) / pl["error"]
        rows.append(row)
        own = "a" if action == "heat_kernel" else "w"
        gates[f"Q2-{own}_beta{beta}"] = "mass" in pl and abs(row["pull_vs_own_exact"]) <= 2
        gates[f"Q2-{own}'_beta{beta}"] = max(pulls) <= 4
        gates[f"Q2-n_{action}_beta{beta}"] = "refused" in r_plq["plateau"] and "refused" in r_shuf["plateau"]
        if action == "wilson":
            gates[f"Q2-r_beta{beta}"] = "mass" in pl and abs(row["pull_vs_heat_kernel_exact"]) > 3
        print(json.dumps({k: row[k] for k in row if k not in ("pulls_C",)}), flush=True)

rec = {"kind": "Q2: EFFECTIVE_MASS against the exact 2D SU(2) answer",
       "spec": "docs/HALCYON_Q2_RUN_SPEC.md",
       "lattice": {"ns": NS, "nt": NT}, "run": {"n_therm": N_THERM, "n_meas": N_MEAS, "every": EVERY,
                                                  "jackknife_block": BLOCK, "two_j_max": TWO_J},
       "gates": gates, "all_pass": all(gates.values()), "rows": rows}
(out / "q2_receipt.json").write_text(json.dumps(rec, indent=1) + "\n")
print(json.dumps({"gates": gates, "all_pass": rec["all_pass"]}, indent=1))
