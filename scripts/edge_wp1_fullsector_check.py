"""WP1 independent check of the large-j undercut with the general (non-block) builder (21 November 2026).

For points (t, y, x) with 0 < x < x*_12 (x*_12 = the j <= 12 edge) the near-top analysis
(scripts/edge_wp1_top_family.py) says some sector j > 12 has an H + GG^* eigenvalue below the twist's lambda_j.
Here that is checked with the general builder ricci_su3_test2_family.hessian_and_gauge (full sector matrix, no charge
blocks): mu_min(H + G G^*) - lambda_j per sector, with the twist's own eigenvalue located in the spectrum (distance
of the nearest eigenvalue to lambda_j). Also the gauge-orthogonal (slice) spectrum minimum minus lambda_j, with the
twist removed (as in EE.phi_S). Double precision (COMPUTED).
usage: OMP_NUM_THREADS=1 python scripts/edge_wp1_fullsector_check.py OUT.json t:y:x [t:y:x ...]
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ricci_su2_exact_edge as EE              # noqa: E402
T2 = EE.T2

JS = [8, 12, 20, 40, 60, 80]


def main():
    out_path = Path(sys.argv[1]); pts = [tuple(float(u) for u in a.split(":")) for a in sys.argv[2:]]
    S, C2, eps = T2.su2_data(); res = {"status": "COMPUTED (double precision), general builder", "points": []}
    for tv, yv, xv in pts:
        M = [xv * np.eye(3)[0], yv * np.eye(3)[1], yv * np.eye(3)[2]]
        rec = {"t": tv, "y": yv, "x": xv, "sectors": {}}
        for j in JS:
            H, G = T2.hessian_and_gauge(C2, [tv, 1.0, 1.0], eps, M, S.spin_rep(j))
            ev = np.linalg.eigvalsh(H + G @ G.conj().T); lam = EE.lam_tw(xv, yv, tv, j)
            k = int(np.argmin(np.abs(ev - lam)))
            mu = list(T2.slice_spectrum(H, G))
            for _ in range(2):
                kk = int(np.argmin(np.abs(np.array(mu) - lam)))
                if abs(mu[kk] - lam) < 1e-8 * max(1, abs(lam)):
                    mu.pop(kk)
            rec["sectors"][j] = {"lambda_twist": lam, "nearest_eig_minus_lambda": float(ev[k] - lam),
                                 "min_eig_HGG_minus_lambda": float(ev[0] - lam),
                                 "min_slice_without_twist_minus_lambda": float(min(mu) - lam)}
            print(tv, yv, xv, j, rec["sectors"][j], flush=True)
        res["points"].append(rec)
    out_path.write_text(json.dumps(res, indent=1) + "\n")


if __name__ == "__main__":
    main()
