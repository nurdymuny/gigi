"""Flux-method §5.11, registered checks 2 and 3 with the Rust solver (helike cone-sqrtg).
  --controls OUT : s = 1.15, t = 3, g2 in {0.002, 0.004}; ladder Mq 4, 6, 8 (order Mq + 6), from the bare harmonic start;
                   at Mq 8: gaps vs the log chart's converged Mq 10 gaps <= 1e-4; seam at order Mq + 8 on the gaps <= 1e-8;
                   off-sheet vacuum weight <= 1e-12.
  --smoke OUT    : s = 1.18, t = 3, g2 = 0.03, the same ladder; predictions: competitor step 6 -> 8 <= 0.001.
Identification (dressed w~ in the sqrt(G) frame) is a separate layer, not run here."""
import json
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / ("helike-lab/target/release/helike" + (".exe" if sys.platform == "win32" else ""))
LOG_CONVERGED = ROOT / "research/dressed-chart-20261126/controls/report.json"   # bare log-chart solves, Mq 10


def cone_sqrtg(out, s, t, g2, ladder, offset, frame_in=None, frame_out=None, max_iter=6):
    cmd = [str(BIN), "--out", str(out), "cone-sqrtg", "--s", repr(s), "--t", repr(t), "--g2", repr(g2), "--ladder", ladder,
           "--order-offset", str(offset), "--max-iter", str(max_iter)]
    if frame_in:
        cmd += ["--frame-in", str(frame_in)]
    if frame_out:
        cmd += ["--frame-out", str(frame_out)]
    r = subprocess.run(cmd, capture_output=True, text=True)
    if r.returncode:
        raise RuntimeError(f"cone-sqrtg refused (exit {r.returncode}): {r.stderr.strip()[-400:]}")
    return json.loads(Path(out).read_text())["results"]["rungs"]


def last(r):
    return r["iterations"][-1]


def point(d, s, t, g2):
    d.mkdir(parents=True, exist_ok=True)
    t0 = time.time()
    rungs = cone_sqrtg(d / "ladder.json", s, t, g2, "4,6,8", 6, frame_out=d / "frame.json")
    seam = cone_sqrtg(d / "seam.json", s, t, g2, "8", 8, frame_in=d / "frame.json", max_iter=0)[0]
    g = {r["Mq"]: last(r)["gaps"] for r in rungs}
    rep = {"s": s, "t": t, "g2": g2, "seconds": time.time() - t0,
           "rungs": [{"Mq": r["Mq"], "order": r["order"], "dim": r["dim"], "settled": r["settled"],
                      "iterations": len(r["iterations"]), "E0": last(r)["levels"][0], "gaps": last(r)["gaps"][:6],
                      "off_sheet_weight": last(r)["off_sheet_weight"]} for r in rungs],
           "seam_order": seam["order"], "seam_gap_difference": max(abs(a - b) for a, b in zip(seam["iterations"][0]["gaps"][:6], g[8][:6]))}
    # Helike: the lowest charge-2 pair (the two lowest gaps, degenerate); competitor: the next level
    rep["helike_gap"] = g[8][0]
    rep["competitor_gap"] = g[8][2]
    rep["steps_6_8"] = {"helike": abs(g[8][0] - g[6][0]), "competitor": abs(g[8][2] - g[6][2])}
    rep["pair_splitting_Mq8"] = abs(g[8][1] - g[8][0])
    return rep


def controls(out):
    out = Path(out); out.mkdir(parents=True, exist_ok=True)
    log = {p["g2"]: p for p in json.loads(LOG_CONVERGED.read_text())["points"] if p["Mq"] == 10}
    rep = {"stage": "5.11 check 2: controls", "points": []}
    for g2 in (0.002, 0.004):
        r = point(out / f"g{g2}", 1.15, 3.0, g2)
        b = log[g2]["bare"]
        r["log_chart_Mq10"] = {"helike": b["helike"]["gap"], "competitor": b["competitor"]["gap"]}
        r["parity"] = {"helike": abs(r["helike_gap"] - b["helike"]["gap"]), "competitor": abs(r["competitor_gap"] - b["competitor"]["gap"])}
        r["checks"] = {"parity_le_1e-4": max(r["parity"].values()) <= 1e-4, "seam_le_1e-8": r["seam_gap_difference"] <= 1e-8,
                       "off_sheet_le_1e-12": all(x["off_sheet_weight"] <= 1e-12 for x in r["rungs"]),
                       "all_rungs_settled": all(x["settled"] for x in r["rungs"])}
        rep["points"].append(r)
        print(json.dumps({k: r[k] for k in ("g2", "helike_gap", "competitor_gap", "parity", "seam_gap_difference", "steps_6_8", "checks")}), flush=True)
        (out / "report.json").write_text(json.dumps(rep, indent=1) + "\n")
    rep["controls_pass"] = all(all(p["checks"].values()) for p in rep["points"])
    (out / "report.json").write_text(json.dumps(rep, indent=1) + "\n")
    print("sqrtg_controls_done", rep["controls_pass"], flush=True)


def smoke(out):
    out = Path(out)
    r = point(out, 1.18, 3.0, 0.03)
    r["predictions"] = {"competitor_step_6_8_le_0.001": {"value": r["steps_6_8"]["competitor"], "met": r["steps_6_8"]["competitor"] <= 0.001}}
    (out / "report.json").write_text(json.dumps(r, indent=1) + "\n")
    print(json.dumps({k: r[k] for k in ("helike_gap", "competitor_gap", "steps_6_8", "seam_gap_difference", "predictions")}), flush=True)
    print("sqrtg_smoke_done", flush=True)


if __name__ == "__main__":
    {"--controls": controls, "--smoke": smoke}[sys.argv[1]](sys.argv[2])
