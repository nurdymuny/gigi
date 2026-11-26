"""The Helike heat signal (docs/research/helike-heat-signal-registration-2026-11-26.md, with amendment 1).
Floating point; not a bound. Reads `helike cone-heat` receipts; computes per source space the level weights
w_n = ||P_S psi_n||^2, the heat signal C_S(tau), the effective note m_S(tau), the plateau, the tone and the
normalised curve; and the registered verdicts T1, T1b, T2, T3, T4. T5 is deferred (amendment 1).

usage: python scripts/helike_heat_signal.py --controls OUT_DIR     (the registered first stage: the four controls)
"""
import argparse
import json
import math
from pathlib import Path
import subprocess
import tempfile

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
HELIKE_BIN = ROOT / "helike-lab/target/release/helike"
WEIGHT_MIN = 1e-6
TONE_TOL = 0.02
CURVE_TOL = 0.02
X_GRID = np.linspace(0.0, 6.0, 121)
QUAD_TOL = 1e-5
GAP_TOL = 0.003
SEED = 20261126

# registration order of the 22 monomials over the roles [h1, h2, c, x, s1, s2] (amendment 1; mirrors HEAT_MONOMIALS)
MONO = ["h1", "h2", "c", "x", "h1c", "h2c", "h1x", "h2x", "s1s1", "s2s2", "s1s2", "h1h1", "h2h2", "cc", "xx", "cx",
        "h1h1h1", "h1h2h2", "h1h1h2", "h2h2h2", "h1h1c", "h2h2c"]
IDX = {m: i for i, m in enumerate(MONO)}


def vec(**coef):
    v = np.zeros(len(MONO))
    for k, c in coef.items():
        v[IDX[k]] += c
    return v


def random_sources():
    rng = np.random.default_rng(SEED)
    pairs = [(vec(h1=1), vec(h2=1)), (vec(h1c=1), vec(h2c=1)), (vec(h1x=1), vec(h2x=1)),
             (vec(s1s1=1, s2s2=-1), vec(s1s2=2)),
             (vec(h1h1h1=1, h1h2h2=1), vec(h1h1h2=1, h2h2h2=1))]
    a = rng.normal(size=len(pairs))
    r2 = np.array([sum(c * p[0] for c, p in zip(a, pairs)), sum(c * p[1] for c, p in zip(a, pairs))])
    zero = [vec(c=1), vec(x=1), vec(h1h1=1, h2h2=1), vec(s1s1=1, s2s2=1), vec(cc=1), vec(xx=1), vec(cx=1),
            vec(h1h1c=1, h2h2c=1)]
    b = rng.normal(size=len(zero))
    r0 = np.array([sum(c * z for c, z in zip(b, zero))])
    return r2, r0


R2, R0 = random_sources()
SOURCES = {
    # charge +-2 sector (two-dimensional spaces)
    "H": ("helike", np.array([vec(h1=1), vec(h2=1)])),
    "H'": ("helike", "bare"),
    "H''": ("helike", np.array([vec(s1s1=1, s2s2=-1), vec(s1s2=2)])),
    "H'''": ("helike", np.array([vec(h1c=1), vec(h2c=1)])),
    "R2": ("helike", R2),
    # charge 0 sector (one-dimensional)
    "C": ("competitor", np.array([vec(c=1)])),
    "C'": ("competitor", "bare"),
    "C''": ("competitor", np.array([vec(h1h1=1, h2h2=1)])),
    "R0": ("competitor", R0),
}
REGISTERED = {"helike": "H", "competitor": "C"}


# ------------------------------------------------------------------ weights and signal
def level_weights(rec, name, order="fine", k=None):
    """w_n = ||P_S psi_n||^2 / dim S for the levels (normalised source space), and the missing weight."""
    k_all = len(rec["levels"])
    k = k or k_all
    F = rec[order]
    nm = len(MONO)
    _, spec = SOURCES[name]
    if isinstance(spec, str):                               # bare reference: orthonormal, rows by mode index
        amp = np.array(F["reference_amplitudes"]).reshape(7, k_all)
        roles = rec["roles"]
        modes = [roles[0], roles[1]] if name == "H'" else [roles[2]]
        w = np.array([sum(amp[m, c] ** 2 for m in modes) for c in range(k_all)]) / len(modes)
        w[0] = 0.0                                          # the plateau is read from excited levels only
    else:
        d = np.array(F["d"]).reshape(nm, k_all); n = np.array(F["n"]).reshape(nm, nm)
        p = np.array(F["p"]); s = np.array(F["s"])
        A = np.atleast_2d(spec)
        B = A @ (n - np.outer(p, p)) @ A.T                  # Gram of the G-projected sources
        O = A @ (d - np.outer(p, s))                        # overlaps <b_i | psi_c>
        w = np.array([O[:, c] @ np.linalg.solve(B, O[:, c]) for c in range(k_all)]) / A.shape[0]
    w = w[:k]
    return w, 1.0 - w.sum()


