"""R2 feasibility (floating point, not a bound): how much of the VSCF trial states lies beyond candidate cut radii.

The blocks are built on large proxy balls (soft radius 9, stiff radius 6, beta = 0), standing in for R^3 x R^3, so
the trial states are free of the tube's boundary. For a state f = sum_ij C_ij phi_i(u) psi_j(v),
    ||1_{|u| >= r} f||^2 = sum_j C_.j^T T_u(r) C_.j,   T_u(r)_{ii'} = delta_{l l'} delta_{m m'} int_r^L R_i R_i' r^2 dr,
and likewise in v. Reports, per step (s, N), the worst tail norm over the N trial states, in each block, at each
candidate cut radius.
usage: python scripts/r2_cut_feasibility.py --g2 0.004 --fold 0.45,0.25 --vj X --steps 0.23:47,0.51:47,0.99:13
"""
import argparse

import numpy as np

import dryrun_helike_sc_lehmann_homotopy as dr
import estimate_helike_sc_gbase as gb
import vscf_helike_sc_lehmann as vs

ap = argparse.ArgumentParser()
ap.add_argument("--g2", type=float, required=True)
ap.add_argument("--fold", required=True)
ap.add_argument("--vj", type=float, required=True)
ap.add_argument("--steps", required=True)
a = ap.parse_args()

PROXY_U, PROXY_V = 9.0, 6.0
dr.LU, dr.LV = PROXY_U, PROXY_V
vs.FOLD = tuple(float(x) for x in a.fold.split(","))
gb.curvature = lambda m, L, rng, n=0: (a.vj, a.vj, 0.0)          # beta = 0 on the proxy balls
E0 = dict(gb.CONTROLS)[a.g2]

V = vs.VSCF(a.g2, E0, lambda s: print(s, flush=True))


def tail_gram(block, idx, r):
    """T(r) restricted to the block functions idx: radial tail integrals, diagonal in (l, m)."""
    rq, wq = block.rq, block.wq
    mask = (rq >= r).astype(float)
    n = len(idx); T = np.zeros((n, n))
    for p, i in enumerate(idx):
        ri, li, mi, _ = block.funcs[i]
        for q, j in enumerate(idx):
            rj, lj, mj, _ = block.funcs[j]
            if li == lj and mi == mj:
                T[p, q] = np.sum(wq * mask * rq * rq * block.rad[ri][2] * block.rad[rj][2])
    return T


ub_idx = list(V.ui); vb_idx = list(V.vi)
ublock, vblock = V.ublock if hasattr(V, "ublock") else None, V.vblock if hasattr(V, "vblock") else None
if ublock is None:
    raise SystemExit("VSCF object does not expose its blocks; add self.ublock / self.vblock")
for spec in a.steps.split(","):
    s, N = spec.split(":"); s, N = float(s), int(N)
    C = V.trial(s, N)                                     # (N, n_u, n_v)
    nrm = np.sqrt(np.einsum("nij,nij->n", C, C))
    print(f"step s = {s}, N = {N}: trial norms in [{nrm.min():.6f}, {nrm.max():.6f}]")
    for r in (3.0, 3.5, 4.0, 4.5):
        T = tail_gram(ublock, ub_idx, r)
        t = np.sqrt(np.maximum(np.einsum("nij,ik,nkj->n", C, T, C), 0)) / nrm
        print(f"   soft  |u| >= {r}: worst tail {t.max():.2e}  (state {t.argmax() + 1}); median {np.median(t):.1e}")
    for r in (1.4, 1.6, 1.8, 2.0, 2.3, 2.6):
        T = tail_gram(vblock, vb_idx, r)
        t = np.sqrt(np.maximum(np.einsum("nij,jk,nik->n", C, T, C), 0)) / nrm
        print(f"   stiff |v| >= {r}: worst tail {t.max():.2e}  (state {t.argmax() + 1}); median {np.median(t):.1e}")
