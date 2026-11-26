"""Re-run the section-13 VSCF homotopy with the RAY-AWARE FOLDED base: per-block reductions (a, b) in place of the
uniform 2 C3 sqrt(Lu^2 + Lv^2), with the R1 enclosed v_J and face-wise beta. Floating point; not a bound.
The fold must satisfy D = V - V_bg - W0 >= 0 on the product tube, W0 = 1/2 (w_s - a)|u|^2 + 1/2 (w_t - b)|v|^2 + vJ;
along each ray z = t y that is q(y) + t P3(y) + t^2 P4(y) >= 0 for t <= min(Lu/|u|, Lv/|v|), q = 1/2 y.(K - K0)y.
usage: python scripts/vscf_folded_base.py --g2 0.004 --a 0.403 --b 0.193 --vj X --beta-u X --beta-v X --cache DIR --out PREFIX
"""
import argparse
import sys

import dryrun_helike_sc_lehmann_homotopy as dr
import estimate_helike_sc_gbase as gb
import vscf_helike_sc_lehmann as vs

ap = argparse.ArgumentParser()
for k in ("g2", "cache", "out"):
    ap.add_argument(f"--{k}", required=True)
for k in ("a", "b", "vj", "beta-u", "beta-v"):
    ap.add_argument(f"--{k}", type=float, required=True)
ap.add_argument("--kmax", type=int, default=dr.KMAX)
a = ap.parse_args()

dr.KMAX = a.kmax
vs.FOLD = (a.a, a.b)
gb.curvature = lambda m, L, rng, n=0: (a.vj, a.vj, a.beta_u)
_Block = dr.Block


class FaceBlock(_Block):
    def __init__(self, w2r, L, beta, shift, ecut, K=dr.K_RAD):
        super().__init__(w2r, L, a.beta_u if L == dr.LU else a.beta_v, shift, ecut, K)


dr.Block = FaceBlock
sys.argv = [sys.argv[0], "--controls", a.g2, "--cache", a.cache, "--out", a.out]
vs.main()
