"""External review R6: the SU(2) two-ring force with the face read by its arc (log U_f) instead of the defect
(vector part of U_f, what the committed force uses). Same norm as the paper's error (relative full-mass error
against the continuum target), plus the relative difference of the two gradients.
usage: python scripts/ricci_su2_force_log_vs_defect.py OUT.json [LEVELS]
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ricci_su2_pg_force as F       # noqa: E402
import ricci_su2_recovery as su      # noqa: E402

_orig = su.recover_cell


def with_flux(flux):
    def rc(*a, **k):
        k["flux"] = flux
        return _orig(*a, **k)
    return rc


def main():
    outp = Path(sys.argv[1]); levels = [int(x) for x in (sys.argv[2] if len(sys.argv) > 2 else "0,1,2").split(",")]
    res = {}
    for lv in levels:
        t0 = time.time(); r = {}; g = {}
        for flux in ("defect", "log"):
            su.recover_cell = with_flux(flux)
            out = F.run(lv, 6, "pg")
            g[flux] = out.pop("_grad"); r[flux] = {"relative_error": out["relative_error"]}
        su.recover_cell = _orig
        r["gradient_difference_rel"] = float(np.linalg.norm(g["log"] - g["defect"]) / np.linalg.norm(g["defect"]))
        r["seconds"] = round(time.time() - t0, 1)
        res[f"n={lv + 1}"] = r
        print(f"n={lv + 1}", json.dumps(r), flush=True)
        outp.write_text(json.dumps(res, indent=1) + "\n")


if __name__ == "__main__":
    main()
