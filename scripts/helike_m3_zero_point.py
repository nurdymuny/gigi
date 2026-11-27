"""M3 as Born-Oppenheimer (docs/research/helike-bundle-framing-math-2026-11-27.md §14): the fast modes' zero-point
energy E0(q) = 1/2 sum_n omega_n(q) over the gauge-slice fluctuation spectrum of the spatial Yang-Mills Hessian at a
left-invariant field q on the Berger sphere (metric (t, 1, 1), unit S^3 at t = 1), from `helike zero-point`.

Regulator: a smooth weight over the WHOLE spectrum, E0(eps) = 1/2 sum mult * omega * exp(-eps omega). A sharp cutoff
in the sector index j is not gauge invariant (a gauge transformation shifts sectors j -> j + 2 at the flat point), so it
is not used. Expansion (zeta poles of a 3-D Laplace-type operator):
    E0(eps) = A eps^-4 + B eps^-2 + C log eps + D + E1 eps + eps^2 (F log eps + G) + ...
A and B are local and field-independent (volume, scalar curvature: the trace of the F-term over the adjoint vanishes);
C is the a_2 heat coefficient and carries int |F|^2, i.e. the running coupling.

Registered checks (written before the run):
  K1  lam = 0: the slice spectrum is the transverse-field spectrum of the empty Berger sphere; at t = 1 it is (k+1)^2.
  K2  flat points: E0_all(lam = 1) = E0_all(lam = 0) at every eps (gauge equivalence, any t), to 1e-9 relative.
  K3  t = 1: E0_all(lam) = E0_all(1 - lam) at every eps (gauge transformation composed with g -> g^-1), to 1e-9 rel.
  K4  dA = dB = 0 in the free fit of E0(lam) - E0(0) (field-independent divergences), |dA|, |dB| <= 1e-6 |C|-scale.
  K5  dC(lam) = (b0 / 8 pi^2) Vol density(lam) with b0 = 22/3 (SU(2)), Vol = 2 pi^2 sqrt t: the log divergence is the
      one-loop running of 1/g^2 (asymptotic freedom), shape AND value. Exact on-shell (lam = 1/2, the sphaleron);
      off-shell the background-gauge slice is used and the value is reported, not assumed.
  K6  truncation: the jmax and jmax - 20 sums agree to 1e-10 relative at the smallest eps used.
usage: python scripts/helike_m3_zero_point.py OUT.json [--t 3] [--jmax 120] [--lams ...]"""
import argparse
import json
import math
import subprocess
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "helike-lab/target/release/helike"
B0 = 22.0 / 3.0
EPS = np.linspace(0.40, 1.20, 33)


def spectra(t, jmax, lams=(), fields=None, work=None):
    out = Path(work) / f"zp_t{t}_j{jmax}_{abs(hash((tuple(lams), str(fields)))) % 10**8}.json"
    cmd = [str(BIN), "--out", str(out), "zero-point", "--t", repr(t), "--jmax", str(jmax), "--lams", ",".join(map(repr, lams))]
    if fields is not None:
        fp = out.with_suffix(".fields.json"); fp.write_text(json.dumps(fields)); cmd += ["--fields", str(fp)]
    subprocess.run(cmd, check=True, capture_output=True, text=True)
    return json.loads(out.read_text())["results"]["points"]


def regulated(point, eps, jmin=0, jcap=None):
    """1/2 sum_{j in [jmin, jcap]} (j+1) sum_slice omega exp(-eps omega), with negative modes counted separately."""
    tot = np.zeros_like(eps, dtype=float); neg = []
    for s in point["sectors"]:
        if s["j"] < jmin or (jcap is not None and s["j"] > jcap):
            continue
        mu = np.array(s["slice"])
        neg += [(s["j"], float(m)) for m in mu if m < -1e-10]
        w = np.sqrt(mu[mu > 1e-10])
        tot += 0.5 * s["mult"] * (w[None, :] * np.exp(-np.outer(eps, w))).sum(axis=1)
    return tot, neg


BASIS = ["eps^-4", "eps^-2", "log eps", "1", "eps", "eps^2 log eps", "eps^2"]


