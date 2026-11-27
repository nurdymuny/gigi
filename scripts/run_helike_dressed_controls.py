"""Flux-method §5.10, order of work step 2: the four controls in the dressed chart.
At s = 1.15, t = 3, g2 in {0.002, 0.004}, Mq in {8, 10}: the bare solve, then the self-consistent dressed chart and the
solve in it. Registered checks: at Mq = 10 the dressed and bare levels agree to <= 1e-4 (dressing is O(g2) there), and
the dressed w~ >= the bare w~ for both channels.
usage: python scripts/run_helike_dressed_controls.py OUT_DIR [--smoke]   (--smoke: step 3, s = 1.18, t = 3, g2 = 0.03,
       Mq = 6, 8, 10, with the registered predictions)"""
import json
import subprocess
import sys
import time
from pathlib import Path

import helike_dressed_point as dp

AGREE = 1e-4


def cone(s, t, g2, mq, out, chart=None):
    cmd = [str(dp.BIN), "--out", str(out), "cone", "--s", repr(s), "--t", repr(t), "--g2", repr(g2), "--mq", str(mq)]
    if chart:
        cmd += ["--chart", str(chart)]
    r = subprocess.run(cmd, capture_output=True, text=True)
    if r.returncode:
        raise RuntimeError(f"cone refused (exit {r.returncode}): {r.stderr.strip() or r.stdout.strip()}")
    res = json.loads(Path(out).read_text())["results"]["result"]
    # E0 and levels are Ritz values without the chart's energy shift V(G0); only gaps compare across charts
    return {"E0": res["E0"], "energy_shift": res["energy_shift"], "levels": res["levels"][:6],
            **{n: {k: res[n][k] for k in ("gap", "weight", "dressed_weight", "dressed_overlap", "level_index")}
               for n in ("helike", "competitor")}}


def point(s, t, g2, mq, d):
    d.mkdir(parents=True, exist_ok=True)
    t0 = time.time()
    bare = cone(s, t, g2, mq, d / "cone_bare.json")
    try:
        chart, X, hist = dp.dressed_point(s, t, g2, mq, d / "loop")
    except Exception as exc:   # noqa: BLE001 - a refusal is OPEN, recorded with its reason
        return {"s": s, "t": t, "g2": g2, "Mq": mq, "status": "OPEN", "reason": f"{type(exc).__name__}: {exc}",
                "bare": bare, "seconds": time.time() - t0}
    dressed = cone(s, t, g2, mq, d / "cone_dressed.json", chart)
    rep = {"s": s, "t": t, "g2": g2, "Mq": mq, "status": "RESOLVED", "bare": bare, "dressed": dressed,
           "iterations": len(hist) - 1, "history": hist, "chart": str(chart), "seconds": time.time() - t0}
    rep["gap_difference"] = {n: dressed[n]["gap"] - bare[n]["gap"] for n in ("helike", "competitor")}
    rep["w_tilde_gain"] = {n: dressed[n]["dressed_weight"] - bare[n]["dressed_weight"] for n in ("helike", "competitor")}
    return rep


def asymmetry(r):
    """The asymmetry of the final dressed iterate at a rung (A2)."""
    return r["history"][-1]["asymmetry"] if r.get("status") == "RESOLVED" else None


def ladder_rule(rungs):
    """A2: strictly decreasing along the ladder and <= ASYM_FINAL at the final rung."""
    a = [asymmetry(r) for r in rungs]
    if any(x is None for x in a):
        return False, a
    return all(a[i + 1] < a[i] for i in range(len(a) - 1)) and a[-1] <= dp.ASYM_FINAL, a


def controls(out):
    out = Path(out); out.mkdir(parents=True, exist_ok=True)
    rep = {"stage": "5.10 dressed controls", "points": []}
    for g2 in (0.002, 0.004):
        for mq in (8, 10):
            r = point(1.15, 3.0, g2, mq, out / f"g{g2}_Mq{mq}")
            if r["status"] == "RESOLVED":
                r["checks"] = {"w_tilde_not_lower": all(v >= -1e-12 for v in r["w_tilde_gain"].values())}
                if mq == 10:
                    r["checks"]["levels_agree_1e-4"] = all(abs(v) <= AGREE for v in r["gap_difference"].values())
            rep["points"].append(r)
            print(json.dumps({k: r.get(k) for k in ("g2", "Mq", "status", "reason", "iterations", "gap_difference",
                                                    "w_tilde_gain", "checks")}), flush=True)
            (out / "report.json").write_text(json.dumps(rep, indent=1) + "\n")
    for g2 in (0.002, 0.004):
        rungs = [p for p in rep["points"] if p["g2"] == g2]
        ok, a = ladder_rule(rungs)
        rep.setdefault("asymmetry_ladder", {})[str(g2)] = {"asymmetry": a, "passes_A2": ok}
    rep["controls_pass"] = (all(p["status"] == "RESOLVED" and all(p["checks"].values()) for p in rep["points"])
                            and all(v["passes_A2"] for v in rep["asymmetry_ladder"].values()))
    (out / "report.json").write_text(json.dumps(rep, indent=1) + "\n")
    print("dressed_controls_done", rep["controls_pass"], flush=True)
    return rep


def smoke(out):
    """Step 3: s = 1.18, t = 3, g2 = 0.03, the full ladder in the dressed chart, against the registered predictions."""
    out = Path(out); out.mkdir(parents=True, exist_ok=True)
    rep = {"stage": "5.10 smoke point", "s": 1.18, "t": 3.0, "g2": 0.03, "rungs": []}
    for mq in (6, 8, 10):
        r = point(1.18, 3.0, 0.03, mq, out / f"Mq{mq}")
        rep["rungs"].append(r)
        print(json.dumps({k: r.get(k) for k in ("Mq", "status", "reason", "iterations", "gap_difference", "w_tilde_gain")}),
              flush=True)
        (out / "report.json").write_text(json.dumps(rep, indent=1) + "\n")
    rep["asymmetry_ladder"] = dict(zip(("ok", "asymmetry"), ladder_rule(rep["rungs"])))
    ok = [r for r in rep["rungs"] if r["status"] == "RESOLVED"]
    if len(ok) == 3:
        g = {n: [r["dressed"][n]["gap"] for r in ok] for n in ("helike", "competitor")}
        step = {n: abs(g[n][2] - g[n][1]) for n in g}
        wC = ok[2]["dressed"]["competitor"]["dressed_weight"]
        rep["predictions"] = {
            "competitor_dressed_step_8_10_le_0.003": {"value": step["competitor"], "met": step["competitor"] <= 0.003},
            "helike_dressed_step_8_10": {"value": step["helike"]},
            "competitor_dressed_w_tilde_ge_0.95": {"value": wC, "met": wC >= 0.95},
            "bare_reference": {"competitor_step_8_10": 0.009140197642756753, "competitor_w_tilde_Mq10": 0.9136404500327204},
        }
    (out / "report.json").write_text(json.dumps(rep, indent=1) + "\n")
    print("dressed_smoke_done", flush=True)
    return rep


if __name__ == "__main__":
    if "--smoke" in sys.argv:
        smoke(sys.argv[1])
    else:
        controls(sys.argv[1])
