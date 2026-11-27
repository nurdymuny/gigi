"""Recompute every number in The Lowest Note's statement "How large the kinetic term must be to reverse the order"
(sections/s6-fast.tex, label b:threshold) from committed receipts. No solver is run.

Inputs:
  - the harmonic charge-0 block of the reduced model (closed form, H8 note Step 2):
      Q_s = s(t+2) x^2 - 2 sqrt2 s (sqrt t + 2(1-s)/sqrt t) x y + 2 s^2 y^2,  omega_H(s)^2 = 2 s (1 + 2/t - s)
  - the exact O(g^2) reduced-model coefficients at t = 3, s = 1.15: c_H = -0.530, c_C = -2.738
    (docs/research/helike-flux-background-registration-2026-11-24.md), cross-checked against the converged gaps
    (research/dressed-chart-20261126/controls/report.json, bare chart, Mq 10);
  - the fast potential dressing (research/m3-zero-point-20261127/background_deep.json).
usage: python scripts/lowest_note_threshold.py OUT.json"""
import json
import math
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
T = 3.0
C_H, C_C = -0.530, -2.738


def harmonic(s, t=T):
    a = s * (t + 2)
    b = -math.sqrt(2) * s * (math.sqrt(t) + 2 * (1 - s) / math.sqrt(t))
    c = 2 * s * s
    w_c = math.sqrt(np.linalg.eigvalsh(np.array([[a, b], [b, c]]))[0])
    w_h = math.sqrt(2 * s * (1 + 2 / t - s))
    return w_c, w_h


def main(out):
    w_c, w_h = harmonic(1.15)
    sep0 = w_c - w_h
    s0 = (T + 2) * (math.sqrt(T + 8) - math.sqrt(T)) / (4 * math.sqrt(T))
    h = 1e-6
    slope = ((harmonic(s0 + h)[0] - harmonic(s0 + h)[1]) - (harmonic(s0 - h)[0] - harmonic(s0 - h)[1])) / (2 * h)
    # converged gaps (cross-check of the anharmonic coefficient)
    rep = json.loads((ROOT / "research/dressed-chart-20261126/controls/report.json").read_text())
    conv = {p["g2"]: p["bare"]["competitor"]["gap"] - p["bare"]["helike"]["gap"] for p in rep["points"] if p["Mq"] == 10}
    anh = C_C - C_H
    # fast potential dressing per unit g^2 (competitor minus Helike)
    bg = json.loads((ROOT / "research/m3-zero-point-20261127/background_deep.json").read_text())
    m = {x["name"]: x for x in bg["modes"]}
    fast = (m["competitor"]["per_g2"]["0.002"]["d_omega_zeta"] - m["helike"]["per_g2"]["0.002"]["d_omega_zeta"]) / 0.002
    known = anh + fast                                   # O(g^2) coefficient without the kinetic term
    rec = {
        "harmonic_at_s_1.15": {"omega_C": w_c, "omega_H": w_h, "separation": sep0},
        "s0_star": s0, "d_sep_ds_at_s0": slope,
        "anharmonic_coefficient_registered": anh,
        "anharmonic_coefficient_from_converged_gaps": {str(g): (d - sep0) / g for g, d in conv.items()},
        "fast_potential_coefficient": fast,
        "known_coefficient_without_kinetic": known,
        "kappa_threshold": {str(g): sep0 / g + known for g in (0.002, 0.004)},
        "dressed_switch_slope_kappa0": -known / slope,
        "reduced_switch_slope": -anh / slope,
        "registered_s_star": {"0.03": 1.1674, "0.05": 1.1809, "0.07": 1.1931},
        "reduced_first_order_s_star": {str(g): s0 - anh / slope * g for g in (0.03, 0.05, 0.07)},
    }
    Path(out).parent.mkdir(parents=True, exist_ok=True)
    Path(out).write_text(json.dumps(rec, indent=1) + "\n")
    print(json.dumps(rec, indent=1))


if __name__ == "__main__":
    main(sys.argv[1])
