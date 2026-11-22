"""Pre-registration S6: the current sweep (docs/research/current-sweep-preregistration-2026-11-22.md, commit 78414bc6,
pushed before this script was written). At y = 0, sweep x from the Berger value (2 - t)/sqrt t up to 0 and find, for
each family j <= 40, where she stops being the floor (H + GG^*, charge blocks). Writes current_sweep_receipt.json.
usage (repository root): python docs/research/stress-2026-11-22/current_sweep.py
"""
import json
import sys
from pathlib import Path

import numpy as np
from scipy.optimize import brentq

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[2] / "scripts"))
_a, sys.argv = sys.argv, [sys.argv[0]]
import ricci_su2_exact_edge as EE  # noqa: E402
sys.argv = _a

J = 40
TOL = 1e-8   # an exact tie with her (the flat field is degenerate) counts as her still holding the floor
BLK = EE.Blocks()


def margin(x, t, j):
    """lowest state of family j minus her price, at (x, y = 0); >= 0 means she is the floor of that family."""
    lam = (x + (j + 2) / np.sqrt(t)) ** 2 + 2 * np.sqrt(t) * x
    ev = BLK.sector_spectrum(x, 0.0, t, j)
    # she and her mirror twin (charge -(l+2), same family, same price) are both in the spectrum: remove both copies
    others = np.sort(ev)
    for _ in range(2):
        k = int(np.argmin(np.abs(others - lam)))
        assert abs(others[k] - lam) < 1e-8 * max(1.0, abs(lam)), "her value not found in the family spectrum"
        others = np.delete(others, k)
    return float(others.min() - lam)


def main():
    out = {}
    for t in (3.0, 3.55, 5.0, 7.0, 10.0):
        xb = (2 - t) / np.sqrt(t)
        xs = np.linspace(xb, 0.0, 401)
        fam = {}
        for j in range(J + 1):
            m = np.array([margin(x, t, j) for x in xs])
            changes = [i for i in range(len(xs) - 1) if (m[i] >= -TOL) != (m[i + 1] >= -TOL)]
            roots = [brentq(lambda x: margin(x, t, j) + TOL, xs[i], xs[i + 1], xtol=1e-10) for i in changes]
            fam[j] = dict(at_berger=float(m[0]), at_zero=float(m[-1]), roots=[float(r) for r in roots],
                          floor_at_berger=bool(m[0] >= -TOL), floor_at_zero=bool(m[-1] >= -TOL),
                          tied_at_zero=bool(abs(m[-1]) <= TOL))
        lost = sorted(j for j, f in fam.items() if not f["floor_at_zero"])
        single = all(len(f["roots"]) == (0 if f["floor_at_zero"] else 1) for f in fam.values())
        out[str(t)] = dict(x_berger=float(xb), families=fam, lost_at_zero=lost, all_floor_at_berger=all(
            f["floor_at_berger"] for f in fam.values()), single_threshold=single,
            thresholds={j: fam[j]["roots"] for j in lost})
        print(f"t={t}: x_B={xb:+.4f}  floor of all at Berger: {out[str(t)]['all_floor_at_berger']}  "
              f"single threshold each: {single}  lost at x=0: {lost}")
        for j in lost:
            r = fam[j]["roots"]
            print(f"    family {j:2d}: threshold x* = {r}  (fraction of the Berger current left: "
                  f"{[round(v / xb, 4) for v in r]})")
    (HERE / "current_sweep_receipt.json").write_text(json.dumps(out, indent=1))


if __name__ == "__main__":
    main()
