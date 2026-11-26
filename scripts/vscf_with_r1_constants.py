"""Re-run the section-13 floating-point homotopy with the R1 ENCLOSED constants substituted for the sampled ones:
C3 (upper bound), v_J (lower bound) and the face-wise Robin coefficients beta_u, beta_v (upper bounds), each block's
base using its own face's beta. Floating point; not a bound. This replaces the R1.2 tightness gate
(R1 registration, amendment R1.7): C3 enters only the base, so the test that matters is whether the homotopy still
passes with the enclosed constants.

usage: python scripts/vscf_with_r1_constants.py --g2 0.004 --c3 X --vj X --beta-u X --beta-v X --cache DIR --out PREFIX
"""
import argparse
import sys

import dryrun_helike_sc_lehmann_homotopy as dr
import estimate_helike_sc_gbase as gb
import vscf_helike_sc_lehmann as vs

ap = argparse.ArgumentParser()
ap.add_argument("--g2", required=True)
ap.add_argument("--c3", type=float, required=True)
ap.add_argument("--vj", type=float, required=True)
ap.add_argument("--beta-u", type=float, required=True)
ap.add_argument("--beta-v", type=float, required=True)
ap.add_argument("--cache", required=True)
ap.add_argument("--out", required=True)
ap.add_argument("--kmax", type=int, default=dr.KMAX)
a = ap.parse_args()

dr.KMAX = a.kmax                                                   # Ritz levels tracked by the planner
dr.cubic_constant_poly = lambda P, rng, starts=0: a.c3              # C3 = max(sampled, this) = the enclosure
gb.curvature = lambda m, L, rng, n=0: (a.vj, a.vj, a.beta_u)
_Block = dr.Block


class FaceBlock(_Block):
    def __init__(self, w2r, L, beta, shift, ecut, K=dr.K_RAD):
        super().__init__(w2r, L, a.beta_u if L == dr.LU else a.beta_v, shift, ecut, K)


dr.Block = FaceBlock
sys.argv = [sys.argv[0], "--controls", a.g2, "--cache", a.cache, "--out", a.out]
vs.main()