def plateau(levels, w):
    E = np.asarray(levels) - levels[0]
    for n in range(1, len(w)):
        if w[n] >= WEIGHT_MIN:
            return n, E[n]
    return None, math.inf


def signal(levels, w, tau):
    E = np.asarray(levels)[: len(w)] - levels[0]
    tot = w.sum()
    return np.array([np.sum(w * np.exp(-E * t)) for t in tau]) / tot


def clusters(levels, w):
    """Distinct weighted energies above the ground: list of (energy, member levels, summed weight)."""
    E = np.asarray(levels) - levels[0]
    out = []
    for n in range(1, len(w)):
        if out and abs(E[n] - out[-1][0]) <= 1e-6:
            out[-1][1].append(n); out[-1][2] += w[n]
        else:
            out.append([E[n], [n], w[n]])
    return [c for c in out if c[2] >= WEIGHT_MIN]


def tone(levels, w, nlev=3):
    cl = clusters(levels, w)[:nlev]
    if not cl:
        return None
    delta = cl[0][0]
    tot = w[1:].sum()
    return {"levels": [c[1] for c in cl], "ratios": [float(c[0] / delta) for c in cl],
            "weight_fractions": [float(c[2] / tot) for c in cl], "delta": float(delta)}


def normalised_curve(levels, w):
    n, delta = plateau(levels, w)
    return signal(levels, w, X_GRID / delta)


# ------------------------------------------------------------------ verdicts
def t1(rec, k=None):
    """Amendment 2: the corridor is the registered source's plateau; every other source must reach its cluster."""
    out = {}
    E = np.asarray(rec["levels"]) - rec["levels"][0]
    for sector in ("helike", "competitor"):
        rows = {}
        for name, (sec, _) in SOURCES.items():
            if sec != sector:
                continue
            w, miss = level_weights(rec, name, k=k)
            n, d = plateau(rec["levels"], w)
            rows[name] = {"plateau_level": n, "plateau": d, "missing_weight": float(miss),
                          "weights": [float(x) for x in w]}
        corridor = rows[REGISTERED[sector]]["plateau_level"]
        cluster = [m for m in range(1, len(E)) if corridor is not None and abs(E[m] - E[corridor]) <= 1e-6
                   and m < len(rows[REGISTERED[sector]]["weights"])]
        miss_names = [nm for nm, r in rows.items()
                      if sum(r["weights"][m] for m in cluster if m < len(r["weights"])) < WEIGHT_MIN]
        out[sector] = {"sources": rows, "corridor_level": corridor, "corridor_cluster": cluster,
                       "zero_at_corridor": miss_names,
                       "verdict": "CORRIDOR" if corridor is not None and not miss_names else "SPLIT"}
    return out


def t1b(rec, t1res):
    ident = rec["identification"]
    a = t1res["helike"]["corridor_level"] == ident["helike"]["level_index"]
    b = t1res["competitor"]["corridor_level"] == ident["competitor"]["level_index"]
    return {"helike": "AGREES" if a else "DISAGREES", "competitor": "AGREES" if b else "DISAGREES",
            "identified_levels": [ident["helike"]["level_index"], ident["competitor"]["level_index"]]}


def t2(rec_a, rec_b):
    out = {}
    for sector, name in REGISTERED.items():
        wa, _ = level_weights(rec_a, name); wb, _ = level_weights(rec_b, name)
        ta, tb = tone(rec_a["levels"], wa), tone(rec_b["levels"], wb)
        m = min(len(ta["ratios"]), len(tb["ratios"]))
        dr = max(abs(ta["ratios"][j] - tb["ratios"][j]) for j in range(m))
        dc = float(np.max(np.abs(normalised_curve(rec_a["levels"], wa) - normalised_curve(rec_b["levels"], wb))))
        out[sector] = {"tone_a": ta, "tone_b": tb, "max_ratio_change": float(dr), "max_curve_change": dc,
                       "verdict": "TONE-STABLE" if dr <= TONE_TOL and dc <= CURVE_TOL else "TONE-SHIFT"}
    return out


def t3(recs):
    out = {}
    for sector, name in REGISTERED.items():
        seq = []
        for r in sorted(recs, key=lambda r: r["g2"]):
            w, _ = level_weights(r, name)
            seq.append((r["g2"], plateau(r["levels"], w)[1]))
        diffs = [b[1] - a[1] for a, b in zip(seq, seq[1:])]
        trend = "increasing" if all(x > 0 for x in diffs) else "decreasing" if all(x < 0 for x in diffs) else "flat/mixed"
        out[sector] = {"sequence": seq, "differences": diffs, "trend": trend}
    return out


