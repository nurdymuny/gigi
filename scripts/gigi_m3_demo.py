"""GIGI demo for M3 as Born-Oppenheimer (docs/research/helike-bundle-framing-math-2026-11-27.md §14).

The fast fibre over the slow base, stored as bundles and read back by point queries (SECTION ... AT id=):
  (A) `m3_sector_spectra`: base (lam, j, level), fibre (mu, count, mult), the gauge-slice fluctuation spectra at t = 1 on
      the empty sphere (lam = 0) and the flat Maurer-Cartan connection (lam = 1). Planted answers, read from point queries:
        A1 lam = 0: the multiplicity of mu = (k+1)^2, summed with Peter-Weyl weight (j+1), is 3 * 2k(k+2) (three colours
           times the coexact 1-forms of the unit S^3), for every k whose sectors are all stored;
        A2 lam = 1: the same multiset (gauge equivalence), reached through sectors shifted j -> j + 2.
  (B) `m3_line`: base lam on the Maurer-Cartan line at t = 3; fibre: density, the log coefficient dC and its prediction
      (b0/8pi^2) Vol density with b0 = 22/3, the zeta-finite fast zero-point energy E0_fast (mu R = 1, relative to lam = 0),
      the slow sector's harmonic sum. Planted: dC / prediction = 1 (the running coupling); at lam = 1,
      E0_fast = -(slow(1) - slow(0)) (gauge equivalence: the total is unchanged, the split moves).
  (C) `m3_background`: base mode; fibre: harmonic frequency, the curvature of E0_fast along the mode, the first-order shift
      d omega at g2 = 0.002 / 0.004 in the zeta scheme and per unit kappa (a shift of 1/g^2 at 1/R). Read back: the
      Helike-competitor separation with and without the fast fibre.
usage: python scripts/gigi_m3_demo.py OUT.json --t1 ZP_T1.json --line ZP_T3_LINE.json --background BACKGROUND.json"""
import argparse
import gzip
import json
import math
import time
from collections import Counter

import numpy as np

import gigi_lattice_gauge_demo as lg
import helike_m3_zeta as z

B0 = 22.0 / 3.0
JS = 24


def rid(li, j, lev):
    return (li * 100 + j) * 1000 + lev


def demo_sectors(db, t1_points):
    db.gql("COLLAPSE m3_sector_spectra;")
    db.gql("BUNDLE m3_sector_spectra BASE (id NUMERIC) FIBER (lam NUMERIC, j NUMERIC, mult NUMERIC, level NUMERIC, mu NUMERIC, count NUMERIC);")
    recs = []
    pts = {p["lam"]: p for p in t1_points}
    for li, lam in enumerate((0.0, 1.0)):
        for s in pts[lam]["sectors"][:JS + 1]:
            c = Counter(np.round(s["slice"], 9))
            levels = sorted(c.items())
            recs.append({"id": rid(li, s["j"], 999), "lam": lam, "j": s["j"], "mult": s["mult"], "level": -1, "mu": 0.0, "count": len(levels)})
            for k, (mu, n) in enumerate(levels):
                recs.append({"id": rid(li, s["j"], k), "lam": lam, "j": s["j"], "mult": s["mult"], "level": k, "mu": float(mu), "count": n})
    for i in range(0, len(recs), 1000):
        r = db.post("/bundles/m3_sector_spectra/insert", {"records": recs[i:i + 1000]})
        if "error" in r:
            raise RuntimeError(r)

    def read(li):
        tot = Counter()
        for j in range(JS + 1):
            db.queries += 1
            h = db.gql(f"SECTION m3_sector_spectra AT id={rid(li, j, 999)};")["rows"][0]
            for k in range(int(h["count"])):
                db.queries += 1
                r = db.gql(f"SECTION m3_sector_spectra AT id={rid(li, j, k)};")["rows"][0]
                tot[round(r["mu"], 6)] += int(r["count"]) * int(r["mult"])
        return tot
    m0, m1 = read(0), read(1)
    out = {"A1_round_sphere": {}, "A2_flat_equals_empty": {}}
    for k in range(1, JS - 3):
        mu = float((k + 1) ** 2)
        out["A1_round_sphere"][str(k)] = {"mu": mu, "from_gigi": m0.get(mu, 0), "exact_6k(k+2)": 6 * k * (k + 2)}
        out["A2_flat_equals_empty"][str(k)] = {"lam0": m0.get(mu, 0), "lam1": m1.get(mu, 0)}
    out["A1_pass"] = all(v["from_gigi"] == v["exact_6k(k+2)"] for v in out["A1_round_sphere"].values())
    out["A2_pass"] = all(v["lam0"] == v["lam1"] for v in out["A2_flat_equals_empty"].values())
    out["A1_only_squares_below_cut"] = all(abs(math.sqrt(m) - round(math.sqrt(m))) < 1e-9 for m in m0 if m < (JS - 2) ** 2)
    return out


