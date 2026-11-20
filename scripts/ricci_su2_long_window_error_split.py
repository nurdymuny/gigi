"""S3c follow-up: are the native and two-ring trajectories wrong in the same direction at t = 0.1?

Reads the saved final states of S3c (research/su2-long-window-20261119/L{l}_{trial}_N{N}.final.npy) and the
exact links at t = 0.1. For each run the error vector is d = log(L^-1 U) in right-trivialised link coordinates,
exactly as S3's E = sqrt(d^T B d / vol) with B the native mass at the exact links. Reports:
  - E for each run (C-E: equals the receipt's E_final);
  - the B-cosine between the native and two-ring error vectors, and |d_pg - d_native|_B / |d_native|_B;
  - the fraction of each error lying in the 9 discrete left-invariant directions at g(0.1)
    (perturbations of the exact left-invariant connection, as in ricci_su2_sector_eigvec_check.py).
Research script (diagnostic, no prediction attached).
usage: python scripts/ricci_su2_long_window_error_split.py [OUT.json]
"""
import json
import sys
from pathlib import Path

import numpy as np
from scipy.linalg import expm, logm

HERE = Path(__file__).resolve().parent
REPO = HERE.parent
sys.path.insert(0, str(HERE)); sys.path.insert(0, str(REPO / "e2e" / "probes"))
import ricci_su2_pg_flow as FL                                  # noqa: E402
from ricci_su2_pg_force import compose, inverse                  # noqa: E402
from ricci_lattice_demo import matrix, SIGMA                     # noqa: E402
from ricci_wilson_oracle import connection_matrices, axial       # noqa: E402

D = REPO / "research/su2-long-window-20261119"
T = 0.1


def sector_basis(lat, g):
    x = np.diag(axial(connection_matrices(np.asarray(g, float))))
    n = lat.n; L9 = np.zeros((3 * n, 9)); eps = 1e-6
    lk = lambda c: expm(-0.5j * np.einsum("a,ajk->jk", c, SIGMA))   # noqa: E731
    for e, (p, q) in enumerate(lat.edge_points):
        p, q = np.asarray(p, float), np.asarray(q, float)
        lg = logm(matrix(np.r_[p[0], -p[1:]]).conj().T @ matrix(np.r_[q[0], -q[1:]]))
        xi = np.array([(0.5j * np.trace(s @ lg)).real for s in SIGMA])
        c0 = x * xi; U0i = lk(c0).conj().T
        for i in range(3):
            for a in range(3):
                d = np.zeros(3); d[a] = xi[i]
                tp = FL.qlog(FL.to_quat(U0i @ lk(c0 + eps * d))); tm = FL.qlog(FL.to_quat(U0i @ lk(c0 - eps * d)))
                L9[3 * e:3 * e + 3, 3 * i + a] = (tp - tm) / (2 * eps)
    return L9


def main():
    out = {"t": T, "kind": "diagnostic (research script, not an engine run)"}
    met = FL.Metric(t_end=T)
    g = met(T)
    for lv in (0, 1):
        lat = FL.Lattice(lv, "native")
        L = lat.exact_links(g)
        mp = lat.metric_pieces(g)
        B = lat.mass(L, mp).toarray(); vol = mp["vol"]
        L9 = sector_basis(lat, g)
        G9 = L9.T @ B @ L9
        P = L9 @ np.linalg.solve(G9, L9.T @ B)                      # B-orthogonal projector onto the sector
        nb = lambda v: float(np.sqrt(max(v @ B @ v, 0.0)))           # noqa: E731
        res = {}
        vecs = {}
        for t in ("native", "pg"):
            for N in (40, 80):
                f = D / f"L{lv}_{t}_N{N}.final.npy"
                U = np.load(f)
                d = FL.qlog(compose(inverse(L), U)).ravel()
                rec = json.loads((D / f"L{lv}_{t}_N{N}.json").read_text())
                E = nb(d) / np.sqrt(vol)
                res[f"{t}_N{N}"] = {"E": E, "E_receipt": rec["E_final"], "C_E_rel": abs(E - rec["E_final"]) / rec["E_final"],
                                    "sector_fraction": nb(P @ d) ** 2 / nb(d) ** 2}
                vecs[(t, N)] = d
        for N in (40, 80):
            dn, dp = vecs[("native", N)], vecs[("pg", N)]
            res[f"pair_N{N}"] = {"cosine_B": float(dn @ B @ dp) / (nb(dn) * nb(dp)),
                                 "difference_over_native": nb(dp - dn) / nb(dn),
                                 "sector_part_cosine_B": float((P @ dn) @ B @ (P @ dp)) / max(nb(P @ dn) * nb(P @ dp), 1e-300),
                                 "off_sector_difference_over_native_off_sector":
                                     nb((dp - P @ dp) - (dn - P @ dn)) / max(nb(dn - P @ dn), 1e-300)}
        out[f"n{lv + 1}"] = res
        print(f"n = {lv + 1}:", json.dumps(res, indent=None), flush=True)
    if len(sys.argv) > 1:
        Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + "\n")


if __name__ == "__main__":
    main()