def t4(rec_hi, rec_lo_mq=None):
    out = {"seams": {}}
    ok = True
    for sector, name in REGISTERED.items():
        w12, _ = level_weights(rec_hi, name)
        w8, miss8 = level_weights(rec_hi, name, k=8)
        n12, d12 = plateau(rec_hi["levels"], w12); n8, d8 = plateau(rec_hi["levels"], w8)
        tau = X_GRID / d12
        c12 = signal(rec_hi["levels"], w12, tau)
        c8 = signal(rec_hi["levels"], w8, tau)
        band = max(miss8, 0) + float(w12[8:].sum() / max(w12.sum(), 1e-300))
        kseam = bool(n12 == n8 and np.max(np.abs(c12 - c8)) <= band + 1e-12)
        wc, _ = level_weights(rec_hi, name, order="coarse")
        qseam = bool(np.max(np.abs(w12 - wc)) <= QUAD_TOL)
        out["seams"][sector] = {"levels_k8_vs_k12": kseam, "quadrature": qseam,
                                "max_weight_change_quadrature": float(np.max(np.abs(w12 - wc)))}
        ok &= kseam and qseam
        if rec_lo_mq is not None:
            wl, _ = level_weights(rec_lo_mq, name)
            dl = plateau(rec_lo_mq["levels"], wl)[1]
            mseam = bool(abs(d12 - dl) <= GAP_TOL)
            out["seams"][sector]["truncation_Mq"] = mseam
            out["seams"][sector]["plateau_change_Mq"] = float(abs(d12 - dl))
            ok &= mseam
    dres = t1(rec_hi)
    for sector in ("helike", "competitor"):
        bare = "H'" if sector == "helike" else "C'"
        reg = REGISTERED[sector]
        same = dres[sector]["sources"][bare]["plateau_level"] == dres[sector]["sources"][reg]["plateau_level"]
        out["seams"][sector]["source_dressing"] = bool(same)
        ok &= same
    out["verdict"] = "SIGNAL-INVARIANT" if ok else "SIGNAL-SENSITIVE"
    return out


# ------------------------------------------------------------------ runs
def cone_heat(s, t, g2, mq):
    with tempfile.TemporaryDirectory() as d:
        out = Path(d) / "h.json"
        subprocess.run([str(HELIKE_BIN), "--out", str(out), "cone-heat", "--s", str(s), "--t", str(t), "--g2", str(g2),
                        "--mq", str(mq)], check=True, capture_output=True, text=True)
        return json.loads(out.read_text())["results"]


def controls(out_dir):
    out = Path(out_dir); out.mkdir(parents=True, exist_ok=True)
    recs = {}
    for g2 in (0.002, 0.004):
        for mq in (8, 10):
            r = cone_heat(1.15, 3.0, g2, mq)
            recs[(g2, mq)] = r
            (out / f"cone_heat_g{g2}_Mq{mq}.json").write_text(json.dumps(r) + "\n")
    report = {"stage": "controls (registered first stage)", "points": {}}
    for g2 in (0.002, 0.004):
        r = recs[(g2, 10)]
        a = t1(r)
        report["points"][str(g2)] = {"T1": {s: {"verdict": a[s]["verdict"], "corridor_level": a[s]["corridor_level"],
                                                "zero_at_corridor": a[s]["zero_at_corridor"],
                                                "plateaus": {k: v["plateau"] for k, v in a[s]["sources"].items()},
                                                "missing": {k: v["missing_weight"] for k, v in a[s]["sources"].items()}}
                                            for s in a},
                                     "T1b": t1b(r, a), "T4": t4(r, recs[(g2, 8)])}
    report["T2_controls"] = t2(recs[(0.002, 10)], recs[(0.004, 10)])
    report["T3_controls"] = t3([recs[(0.002, 10)], recs[(0.004, 10)]])
    passed = all(report["points"][g]["T1"][s]["verdict"] == "CORRIDOR" for g in report["points"] for s in ("helike", "competitor")) \
        and all(report["points"][g]["T4"]["verdict"] == "SIGNAL-INVARIANT" for g in report["points"])
    report["controls_pass"] = bool(passed)
    (out / "report.json").write_text(json.dumps(report, indent=1, default=float) + "\n")
    return report


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--controls", metavar="OUT_DIR")
    a = ap.parse_args()
    if a.controls:
        rep = controls(a.controls)
        print(json.dumps({"controls_pass": rep["controls_pass"],
                          "T1": {g: {s: rep["points"][g]["T1"][s]["verdict"] for s in ("helike", "competitor")} for g in rep["points"]},
                          "T1b": {g: rep["points"][g]["T1b"] for g in rep["points"]},
                          "T4": {g: rep["points"][g]["T4"]["verdict"] for g in rep["points"]},
                          "T2": {s: rep["T2_controls"][s]["verdict"] for s in rep["T2_controls"]}}, indent=1))