def design(eps, with_power=True):
    cols = [eps ** -4, eps ** -2] if with_power else []
    cols += [np.log(eps), np.ones_like(eps), eps, eps ** 2 * np.log(eps), eps ** 2]
    return np.stack(cols, axis=1)


def fit(eps, y, with_power=True):
    X = design(eps, with_power)
    c, *_ = np.linalg.lstsq(X, y, rcond=None)
    names = BASIS if with_power else BASIS[2:]
    return dict(zip(names, map(float, c))), float(np.max(np.abs(X @ c - y)))


def line_fields(lams, t):
    return [[[l * 2 / math.sqrt(t), 0, 0], [0, 2 * l, 0], [0, 0, 2 * l]] for l in lams]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--t", type=float, default=3.0)
    ap.add_argument("--jmax", type=int, default=120)
    ap.add_argument("--lams", default="0,0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,1.1,1.15,1.2")
    ap.add_argument("--work", default=None)
    a = ap.parse_args()
    work = Path(a.work or Path(a.out).parent); work.mkdir(parents=True, exist_ok=True)
    lams = [float(v) for v in a.lams.split(",")]
    pts = spectra(a.t, a.jmax, lams, work=work)
    vol = 2 * math.pi ** 2 * math.sqrt(a.t)
    E = {p["lam"]: regulated(p, EPS) for p in pts}
    Ecut = {p["lam"]: regulated(p, EPS, jcap=a.jmax - 20)[0] for p in pts}
    Eslow = {p["lam"]: regulated(p, EPS, jcap=0)[0] for p in pts}
    dens = {p["lam"]: p["energy_density"] for p in pts}
    rep = {"t": a.t, "jmax": a.jmax, "eps": EPS.tolist(), "vol": vol, "b0": B0, "points": []}
    e0 = E[0.0][0]
    for lam in lams:
        y, neg = E[lam]
        d = y - e0
        free, res_free = fit(EPS, d, True)
        fixed, res_fixed = fit(EPS, d, False)
        pred_C = B0 / (8 * math.pi ** 2) * vol * dens[lam]
        rep["points"].append({
            "lam": lam, "energy_density": dens[lam], "S_classical": vol * dens[lam],
            "negative_modes": neg,
            "trunc_rel": float(np.max(np.abs(y - Ecut[lam]) / np.abs(y))),
            "fit_free": free, "fit_free_resid": res_free, "fit_fixed": fixed, "fit_fixed_resid": res_fixed,
            "dC": fixed["log eps"], "dC_pred_b0": pred_C, "dD": fixed["1"],
            "dD_fast": fit(EPS, d - (Eslow[lam] - Eslow[0.0]), False)[0]["1"],
            "slow_sector_E0_eps_to_0": float(np.polyval(np.polyfit(EPS[:8], Eslow[lam][:8], 3), 0.0)),
        })
        r = rep["points"][-1]
        print(json.dumps({k: r[k] for k in ("lam", "energy_density", "dC", "dC_pred_b0", "dD", "dD_fast", "trunc_rel", "fit_fixed_resid")}
                         | {"dA": free["eps^-4"], "dB": free["eps^-2"], "neg_fast": [m for m in neg if m[0] >= 1][:4]}), flush=True)
    by = {p["lam"]: p for p in rep["points"]}
    rep["K2_flat_points_rel"] = float(np.max(np.abs(E[1.0][0] - e0) / np.abs(e0))) if 1.0 in E else None
    if abs(a.t - 1) < 1e-12:
        rep["K3_reflection_rel"] = {str(l): float(np.max(np.abs(E[l][0] - E[round(1 - l, 10)][0]) / np.abs(e0)))
                                    for l in lams if round(1 - l, 10) in E and l < 0.5}
    rep["K6_truncation_max_rel"] = max(p["trunc_rel"] for p in rep["points"])
    rep["K5_ratio_dC_over_pred"] = {str(l): (by[l]["dC"] / by[l]["dC_pred_b0"] if by[l]["dC_pred_b0"] else None) for l in lams}
    Path(a.out).write_text(json.dumps(rep, indent=1) + "\n")
    print(json.dumps({k: rep[k] for k in rep if k.startswith("K")}, indent=1))


if __name__ == "__main__":
    main()