def demo_line(db, line_points, t):
    vol = 2 * math.pi ** 2 * math.sqrt(t)
    pts = sorted(line_points, key=lambda p: p["lam"])
    ref = [p for p in pts if p["lam"] == 0.0][0]
    db.gql("COLLAPSE m3_line;")
    db.gql("BUNDLE m3_line BASE (id NUMERIC) FIBER (lam NUMERIC, density NUMERIC, dC NUMERIC, dC_pred NUMERIC, E0_fast NUMERIC, slow_harmonic NUMERIC, slow_negative NUMERIC);")
    recs = []
    for i, p in enumerate(pts):
        if p is ref:
            r = {"dE0_fast": 0.0, "dC": 0.0, "slow_harmonic_diff": 0.0, "slow_negative": []}
        else:
            r = z.zeta_fast(ref, p)
        recs.append({"id": i, "lam": p["lam"], "density": p["energy_density"], "dC": r["dC"],
                     "dC_pred": B0 / (8 * math.pi ** 2) * vol * p["energy_density"], "E0_fast": r["dE0_fast"],
                     "slow_harmonic": r["slow_harmonic_diff"], "slow_negative": len(r["slow_negative"])})
    db.post("/bundles/m3_line/insert", {"records": recs})

    def at(lam):
        i = [k for k, p in enumerate(pts) if abs(p["lam"] - lam) < 1e-12][0]
        db.queries += 1
        return db.gql(f"SECTION m3_line AT id={i};")["rows"][0]
    rows = [at(p["lam"]) for p in pts]
    ratios = {str(r["lam"]): r["dC"] / r["dC_pred"] for r in rows if r["dC_pred"] > 1e-9}
    one = at(1.0)
    return {"t": t, "rows": rows, "dC_over_prediction": ratios,
            "B1_running_coupling_max_dev": max(abs(v - 1) for v in ratios.values()),
            "B2_flat_point": {"E0_fast": one["E0_fast"], "minus_slow_shift": -one["slow_harmonic"],
                              "diff": one["E0_fast"] + one["slow_harmonic"]},
            "background": at(1.15) if any(abs(p["lam"] - 1.15) < 1e-12 for p in pts) else None}


def demo_background(db, bg):
    db.gql("COLLAPSE m3_background;")
    db.gql("BUNDLE m3_background BASE (id NUMERIC) FIBER (mode NUMERIC, mult NUMERIC, omega NUMERIC, H_zeta NUMERIC, H_C NUMERIC, "
           "dw_zeta_002 NUMERIC, dw_kappa_002 NUMERIC, dw_zeta_004 NUMERIC, dw_kappa_004 NUMERIC);")
    recs = []
    for i, m in enumerate(bg["modes"]):
        a, b = m["per_g2"]["0.002"], m["per_g2"]["0.004"]
        recs.append({"id": i, "mode": m["index"], "mult": m["mult"], "omega": a["omega"], "H_zeta": m["H_zeta"], "H_C": m["H_C"],
                     "dw_zeta_002": a["d_omega_zeta"], "dw_kappa_002": a["d_omega_per_kappa"],
                     "dw_zeta_004": b["d_omega_zeta"], "dw_kappa_004": b["d_omega_per_kappa"]})
    db.post("/bundles/m3_background/insert", {"records": recs})
    rows = []
    for i in range(len(recs)):
        db.queries += 1
        rows.append(db.gql(f"SECTION m3_background AT id={i};")["rows"][0])
    hel = [r for r in rows if r["mult"] == 2 and r["omega"] < 1.1][0]       # the charge-2 doublet (Helike's mode)
    comp = [r for r in rows if r["mult"] == 1 and 1.1 < r["omega"] < 1.2][0]  # the singlet above it (the competitor's)
    out = {"helike_mode": hel, "competitor_mode": comp}
    for g in ("002", "004"):
        sep0 = comp["omega"] - hel["omega"]
        dsep = comp[f"dw_zeta_{g}"] - hel[f"dw_zeta_{g}"]
        dk = comp[f"dw_kappa_{g}"] - hel[f"dw_kappa_{g}"]
        out[f"g2_0.{g}"] = {"harmonic_separation": sep0, "fast_fibre_shift_zeta": dsep, "shift_per_unit_kappa": dk,
                            "separation_with_fibre_zeta": sep0 + dsep,
                            "kappa_that_would_close_the_gap": (-(sep0 + dsep) / dk) if dk else None}
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--port", type=int, default=3147)
    ap.add_argument("--t1", required=True)
    ap.add_argument("--line", required=True)
    ap.add_argument("--background", required=True)
    a = ap.parse_args()
    db = lg.Gigi(a.port)
    t0 = time.time()
    rd = lambda f: json.load(gzip.open(f, "rt") if f.endswith(".gz") else open(f))
    t1 = rd(a.t1)
    line = rd(a.line)
    rep = {"sectors": demo_sectors(db, t1["results"]["points"]),
           "line": demo_line(db, line["results"]["points"], line["params"]["t"]),
           "background": demo_background(db, json.load(open(a.background)))}
    rep["gigi_point_queries"] = db.queries
    rep["seconds"] = time.time() - t0
    open(a.out, "w").write(json.dumps(rep, indent=1, default=float) + "\n")
    print(json.dumps({k: rep["sectors"][k] for k in ("A1_pass", "A2_pass", "A1_only_squares_below_cut")}))
    print(json.dumps({k: rep["line"][k] for k in ("B1_running_coupling_max_dev", "B2_flat_point")}))
    print(json.dumps({k: v for k, v in rep["background"].items() if k.startswith("g2")}, indent=1))


if __name__ == "__main__":
    main()
