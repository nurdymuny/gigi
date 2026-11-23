"""Which modes of the PG operator are unstable? Read-only, writes a receipt.

For levels 0 and 1: every generalized eigenpair of the PG operator against the
native mass; each physical mode's roughness is its Whitney Rayleigh quotient
(the smooth target mode is 4). Also records the largest Whitney eigenvalue, the
squared gain of curl over Whitney variations, at each level.
"""
import json
import sys
from pathlib import Path

import numpy as np
from scipy.linalg import eig, eigh

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_pg_curvature as pg   # noqa: E402


def main():
    out = {"kind": "read-only diagnostic of the S3 failure"}
    for level in (0, 1):
        L = pg.load_level(level, 6)
        K = pg.build(level, 6, "reconstruction", data=L)["operator"].toarray()
        M, Kw = L["mass"].toarray(), L["stiffness"].toarray()
        lam, V = eig(K, M)
        phys = abs(lam) >= 1
        neg, posv = phys & (lam.real < 0), phys & (lam.real > 0)
        rq = lambda v: float(np.real(np.conj(v) @ Kw @ v) / np.real(np.conj(v) @ M @ v))  # noqa: E731
        ru = np.array([rq(V[:, i]) for i in np.flatnonzero(neg)])
        rs = np.array([rq(V[:, i]) for i in np.flatnonzero(posv)])
        out[f"level{level}"] = {
            "physical_modes": int(phys.sum()), "negative_real_part": int(neg.sum()),
            "most_negative_real_part": float(lam[phys].real.min()),
            "unstable_roughness_min_median": [float(ru.min()), float(np.median(ru))],
            "stable_roughness_min_median": [float(rs.min()), float(np.median(rs))],
            "max_whitney_eigenvalue": float(eigh(Kw, M, eigvals_only=True).max()),
        }
        print(level, json.dumps(out[f"level{level}"]))
    r = out["level1"]["max_whitney_eigenvalue"] / out["level0"]["max_whitney_eigenvalue"]
    out["max_whitney_eigenvalue_ratio_n2_over_n1"] = r
    out["note"] = "h halves from n=1 to n=2; one-over-h-squared scaling predicts ratio 4"
    out["source_sha256"] = {"scripts/ricci_pg_curvature.py": pg.sha(pg.__file__),
                            "scripts/ricci_pg_s3_modes.py": pg.sha(__file__)}
    path = pg.OUT / "s3_modes.json"
    if path.exists():
        raise SystemExit(f"refusing to overwrite {path}")
    path.write_text(json.dumps(out, indent=2) + "\n")
    print(f"ratio {r:.3f}; wrote {path.relative_to(pg.ROOT)}")


if __name__ == "__main__":
    main()
